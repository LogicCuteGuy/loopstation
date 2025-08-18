use anyhow::Result;
use esp_idf_svc::hal::{
    gpio::{Gpio16, Gpio17},
    peripheral::Peripheral,
    uart::{UartConfig, UartDriver},
};
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub tracks: Vec<TrackState>,
    pub tempo: f32,
    pub network_connected: bool,
    pub current_memory: u8,
    pub fx_states: Vec<bool>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackState {
    pub id: u8,
    pub state: TrackStateEnum,
    pub volume: f32,
    pub pan: f32,
    pub muted: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackStateEnum {
    Stopped,
    Recording,
    Playing,
    Overdubbing,
    Muted,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    TrackPlay { track_id: u8 },
    TrackStop { track_id: u8 },
    TrackRecord { track_id: u8 },
    TrackClear { track_id: u8 },
    TrackMute { track_id: u8 },
    TrackVolume { track_id: u8, volume: f32 },
    SetTempo { tempo: f32 },
    FxToggle { fx_id: u8 },
    MemoryLoad { memory_id: u8 },
    MemorySave { memory_id: u8 },
    GetStatus,
    Heartbeat,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    SystemState(SystemState),
    ParameterChange { parameter: String, value: f32 },
    Ack { command_id: Option<u32> },
    Error { message: String, command_id: Option<u32> },
    Heartbeat,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: u32,
    pub timestamp: u64,
    pub payload: MessagePayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessagePayload {
    Command(Command),
    Response(Response),
}

pub struct CommunicationManager {
    uart: Arc<Mutex<UartDriver<'static>>>,
    last_state: Arc<Mutex<SystemState>>,
    message_id_counter: Arc<Mutex<u32>>,
    pending_commands: Arc<Mutex<std::collections::HashMap<u32, Instant>>>,
    error_count: Arc<Mutex<u32>>,
    last_heartbeat: Arc<Mutex<Instant>>,
    connection_status: Arc<Mutex<bool>>,
}

impl CommunicationManager {
    pub fn new(
        uart: impl Peripheral<P = esp_idf_svc::hal::uart::UART1> + 'static,
    ) -> Result<Self> {
        // Configure UART for STM32 communication (TX: GPIO17, RX: GPIO16)
        let config = UartConfig::new().baudrate(esp_idf_svc::hal::units::Hertz(115200));
        let uart = UartDriver::new(
            uart,
            esp_idf_svc::hal::gpio::Gpio17,  // TX
            esp_idf_svc::hal::gpio::Gpio16,  // RX
            Option::<esp_idf_svc::hal::gpio::Gpio0>::None,  // RTS
            Option::<esp_idf_svc::hal::gpio::Gpio0>::None,  // CTS
            &config,
        )?;

        let default_state = SystemState {
            tracks: (0..6)
                .map(|i| TrackState {
                    id: i,
                    state: TrackStateEnum::Stopped,
                    volume: 1.0,
                    pan: 0.0,
                    muted: false,
                    selected: false,
                })
                .collect(),
            tempo: 120.0,
            network_connected: false,
            current_memory: 1,
            fx_states: vec![false; 5],
            timestamp: 0,
        };

        Ok(Self {
            uart: Arc::new(Mutex::new(uart)),
            last_state: Arc::new(Mutex::new(default_state)),
            message_id_counter: Arc::new(Mutex::new(0)),
            pending_commands: Arc::new(Mutex::new(std::collections::HashMap::new())),
            error_count: Arc::new(Mutex::new(0)),
            last_heartbeat: Arc::new(Mutex::new(Instant::now())),
            connection_status: Arc::new(Mutex::new(false)),
        })
    }

    pub fn send_track_command(&self, track_id: u8, command: &str) -> Result<()> {
        let cmd = match command {
            "play" => Command::TrackPlay { track_id },
            "stop" => Command::TrackStop { track_id },
            "record" => Command::TrackRecord { track_id },
            "clear" => Command::TrackClear { track_id },
            "mute" => Command::TrackMute { track_id },
            _ => return Err(anyhow::anyhow!("Unknown track command: {}", command)),
        };

        self.send_command_with_retry(&cmd, 3)
    }

    pub fn send_track_volume(&self, track_id: u8, volume: f32) -> Result<()> {
        let cmd = Command::TrackVolume { track_id, volume };
        self.send_command_with_retry(&cmd, 3)
    }

    pub fn send_tempo_command(&self, tempo: f32) -> Result<()> {
        let cmd = Command::SetTempo { tempo };
        self.send_command_with_retry(&cmd, 3)
    }

    pub fn send_fx_command(&self, fx_id: u8, command: &str) -> Result<()> {
        let cmd = match command {
            "toggle" => Command::FxToggle { fx_id },
            _ => return Err(anyhow::anyhow!("Unknown FX command: {}", command)),
        };

        self.send_command_with_retry(&cmd, 3)
    }

    pub fn send_memory_command(&self, memory_id: u8, command: &str) -> Result<()> {
        let cmd = match command {
            "load" => Command::MemoryLoad { memory_id },
            "save" => Command::MemorySave { memory_id },
            _ => return Err(anyhow::anyhow!("Unknown memory command: {}", command)),
        };

        self.send_command_with_retry(&cmd, 3)
    }

    pub fn get_system_state(&self) -> Result<SystemState> {
        // Try to get fresh state from STM32
        if let Err(e) = self.send_command_with_retry(&Command::GetStatus, 2) {
            warn!("Failed to request fresh state from STM32: {:?}", e);
        }

        // Return last known state
        Ok(self.last_state.lock().unwrap().clone())
    }

    fn send_command_with_retry(&self, command: &Command, max_retries: u32) -> Result<()> {
        for attempt in 0..max_retries {
            match self.send_command(command) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt == max_retries - 1 {
                        error!("Command failed after {} attempts: {:?}", max_retries, e);
                        *self.error_count.lock().unwrap() += 1;
                        return Err(e);
                    } else {
                        warn!("Command attempt {} failed, retrying: {:?}", attempt + 1, e);
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }
        unreachable!()
    }

    fn send_command(&self, command: &Command) -> Result<()> {
        let start_time = Instant::now();
        
        // Generate unique message ID
        let message_id = {
            let mut counter = self.message_id_counter.lock().unwrap();
            *counter += 1;
            *counter
        };

        // Create message with ID and timestamp
        let message = Message {
            id: message_id,
            timestamp: start_time.duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64,
            payload: MessagePayload::Command(command.clone()),
        };

        let serialized = serde_json::to_string(&message)?;
        let message_str = format!("{}\n", serialized);

        {
            let mut uart = self.uart.lock().unwrap();
            uart.write(message_str.as_bytes())?;
        }

        debug!("Sent command to STM32 (ID: {}): {}", message_id, serialized);

        // Track pending command for timeout handling
        {
            let mut pending = self.pending_commands.lock().unwrap();
            pending.insert(message_id, start_time);
        }

        // Try to read response with timeout
        if let Ok(response) = self.read_response(Duration::from_millis(100)) {
            // Remove from pending commands
            {
                let mut pending = self.pending_commands.lock().unwrap();
                pending.remove(&message_id);
            }

            match response {
                Response::SystemState(state) => {
                    *self.last_state.lock().unwrap() = state;
                    *self.connection_status.lock().unwrap() = true;
                }
                Response::Ack { command_id } => {
                    debug!("Command acknowledged by STM32 (ID: {:?})", command_id);
                    *self.connection_status.lock().unwrap() = true;
                }
                Response::Error { message, command_id } => {
                    error!("STM32 reported error (ID: {:?}): {}", command_id, message);
                    return Err(anyhow::anyhow!("STM32 error: {}", message));
                }
                Response::ParameterChange { parameter, value } => {
                    debug!("Parameter change from STM32: {} = {}", parameter, value);
                    *self.connection_status.lock().unwrap() = true;
                }
                Response::Heartbeat => {
                    *self.last_heartbeat.lock().unwrap() = Instant::now();
                    *self.connection_status.lock().unwrap() = true;
                }
            }
        } else {
            // Timeout - mark as potential connection issue
            warn!("No response from STM32 for command ID: {}", message_id);
        }

        let processing_time = start_time.elapsed();
        if processing_time > Duration::from_millis(20) {
            warn!("Command processing took {:?} (>20ms target)", processing_time);
        }

        Ok(())
    }

    fn read_response(&self, timeout: Duration) -> Result<Response> {
        let start_time = Instant::now();
        let mut buffer = Vec::new();
        let mut temp_buf = [0u8; 1];

        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!("Response timeout"));
            }

            let uart = self.uart.lock().unwrap();
            match uart.read(&mut temp_buf, Duration::from_millis(10)) {
                Ok(1) => {
                    if temp_buf[0] == b'\n' {
                        // End of message
                        break;
                    }
                    buffer.push(temp_buf[0]);
                }
                Ok(0) => continue, // No data available
                Err(e) => {
                    debug!("UART read error (may be timeout): {:?}", e);
                    continue;
                }
            }
        }

        let message_str = String::from_utf8(buffer)?;
        
        // Try to parse as new message format first
        if let Ok(message) = serde_json::from_str::<Message>(&message_str) {
            match message.payload {
                MessagePayload::Response(response) => {
                    debug!("Received response from STM32 (ID: {}): {}", message.id, message_str);
                    return Ok(response);
                }
                MessagePayload::Command(_) => {
                    // This is a command from STM32, not a response
                    debug!("Received command from STM32 (ID: {}): {}", message.id, message_str);
                    return Err(anyhow::anyhow!("Received command when expecting response"));
                }
            }
        }
        
        // Fallback to old format for compatibility
        if let Ok(response) = serde_json::from_str::<Response>(&message_str) {
            debug!("Received legacy response from STM32: {}", message_str);
            return Ok(response);
        }
        
        Err(anyhow::anyhow!("Failed to parse response: {}", message_str))
    }

    pub fn start_status_polling(&self) {
        let uart_clone = self.uart.clone();
        let state_clone = self.last_state.clone();
        let heartbeat_clone = self.last_heartbeat.clone();
        let connection_clone = self.connection_status.clone();
        let pending_clone = self.pending_commands.clone();
        let error_count_clone = self.error_count.clone();
        
        std::thread::spawn(move || {
            let mut last_heartbeat_sent = Instant::now();
            
            loop {
                let now = Instant::now();
                
                // Send heartbeat every 5 seconds
                if now.duration_since(last_heartbeat_sent) > Duration::from_secs(5) {
                    if let Err(e) = Self::send_heartbeat(&uart_clone) {
                        warn!("Failed to send heartbeat: {:?}", e);
                        *error_count_clone.lock().unwrap() += 1;
                    }
                    last_heartbeat_sent = now;
                }
                
                // Check for connection timeout (no heartbeat response in 10 seconds)
                {
                    let last_hb = *heartbeat_clone.lock().unwrap();
                    let connection_ok = now.duration_since(last_hb) < Duration::from_secs(10);
                    *connection_clone.lock().unwrap() = connection_ok;
                    
                    if !connection_ok {
                        warn!("STM32 connection timeout - no heartbeat for {:?}", now.duration_since(last_hb));
                    }
                }
                
                // Clean up expired pending commands
                {
                    let mut pending = pending_clone.lock().unwrap();
                    pending.retain(|_, &mut timestamp| {
                        now.duration_since(timestamp) < Duration::from_secs(5)
                    });
                }
                
                // Poll STM32 for status updates every 100ms
                if let Ok(response) = Self::poll_status(&uart_clone) {
                    if let Response::SystemState(state) = response {
                        *state_clone.lock().unwrap() = state;
                        *connection_clone.lock().unwrap() = true;
                    }
                }
                
                std::thread::sleep(Duration::from_millis(100));
            }
        });
    }

    fn send_heartbeat(uart: &Arc<Mutex<UartDriver<'static>>>) -> Result<()> {
        let message = Message {
            id: 0, // Heartbeat messages use ID 0
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64,
            payload: MessagePayload::Command(Command::Heartbeat),
        };

        let serialized = serde_json::to_string(&message)?;
        let message_str = format!("{}\n", serialized);

        {
            let mut uart_guard = uart.lock().unwrap();
            uart_guard.write(message_str.as_bytes())?;
        }

        debug!("Sent heartbeat to STM32");
        Ok(())
    }

    fn poll_status(uart: &Arc<Mutex<UartDriver<'static>>>) -> Result<Response> {
        let message = Message {
            id: 0, // Status polls use ID 0
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            payload: MessagePayload::Command(Command::GetStatus),
        };

        let serialized = serde_json::to_string(&message)?;
        let message_str = format!("{}\n", serialized);

        {
            let mut uart_guard = uart.lock().unwrap();
            uart_guard.write(message_str.as_bytes())?;
        }

        // Read response
        let start_time = Instant::now();
        let mut buffer = Vec::new();
        let mut temp_buf = [0u8; 1];

        loop {
            if start_time.elapsed() > Duration::from_millis(50) {
                return Err(anyhow::anyhow!("Status poll timeout"));
            }

            let uart_guard = uart.lock().unwrap();
            match uart_guard.read(&mut temp_buf, Duration::from_millis(5)) {
                Ok(1) => {
                    if temp_buf[0] == b'\n' {
                        break;
                    }
                    buffer.push(temp_buf[0]);
                }
                Ok(0) => continue,
                Err(_) => continue,
            }
        }

        let message_str = String::from_utf8(buffer)?;
        
        // Try to parse as new message format first
        if let Ok(msg) = serde_json::from_str::<Message>(&message_str) {
            match msg.payload {
                MessagePayload::Response(response) => return Ok(response),
                _ => {}
            }
        }
        
        // Fallback to old format
        if let Ok(response) = serde_json::from_str::<Response>(&message_str) {
            return Ok(response);
        }
        
        Err(anyhow::anyhow!("Failed to parse status response"))
    }

    /// Get connection status and statistics
    pub fn get_connection_status(&self) -> (bool, u32, Duration) {
        let connected = *self.connection_status.lock().unwrap();
        let errors = *self.error_count.lock().unwrap();
        let last_hb = *self.last_heartbeat.lock().unwrap();
        let since_heartbeat = Instant::now().duration_since(last_hb);
        
        (connected, errors, since_heartbeat)
    }

    /// Reset error count
    pub fn reset_error_count(&self) {
        *self.error_count.lock().unwrap() = 0;
    }

    /// Check if STM32 is responding
    pub fn is_stm32_connected(&self) -> bool {
        *self.connection_status.lock().unwrap()
    }
}