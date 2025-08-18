//! Hardware Abstraction Layer stub for non-embedded builds
//!
//! This module provides stub implementations for PC/testing builds

use heapless::Vec;

/// Hardware abstraction layer stub for PC builds
pub struct HardwareHal {
    /// Stub field for PC compatibility
    pub initialized: bool,
}

/// Control ID enumeration (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlId {
    Track1,
    Track2,
    Track3,
    Track4,
    Track5,
    Track6,
    Fx1,
    Fx2,
    Fx3,
    Fx4,
    Fx5,
    Knob(u8),
    OutputLevel,
    TrackFader(u8),
    ExpressionPedal(u8),
}

/// Button ID enumeration (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
    Track(u8),
    TrackSelect(u8),
    FX(u8),
    Play,
    Stop,
    Rec,
    Menu,
    PageLeft,
    PageRight,
    Enter,
    Exit,
    TapTempo,
    Memory,
    UndoRedo,
    Edit,
}

/// HAL error types (stub)
#[derive(Debug)]
pub enum HalError {
    InitializationFailed,
    PeripheralAccess,
    InvalidParameter,
}

/// MIDI event types (stub)
#[derive(Debug, Clone, Copy)]
pub enum MidiEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    ControlChange { channel: u8, controller: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 },
}

/// Track status for communication (stub)
#[derive(Debug, Clone, Copy)]
pub struct TrackStatus {
    pub state: TrackStateComm,
    pub volume: f32,
    pub pan: f32,
    pub muted: bool,
    pub selected: bool,
}

/// Track state for communication (stub)
#[derive(Debug, Clone, Copy)]
pub enum TrackStateComm {
    Stopped,
    Recording,
    Playing,
    Overdubbing,
    Muted,
}

impl HardwareHal {
    /// Initialize the hardware abstraction layer (PC stub)
    pub fn init() -> Result<Self, HalError> {
        Ok(Self {
            initialized: true,
        })
    }

    /// Process audio callback (stub)
    pub fn process_audio(&mut self, _input: &[f32], _output: &mut [f32]) {
        // Stub implementation
    }

    /// Update controls (stub)
    pub fn update_controls(&mut self) -> Vec<(ControlId, f32), 32> {
        Vec::new()
    }

    /// Update buttons (stub)
    pub fn update_buttons(&mut self) -> Vec<(ButtonId, bool), 32> {
        Vec::new()
    }

    /// Send UART data (stub)
    pub fn send_uart_data(&mut self, _data: &[u8]) -> Result<(), HalError> {
        Ok(())
    }

    /// Receive UART data (stub)
    pub fn receive_uart_data(&mut self) -> Result<Vec<u8, 256>, HalError> {
        Ok(Vec::new())
    }

    /// Get MIDI events (stub)
    pub fn get_midi_events(&mut self) -> Vec<MidiEvent, 16> {
        Vec::new()
    }

    /// Get footswitch events (stub)
    pub fn get_footswitch_events(&mut self) -> Vec<(usize, bool), 4> {
        Vec::new()
    }
}