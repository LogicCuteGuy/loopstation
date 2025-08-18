use heapless::Vec;
use serde::{Deserialize, Serialize};

/// MIDI channel configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiChannel {
    /// Specific MIDI channel (1-16)
    Channel(u8),
    /// Omni mode (respond to all channels)
    Omni,
}

impl Default for MidiChannel {
    fn default() -> Self {
        MidiChannel::Channel(1)
    }
}

/// MIDI message types supported by the loopstation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiMessage {
    /// Note On message
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    /// Note Off message
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    /// Control Change message
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    /// Program Change message
    ProgramChange {
        channel: u8,
        program: u8,
    },
    /// MIDI Clock message
    Clock,
    /// MIDI Start message
    Start,
    /// MIDI Stop message
    Stop,
    /// MIDI Continue message
    Continue,
    /// System Exclusive message (simplified)
    SysEx {
        data: Vec<u8, 64>,
    },
}

impl MidiMessage {
    /// Parse MIDI message from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        let status = bytes[0];
        
        match status {
            // Note Off (0x80-0x8F)
            0x80..=0x8F => {
                if bytes.len() >= 3 {
                    Some(MidiMessage::NoteOff {
                        channel: (status & 0x0F) + 1,
                        note: bytes[1],
                        velocity: bytes[2],
                    })
                } else {
                    None
                }
            },
            // Note On (0x90-0x9F)
            0x90..=0x9F => {
                if bytes.len() >= 3 {
                    Some(MidiMessage::NoteOn {
                        channel: (status & 0x0F) + 1,
                        note: bytes[1],
                        velocity: bytes[2],
                    })
                } else {
                    None
                }
            },
            // Control Change (0xB0-0xBF)
            0xB0..=0xBF => {
                if bytes.len() >= 3 {
                    Some(MidiMessage::ControlChange {
                        channel: (status & 0x0F) + 1,
                        controller: bytes[1],
                        value: bytes[2],
                    })
                } else {
                    None
                }
            },
            // Program Change (0xC0-0xCF)
            0xC0..=0xCF => {
                if bytes.len() >= 2 {
                    Some(MidiMessage::ProgramChange {
                        channel: (status & 0x0F) + 1,
                        program: bytes[1],
                    })
                } else {
                    None
                }
            },
            // System Real-Time Messages
            0xF8 => Some(MidiMessage::Clock),
            0xFA => Some(MidiMessage::Start),
            0xFB => Some(MidiMessage::Continue),
            0xFC => Some(MidiMessage::Stop),
            
            // System Exclusive (0xF0)
            0xF0 => {
                if let Some(end_pos) = bytes.iter().position(|&b| b == 0xF7) {
                    let mut data = Vec::new();
                    for &byte in &bytes[1..end_pos] {
                        if data.push(byte).is_err() {
                            break; // Buffer full
                        }
                    }
                    Some(MidiMessage::SysEx { data })
                } else {
                    None
                }
            },
            
            _ => None,
        }
    }

    /// Convert MIDI message to bytes
    pub fn to_bytes(&self) -> Vec<u8, 8> {
        let mut bytes = Vec::new();
        
        match self {
            MidiMessage::NoteOff { channel, note, velocity } => {
                let _ = bytes.push(0x80 | (channel - 1));
                let _ = bytes.push(*note);
                let _ = bytes.push(*velocity);
            },
            MidiMessage::NoteOn { channel, note, velocity } => {
                let _ = bytes.push(0x90 | (channel - 1));
                let _ = bytes.push(*note);
                let _ = bytes.push(*velocity);
            },
            MidiMessage::ControlChange { channel, controller, value } => {
                let _ = bytes.push(0xB0 | (channel - 1));
                let _ = bytes.push(*controller);
                let _ = bytes.push(*value);
            },
            MidiMessage::ProgramChange { channel, program } => {
                let _ = bytes.push(0xC0 | (channel - 1));
                let _ = bytes.push(*program);
            },
            MidiMessage::Clock => {
                let _ = bytes.push(0xF8);
            },
            MidiMessage::Start => {
                let _ = bytes.push(0xFA);
            },
            MidiMessage::Continue => {
                let _ = bytes.push(0xFB);
            },
            MidiMessage::Stop => {
                let _ = bytes.push(0xFC);
            },
            MidiMessage::SysEx { data } => {
                let _ = bytes.push(0xF0);
                for &byte in data {
                    if bytes.push(byte).is_err() {
                        break;
                    }
                }
                let _ = bytes.push(0xF7);
            },
        }
        
        bytes
    }

    /// Get the MIDI channel for this message (if applicable)
    pub fn channel(&self) -> Option<u8> {
        match self {
            MidiMessage::NoteOn { channel, .. } |
            MidiMessage::NoteOff { channel, .. } |
            MidiMessage::ControlChange { channel, .. } |
            MidiMessage::ProgramChange { channel, .. } => Some(*channel),
            _ => None,
        }
    }
}

/// MIDI settings configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MidiSettings {
    /// MIDI channel configuration
    pub midi_channel: MidiChannel,
    /// Local control enabled (disable for DAW controller use)
    pub local_control: bool,
    /// Send Program Change messages on memory slot changes
    pub pc_out: bool,
    /// Enable Control Change transmission and reception
    pub cc_tx_rx: bool,
    /// MIDI clock sync enabled
    pub clock_sync: bool,
}

impl Default for MidiSettings {
    fn default() -> Self {
        Self {
            midi_channel: MidiChannel::default(),
            local_control: true,
            pc_out: false,
            cc_tx_rx: true,
            clock_sync: false,
        }
    }
}

/// MIDI handler for input/output processing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MidiHandler {
    /// MIDI settings
    pub settings: MidiSettings,
    /// Input buffer for incoming MIDI messages
    pub input_buffer: Vec<u8, 256>,
    /// Output buffer for outgoing MIDI messages
    pub output_buffer: Vec<u8, 256>,
    /// Current MIDI clock tempo (BPM)
    pub clock_tempo: f32,
    /// MIDI clock counter
    pub clock_counter: u32,
}

impl MidiHandler {
    /// Create a new MIDI handler
    pub fn new() -> Self {
        Self {
            settings: MidiSettings::default(),
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
            clock_tempo: 120.0,
            clock_counter: 0,
        }
    }

    /// Process incoming MIDI data
    pub fn process_input(&mut self, data: &[u8]) -> Vec<MidiMessage, 16> {
        let mut messages = Vec::new();
        
        // Add incoming data to input buffer
        for &byte in data {
            if self.input_buffer.push(byte).is_err() {
                // Buffer full, clear and start over
                self.input_buffer.clear();
                let _ = self.input_buffer.push(byte);
            }
        }

        // Parse complete messages from buffer
        let mut parse_pos = 0;
        while parse_pos < self.input_buffer.len() {
            if let Some(message) = self.parse_message_at_position(parse_pos) {
                // Get message length before moving the message
                let message_length = self.get_message_length(&message);
                
                // Check if message should be processed based on channel settings
                if self.should_process_message(&message) {
                    if messages.push(message).is_err() {
                        break; // Output buffer full
                    }
                }
                
                // Move past this message
                parse_pos += message_length;
            } else {
                parse_pos += 1;
            }
        }

        // Clear processed data from input buffer
        if parse_pos > 0 {
            let end_pos = parse_pos.min(self.input_buffer.len());
            for _ in 0..end_pos {
                self.input_buffer.remove(0);
            }
        }

        messages
    }

    /// Parse MIDI message at specific position in input buffer
    fn parse_message_at_position(&self, pos: usize) -> Option<MidiMessage> {
        if pos >= self.input_buffer.len() {
            return None;
        }

        let remaining = &self.input_buffer[pos..];
        MidiMessage::from_bytes(remaining)
    }

    /// Get the byte length of a MIDI message
    fn get_message_length(&self, message: &MidiMessage) -> usize {
        match message {
            MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. } | MidiMessage::ControlChange { .. } => 3,
            MidiMessage::ProgramChange { .. } => 2,
            MidiMessage::Clock | MidiMessage::Start | MidiMessage::Stop | MidiMessage::Continue => 1,
            MidiMessage::SysEx { data } => data.len() + 2, // +2 for 0xF0 and 0xF7
        }
    }

    /// Check if message should be processed based on channel settings
    fn should_process_message(&self, message: &MidiMessage) -> bool {
        match (&self.settings.midi_channel, message.channel()) {
            (MidiChannel::Omni, _) => true,
            (MidiChannel::Channel(ch), Some(msg_ch)) => *ch == msg_ch,
            (MidiChannel::Channel(_), None) => true, // System messages
        }
    }

    /// Send MIDI message
    pub fn send_message(&mut self, message: MidiMessage) -> Result<(), ()> {
        if !self.settings.local_control {
            return Ok(()); // Local control disabled
        }

        let bytes = message.to_bytes();
        for byte in bytes {
            if self.output_buffer.push(byte).is_err() {
                return Err(()); // Output buffer full
            }
        }
        Ok(())
    }

    /// Send Program Change message for memory slot
    pub fn send_program_change(&mut self, memory_slot: u8) -> Result<(), ()> {
        if !self.settings.pc_out {
            return Ok(()); // PC out disabled
        }

        let channel = match self.settings.midi_channel {
            MidiChannel::Channel(ch) => ch,
            MidiChannel::Omni => 1, // Default to channel 1 for output
        };

        let message = MidiMessage::ProgramChange {
            channel,
            program: memory_slot.saturating_sub(1), // Memory 1 = PC#0
        };

        self.send_message(message)
    }

    /// Send Control Change message
    pub fn send_control_change(&mut self, controller: u8, value: u8) -> Result<(), ()> {
        if !self.settings.cc_tx_rx {
            return Ok(()); // CC disabled
        }

        let channel = match self.settings.midi_channel {
            MidiChannel::Channel(ch) => ch,
            MidiChannel::Omni => 1, // Default to channel 1 for output
        };

        let message = MidiMessage::ControlChange {
            channel,
            controller,
            value,
        };

        self.send_message(message)
    }

    /// Process MIDI clock message
    pub fn process_clock(&mut self) {
        self.clock_counter += 1;
        
        // MIDI clock runs at 24 pulses per quarter note
        // Calculate tempo based on clock timing (this is simplified)
        if self.clock_counter % 24 == 0 {
            // One quarter note completed
            // Tempo calculation would need timing information
        }
    }

    /// Get output data to send
    pub fn get_output_data(&mut self) -> &[u8] {
        &self.output_buffer
    }

    /// Clear output buffer after sending
    pub fn clear_output_buffer(&mut self) {
        self.output_buffer.clear();
    }

    /// Set MIDI settings
    pub fn set_settings(&mut self, settings: MidiSettings) {
        self.settings = settings;
    }

    /// Get current MIDI settings
    pub fn get_settings(&self) -> &MidiSettings {
        &self.settings
    }
}

impl Default for MidiHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// MIDI CC mappings for common loopstation functions
pub mod cc_mappings {
    /// Standard CC numbers for loopstation control
    pub const TRACK_1_VOLUME: u8 = 7;
    pub const TRACK_2_VOLUME: u8 = 8;
    pub const TRACK_3_VOLUME: u8 = 9;
    pub const TRACK_4_VOLUME: u8 = 10;
    pub const TRACK_5_VOLUME: u8 = 11;
    pub const TRACK_6_VOLUME: u8 = 12;
    
    pub const TRACK_1_PAN: u8 = 13;
    pub const TRACK_2_PAN: u8 = 14;
    pub const TRACK_3_PAN: u8 = 15;
    pub const TRACK_4_PAN: u8 = 16;
    pub const TRACK_5_PAN: u8 = 17;
    pub const TRACK_6_PAN: u8 = 18;
    
    pub const MASTER_VOLUME: u8 = 19;
    pub const TEMPO: u8 = 20;
    
    pub const FX_1_PARAM_1: u8 = 21;
    pub const FX_1_PARAM_2: u8 = 22;
    pub const FX_1_PARAM_3: u8 = 23;
    pub const FX_1_PARAM_4: u8 = 24;
    
    // Expression pedal CCs
    pub const EXPRESSION_1: u8 = 1;
    pub const EXPRESSION_2: u8 = 2;
    pub const EXPRESSION_3: u8 = 3;
    pub const EXPRESSION_4: u8 = 4;
}

/// MIDI note mappings for track control
pub mod note_mappings {
    /// MIDI notes for track record/play control
    pub const TRACK_1_REC_PLAY: u8 = 36; // C2
    pub const TRACK_2_REC_PLAY: u8 = 37; // C#2
    pub const TRACK_3_REC_PLAY: u8 = 38; // D2
    pub const TRACK_4_REC_PLAY: u8 = 39; // D#2
    pub const TRACK_5_REC_PLAY: u8 = 40; // E2
    pub const TRACK_6_REC_PLAY: u8 = 41; // F2
    
    /// MIDI notes for track stop
    pub const TRACK_1_STOP: u8 = 42; // F#2
    pub const TRACK_2_STOP: u8 = 43; // G2
    pub const TRACK_3_STOP: u8 = 44; // G#2
    pub const TRACK_4_STOP: u8 = 45; // A2
    pub const TRACK_5_STOP: u8 = 46; // A#2
    pub const TRACK_6_STOP: u8 = 47; // B2
    
    /// Transport control
    pub const ALL_START: u8 = 48; // C3
    pub const ALL_STOP: u8 = 49;  // C#3
    pub const TAP_TEMPO: u8 = 50; // D3
}