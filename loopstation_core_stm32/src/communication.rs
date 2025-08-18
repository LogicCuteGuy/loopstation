//! STM32-ESP32 Communication Module
//!
//! This module handles UART communication between the STM32 core and ESP32 display module.
//! It implements a JSON-based message protocol with error recovery and heartbeat monitoring.

#![allow(unused)]

use crate::hal::{HalError, Esp32Message, TrackStatus, TrackStateComm, ParameterId, CommandType};
use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

/// Communication manager for STM32-ESP32 UART interface
pub struct Esp32CommunicationManager {
    /// Message buffer for incoming data
    pub rx_buffer: Vec<u8, 512>,
    /// Message buffer for outgoing data  
    pub tx_buffer: Vec<u8, 512>,
    /// Last message ID received
    pub last_message_id: u32,
    /// Message ID counter for outgoing messages
    pub message_id_counter: u32,
    /// Last communication timestamp
    pub last_communication: u32,
    /// Communication error count
    pub error_count: u32,
    /// Interface enabled flag
    pub enabled: bool,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval: u32,
    /// Last heartbeat sent timestamp
    pub last_heartbeat_sent: u32,
}

/// Message structure for STM32-ESP32 communication
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: u32,
    pub timestamp: u64,
    pub payload: MessagePayload,
}

/// Message payload types
#[derive(Debug, Serialize, Deserialize)]
pub enum MessagePayload {
    Command(Command),
    Response(Response),
}

/// Commands from ESP32 to STM32
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

/// Responses from STM32 to ESP32
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    SystemState(SystemState),
    ParameterChange { parameter: String<64>, value: f32 },
    Ack { command_id: Option<u32> },
    Error { message: String<128>, command_id: Option<u32> },
    Heartbeat,
}

/// System state for communication
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemState {
    pub tracks: Vec<TrackState, 6>,
    pub tempo: f32,
    pub network_connected: bool,
    pub current_memory: u8,
    pub fx_states: Vec<bool, 5>,
    pub timestamp: u64,
}

/// Track state for communication
#[derive(Debug, Serialize, Deserialize)]
pub struct TrackState {
    pub id: u8,
    pub state: TrackStateEnum,
    pub volume: f32,
    pub pan: f32,
    pub muted: bool,
    pub selected: bool,
}

/// Track state enumeration
#[derive(Debug, Serialize, Deserialize)]
pub enum TrackStateEnum {
    Stopped,
    Recording,
    Playing,
    Overdubbing,
    Muted,
}

impl Esp32CommunicationManager {
    /// Create a new communication manager
    pub fn new() -> Self {
        Self {
            rx_buffer: Vec::new(),
            tx_buffer: Vec::new(),
            last_message_id: 0,
            message_id_counter: 0,
            last_communication: 0,
            error_count: 0,
            enabled: true,
            heartbeat_interval: 5000, // 5 seconds
            last_heartbeat_sent: 0,
        }
    }

    /// Process incoming UART data
    pub fn process_incoming_data(&mut self, data: &[u8], timestamp: u32) -> Result<Vec<Command, 8>, HalError> {
        let mut commands = Vec::new();

        // Add data to receive buffer
        for &byte in data {
            if self.rx_buffer.push(byte).is_err() {
                // Buffer full - clear and start over
                self.rx_buffer.clear();
                self.error_count += 1;
                return Err(HalError::BufferFull);
            }

            // Check for message delimiter (newline)
            if byte == b'\n' {
                // Process complete message
                if let Ok(command) = self.parse_message() {
                    if commands.push(command).is_err() {
                        break; // Commands buffer full
                    }
                }
                self.rx_buffer.clear();
            }
        }

        self.last_communication = timestamp;
        Ok(commands)
    }

    /// Parse a complete message from the receive buffer
    fn parse_message(&mut self) -> Result<Command, HalError> {
        // Convert buffer to string
        let message_str = core::str::from_utf8(&self.rx_buffer)
            .map_err(|_| HalError::ParseError)?;

        // Try to parse as new message format
        if let Ok(message) = serde_json_core::from_str::<Message>(message_str) {
            match message.0.payload {
                MessagePayload::Command(command) => {
                    self.last_message_id = message.0.id;
                    return Ok(command);
                }
                _ => return Err(HalError::ParseError),
            }
        }

        // Fallback to old format for compatibility
        if let Ok(command) = serde_json_core::from_str::<Command>(message_str) {
            return Ok(command.0);
        }

        Err(HalError::ParseError)
    }

    /// Send response to ESP32
    pub fn send_response(&mut self, response: Response) -> Result<Vec<u8, 512>, HalError> {
        self.message_id_counter += 1;
        
        let message = Message {
            id: self.message_id_counter,
            timestamp: 0, // Will be filled by caller with actual timestamp
            payload: MessagePayload::Response(response),
        };

        // Serialize message
        let mut buffer = [0u8; 512];
        let serialized_len = serde_json_core::to_slice(&message, &mut buffer)
            .map_err(|_| HalError::SerializationError)?;

        // Create output buffer with newline
        let mut output = Vec::new();
        for &byte in &buffer[..serialized_len] {
            output.push(byte).map_err(|_| HalError::BufferFull)?;
        }
        output.push(b'\n').map_err(|_| HalError::BufferFull)?;

        Ok(output)
    }

    /// Send system status update
    pub fn send_status_update(&mut self, tracks: &[TrackStatus; 6], tempo: f32, current_memory: u8, fx_states: &[bool; 5]) -> Result<Vec<u8, 512>, HalError> {
        // Convert track states
        let mut track_states = Vec::new();
        for (i, track) in tracks.iter().enumerate() {
            let state = TrackState {
                id: i as u8,
                state: match track.state {
                    TrackStateComm::Stopped => TrackStateEnum::Stopped,
                    TrackStateComm::Recording => TrackStateEnum::Recording,
                    TrackStateComm::Playing => TrackStateEnum::Playing,
                    TrackStateComm::Overdubbing => TrackStateEnum::Overdubbing,
                    TrackStateComm::Muted => TrackStateEnum::Muted,
                },
                volume: track.volume,
                pan: track.pan,
                muted: track.muted,
                selected: track.selected,
            };
            track_states.push(state).map_err(|_| HalError::BufferFull)?;
        }

        // Convert FX states
        let mut fx_vec = Vec::new();
        for &fx_state in fx_states {
            fx_vec.push(fx_state).map_err(|_| HalError::BufferFull)?;
        }

        let system_state = SystemState {
            tracks: track_states,
            tempo,
            network_connected: true, // TODO: Get actual network status
            current_memory,
            fx_states: fx_vec,
            timestamp: 0, // Will be filled by caller
        };

        self.send_response(Response::SystemState(system_state))
    }

    /// Send parameter change notification
    pub fn send_parameter_change(&mut self, parameter: &str, value: f32) -> Result<Vec<u8, 512>, HalError> {
        let mut param_string = String::<64>::new();
        param_string.push_str(parameter).map_err(|_| HalError::BufferFull)?;
        
        let response = Response::ParameterChange {
            parameter: param_string,
            value,
        };
        
        self.send_response(response)
    }

    /// Send acknowledgment
    pub fn send_ack(&mut self, command_id: Option<u32>) -> Result<Vec<u8, 512>, HalError> {
        self.send_response(Response::Ack { command_id })
    }

    /// Send error response
    pub fn send_error(&mut self, message: &str, command_id: Option<u32>) -> Result<Vec<u8, 512>, HalError> {
        let mut error_string = String::<128>::new();
        error_string.push_str(message).map_err(|_| HalError::BufferFull)?;
        
        let response = Response::Error {
            message: error_string,
            command_id,
        };
        
        self.send_response(response)
    }

    /// Send heartbeat response
    pub fn send_heartbeat(&mut self) -> Result<Vec<u8, 512>, HalError> {
        self.send_response(Response::Heartbeat)
    }

    /// Check if heartbeat should be sent
    pub fn should_send_heartbeat(&self, timestamp: u32) -> bool {
        timestamp.saturating_sub(self.last_heartbeat_sent) >= self.heartbeat_interval
    }

    /// Update heartbeat timestamp
    pub fn update_heartbeat_sent(&mut self, timestamp: u32) {
        self.last_heartbeat_sent = timestamp;
    }

    /// Get communication statistics
    pub fn get_stats(&self) -> (u32, u32, bool) {
        (self.error_count, self.last_communication, self.enabled)
    }

    /// Reset error count
    pub fn reset_error_count(&mut self) {
        self.error_count = 0;
    }

    /// Enable/disable communication
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if communication is healthy
    pub fn is_communication_healthy(&self, timestamp: u32) -> bool {
        self.enabled && timestamp.saturating_sub(self.last_communication) < 10000 // 10 seconds timeout
    }
}

impl Default for Esp32CommunicationManager {
    fn default() -> Self {
        Self::new()
    }
}