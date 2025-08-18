use anyhow::Result;
use log::*;
use rosc::{OscMessage, OscPacket, OscType};
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::communication::{CommunicationManager, SystemState};

pub struct NetworkManager {
    osc_port: u16,
    client_registry: Arc<Mutex<HashMap<SocketAddr, ClientInfo>>>,
    command_stats: Arc<Mutex<CommandStats>>,
}

#[derive(Debug, Clone)]
struct ClientInfo {
    last_seen: Instant,
    commands_received: u32,
    client_name: Option<String>,
}

#[derive(Debug, Default)]
struct CommandStats {
    total_commands: u64,
    commands_per_second: f32,
    average_response_time: Duration,
    last_command_time: Option<Instant>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self { 
            osc_port: 8000,
            client_registry: Arc::new(Mutex::new(HashMap::new())),
            command_stats: Arc::new(Mutex::new(CommandStats::default())),
        }
    }

    pub fn set_port(&mut self, port: u16) {
        self.osc_port = port;
    }

    pub fn get_client_count(&self) -> usize {
        self.client_registry.lock().unwrap().len()
    }

    pub fn get_command_stats(&self) -> CommandStats {
        self.command_stats.lock().unwrap().clone()
    }

    pub fn start_server(&self, communication_manager: Arc<CommunicationManager>) -> Result<()> {
        info!("Starting OSC server on port {}", self.osc_port);

        let socket = UdpSocket::bind(format!("0.0.0.0:{}", self.osc_port))?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        info!("OSC server listening on port {}", self.osc_port);

        // Start mDNS service advertisement
        self.start_mdns_advertisement()?;

        // Start client cleanup thread
        self.start_client_cleanup_thread();

        // Start statistics update thread
        self.start_stats_thread();

        // Start status broadcasting thread
        self.start_status_broadcast_thread(communication_manager.clone());

        let mut buffer = [0u8; 2048]; // Increased buffer size for larger OSC messages
        let mut consecutive_errors = 0;

        loop {
            match socket.recv_from(&mut buffer) {
                Ok((size, addr)) => {
                    consecutive_errors = 0; // Reset error counter on successful receive
                    let start_time = Instant::now();
                    
                    // Update client registry
                    self.update_client_info(addr);
                    
                    // Handle the message
                    match self.handle_osc_message(&buffer[..size], addr, &communication_manager, &socket) {
                        Ok(_) => {
                            let processing_time = start_time.elapsed();
                            self.update_command_stats(processing_time);
                            
                            if processing_time > Duration::from_millis(20) {
                                warn!("OSC message processing took {:?} (>20ms target)", processing_time);
                            }
                        }
                        Err(e) => {
                            error!("Error handling OSC message from {}: {:?}", addr, e);
                            // Send error response to client
                            let _ = self.send_error_response(&socket, addr, &format!("Error: {}", e));
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Timeout - continue loop (this is normal)
                    continue;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    error!("UDP receive error ({}): {:?}", consecutive_errors, e);
                    
                    if consecutive_errors > 10 {
                        error!("Too many consecutive UDP errors, attempting to restart socket");
                        // Attempt to recreate socket
                        match UdpSocket::bind(format!("0.0.0.0:{}", self.osc_port)) {
                            Ok(new_socket) => {
                                info!("Socket recreated successfully");
                                consecutive_errors = 0;
                                // Continue with new socket (this is a simplified approach)
                            }
                            Err(bind_err) => {
                                error!("Failed to recreate socket: {:?}", bind_err);
                                thread::sleep(Duration::from_secs(1));
                            }
                        }
                    } else {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }

    fn handle_osc_message(
        &self,
        data: &[u8],
        addr: SocketAddr,
        communication_manager: &CommunicationManager,
        socket: &UdpSocket,
    ) -> Result<()> {
        let packet = rosc::decoder::decode_udp(data)?;

        match packet {
            OscPacket::Message(msg) => {
                info!("Received OSC message from {}: {}", addr, msg.addr);
                let response = self.process_osc_command(&msg, communication_manager)?;
                if let Some(resp_msg) = response {
                    self.send_osc_response(socket, addr, resp_msg)?;
                }
            }
            OscPacket::Bundle(bundle) => {
                info!("Received OSC bundle from {} with {} messages", addr, bundle.content.len());
                let mut responses = Vec::new();
                
                for packet in bundle.content {
                    if let OscPacket::Message(msg) = packet {
                        if let Ok(Some(resp)) = self.process_osc_command(&msg, communication_manager) {
                            responses.push(resp);
                        }
                    }
                }
                
                // Send bundle response if we have responses
                if !responses.is_empty() {
                    self.send_osc_bundle_response(socket, addr, responses)?;
                }
            }
        }

        Ok(())
    }

    fn process_osc_command(
        &self,
        msg: &OscMessage,
        communication_manager: &CommunicationManager,
    ) -> Result<Option<OscMessage>> {
        let response = match msg.addr.as_str() {
            "/loopstation/track/play" => {
                if let Some(OscType::Int(track_id)) = msg.args.first() {
                    info!("OSC: Play track {}", track_id);
                    communication_manager.send_track_command(*track_id as u8, "play")?;
                    Some(OscMessage {
                        addr: "/loopstation/track/play/ack".to_string(),
                        args: vec![OscType::Int(*track_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid track ID for play command"))
                }
            }
            "/loopstation/track/stop" => {
                if let Some(OscType::Int(track_id)) = msg.args.first() {
                    info!("OSC: Stop track {}", track_id);
                    communication_manager.send_track_command(*track_id as u8, "stop")?;
                    Some(OscMessage {
                        addr: "/loopstation/track/stop/ack".to_string(),
                        args: vec![OscType::Int(*track_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid track ID for stop command"))
                }
            }
            "/loopstation/track/record" => {
                if let Some(OscType::Int(track_id)) = msg.args.first() {
                    info!("OSC: Record track {}", track_id);
                    communication_manager.send_track_command(*track_id as u8, "record")?;
                    Some(OscMessage {
                        addr: "/loopstation/track/record/ack".to_string(),
                        args: vec![OscType::Int(*track_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid track ID for record command"))
                }
            }
            "/loopstation/track/clear" => {
                if let Some(OscType::Int(track_id)) = msg.args.first() {
                    info!("OSC: Clear track {}", track_id);
                    communication_manager.send_track_command(*track_id as u8, "clear")?;
                    Some(OscMessage {
                        addr: "/loopstation/track/clear/ack".to_string(),
                        args: vec![OscType::Int(*track_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid track ID for clear command"))
                }
            }
            "/loopstation/track/mute" => {
                if let Some(OscType::Int(track_id)) = msg.args.first() {
                    info!("OSC: Mute track {}", track_id);
                    communication_manager.send_track_command(*track_id as u8, "mute")?;
                    Some(OscMessage {
                        addr: "/loopstation/track/mute/ack".to_string(),
                        args: vec![OscType::Int(*track_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid track ID for mute command"))
                }
            }
            "/loopstation/track/volume" => {
                if let (Some(OscType::Int(track_id)), Some(OscType::Float(volume))) = 
                    (msg.args.get(0), msg.args.get(1)) {
                    info!("OSC: Set track {} volume to {}", track_id, volume);
                    communication_manager.send_track_volume(*track_id as u8, *volume)?;
                    Some(OscMessage {
                        addr: "/loopstation/track/volume/ack".to_string(),
                        args: vec![OscType::Int(*track_id), OscType::Float(*volume)],
                    })
                } else {
                    Some(self.create_error_message("Invalid arguments for volume command"))
                }
            }
            "/loopstation/tempo" => {
                if let Some(OscType::Float(tempo)) = msg.args.first() {
                    info!("OSC: Set tempo to {}", tempo);
                    communication_manager.send_tempo_command(*tempo)?;
                    Some(OscMessage {
                        addr: "/loopstation/tempo/ack".to_string(),
                        args: vec![OscType::Float(*tempo)],
                    })
                } else {
                    Some(self.create_error_message("Invalid tempo value"))
                }
            }
            "/loopstation/fx/toggle" => {
                if let Some(OscType::Int(fx_id)) = msg.args.first() {
                    info!("OSC: Toggle FX {}", fx_id);
                    communication_manager.send_fx_command(*fx_id as u8, "toggle")?;
                    Some(OscMessage {
                        addr: "/loopstation/fx/toggle/ack".to_string(),
                        args: vec![OscType::Int(*fx_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid FX ID"))
                }
            }
            "/loopstation/memory/load" => {
                if let Some(OscType::Int(memory_id)) = msg.args.first() {
                    info!("OSC: Load memory {}", memory_id);
                    communication_manager.send_memory_command(*memory_id as u8, "load")?;
                    Some(OscMessage {
                        addr: "/loopstation/memory/load/ack".to_string(),
                        args: vec![OscType::Int(*memory_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid memory ID"))
                }
            }
            "/loopstation/memory/save" => {
                if let Some(OscType::Int(memory_id)) = msg.args.first() {
                    info!("OSC: Save memory {}", memory_id);
                    communication_manager.send_memory_command(*memory_id as u8, "save")?;
                    Some(OscMessage {
                        addr: "/loopstation/memory/save/ack".to_string(),
                        args: vec![OscType::Int(*memory_id)],
                    })
                } else {
                    Some(self.create_error_message("Invalid memory ID"))
                }
            }
            "/loopstation/status" => {
                info!("OSC: Status request");
                let state = communication_manager.get_system_state()?;
                Some(self.create_status_response(&state))
            }
            "/loopstation/ping" => {
                Some(OscMessage {
                    addr: "/loopstation/pong".to_string(),
                    args: vec![OscType::String("RC-505 MKII Clone".to_string())],
                })
            }
            "/loopstation/info" => {
                Some(OscMessage {
                    addr: "/loopstation/info/response".to_string(),
                    args: vec![
                        OscType::String("RC-505 MKII Clone".to_string()),
                        OscType::String("v1.0.0".to_string()),
                        OscType::Int(6), // Number of tracks
                        OscType::Int(self.osc_port as i32),
                    ],
                })
            }
            _ => {
                warn!("Unknown OSC command: {}", msg.addr);
                Some(self.create_error_message(&format!("Unknown command: {}", msg.addr)))
            }
        };

        Ok(response)
    }

    fn start_mdns_advertisement(&self) -> Result<()> {
        info!("Starting mDNS service advertisement");
        
        // Spawn mDNS advertisement in background thread
        let port = self.osc_port;
        thread::spawn(move || {
            if let Err(e) = Self::run_mdns_service(port) {
                error!("mDNS service error: {:?}", e);
            }
        });

        Ok(())
    }

    fn run_mdns_service(port: u16) -> Result<()> {
        use mdns_sd::{ServiceDaemon, ServiceInfo};

        let mdns = ServiceDaemon::new()?;
        
        let service_type = "_osc._udp.local.";
        let instance_name = "RC-505 MKII Clone";
        let host_name = "loopstation.local.";
        
        let properties = [("version", "1.0"), ("device", "RC-505-Clone")];
        
        let service_info = ServiceInfo::new(
            service_type,
            instance_name,
            host_name,
            "0.0.0.0", // Let mDNS determine the IP
            port,
            &properties[..],
        )?;

        mdns.register(service_info)?;
        info!("mDNS service registered: {} on port {}", instance_name, port);

        // Keep the service running
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }
}    // Helpe
r methods for OSC response creation
    fn create_error_message(&self, error: &str) -> OscMessage {
        OscMessage {
            addr: "/loopstation/error".to_string(),
            args: vec![OscType::String(error.to_string())],
        }
    }

    fn create_status_response(&self, state: &SystemState) -> OscMessage {
        let mut args = vec![
            OscType::Float(state.tempo),
            OscType::Int(state.current_memory as i32),
            OscType::Bool(state.network_connected),
        ];

        // Add track states
        for (i, track) in state.tracks.iter().enumerate() {
            args.push(OscType::Int(i as i32)); // Track ID
            args.push(OscType::String(format!("{:?}", track.state))); // Track state
            args.push(OscType::Float(track.volume)); // Track volume
            args.push(OscType::Float(track.pan)); // Track pan
            args.push(OscType::Bool(track.muted)); // Track muted
        }

        // Add FX states
        for (i, &fx_active) in state.fx_states.iter().enumerate() {
            args.push(OscType::Int(i as i32)); // FX ID
            args.push(OscType::Bool(fx_active)); // FX active
        }

        OscMessage {
            addr: "/loopstation/status/response".to_string(),
            args,
        }
    }

    fn send_osc_response(&self, socket: &UdpSocket, addr: SocketAddr, msg: OscMessage) -> Result<()> {
        let packet = OscPacket::Message(msg);
        let encoded = rosc::encoder::encode(&packet)?;
        socket.send_to(&encoded, addr)?;
        Ok(())
    }

    fn send_osc_bundle_response(&self, socket: &UdpSocket, addr: SocketAddr, messages: Vec<OscMessage>) -> Result<()> {
        let bundle = rosc::OscBundle {
            timetag: rosc::OscTime::now(),
            content: messages.into_iter().map(OscPacket::Message).collect(),
        };
        let packet = OscPacket::Bundle(bundle);
        let encoded = rosc::encoder::encode(&packet)?;
        socket.send_to(&encoded, addr)?;
        Ok(())
    }

    fn send_error_response(&self, socket: &UdpSocket, addr: SocketAddr, error: &str) -> Result<()> {
        let error_msg = self.create_error_message(error);
        self.send_osc_response(socket, addr, error_msg)
    }

    // Client management methods
    fn update_client_info(&self, addr: SocketAddr) {
        let mut registry = self.client_registry.lock().unwrap();
        let client = registry.entry(addr).or_insert(ClientInfo {
            last_seen: Instant::now(),
            commands_received: 0,
            client_name: None,
        });
        
        client.last_seen = Instant::now();
        client.commands_received += 1;
    }

    fn start_client_cleanup_thread(&self) {
        let registry = self.client_registry.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(30)); // Cleanup every 30 seconds
                
                let mut registry = registry.lock().unwrap();
                let now = Instant::now();
                
                // Remove clients that haven't been seen for 5 minutes
                registry.retain(|addr, client| {
                    let keep = now.duration_since(client.last_seen) < Duration::from_secs(300);
                    if !keep {
                        info!("Removing inactive client: {}", addr);
                    }
                    keep
                });
            }
        });
    }

    fn update_command_stats(&self, processing_time: Duration) {
        let mut stats = self.command_stats.lock().unwrap();
        stats.total_commands += 1;
        
        // Update average response time (simple moving average)
        if stats.average_response_time.is_zero() {
            stats.average_response_time = processing_time;
        } else {
            let current_avg_nanos = stats.average_response_time.as_nanos() as f64;
            let new_time_nanos = processing_time.as_nanos() as f64;
            let new_avg_nanos = (current_avg_nanos * 0.9) + (new_time_nanos * 0.1);
            stats.average_response_time = Duration::from_nanos(new_avg_nanos as u64);
        }
        
        // Update commands per second
        if let Some(last_time) = stats.last_command_time {
            let time_diff = Instant::now().duration_since(last_time).as_secs_f32();
            if time_diff > 0.0 {
                stats.commands_per_second = 1.0 / time_diff;
            }
        }
        
        stats.last_command_time = Some(Instant::now());
    }

    fn start_stats_thread(&self) {
        let stats = self.command_stats.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(60)); // Log stats every minute
                
                let stats_snapshot = stats.lock().unwrap().clone();
                info!(
                    "OSC Stats - Total commands: {}, Avg response time: {:?}, Commands/sec: {:.2}",
                    stats_snapshot.total_commands,
                    stats_snapshot.average_response_time,
                    stats_snapshot.commands_per_second
                );
            }
        });
    }

    // Network discovery and health methods
    pub fn broadcast_presence(&self) -> Result<()> {
        // Send a broadcast message to announce presence
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;
        
        let announce_msg = OscMessage {
            addr: "/loopstation/announce".to_string(),
            args: vec![
                OscType::String("RC-505 MKII Clone".to_string()),
                OscType::Int(self.osc_port as i32),
                OscType::String("v1.0.0".to_string()),
            ],
        };
        
        let packet = OscPacket::Message(announce_msg);
        let encoded = rosc::encoder::encode(&packet)?;
        
        // Broadcast to common OSC discovery ports
        let broadcast_ports = [8000, 9000, 10000];
        for port in broadcast_ports {
            let _ = socket.send_to(&encoded, format!("255.255.255.255:{}", port));
        }
        
        Ok(())
    }

    pub fn get_network_health(&self) -> NetworkHealth {
        let stats = self.command_stats.lock().unwrap();
        let client_count = self.client_registry.lock().unwrap().len();
        
        NetworkHealth {
            active_clients: client_count,
            total_commands: stats.total_commands,
            average_response_time: stats.average_response_time,
            commands_per_second: stats.commands_per_second,
            server_uptime: stats.last_command_time.map(|t| Instant::now().duration_since(t)),
        }
    }

    fn start_status_broadcast_thread(&self, communication_manager: Arc<CommunicationManager>) {
        let registry = self.client_registry.clone();
        let port = self.osc_port;
        
        thread::spawn(move || {
            let mut last_broadcast = Instant::now();
            let mut last_state: Option<SystemState> = None;
            
            loop {
                thread::sleep(Duration::from_millis(100)); // Check every 100ms
                
                // Get current system state
                if let Ok(current_state) = communication_manager.get_system_state() {
                    let should_broadcast = match &last_state {
                        None => true, // First time
                        Some(prev_state) => {
                            // Check if state has changed significantly
                            Self::state_changed_significantly(prev_state, &current_state) ||
                            // Or if it's been more than 5 seconds since last broadcast
                            last_broadcast.elapsed() > Duration::from_secs(5)
                        }
                    };
                    
                    if should_broadcast {
                        Self::broadcast_status_to_clients(&registry, &current_state, port);
                        last_broadcast = Instant::now();
                        last_state = Some(current_state);
                    }
                }
            }
        });
    }

    fn state_changed_significantly(prev: &SystemState, current: &SystemState) -> bool {
        // Check for significant changes that warrant a broadcast
        if prev.tempo != current.tempo || 
           prev.current_memory != current.current_memory ||
           prev.network_connected != current.network_connected {
            return true;
        }
        
        // Check track state changes
        for (prev_track, current_track) in prev.tracks.iter().zip(current.tracks.iter()) {
            if prev_track.state != current_track.state ||
               prev_track.muted != current_track.muted ||
               prev_track.selected != current_track.selected ||
               (prev_track.volume - current_track.volume).abs() > 0.01 {
                return true;
            }
        }
        
        // Check FX state changes
        for (prev_fx, current_fx) in prev.fx_states.iter().zip(current.fx_states.iter()) {
            if prev_fx != current_fx {
                return true;
            }
        }
        
        false
    }

    fn broadcast_status_to_clients(
        registry: &Arc<Mutex<HashMap<SocketAddr, ClientInfo>>>,
        state: &SystemState,
        port: u16,
    ) {
        let clients: Vec<SocketAddr> = {
            let registry = registry.lock().unwrap();
            registry.keys().cloned().collect()
        };
        
        if clients.is_empty() {
            return; // No clients to broadcast to
        }
        
        // Create status broadcast message
        let status_msg = OscMessage {
            addr: "/loopstation/status/broadcast".to_string(),
            args: Self::create_status_args(state),
        };
        
        // Send to all connected clients
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            let packet = OscPacket::Message(status_msg);
            if let Ok(encoded) = rosc::encoder::encode(&packet) {
                for client_addr in clients {
                    if let Err(e) = socket.send_to(&encoded, client_addr) {
                        warn!("Failed to broadcast status to {}: {:?}", client_addr, e);
                    }
                }
            }
        }
    }

    fn create_status_args(state: &SystemState) -> Vec<OscType> {
        let mut args = vec![
            OscType::Float(state.tempo),
            OscType::Int(state.current_memory as i32),
            OscType::Bool(state.network_connected),
            OscType::Long(state.timestamp as i64),
        ];

        // Add track states
        for track in &state.tracks {
            args.push(OscType::Int(track.id as i32));
            args.push(OscType::String(format!("{:?}", track.state)));
            args.push(OscType::Float(track.volume));
            args.push(OscType::Float(track.pan));
            args.push(OscType::Bool(track.muted));
            args.push(OscType::Bool(track.selected));
        }

        // Add FX states
        for (i, &fx_active) in state.fx_states.iter().enumerate() {
            args.push(OscType::Int(i as i32));
            args.push(OscType::Bool(fx_active));
        }

        args
    }
}

#[derive(Debug, Clone)]
pub struct NetworkHealth {
    pub active_clients: usize,
    pub total_commands: u64,
    pub average_response_time: Duration,
    pub commands_per_second: f32,
    pub server_uptime: Option<Duration>,
}