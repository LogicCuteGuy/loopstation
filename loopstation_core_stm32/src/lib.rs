#![no_std]
#![no_main]

//! Loopstation Core STM32 - Real-time audio looping system
//! 
//! This crate provides the core functionality for a 6-track loopstation
//! with comprehensive effects processing, MIDI control, and project management.

use panic_halt as _; // Panic handler

pub mod audio;
pub mod effects;
pub mod storage;
pub mod controls;
pub mod midi;
pub mod hal;

// Re-export core types for easier access
pub use audio::{Track, TrackState, AudioBuffer, AudioEngine, CircularBuffer, AudioError};
pub use effects::{EffectChain, EffectType, Effect};
pub use storage::{MemorySystem, Project};
pub use controls::ControlAssignments;
pub use midi::MidiHandler;
pub use hal::{HardwareHal, ControlId, ButtonId, HalError};

/// Core loopstation system managing all subsystems
pub struct LoopstationCore {
    /// Audio processing engine with 6 tracks
    pub audio_engine: AudioEngine,
    /// Input effects chain (pre-recording)
    pub input_fx: EffectChain,
    /// Master effects chain (final output)
    pub master_fx: EffectChain,
    /// Memory system for project storage
    pub memory: MemorySystem,
    /// Control assignments and mappings
    pub controls: ControlAssignments,
    /// MIDI input/output handler
    pub midi: MidiHandler,
    /// Current tempo in BPM
    pub tempo: f32,
    /// Hardware abstraction layer
    pub hal: Option<HardwareHal>,
}

impl LoopstationCore {
    /// Create a new loopstation core instance
    pub fn new() -> Self {
        Self {
            audio_engine: AudioEngine::new(44100, 256),
            input_fx: EffectChain::new_input_fx(),
            master_fx: EffectChain::new_master_fx(),
            memory: MemorySystem::new(),
            controls: ControlAssignments::new(),
            midi: MidiHandler::new(),
            tempo: 120.0,
            hal: None,
        }
    }

    /// Initialize with hardware abstraction layer
    pub fn init_hardware(&mut self) -> Result<(), HalError> {
        self.hal = Some(HardwareHal::init()?);
        self.audio_engine.start_callback();
        Ok(())
    }

    /// Main audio processing callback - called from DMA interrupt
    pub fn process_audio(&mut self, input: &[f32], output: &mut [f32]) {
        // Process audio through the engine
        self.audio_engine.process_callback(input, output);
    }

    /// Update system state (called from main loop)
    pub fn update(&mut self) {
        // Update track states, handle MIDI, etc.
        // Implementation will be added in later tasks
    }

    /// Start recording on a track
    pub fn start_recording(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.start_recording(track_id)
    }

    /// Stop a track
    pub fn stop_track(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.stop_track(track_id)
    }

    /// Toggle mute on a track
    pub fn toggle_mute(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.toggle_mute(track_id)
    }

    /// Clear a track
    pub fn clear_track(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.clear_track(track_id)
    }

    /// Set track level
    pub fn set_track_level(&mut self, track_id: u8, level: f32) -> Result<(), AudioError> {
        self.audio_engine.set_track_level(track_id, level)
    }

    /// Set master level
    pub fn set_master_level(&mut self, level: f32) {
        self.audio_engine.set_master_level(level)
    }

    /// Get audio engine reference
    pub fn audio_engine(&self) -> &AudioEngine {
        &self.audio_engine
    }

    /// Get mutable audio engine reference
    pub fn audio_engine_mut(&mut self) -> &mut AudioEngine {
        &mut self.audio_engine
    }
}

impl Default for LoopstationCore {
    fn default() -> Self {
        Self::new()
    }
}