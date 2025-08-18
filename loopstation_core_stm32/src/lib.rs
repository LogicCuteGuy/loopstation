#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(all(not(feature = "std"), not(test)), no_main)]

//! Loopstation Core STM32 - Real-time audio looping system
//! 
//! This crate provides the core functionality for a 6-track loopstation
//! with comprehensive effects processing, MIDI control, and project management.

// Conditional panic handler for embedded builds
#[cfg(all(feature = "embedded", not(test)))]
extern crate panic_halt;

pub mod audio;
pub mod effects;
pub mod storage;
pub mod controls;
// pub mod midi;
#[cfg(feature = "embedded")]
pub mod hal;

#[cfg(not(feature = "embedded"))]
pub mod hal_stub;
// pub mod communication;

// Re-export core types for easier access
pub use audio::{Track, TrackState, AudioBuffer, AudioEngine, CircularBuffer, AudioError};
pub use effects::{EffectChain, EffectType, Effect};
pub use storage::{MemorySystem, Project};
pub use controls::{
    ControlAssignments, ControlInterfaceHal, ControlEvent, ButtonFunction, 
    ButtonId as ControlButtonId, KnobId, FaderId, ExpressionInput, ButtonPress
};
// pub use midi::MidiHandler;
#[cfg(feature = "embedded")]
pub use hal::{HardwareHal, ControlId, ButtonId, HalError, MidiEvent};

#[cfg(not(feature = "embedded"))]
pub use hal_stub::{HardwareHal, ControlId, ButtonId, HalError, MidiEvent};
// pub use communication::Esp32CommunicationManager;

/// Core loopstation system managing all subsystems
pub struct LoopstationCore {
    /// Audio processing engine with 6 tracks and integrated effect chains
    pub audio_engine: AudioEngine,
    /// Memory system for project storage
    pub memory: MemorySystem,
    /// Current tempo in BPM
    pub tempo: f32,
    /// Hardware abstraction layer
    pub hal: Option<HardwareHal>,
    /// Control interface for hardware controls
    pub control_interface: Option<ControlInterfaceHal>,
    /// Currently selected track (1-6)
    pub selected_track: u8,
    /// System timestamp for control timing
    pub system_time_ms: u32,
}

impl LoopstationCore {
    /// Create a new loopstation core instance
    pub fn new() -> Self {
        Self {
            audio_engine: AudioEngine::new(44100, 256),
            memory: MemorySystem::new(),
            tempo: 120.0,
            hal: None,
            control_interface: None,
            selected_track: 1,
            system_time_ms: 0,
        }
    }

    /// Initialize with hardware abstraction layer
    pub fn init_hardware(&mut self) -> Result<(), HalError> {
        self.hal = Some(HardwareHal::init()?);
        self.control_interface = Some(ControlInterfaceHal::new());
        self.audio_engine.start_callback();
        Ok(())
    }

    /// Main audio processing callback - called from DMA interrupt
    pub fn process_audio(&mut self, input: &[f32], output: &mut [f32]) {
        // Process audio through the engine
        self.audio_engine.process_callback(input, output);
    }

    /// Update system state (called from main loop)
    /// This integrates ControlInterfaceHal with LoopstationCore for 10ms response time
    pub fn update(&mut self, time_ms: u32) {
        self.system_time_ms = time_ms;
        
        // Collect control events first
        let mut control_events = heapless::Vec::<ControlEvent, 32>::new();
        let mut analog_events = heapless::Vec::<ControlEvent, 32>::new();
        
        // Update control interface and collect events
        if let (Some(control_interface), Some(hal)) = 
            (self.control_interface.as_mut(), self.hal.as_mut()) {
            
            // Get control events from hardware (buttons, faders, knobs, expression pedals)
            let events = control_interface.update(hal, time_ms);
            for event in events {
                let _ = control_events.push(event);
            }
            
            // Update analog controls (faders, knobs, expression pedals)
            let events = control_interface.update_analog_controls(hal);
            for event in events {
                let _ = analog_events.push(event);
            }
            
            // Update LED states to reflect current system state
            let _ = control_interface.update_leds(hal);
        }
        
        // Process collected control events
        for event in control_events {
            self.process_control_event(event);
        }
        
        for event in analog_events {
            self.process_analog_control_event(event);
        }
        
        // Update audio engine state
        self.audio_engine.update();
        
        // Update memory system
        self.memory.update();
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

    /// Get input effects chain
    pub fn input_fx(&self) -> &EffectChain {
        self.audio_engine.input_fx()
    }

    /// Get mutable input effects chain
    pub fn input_fx_mut(&mut self) -> &mut EffectChain {
        self.audio_engine.input_fx_mut()
    }

    /// Get master effects chain
    pub fn master_fx(&self) -> &EffectChain {
        self.audio_engine.master_fx()
    }

    /// Get mutable master effects chain
    pub fn master_fx_mut(&mut self) -> &mut EffectChain {
        self.audio_engine.master_fx_mut()
    }

    /// Get track effects chain for a specific track
    pub fn track_fx(&self, track_id: u8) -> Option<&EffectChain> {
        self.audio_engine.track_fx(track_id)
    }

    /// Get mutable track effects chain for a specific track
    pub fn track_fx_mut(&mut self, track_id: u8) -> Option<&mut EffectChain> {
        self.audio_engine.track_fx_mut(track_id)
    }

    /// Set tempo and update tempo-synced effects
    pub fn set_tempo(&mut self, bpm: f32) {
        self.tempo = bpm;
        self.audio_engine.update_tempo(bpm);
    }

    /// Process a control event and execute the corresponding loopstation function
    fn process_control_event(&mut self, event: ControlEvent) {
        // Convert control event to control result using control system
        if let Some(control_interface) = &self.control_interface {
            if let Some(result) = control_interface.control_system().process_control_event(event) {
                match result {
                    crate::controls::ControlResult::ButtonFunction(function) => {
                        self.execute_button_function(function);
                    },
                    crate::controls::ControlResult::MidiAction(action) => {
                        self.execute_midi_action(action);
                    },
                }
            }
        }
    }

    /// Process analog control events (faders, knobs, expression pedals)
    fn process_analog_control_event(&mut self, event: ControlEvent) {
        match event {
            ControlEvent::FaderMove { fader, value } => {
                self.handle_fader_control(fader, value);
            },
            ControlEvent::KnobTurn { knob, value } => {
                self.handle_knob_control(knob, value);
            },
            ControlEvent::ExpressionInput { input, value } => {
                self.handle_expression_control(input, value);
            },
            _ => {
                // Other events are handled by process_control_event
            }
        }
    }

    /// Execute a button function on the loopstation
    fn execute_button_function(&mut self, function: ButtonFunction) {
        match function {
            // Track operations
            ButtonFunction::TrackPlayStop(track_id) => {
                if let Err(_) = self.toggle_track_playback(track_id) {
                    // Track operation failed - continue
                }
            },
            ButtonFunction::TrackRecord(track_id) => {
                if let Err(_) = self.start_recording(track_id) {
                    // Recording failed - continue
                }
            },
            ButtonFunction::TrackClear(track_id) => {
                if let Err(_) = self.clear_track(track_id) {
                    // Clear failed - continue
                }
            },
            ButtonFunction::TrackSelect(track_id) => {
                self.select_track(track_id);
            },
            ButtonFunction::TrackMute(track_id) => {
                if let Err(_) = self.toggle_mute(track_id) {
                    // Mute failed - continue
                }
            },

            // Effect operations
            ButtonFunction::EffectMomentary(effect_slot) => {
                self.activate_effect_momentary(effect_slot);
            },
            ButtonFunction::EffectToggle(effect_slot) => {
                self.toggle_effect(effect_slot);
            },

            // Transport controls
            ButtonFunction::AllStart => {
                self.start_all_tracks();
            },
            ButtonFunction::AllStop => {
                self.stop_all_tracks();
            },
            ButtonFunction::AllClear => {
                self.clear_all_tracks();
            },

            // Utility functions
            ButtonFunction::Undo => {
                self.undo_last_action();
            },
            ButtonFunction::Redo => {
                self.redo_last_action();
            },
            ButtonFunction::TapTempo => {
                self.tap_tempo();
            },
            ButtonFunction::TempoReset => {
                self.reset_tempo();
            },

            // Memory operations
            ButtonFunction::MemorySave => {
                self.save_current_project();
            },
            ButtonFunction::MemoryLoad => {
                self.load_project();
            },
            ButtonFunction::MemoryInc => {
                self.increment_memory_slot();
            },
            ButtonFunction::MemoryDec => {
                self.decrement_memory_slot();
            },

            // System functions
            ButtonFunction::Panic => {
                self.panic_stop();
            },

            // Menu functions (handled by display system)
            ButtonFunction::MenuOpen |
            ButtonFunction::MenuExit |
            ButtonFunction::PageLeft |
            ButtonFunction::PageRight |
            ButtonFunction::Enter => {
                // These are handled by the ESP32 display system
                // Send command to ESP32 via communication interface
                self.send_menu_command(function);
            },
        }
    }

    /// Handle fader control for track levels
    fn handle_fader_control(&mut self, fader: FaderId, value: f32) {
        match fader {
            FaderId::Track1Level => { let _ = self.set_track_level(1, value); },
            FaderId::Track2Level => { let _ = self.set_track_level(2, value); },
            FaderId::Track3Level => { let _ = self.set_track_level(3, value); },
            FaderId::Track4Level => { let _ = self.set_track_level(4, value); },
            FaderId::Track5Level => { let _ = self.set_track_level(5, value); },
            FaderId::Track6Level => { let _ = self.set_track_level(6, value); },
        }
    }

    /// Handle knob control for context-sensitive parameters
    fn handle_knob_control(&mut self, knob: KnobId, value: f32) {
        match knob {
            KnobId::OutputLevel => {
                self.set_master_level(value);
            },
            KnobId::Knob1 | KnobId::Knob2 | KnobId::Knob3 | KnobId::Knob4 => {
                // Context-sensitive knob control
                self.handle_context_knob(knob, value);
            },
        }
    }

    /// Handle expression pedal control for real-time parameter control
    fn handle_expression_control(&mut self, input: ExpressionInput, value: f32) {
        // Map expression input to assignment index
        let assignment_index = match input {
            ExpressionInput::CTL1_EXP1 => 0,
            ExpressionInput::CTL2_EXP1 => 1,
            ExpressionInput::CTL3_EXP2 => 2,
            ExpressionInput::CTL4_EXP2 => 3,
        };
        
        // Get assignment and apply it
        let assignment = self.control_interface
            .as_ref()
            .and_then(|ci| ci.control_system().assignments.expression_assignments.get(assignment_index))
            .and_then(|opt| opt.as_ref())
            .cloned();
            
        if let Some(assignment) = assignment {
            self.apply_expression_assignment(&assignment, value);
        }
    }

    /// Handle context-sensitive knob control based on current mode
    fn handle_context_knob(&mut self, knob: KnobId, value: f32) {
        let knob_index = match knob {
            KnobId::Knob1 => 0,
            KnobId::Knob2 => 1,
            KnobId::Knob3 => 2,
            KnobId::Knob4 => 3,
            _ => return,
        };

        // In performance mode, knobs control active effects
        // Get the currently active effect chain for the selected track
        if let Some(track_fx) = self.track_fx_mut(self.selected_track) {
            // Find the first active effect and control its parameters
            for (_slot_index, effect) in track_fx.effects_mut().iter_mut().enumerate() {
                if let Some(effect) = effect {
                    if effect.enabled {
                        // Map knob to effect parameter
                        effect.set_parameter(knob_index, value);
                        break;
                    }
                }
            }
        }
    }

    /// Apply expression pedal assignment to target parameter
    fn apply_expression_assignment(&mut self, assignment: &crate::storage::ExpressionAssignment, value: f32) {
        use crate::storage::MidiTarget;
        
        match &assignment.target {
            MidiTarget::TrackVolume(track_id) => {
                let _ = self.set_track_level(*track_id, value);
            },
            MidiTarget::MasterVolume => {
                self.set_master_level(value);
            },
            MidiTarget::EffectParameter { chain_type, slot_index, parameter_index, track_id } => {
                self.set_effect_parameter(*chain_type, *slot_index, *parameter_index, *track_id, value);
            },
            _ => {
                // Other MIDI targets not applicable to expression pedals
            }
        }
    }

    /// Demonstrate the 3-layer effect processing pipeline
    /// This method shows how effects are integrated into the audio processing
    pub fn demo_effect_processing(&mut self) -> Result<(), &'static str> {
        // Add a compressor to Input FX (affects recorded audio)
        let mut compressor = Effect::new(EffectType::Compressor);
        compressor.set_enabled(true);
        compressor.set_parameter(0, 0.3); // Threshold: -20dB
        compressor.set_parameter(1, 0.5); // Ratio: 4:1
        self.input_fx_mut().add_effect(compressor).map_err(|_| "Failed to add compressor to Input FX")?;

        // Add reverb to Track 1 FX (affects track playback)
        if let Some(track_fx) = self.track_fx_mut(1) {
            let mut reverb = Effect::new(EffectType::SpaceReverb);
            reverb.set_enabled(true);
            reverb.set_parameter(0, 0.3); // Time: 2s
            reverb.set_parameter(2, 0.4); // Mix: 40%
            track_fx.add_effect(reverb).map_err(|_| "Failed to add reverb to Track 1 FX")?;
        }

        // Add EQ to Master FX (affects final output)
        let mut eq = Effect::new(EffectType::MasteringEQ);
        eq.set_enabled(true);
        eq.set_parameter(0, 0.6); // Low: +3dB
        eq.set_parameter(3, 0.7); // High: +6dB
        self.master_fx_mut().add_effect(eq).map_err(|_| "Failed to add EQ to Master FX")?;

        Ok(())
    }

    /// Toggle track playback (play if stopped, stop if playing)
    pub fn toggle_track_playback(&mut self, track_id: u8) -> Result<(), AudioError> {
        // Check current track state and toggle appropriately
        if let Some(track) = self.audio_engine.get_track(track_id) {
            match track.state {
                TrackState::Stopped => self.audio_engine.start_playback(track_id),
                TrackState::Playing => self.stop_track(track_id),
                TrackState::Recording => self.stop_track(track_id),
                TrackState::Overdubbing => self.stop_track(track_id),
                TrackState::Muted => self.audio_engine.start_playback(track_id),
            }
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Select a track for editing and control
    pub fn select_track(&mut self, track_id: u8) {
        if track_id >= 1 && track_id <= 6 {
            self.selected_track = track_id;
        }
    }

    /// Get currently selected track
    pub fn get_selected_track(&self) -> u8 {
        self.selected_track
    }

    /// Activate effect momentarily (for short button presses)
    fn activate_effect_momentary(&mut self, effect_slot: crate::controls::EffectSlot) {
        if let Some(effect_chain) = self.get_effect_chain_mut(effect_slot.chain_type, effect_slot.track_id) {
            if let Some(Some(effect)) = effect_chain.effects_mut().get_mut(effect_slot.slot_index as usize) {
                effect.set_momentary(true);
            }
        }
    }

    /// Toggle effect on/off (for long button presses)
    fn toggle_effect(&mut self, effect_slot: crate::controls::EffectSlot) {
        if let Some(effect_chain) = self.get_effect_chain_mut(effect_slot.chain_type, effect_slot.track_id) {
            if let Some(Some(effect)) = effect_chain.effects_mut().get_mut(effect_slot.slot_index as usize) {
                effect.set_enabled(!effect.enabled);
            }
        }
    }

    /// Get mutable effect chain based on type and track
    fn get_effect_chain_mut(&mut self, chain_type: crate::effects::EffectChainType, track_id: Option<u8>) -> Option<&mut EffectChain> {
        match chain_type {
            crate::effects::EffectChainType::InputFX => Some(self.input_fx_mut()),
            crate::effects::EffectChainType::TrackFX => {
                if let Some(track_id) = track_id {
                    self.track_fx_mut(track_id)
                } else {
                    None
                }
            },
            crate::effects::EffectChainType::MasterFX => Some(self.master_fx_mut()),
        }
    }

    /// Set effect parameter value
    fn set_effect_parameter(&mut self, chain_type: crate::effects::EffectChainType, slot_index: u8, param_index: u8, track_id: Option<u8>, value: f32) {
        if let Some(effect_chain) = self.get_effect_chain_mut(chain_type, track_id) {
            if let Some(Some(effect)) = effect_chain.effects_mut().get_mut(slot_index as usize) {
                effect.set_parameter(param_index as usize, value);
            }
        }
    }

    /// Start all tracks simultaneously
    pub fn start_all_tracks(&mut self) {
        for track_id in 1..=6 {
            let _ = self.audio_engine.start_playback(track_id);
        }
    }

    /// Stop all tracks simultaneously
    pub fn stop_all_tracks(&mut self) {
        for track_id in 1..=6 {
            let _ = self.stop_track(track_id);
        }
    }

    /// Clear all tracks
    pub fn clear_all_tracks(&mut self) {
        for track_id in 1..=6 {
            let _ = self.clear_track(track_id);
        }
    }

    /// Undo last action on selected track
    fn undo_last_action(&mut self) {
        // Implementation depends on undo system - placeholder for now
        if let Some(track) = self.audio_engine.get_track_mut(self.selected_track) {
            track.undo_last_action();
        }
    }

    /// Redo last undone action on selected track
    fn redo_last_action(&mut self) {
        // Implementation depends on undo system - placeholder for now
        if let Some(track) = self.audio_engine.get_track_mut(self.selected_track) {
            track.redo_last_action();
        }
    }

    /// Process tap tempo input
    fn tap_tempo(&mut self) {
        // Simple tap tempo implementation
        // In a full implementation, this would track multiple taps and calculate BPM
        static mut LAST_TAP_TIME: u32 = 0;
        static mut TAP_COUNT: u8 = 0;
        
        unsafe {
            let current_time = self.system_time_ms;
            let time_diff = current_time.saturating_sub(LAST_TAP_TIME);
            
            if time_diff < 2000 && time_diff > 200 { // Valid tap range: 30-300 BPM
                TAP_COUNT += 1;
                if TAP_COUNT >= 2 {
                    // Calculate BPM from tap interval
                    let bpm = 60000.0 / time_diff as f32;
                    self.set_tempo(bpm.clamp(60.0, 200.0));
                }
            } else {
                TAP_COUNT = 1; // Reset tap count
            }
            
            LAST_TAP_TIME = current_time;
        }
    }

    /// Reset tempo to default (120 BPM)
    fn reset_tempo(&mut self) {
        self.set_tempo(120.0);
    }

    /// Save current project to memory
    fn save_current_project(&mut self) {
        // Create project from current state
        let project = self.create_current_project();
        
        // Save to current memory slot
        if let Err(_) = self.memory.save_project(self.memory.current_memory, project) {
            // Save failed - continue operation
        }
    }

    /// Load project from current memory slot
    fn load_project(&mut self) {
        if let Ok(project) = self.memory.load_project(self.memory.current_memory) {
            self.load_project_state(project.clone());
        }
    }

    /// Increment memory slot
    fn increment_memory_slot(&mut self) {
        self.memory.current_memory = (self.memory.current_memory % 255) + 1;
    }

    /// Decrement memory slot
    fn decrement_memory_slot(&mut self) {
        self.memory.current_memory = if self.memory.current_memory > 1 {
            self.memory.current_memory - 1
        } else {
            255
        };
    }

    /// Emergency stop all audio immediately
    fn panic_stop(&mut self) {
        self.stop_all_tracks();
        self.set_master_level(0.0);
        
        // Clear all effects
        self.input_fx_mut().clear_all_effects();
        self.master_fx_mut().clear_all_effects();
        for track_id in 1..=6 {
            if let Some(track_fx) = self.track_fx_mut(track_id) {
                track_fx.clear_all_effects();
            }
        }
    }

    /// Send menu command to ESP32 display system
    fn send_menu_command(&mut self, _function: ButtonFunction) {
        // This would send commands to ESP32 via UART communication
        // Implementation depends on communication protocol
        // Placeholder for now
    }

    /// Create project from current loopstation state
    fn create_current_project(&self) -> Project {
        // This would capture all current state into a Project
        // Implementation depends on Project structure
        Project::new(self.memory.current_memory)
    }

    /// Load project state into loopstation
    fn load_project_state(&mut self, _project: Project) {
        // This would restore all state from a Project
        // Implementation depends on Project structure
    }

    /// Get control interface reference
    pub fn control_interface(&self) -> Option<&ControlInterfaceHal> {
        self.control_interface.as_ref()
    }

    /// Get mutable control interface reference
    pub fn control_interface_mut(&mut self) -> Option<&mut ControlInterfaceHal> {
        self.control_interface.as_mut()
    }

    /// Execute a MIDI control action
    fn execute_midi_action(&mut self, action: crate::controls::MidiControlAction) {
        use crate::storage::MidiTarget;
        
        match action.target {
            MidiTarget::TrackVolume(track_id) => {
                let _ = self.set_track_level(track_id, action.value);
            },
            MidiTarget::TrackPan(track_id) => {
                // Set track pan - implementation depends on audio engine pan support
                if let Some(track) = self.audio_engine.get_track_mut(track_id) {
                    track.set_pan(action.value);
                }
            },
            MidiTarget::EffectParameter { chain_type, slot_index, parameter_index, track_id } => {
                self.set_effect_parameter(chain_type, slot_index, parameter_index, track_id, action.value);
            },
            MidiTarget::MasterVolume => {
                self.set_master_level(action.value);
            },
            MidiTarget::Tempo => {
                // Map 0.0-1.0 to reasonable tempo range (60-200 BPM)
                let bpm = 60.0 + (action.value * 140.0);
                self.set_tempo(bpm);
            },
        }
    }

    /// Set control context mode
    pub fn set_control_context(&mut self, context: crate::controls::ControlContext) {
        if let Some(control_interface) = &mut self.control_interface {
            control_interface.control_system_mut().set_context(context);
        }
    }

    /// Get current control context
    pub fn get_control_context(&self) -> crate::controls::ControlContext {
        self.control_interface
            .as_ref()
            .map(|ci| ci.control_system().get_context())
            .unwrap_or(crate::controls::ControlContext::Performance)
    }

    /// Add MIDI CC assignment for external control
    pub fn add_midi_assignment(&mut self, cc_number: u8, target: crate::storage::MidiTarget) -> Result<(), &'static str> {
        if let Some(control_interface) = &mut self.control_interface {
            control_interface.control_system_mut().add_midi_assignment(cc_number, target)
        } else {
            Err("Control interface not initialized")
        }
    }

    /// Remove MIDI CC assignment
    pub fn remove_midi_assignment(&mut self, cc_number: u8) -> bool {
        if let Some(control_interface) = &mut self.control_interface {
            control_interface.control_system_mut().remove_midi_assignment(cc_number)
        } else {
            false
        }
    }

    /// Set FX button assignment for effect chain control
    pub fn set_fx_button_assignment(&mut self, button_index: usize, assignment: Option<crate::storage::FXButtonAssignment>) {
        if let Some(control_interface) = &mut self.control_interface {
            control_interface.control_system_mut().set_fx_button_assignment(button_index, assignment);
        }
    }

    /// Set footswitch assignment for hands-free control
    pub fn set_footswitch_assignment(&mut self, footswitch_index: usize, assignment: Option<crate::storage::FootswitchAssignment>) {
        if let Some(control_interface) = &mut self.control_interface {
            control_interface.control_system_mut().set_footswitch_assignment(footswitch_index, assignment);
        }
    }

    /// Set expression pedal assignment for real-time parameter control
    pub fn set_expression_assignment(&mut self, input_index: usize, assignment: Option<crate::storage::ExpressionAssignment>) {
        if let Some(control_interface) = &mut self.control_interface {
            control_interface.control_system_mut().set_expression_assignment(input_index, assignment);
        }
    }

    /// Configure MIDI settings
    pub fn configure_midi(&mut self, channel: u8, cc_enabled: bool) {
        if let Some(control_interface) = &mut self.control_interface {
            let control_system = control_interface.control_system_mut();
            control_system.set_midi_channel(channel);
            control_system.set_midi_cc_enabled(cc_enabled);
        }
    }
}

impl Default for LoopstationCore {
    fn default() -> Self {
        Self::new()
    }
}