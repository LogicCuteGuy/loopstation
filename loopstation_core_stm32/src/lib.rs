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
pub mod midi;
pub mod settings;
pub mod tempo;
pub mod rhythm;
pub mod modulation;
pub mod performance;
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
pub use midi::{MidiHandler, MidiMessage, MidiSettings, MidiChannel};
pub use settings::{
    SystemSettings, GeneralSettings, ClockSettings, MidiSettings as SystemMidiSettings, 
    ControlSettings, UtilitySettings, BackupSettings, QuantizeMode, UndoMode, 
    StartupScreen, AutoOffTime, PhonesMode, StoreMode, ClockSource, SyncOut, 
    RecQuantize, MidiChannel as SystemMidiChannel, CtlFuncAssign, FootSwitchFunction, 
    ExpPedalMode, InitializeMode, InitializeAction, BackupResult, SettingsError
};
pub use tempo::{
    TempoSystem, NoteValue, TapStatus, MidiSyncStatus, TempoLockedParameter
};
pub use rhythm::{
    RhythmSystem, RhythmPattern, DrumSound, PatternStep, DrumTrack, 
    RhythmTrigger, RhythmStatus, DrumSynthesizer
};
pub use modulation::{
    ModulationSystem, Lfo, StepSequencer, LfoWaveform, LfoSyncMode, TempoSyncDivision,
    ModulationTarget, ModulationAssignment, SequencerStep, ModulationValues, ModulationActivity
};
pub use performance::{
    PerformanceProfiler, PerformanceMetrics, OptimizationFlags, PerformanceStatus,
    AudioOptimizer, AudioMemoryPool, LatencyMeasurement, PerformanceTestSuite, PerformanceTestResult
};
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
    /// System settings and configuration
    pub settings: SystemSettings,
    /// Tempo and rhythm system
    pub tempo_system: TempoSystem,
    /// Rhythm pattern system
    pub rhythm_system: RhythmSystem,
    /// Drum synthesizer for rhythm playback
    pub drum_synth: DrumSynthesizer,
    /// Modulation system for LFOs and Step Sequencers
    pub modulation_system: ModulationSystem,
    /// Performance profiler for monitoring system performance
    pub performance_profiler: PerformanceProfiler,
    /// Audio memory pool for optimized buffer allocation
    pub memory_pool: AudioMemoryPool,
    /// Latency measurement system
    pub latency_measurement: LatencyMeasurement,
    /// Hardware abstraction layer
    pub hal: Option<HardwareHal>,
    /// Control interface for hardware controls
    pub control_interface: Option<ControlInterfaceHal>,
    /// MIDI handler for input/output processing
    pub midi_handler: MidiHandler,
    /// Currently selected track (1-6)
    pub selected_track: u8,
    /// System timestamp for control timing
    pub system_time_ms: u32,
}

impl LoopstationCore {
    /// Create a new loopstation core instance
    pub fn new() -> Self {
        let mut profiler = PerformanceProfiler::new();
        profiler.update_config(44100, 256);
        
        let mut latency = LatencyMeasurement::new();
        latency.measure_impulse_latency(44100, 256);
        
        Self {
            audio_engine: AudioEngine::new(44100, 256),
            memory: MemorySystem::new(),
            settings: SystemSettings::new(),
            tempo_system: TempoSystem::new(44100),
            rhythm_system: RhythmSystem::new(44100),
            drum_synth: DrumSynthesizer::new(44100),
            modulation_system: ModulationSystem::new(44100.0),
            performance_profiler: profiler,
            memory_pool: AudioMemoryPool::new(),
            latency_measurement: latency,
            hal: None,
            control_interface: None,
            midi_handler: MidiHandler::new(),
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
        // Start performance timing
        self.performance_profiler.start_callback_timing();
        
        // Process audio through the engine
        self.audio_engine.process_callback(input, output);
        
        // Process rhythm system and generate drum audio
        if self.rhythm_system.is_playing() {
            // Create temporary buffer for drum audio
            let mut drum_buffer = [0.0f32; 512]; // Max buffer size for embedded
            let buffer_len = output.len().min(512);
            
            // Generate drum audio
            self.drum_synth.process_audio(&mut drum_buffer[..buffer_len]);
            
            // Mix drum audio with main output
            for (i, drum_sample) in drum_buffer[..buffer_len].iter().enumerate() {
                if i < output.len() {
                    output[i] += drum_sample * 0.5; // Mix at 50% level
                }
            }
            
            // Update tempo system with actual processed samples
            self.tempo_system.update_beat_position(buffer_len as u32);
            
            // Process rhythm triggers
            let rhythm_triggers = self.rhythm_system.process(buffer_len as u32, &self.tempo_system);
            for trigger in rhythm_triggers {
                self.drum_synth.trigger(trigger);
            }
        }
        
        // End performance timing
        self.performance_profiler.end_callback_timing();
        
        // Update latency measurement
        self.performance_profiler.update_latency(self.latency_measurement.total_latency);
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
        
        // Process MIDI input messages
        if let Some(hal) = self.hal.as_mut() {
            if let Ok(midi_messages) = hal.process_midi_input(time_ms) {
                for message in midi_messages {
                    self.process_midi_message(message, time_ms);
                }
            }
        }
        
        // Update tempo system beat position based on audio processing
        // This is a simplified update - in practice, this would be called from the audio callback
        let samples_since_last_update = 256; // Typical buffer size
        self.tempo_system.update_beat_position(samples_since_last_update);
        
        // Update modulation system and get modulation values
        let modulation_values = self.modulation_system.update(
            self.tempo_system.get_bpm(), 
            samples_since_last_update
        );
        
        // Sync modulation system to tempo
        self.modulation_system.sync_to_tempo(self.tempo_system.get_beat_position());
        
        // Apply modulation to audio engine parameters
        self.apply_modulation_to_audio_engine(&modulation_values);
        
        // Process rhythm system and generate drum triggers
        let rhythm_triggers = self.rhythm_system.process(samples_since_last_update, &self.tempo_system);
        
        // Trigger drum sounds
        for trigger in rhythm_triggers {
            self.drum_synth.trigger(trigger);
        }
        
        // Sync rhythm to tempo system
        self.rhythm_system.sync_to_tempo(&self.tempo_system);
        
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
        
        // Update performance monitoring
        let active_tracks = self.audio_engine.get_active_track_count();
        let active_effects = self.count_active_effects();
        self.performance_profiler.update_active_components(active_tracks, active_effects);
    }

    /// Start recording on a track
    pub fn start_recording(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.start_recording(track_id, self.system_time_ms)
    }

    /// Stop a track
    pub fn stop_track(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.stop_track(track_id)
    }

    /// Toggle mute on a track
    pub fn toggle_mute(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.toggle_mute(track_id, self.system_time_ms)
    }

    /// Clear a track
    pub fn clear_track(&mut self, track_id: u8) -> Result<(), AudioError> {
        self.audio_engine.clear_track(track_id, self.system_time_ms)
    }

    /// Set track level
    pub fn set_track_level(&mut self, track_id: u8, level: f32) -> Result<(), AudioError> {
        let result = self.audio_engine.set_track_level(track_id, level, self.system_time_ms);
        
        // Send MIDI CC if enabled and successful
        if result.is_ok() && self.midi_handler.get_settings().cc_tx_rx {
            use crate::midi::cc_mappings::*;
            let cc_number = match track_id {
                1 => TRACK_1_VOLUME,
                2 => TRACK_2_VOLUME,
                3 => TRACK_3_VOLUME,
                4 => TRACK_4_VOLUME,
                5 => TRACK_5_VOLUME,
                6 => TRACK_6_VOLUME,
                _ => return result, // Invalid track, don't send MIDI
            };
            
            let midi_value = (level * 127.0).clamp(0.0, 127.0) as u8;
            let _ = self.send_midi_control_change(cc_number, midi_value);
        }
        
        result
    }

    /// Set master level
    pub fn set_master_level(&mut self, level: f32) {
        self.audio_engine.set_master_level(level);
        
        // Send MIDI CC if enabled
        if self.midi_handler.get_settings().cc_tx_rx {
            use crate::midi::cc_mappings::MASTER_VOLUME;
            let midi_value = (level * 127.0).clamp(0.0, 127.0) as u8;
            let _ = self.send_midi_control_change(MASTER_VOLUME, midi_value);
        }
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
        self.tempo_system.set_bpm(bpm);
        self.audio_engine.update_tempo(self.tempo_system.get_bpm());
        
        // Send MIDI CC if enabled
        if self.midi_handler.get_settings().cc_tx_rx {
            use crate::midi::cc_mappings::TEMPO;
            // Map BPM (60-200) to MIDI value (0-127)
            let normalized_tempo = ((bpm - 60.0) / 140.0).clamp(0.0, 1.0);
            let midi_value = (normalized_tempo * 127.0) as u8;
            let _ = self.send_midi_control_change(TEMPO, midi_value);
        }
    }

    /// Get current tempo in BPM
    pub fn get_tempo(&self) -> f32 {
        self.tempo_system.get_bpm()
    }

    /// Get tempo system reference
    pub fn tempo_system(&self) -> &TempoSystem {
        &self.tempo_system
    }

    /// Get mutable tempo system reference
    pub fn tempo_system_mut(&mut self) -> &mut TempoSystem {
        &mut self.tempo_system
    }

    /// Enable/disable MIDI clock sync
    pub fn set_midi_clock_sync(&mut self, enabled: bool) {
        self.tempo_system.set_midi_sync(enabled);
        
        // Update MIDI handler settings
        let mut midi_settings = self.midi_handler.get_settings().clone();
        midi_settings.clock_sync = enabled;
        self.midi_handler.set_settings(midi_settings);
    }

    /// Get tap tempo status for display
    pub fn get_tap_status(&self) -> TapStatus {
        self.tempo_system.get_tap_status()
    }

    /// Get MIDI sync status for display
    pub fn get_midi_sync_status(&self) -> MidiSyncStatus {
        self.tempo_system.get_midi_sync_status()
    }

    /// Get current beat position (0.0-1.0)
    pub fn get_beat_position(&self) -> f32 {
        self.tempo_system.get_beat_position()
    }

    /// Get current bar position (0.0-1.0)
    pub fn get_bar_position(&self) -> f32 {
        self.tempo_system.get_bar_position()
    }

    /// Check if we're at the start of a beat
    pub fn is_beat_start(&self) -> bool {
        self.tempo_system.is_beat_start(0.05) // 5% tolerance
    }

    /// Check if we're at the start of a bar
    pub fn is_bar_start(&self) -> bool {
        self.tempo_system.is_bar_start(0.05) // 5% tolerance
    }

    /// Start rhythm playback
    pub fn start_rhythm(&mut self) {
        self.rhythm_system.start();
    }

    /// Stop rhythm playback
    pub fn stop_rhythm(&mut self) {
        self.rhythm_system.stop();
    }

    /// Toggle rhythm playback
    pub fn toggle_rhythm(&mut self) {
        self.rhythm_system.toggle();
    }

    /// Select rhythm pattern
    pub fn select_rhythm_pattern(&mut self, pattern_index: usize) {
        self.rhythm_system.select_pattern(pattern_index);
    }

    /// Get rhythm system reference
    pub fn rhythm_system(&self) -> &RhythmSystem {
        &self.rhythm_system
    }

    /// Get mutable rhythm system reference
    pub fn rhythm_system_mut(&mut self) -> &mut RhythmSystem {
        &mut self.rhythm_system
    }

    /// Get rhythm status for display
    pub fn get_rhythm_status(&self) -> RhythmStatus {
        self.rhythm_system.get_status()
    }

    /// Set rhythm master volume
    pub fn set_rhythm_volume(&mut self, volume: f32) {
        self.rhythm_system.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Get modulation system reference
    pub fn modulation_system(&self) -> &ModulationSystem {
        &self.modulation_system
    }

    /// Get mutable modulation system reference
    pub fn modulation_system_mut(&mut self) -> &mut ModulationSystem {
        &mut self.modulation_system
    }

    /// Get LFO by ID
    pub fn get_lfo(&self, id: u8) -> Option<&Lfo> {
        self.modulation_system.get_lfo(id)
    }

    /// Get mutable LFO by ID
    pub fn get_lfo_mut(&mut self, id: u8) -> Option<&mut Lfo> {
        self.modulation_system.get_lfo_mut(id)
    }

    /// Get Step Sequencer by ID
    pub fn get_step_sequencer(&self, id: u8) -> Option<&StepSequencer> {
        self.modulation_system.get_step_sequencer(id)
    }

    /// Get mutable Step Sequencer by ID
    pub fn get_step_sequencer_mut(&mut self, id: u8) -> Option<&mut StepSequencer> {
        self.modulation_system.get_step_sequencer_mut(id)
    }

    /// Enable/disable LFO
    pub fn set_lfo_enabled(&mut self, id: u8, enabled: bool) {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.enabled = enabled;
        }
    }

    /// Set LFO waveform
    pub fn set_lfo_waveform(&mut self, id: u8, waveform: LfoWaveform) {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.waveform = waveform;
        }
    }

    /// Set LFO rate (Hz for free-running mode)
    pub fn set_lfo_rate(&mut self, id: u8, rate_hz: f32) {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.set_rate_hz(rate_hz);
        }
    }

    /// Set LFO tempo sync division
    pub fn set_lfo_tempo_division(&mut self, id: u8, division: TempoSyncDivision) {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.set_tempo_division(division);
        }
    }

    /// Set LFO sync mode
    pub fn set_lfo_sync_mode(&mut self, id: u8, mode: LfoSyncMode) {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.set_sync_mode(mode);
        }
    }

    /// Set LFO depth
    pub fn set_lfo_depth(&mut self, id: u8, depth: f32) {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.set_depth(depth);
        }
    }

    /// Set LFO phase offset
    pub fn set_lfo_phase_offset(&mut self, id: u8, offset: f32) {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.set_phase_offset(offset);
        }
    }

    /// Add LFO modulation assignment
    pub fn add_lfo_assignment(&mut self, id: u8, assignment: ModulationAssignment) -> Result<(), ()> {
        if let Some(lfo) = self.get_lfo_mut(id) {
            lfo.add_assignment(assignment)
        } else {
            Err(())
        }
    }

    /// Enable/disable Step Sequencer
    pub fn set_step_sequencer_enabled(&mut self, id: u8, enabled: bool) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.enabled = enabled;
        }
    }

    /// Set Step Sequencer step count
    pub fn set_step_sequencer_length(&mut self, id: u8, length: u8) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.set_step_count(length);
        }
    }

    /// Set Step Sequencer tempo division
    pub fn set_step_sequencer_tempo_division(&mut self, id: u8, division: TempoSyncDivision) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.set_tempo_division(division);
        }
    }

    /// Set Step Sequencer swing
    pub fn set_step_sequencer_swing(&mut self, id: u8, swing: f32) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.set_swing(swing);
        }
    }

    /// Set Step Sequencer step value
    pub fn set_step_sequencer_step_value(&mut self, id: u8, step_index: u8, value: f32) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.set_step_value(step_index, value);
        }
    }

    /// Set Step Sequencer step gate length
    pub fn set_step_sequencer_step_gate(&mut self, id: u8, step_index: u8, gate: f32) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.set_step_gate(step_index, gate);
        }
    }

    /// Set Step Sequencer step velocity
    pub fn set_step_sequencer_step_velocity(&mut self, id: u8, step_index: u8, velocity: f32) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.set_step_velocity(step_index, velocity);
        }
    }

    /// Enable/disable Step Sequencer step
    pub fn set_step_sequencer_step_enabled(&mut self, id: u8, step_index: u8, enabled: bool) {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.set_step_enabled(step_index, enabled);
        }
    }

    /// Add Step Sequencer modulation assignment
    pub fn add_step_sequencer_assignment(&mut self, id: u8, assignment: ModulationAssignment) -> Result<(), ()> {
        if let Some(sequencer) = self.get_step_sequencer_mut(id) {
            sequencer.add_assignment(assignment)
        } else {
            Err(())
        }
    }

    /// Reset all modulation sources
    pub fn reset_modulation(&mut self) {
        self.modulation_system.reset_all();
    }

    /// Get modulation activity status for display
    pub fn get_modulation_activity(&self) -> ModulationActivity {
        self.modulation_system.get_modulation_activity()
    }

    /// Get performance metrics for monitoring
    pub fn get_performance_metrics(&self) -> &PerformanceMetrics {
        &self.performance_profiler.metrics
    }

    /// Check if system meets performance requirements
    pub fn check_performance_requirements(&self) -> PerformanceStatus {
        self.performance_profiler.check_performance_requirements()
    }

    /// Get optimization recommendations
    pub fn get_optimization_recommendations(&self) -> heapless::Vec<&'static str, 8> {
        self.performance_profiler.get_optimization_recommendations()
    }

    /// Reset performance counters
    pub fn reset_performance_counters(&mut self) {
        self.performance_profiler.reset_counters();
    }

    /// Update performance configuration
    pub fn update_performance_config(&mut self, sample_rate: u32, buffer_size: u32) {
        self.performance_profiler.update_config(sample_rate, buffer_size);
        self.latency_measurement.measure_impulse_latency(sample_rate, buffer_size);
    }

    /// Get memory pool utilization
    pub fn get_memory_pool_utilization(&self) -> f32 {
        self.memory_pool.utilization()
    }

    /// Get total system latency in milliseconds
    pub fn get_total_latency_ms(&self) -> f32 {
        self.latency_measurement.total_latency_ms(self.performance_profiler.metrics.sample_rate)
    }

    /// Check if latency meets requirements
    pub fn latency_meets_requirements(&self) -> bool {
        self.latency_measurement.meets_requirements(self.performance_profiler.metrics.sample_rate)
    }

    /// Run performance test suite
    pub fn run_performance_tests(&self) -> PerformanceTestSuite {
        let mut test_suite = PerformanceTestSuite::new();
        test_suite.run_all_tests(&self.performance_profiler);
        test_suite
    }

    /// Enable/disable performance optimizations
    pub fn set_optimization_flags(&mut self, flags: OptimizationFlags) {
        self.performance_profiler.optimizations = flags;
        
        // Apply optimizations to audio engine
        if flags.effect_chain_opt {
            self.optimize_effect_chains();
        }
    }

    /// Optimize effect chain processing order
    fn optimize_effect_chains(&mut self) {
        // Optimize Input FX chain
        AudioOptimizer::optimize_effect_order(self.input_fx_mut().effects_mut());
        
        // Optimize Master FX chain
        AudioOptimizer::optimize_effect_order(self.master_fx_mut().effects_mut());
        
        // Optimize Track FX chains
        for track_id in 1..=6 {
            if let Some(track_fx) = self.track_fx_mut(track_id) {
                AudioOptimizer::optimize_effect_order(track_fx.effects_mut());
            }
        }
    }

    /// Count total active effects across all chains
    fn count_active_effects(&self) -> u8 {
        let mut count = 0u8;
        
        // Count Input FX
        count += self.input_fx().active_effect_count() as u8;
        
        // Count Master FX
        count += self.master_fx().active_effect_count() as u8;
        
        // Count Track FX
        for track_id in 1..=6 {
            if let Some(track_fx) = self.track_fx(track_id) {
                count += track_fx.active_effect_count() as u8;
            }
        }
        
        count
    }

    /// Apply modulation values to audio engine parameters
    fn apply_modulation_to_audio_engine(&mut self, modulation_values: &ModulationValues) {
        // Apply modulation to track volumes
        for track_id in 1..=6 {
            let target = ModulationTarget::TrackVolume(track_id);
            if let Some(track) = self.audio_engine.get_track(track_id) {
                let base_level = track.level;
                let modulated_level = self.modulation_system.apply_modulation(&target, base_level, modulation_values);
                
                // Apply modulated level if different from base
                if (modulated_level - base_level).abs() > 0.001 {
                    let _ = self.audio_engine.set_track_level_internal(track_id, modulated_level);
                }
            }
        }

        // Apply modulation to master volume
        let master_target = ModulationTarget::MasterVolume;
        let base_master_level = self.audio_engine.get_master_level();
        let modulated_master_level = self.modulation_system.apply_modulation(&master_target, base_master_level, modulation_values);
        
        if (modulated_master_level - base_master_level).abs() > 0.001 {
            self.audio_engine.set_master_level_internal(modulated_master_level);
        }

        // Apply modulation to effect parameters
        self.apply_modulation_to_effects(modulation_values);
    }

    /// Apply modulation to effect parameters
    fn apply_modulation_to_effects(&mut self, modulation_values: &ModulationValues) {
        // Collect all modulation updates first to avoid borrowing conflicts
        let mut all_updates = heapless::Vec::<(crate::effects::EffectChainType, Option<u8>, usize, usize, f32), 64>::new();
        
        // Collect Input FX modulation
        self.collect_effect_modulation(
            crate::effects::EffectChainType::InputFX, 
            None, 
            modulation_values,
            &mut all_updates
        );

        // Collect Master FX modulation
        self.collect_effect_modulation(
            crate::effects::EffectChainType::MasterFX, 
            None, 
            modulation_values,
            &mut all_updates
        );

        // Collect Track FX modulation for each track
        for track_id in 1..=6 {
            self.collect_effect_modulation(
                crate::effects::EffectChainType::TrackFX, 
                Some(track_id), 
                modulation_values,
                &mut all_updates
            );
        }
        
        // Apply all collected updates
        for (chain_type, track_id, slot_index, param_index, modulated_value) in all_updates {
            self.apply_single_effect_modulation(chain_type, track_id, slot_index, param_index, modulated_value);
        }
    }

    /// Collect modulation updates for a specific effect chain
    fn collect_effect_modulation(
        &self, 
        chain_type: crate::effects::EffectChainType, 
        track_id: Option<u8>,
        modulation_values: &ModulationValues,
        updates: &mut heapless::Vec<(crate::effects::EffectChainType, Option<u8>, usize, usize, f32), 64>
    ) {
        let effect_chain = match chain_type {
            crate::effects::EffectChainType::InputFX => Some(self.input_fx()),
            crate::effects::EffectChainType::MasterFX => Some(self.master_fx()),
            crate::effects::EffectChainType::TrackFX => {
                if let Some(track_id) = track_id {
                    self.track_fx(track_id)
                } else {
                    None
                }
            }
        };

        if let Some(chain) = effect_chain {
            for slot_index in 0..4 {
                if let Some(Some(effect)) = chain.effects().get(slot_index) {
                    for param_index in 0..effect.parameters.len() {
                        let target = ModulationTarget::EffectParameter {
                            chain_type,
                            slot_index: slot_index as u8,
                            param_index: param_index as u8,
                            track_id,
                        };

                        if let Some(param) = effect.get_parameter(param_index) {
                            let base_value = param.value;
                            let modulated_value = self.modulation_system.apply_modulation(&target, base_value, modulation_values);
                            
                            if (modulated_value - base_value).abs() > 0.001 {
                                let _ = updates.push((chain_type, track_id, slot_index, param_index, modulated_value));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Apply a single effect modulation update
    fn apply_single_effect_modulation(
        &mut self,
        chain_type: crate::effects::EffectChainType,
        track_id: Option<u8>,
        slot_index: usize,
        param_index: usize,
        modulated_value: f32
    ) {
        let effect_chain = match chain_type {
            crate::effects::EffectChainType::InputFX => Some(self.input_fx_mut()),
            crate::effects::EffectChainType::MasterFX => Some(self.master_fx_mut()),
            crate::effects::EffectChainType::TrackFX => {
                if let Some(track_id) = track_id {
                    self.track_fx_mut(track_id)
                } else {
                    None
                }
            }
        };

        if let Some(chain) = effect_chain {
            if let Some(Some(effect)) = chain.effects_mut().get_mut(slot_index) {
                effect.set_parameter(param_index, modulated_value);
            }
        }
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

            // Rhythm operations
            ButtonFunction::RhythmStart => {
                self.start_rhythm();
            },
            ButtonFunction::RhythmStop => {
                self.stop_rhythm();
            },
            ButtonFunction::RhythmToggle => {
                self.toggle_rhythm();
            },
            ButtonFunction::RhythmPatternNext => {
                let current = self.rhythm_system.current_pattern;
                let next = (current + 1) % self.rhythm_system.patterns.len();
                self.select_rhythm_pattern(next);
            },
            ButtonFunction::RhythmPatternPrev => {
                let current = self.rhythm_system.current_pattern;
                let prev = if current > 0 { 
                    current - 1 
                } else { 
                    self.rhythm_system.patterns.len().saturating_sub(1)
                };
                self.select_rhythm_pattern(prev);
            },
            ButtonFunction::RhythmPatternSelect(pattern_num) => {
                self.select_rhythm_pattern(pattern_num as usize);
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
                TrackState::Stopped => self.audio_engine.start_playback(track_id, self.system_time_ms),
                TrackState::Playing => self.stop_track(track_id),
                TrackState::Recording => self.stop_track(track_id),
                TrackState::Overdubbing => self.stop_track(track_id),
                TrackState::Muted => self.audio_engine.start_playback(track_id, self.system_time_ms),
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
        let timestamp = self.system_time_ms;
        
        if let Some(effect_chain) = self.get_effect_chain_mut(chain_type, track_id) {
            // Use the new undo-aware parameter setting method
            let _ = effect_chain.set_effect_parameter(slot_index as usize, param_index as usize, value, timestamp, track_id);
            
            // Send MIDI CC if enabled and this is the selected track's first effect
            if self.midi_handler.get_settings().cc_tx_rx {
                if let (crate::effects::EffectChainType::TrackFX, Some(track_id)) = (chain_type, track_id) {
                    if track_id == self.selected_track && slot_index == 0 && param_index < 4 {
                        use crate::midi::cc_mappings::*;
                        let cc_number = FX_1_PARAM_1 + param_index;
                        let midi_value = (value * 127.0).clamp(0.0, 127.0) as u8;
                        let _ = self.send_midi_control_change(cc_number, midi_value);
                    }
                }
            }
        }
    }

    /// Start all tracks simultaneously
    pub fn start_all_tracks(&mut self) {
        for track_id in 1..=6 {
            let _ = self.audio_engine.start_playback(track_id, self.system_time_ms);
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
        let undo_mode = self.settings.general.undo_mode;
        
        // Try to undo track action first
        if self.audio_engine.undo_track_action(self.selected_track, undo_mode) {
            return; // Successfully undone track action
        }
        
        // If no track action to undo, try effect parameter changes
        // Check Track FX first
        if self.audio_engine.undo_effect_parameter(crate::effects::EffectChainType::TrackFX, Some(self.selected_track)) {
            return;
        }
        
        // Then Input FX
        if self.audio_engine.undo_effect_parameter(crate::effects::EffectChainType::InputFX, None) {
            return;
        }
        
        // Finally Master FX
        let _ = self.audio_engine.undo_effect_parameter(crate::effects::EffectChainType::MasterFX, None);
    }

    /// Redo last undone action on selected track
    fn redo_last_action(&mut self) {
        let undo_mode = self.settings.general.undo_mode;
        
        // Try to redo track action first
        if self.audio_engine.redo_track_action(self.selected_track, undo_mode) {
            return; // Successfully redone track action
        }
        
        // If no track action to redo, try effect parameter changes
        // Check Track FX first
        if self.audio_engine.redo_effect_parameter(crate::effects::EffectChainType::TrackFX, Some(self.selected_track)) {
            return;
        }
        
        // Then Input FX
        if self.audio_engine.redo_effect_parameter(crate::effects::EffectChainType::InputFX, None) {
            return;
        }
        
        // Finally Master FX
        let _ = self.audio_engine.redo_effect_parameter(crate::effects::EffectChainType::MasterFX, None);
    }

    /// Process tap tempo input
    fn tap_tempo(&mut self) {
        if self.tempo_system.tap_tempo(self.system_time_ms) {
            // Tempo was updated, sync with audio engine
            self.audio_engine.update_tempo(self.tempo_system.get_bpm());
            
            // Send MIDI CC if enabled
            if self.midi_handler.get_settings().cc_tx_rx {
                use crate::midi::cc_mappings::TEMPO;
                let bpm = self.tempo_system.get_bpm();
                let normalized_tempo = ((bpm - 60.0) / 140.0).clamp(0.0, 1.0);
                let midi_value = (normalized_tempo * 127.0) as u8;
                let _ = self.send_midi_control_change(TEMPO, midi_value);
            }
        }
    }

    /// Reset tempo to default (120 BPM)
    fn reset_tempo(&mut self) {
        self.tempo_system.reset_tempo();
        self.audio_engine.update_tempo(self.tempo_system.get_bpm());
        
        // Send MIDI CC if enabled
        if self.midi_handler.get_settings().cc_tx_rx {
            use crate::midi::cc_mappings::TEMPO;
            let bpm = self.tempo_system.get_bpm();
            let normalized_tempo = ((bpm - 60.0) / 140.0).clamp(0.0, 1.0);
            let midi_value = (normalized_tempo * 127.0) as u8;
            let _ = self.send_midi_control_change(TEMPO, midi_value);
        }
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
            
            // Send MIDI Program Change if enabled
            if self.midi_handler.get_settings().pc_out {
                let _ = self.send_midi_program_change(self.memory.current_memory);
            }
        }
    }

    /// Increment memory slot
    fn increment_memory_slot(&mut self) {
        let old_memory = self.memory.current_memory;
        self.memory.current_memory = (self.memory.current_memory % 255) + 1;
        
        // Send MIDI Program Change if enabled
        if self.midi_handler.get_settings().pc_out {
            let _ = self.send_midi_program_change(self.memory.current_memory);
        }
    }

    /// Decrement memory slot
    fn decrement_memory_slot(&mut self) {
        let old_memory = self.memory.current_memory;
        self.memory.current_memory = if self.memory.current_memory > 1 {
            self.memory.current_memory - 1
        } else {
            255
        };
        
        // Send MIDI Program Change if enabled
        if self.midi_handler.get_settings().pc_out {
            let _ = self.send_midi_program_change(self.memory.current_memory);
        }
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

    /// Get undo count for the selected track
    pub fn get_undo_count(&self) -> usize {
        self.audio_engine.get_track_undo_count(self.selected_track)
    }

    /// Get redo count for the selected track
    pub fn get_redo_count(&self) -> usize {
        self.audio_engine.get_track_redo_count(self.selected_track)
    }

    /// Clear all undo/redo history
    pub fn clear_undo_history(&mut self) {
        self.audio_engine.clear_all_undo_history();
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
                    track.set_pan(action.value, self.system_time_ms);
                    
                    // Send MIDI CC if enabled
                    if self.midi_handler.get_settings().cc_tx_rx {
                        use crate::midi::cc_mappings::TRACK_1_PAN;
                        let cc_number = TRACK_1_PAN + (track_id - 1);
                        let midi_value = (action.value * 127.0).clamp(0.0, 127.0) as u8;
                        let _ = self.send_midi_control_change(cc_number, midi_value);
                    }
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
        
        // Update MIDI handler settings
        let mut settings = self.midi_handler.get_settings().clone();
        settings.midi_channel = if channel == 0 {
            MidiChannel::Omni
        } else {
            MidiChannel::Channel(channel)
        };
        settings.cc_tx_rx = cc_enabled;
        self.midi_handler.set_settings(settings);
    }

    /// Process incoming MIDI message
    fn process_midi_message(&mut self, message: MidiMessage, timestamp: u32) {
        // Check if message should be processed based on channel settings
        if !self.should_process_midi_message(&message) {
            return;
        }

        match message {
            MidiMessage::NoteOn { note, velocity, .. } => {
                self.process_midi_note_on(note, velocity);
            },
            MidiMessage::NoteOff { note, .. } => {
                self.process_midi_note_off(note);
            },
            MidiMessage::ControlChange { controller, value, .. } => {
                self.process_midi_control_change(controller, value);
            },
            MidiMessage::ProgramChange { program, .. } => {
                self.process_midi_program_change(program);
            },
            MidiMessage::Clock => {
                self.process_midi_clock(timestamp);
            },
            MidiMessage::Start => {
                self.process_midi_start();
            },
            MidiMessage::Stop => {
                self.process_midi_stop();
            },
            MidiMessage::Continue => {
                self.process_midi_continue();
            },
            _ => {
                // Other MIDI messages not currently handled
            }
        }
    }

    /// Check if MIDI message should be processed based on channel settings
    fn should_process_midi_message(&self, message: &MidiMessage) -> bool {
        let settings = self.midi_handler.get_settings();
        
        match (&settings.midi_channel, message.channel()) {
            (MidiChannel::Omni, _) => true,
            (MidiChannel::Channel(ch), Some(msg_ch)) => *ch == msg_ch,
            (MidiChannel::Channel(_), None) => true, // System messages (clock, etc.)
        }
    }

    /// Process MIDI Note On message for track control
    fn process_midi_note_on(&mut self, note: u8, velocity: u8) {
        use crate::midi::note_mappings::*;
        
        if velocity == 0 {
            // Note On with velocity 0 is equivalent to Note Off
            self.process_midi_note_off(note);
            return;
        }

        match note {
            TRACK_1_REC_PLAY => { let _ = self.toggle_track_playback(1); },
            TRACK_2_REC_PLAY => { let _ = self.toggle_track_playback(2); },
            TRACK_3_REC_PLAY => { let _ = self.toggle_track_playback(3); },
            TRACK_4_REC_PLAY => { let _ = self.toggle_track_playback(4); },
            TRACK_5_REC_PLAY => { let _ = self.toggle_track_playback(5); },
            TRACK_6_REC_PLAY => { let _ = self.toggle_track_playback(6); },
            
            TRACK_1_STOP => { let _ = self.stop_track(1); },
            TRACK_2_STOP => { let _ = self.stop_track(2); },
            TRACK_3_STOP => { let _ = self.stop_track(3); },
            TRACK_4_STOP => { let _ = self.stop_track(4); },
            TRACK_5_STOP => { let _ = self.stop_track(5); },
            TRACK_6_STOP => { let _ = self.stop_track(6); },
            
            ALL_START => { self.start_all_tracks(); },
            ALL_STOP => { self.stop_all_tracks(); },
            TAP_TEMPO => { self.tap_tempo(); },
            
            _ => {
                // Unknown note - ignore
            }
        }
    }

    /// Process MIDI Note Off message
    fn process_midi_note_off(&mut self, _note: u8) {
        // Note Off messages are not currently used for control
        // Could be used for momentary effects in the future
    }

    /// Process MIDI Control Change message for parameter control
    fn process_midi_control_change(&mut self, controller: u8, value: u8) {
        use crate::midi::cc_mappings::*;
        
        if !self.midi_handler.get_settings().cc_tx_rx {
            return; // CC disabled
        }

        // Convert 7-bit MIDI value (0-127) to normalized float (0.0-1.0)
        let normalized_value = value as f32 / 127.0;

        match controller {
            TRACK_1_VOLUME => { let _ = self.set_track_level(1, normalized_value); },
            TRACK_2_VOLUME => { let _ = self.set_track_level(2, normalized_value); },
            TRACK_3_VOLUME => { let _ = self.set_track_level(3, normalized_value); },
            TRACK_4_VOLUME => { let _ = self.set_track_level(4, normalized_value); },
            TRACK_5_VOLUME => { let _ = self.set_track_level(5, normalized_value); },
            TRACK_6_VOLUME => { let _ = self.set_track_level(6, normalized_value); },
            
            TRACK_1_PAN..=TRACK_6_PAN => {
                let track_id = (controller - TRACK_1_PAN) + 1;
                if let Some(track) = self.audio_engine.get_track_mut(track_id) {
                    track.set_pan(normalized_value, self.system_time_ms);
                }
            },
            
            MASTER_VOLUME => { self.set_master_level(normalized_value); },
            
            TEMPO => {
                // Map CC value to tempo range (60-200 BPM)
                let bpm = 60.0 + (normalized_value * 140.0);
                self.set_tempo(bpm);
            },
            
            FX_1_PARAM_1..=FX_1_PARAM_4 => {
                // Map to currently selected track's first active effect
                let param_index = (controller - FX_1_PARAM_1) as usize;
                if let Some(track_fx) = self.track_fx_mut(self.selected_track) {
                    if let Some(Some(effect)) = track_fx.effects_mut().get_mut(0) {
                        effect.set_parameter(param_index, normalized_value);
                    }
                }
            },
            
            EXPRESSION_1..=EXPRESSION_4 => {
                // Handle expression pedal input via MIDI CC
                let input_index = (controller - EXPRESSION_1) as usize;
                let assignment = self.control_interface
                    .as_ref()
                    .and_then(|ci| ci.control_system().assignments.expression_assignments.get(input_index))
                    .and_then(|opt| opt.as_ref())
                    .cloned();
                    
                if let Some(assignment) = assignment {
                    self.apply_expression_assignment(&assignment, normalized_value);
                }
            },
            
            _ => {
                // Check for custom MIDI CC assignments
                if let Some(control_interface) = &self.control_interface {
                    // Check for custom MIDI CC assignments
                let midi_assignment = control_interface
                    .control_system()
                    .assignments
                    .midi_assignments
                    .iter()
                    .find(|assignment| assignment.cc_number == controller);
                
                if let Some(assignment) = midi_assignment {
                        let action = crate::controls::MidiControlAction {
                            target: assignment.target.clone(),
                            value: normalized_value,
                        };
                        self.execute_midi_action(action);
                    }
                }
            }
        }
    }

    /// Process MIDI Program Change message for memory slot switching
    fn process_midi_program_change(&mut self, program: u8) {
        // Map Program Change to memory slots (PC#0 = Memory 1, PC#1 = Memory 2, etc.)
        let memory_slot = program.saturating_add(1);
        if memory_slot >= 1 && memory_slot <= 255 {
            self.memory.current_memory = memory_slot;
            self.load_project();
        }
    }

    /// Process MIDI Clock message for tempo synchronization
    fn process_midi_clock(&mut self, timestamp: u32) {
        // Process MIDI clock through tempo system
        self.tempo_system.process_midi_clock(timestamp);
        
        // Update audio engine if tempo changed
        let current_bpm = self.audio_engine.get_current_tempo();
        let new_bpm = self.tempo_system.get_bpm();
        if (current_bpm - new_bpm).abs() > 0.1 {
            self.audio_engine.update_tempo(new_bpm);
        }
    }

    /// Process MIDI Start message
    fn process_midi_start(&mut self) {
        self.tempo_system.process_midi_start();
        
        // Start all tracks when MIDI clock starts (if MIDI sync is enabled)
        if self.tempo_system.midi_sync_enabled {
            self.start_all_tracks();
        }
    }

    /// Process MIDI Stop message
    fn process_midi_stop(&mut self) {
        self.tempo_system.process_midi_stop();
        
        // Stop all tracks when MIDI clock stops (if MIDI sync is enabled)
        if self.tempo_system.midi_sync_enabled {
            self.stop_all_tracks();
        }
    }

    /// Process MIDI Continue message
    fn process_midi_continue(&mut self) {
        self.tempo_system.process_midi_continue();
        
        // Resume playback when MIDI clock continues (if MIDI sync is enabled)
        if self.tempo_system.midi_sync_enabled {
            self.start_all_tracks();
        }
    }

    /// Send MIDI Program Change for memory slot changes
    pub fn send_midi_program_change(&mut self, memory_slot: u8) -> Result<(), &'static str> {
        if let Err(_) = self.midi_handler.send_program_change(memory_slot) {
            return Err("Failed to send MIDI Program Change");
        }

        // Transmit via hardware
        if let Some(hal) = self.hal.as_mut() {
            let message = MidiMessage::ProgramChange {
                channel: match self.midi_handler.get_settings().midi_channel {
                    MidiChannel::Channel(ch) => ch,
                    MidiChannel::Omni => 1,
                },
                program: memory_slot.saturating_sub(1),
            };
            
            if let Err(_) = hal.send_midi_message(message) {
                return Err("Failed to transmit MIDI message");
            }
        }

        Ok(())
    }

    /// Send MIDI Control Change for parameter updates
    pub fn send_midi_control_change(&mut self, controller: u8, value: u8) -> Result<(), &'static str> {
        if let Err(_) = self.midi_handler.send_control_change(controller, value) {
            return Err("Failed to send MIDI Control Change");
        }

        // Transmit via hardware
        if let Some(hal) = self.hal.as_mut() {
            let message = MidiMessage::ControlChange {
                channel: match self.midi_handler.get_settings().midi_channel {
                    MidiChannel::Channel(ch) => ch,
                    MidiChannel::Omni => 1,
                },
                controller,
                value,
            };
            
            if let Err(_) = hal.send_midi_message(message) {
                return Err("Failed to transmit MIDI message");
            }
        }

        Ok(())
    }



    /// Get MIDI handler reference
    pub fn midi_handler(&self) -> &MidiHandler {
        &self.midi_handler
    }

    /// Get mutable MIDI handler reference
    pub fn midi_handler_mut(&mut self) -> &mut MidiHandler {
        &mut self.midi_handler
    }

    /// Configure MIDI output settings
    pub fn configure_midi_output(&mut self, pc_out: bool, cc_tx_rx: bool) {
        let mut settings = self.midi_handler.get_settings().clone();
        settings.pc_out = pc_out;
        settings.cc_tx_rx = cc_tx_rx;
        self.midi_handler.set_settings(settings);
    }

    /// Get current MIDI output settings
    pub fn get_midi_output_settings(&self) -> (bool, bool) {
        let settings = self.midi_handler.get_settings();
        (settings.pc_out, settings.cc_tx_rx)
    }

    /// Send MIDI Program Change for external memory slot switching
    pub fn send_memory_change_midi(&mut self, memory_slot: u8) -> Result<(), &'static str> {
        self.send_midi_program_change(memory_slot)
    }

    /// Send MIDI Control Change for external parameter control
    pub fn send_parameter_change_midi(&mut self, controller: u8, value: f32) -> Result<(), &'static str> {
        let midi_value = (value * 127.0).clamp(0.0, 127.0) as u8;
        self.send_midi_control_change(controller, midi_value)
    }

    /// Broadcast current system state via MIDI
    pub fn broadcast_midi_state(&mut self) -> Result<(), &'static str> {
        if !self.midi_handler.get_settings().cc_tx_rx {
            return Ok(()); // CC transmission disabled
        }

        use crate::midi::cc_mappings::*;

        // Send track volumes
        for track_id in 1..=6 {
            if let Some(track) = self.audio_engine.get_track(track_id) {
                let cc_number = match track_id {
                    1 => TRACK_1_VOLUME,
                    2 => TRACK_2_VOLUME,
                    3 => TRACK_3_VOLUME,
                    4 => TRACK_4_VOLUME,
                    5 => TRACK_5_VOLUME,
                    6 => TRACK_6_VOLUME,
                    _ => continue,
                };
                let midi_value = (track.level * 127.0).clamp(0.0, 127.0) as u8;
                self.send_midi_control_change(cc_number, midi_value)?;
            }
        }

        // Send master volume
        let master_level = self.audio_engine.get_master_level();
        let midi_value = (master_level * 127.0).clamp(0.0, 127.0) as u8;
        self.send_midi_control_change(MASTER_VOLUME, midi_value)?;

        // Send tempo
        let current_tempo = self.tempo_system.get_bpm();
        let normalized_tempo = ((current_tempo - 60.0) / 140.0).clamp(0.0, 1.0);
        let midi_value = (normalized_tempo * 127.0) as u8;
        self.send_midi_control_change(TEMPO, midi_value)?;

        // Send current memory slot as Program Change
        if self.midi_handler.get_settings().pc_out {
            self.send_midi_program_change(self.memory.current_memory)?;
        }

        Ok(())
    }

    /// Get system settings reference
    pub fn settings(&self) -> &SystemSettings {
        &self.settings
    }

    /// Get mutable system settings reference
    pub fn settings_mut(&mut self) -> &mut SystemSettings {
        &mut self.settings
    }

    /// Apply system settings to all subsystems
    pub fn apply_system_settings(&mut self) {
        // Apply MIDI settings to MIDI handler
        let midi_settings = crate::midi::MidiSettings {
            midi_channel: match self.settings.midi.midi_channel {
                crate::settings::MidiChannel::Channel(ch) => crate::midi::MidiChannel::Channel(ch),
                crate::settings::MidiChannel::Omni => crate::midi::MidiChannel::Omni,
            },
            local_control: self.settings.midi.local_control,
            pc_out: self.settings.midi.pc_out,
            cc_tx_rx: self.settings.midi.cc_tx_rx,
            clock_sync: true, // Default to enabled
        };
        self.midi_handler.set_settings(midi_settings);

        // Apply memory settings
        self.memory.set_tempo_memory(self.settings.general.tempo_memory);
        self.memory.set_store_mode(match self.settings.general.store_mode {
            crate::settings::StoreMode::LoopAndSetting => crate::storage::StoreMode::Full,
            crate::settings::StoreMode::SettingOnly => crate::storage::StoreMode::SettingOnly,
        });

        // Clock and quantize settings will be applied when needed
        // The audio engine doesn't have direct methods for these yet
        // They will be implemented as part of the audio processing logic
    }

    /// Perform factory reset of all settings
    pub fn factory_reset(&mut self) {
        self.settings.factory_reset();
        self.apply_system_settings();
    }

    /// Initialize system based on selected mode
    pub fn initialize_system(&mut self, mode: InitializeMode) -> Result<(), &'static str> {
        match self.settings.utility.initialize(mode) {
            InitializeAction::ClearAll => {
                // Clear everything
                self.clear_all_tracks();
                self.settings.factory_reset();
                self.memory = MemorySystem::new();
                self.apply_system_settings();
            },
            InitializeAction::ClearSettings => {
                // Clear settings only
                self.settings.factory_reset();
                self.apply_system_settings();
            },
            InitializeAction::ClearLoops => {
                // Clear all audio loops
                self.clear_all_tracks();
            },
            InitializeAction::ClearMemory => {
                // Clear memory slots
                self.memory = MemorySystem::new();
            },
        }
        Ok(())
    }

    /// Save system settings to storage
    pub fn save_system_settings(&self) -> Result<(), SettingsError> {
        // In a real implementation, this would serialize and save settings to persistent storage
        // For now, this is a placeholder
        Ok(())
    }

    /// Load system settings from storage
    pub fn load_system_settings(&mut self) -> Result<(), SettingsError> {
        // In a real implementation, this would load and deserialize settings from persistent storage
        // For now, this is a placeholder
        Ok(())
    }

    /// Handle backup operations
    pub fn handle_backup_operation(&mut self, operation: BackupOperation) -> BackupResult {
        match operation {
            BackupOperation::SaveToUsb => {
                self.settings.backup.save_to_usb()
            },
            BackupOperation::LoadFromUsb => {
                self.settings.backup.load_from_usb()
            },
            BackupOperation::ExportViaManager => {
                self.settings.backup.export_via_manager()
            },
            BackupOperation::ImportViaManager => {
                self.settings.backup.import_via_manager()
            },
        }
    }

    /// Get firmware version
    pub fn get_firmware_version(&self) -> &str {
        &self.settings.utility.firmware_version
    }

    /// Set firmware version (typically done during system initialization)
    pub fn set_firmware_version(&mut self, version: &str) {
        self.settings.utility.firmware_version.clear();
        let truncated = &version[..version.len().min(crate::settings::MAX_FIRMWARE_VERSION_LEN - 1)];
        let _ = self.settings.utility.firmware_version.push_str(truncated);
    }

    /// Format memory storage (WARNING: This will erase all data)
    pub fn format_memory(&mut self) -> Result<(), &'static str> {
        if self.settings.utility.format_memory() {
            // Clear all memory slots
            self.memory = MemorySystem::new();
            // Reset to factory defaults
            self.settings.factory_reset();
            self.apply_system_settings();
            Ok(())
        } else {
            Err("Format operation cancelled")
        }
    }

    /// Apply control settings to the control interface
    pub fn apply_control_settings(&mut self) {
        if let Some(control_interface) = &mut self.control_interface {
            // Apply footswitch assignments
            for (i, assignment) in self.settings.control.foot_sw_assign.iter().enumerate() {
                if let Some(assignment) = assignment {
                    control_interface.set_footswitch_assignment(i, *assignment);
                }
            }

            // Apply CTL function assignment mode
            control_interface.set_ctl_func_assign_mode(self.settings.control.ctl_func_assign);

            // Apply expression pedal mode
            control_interface.set_exp_pedal_mode(self.settings.control.exp_pedal_mode);
        }
    }

    /// Check if tempo memory is enabled
    pub fn is_tempo_memory_enabled(&self) -> bool {
        self.settings.general.tempo_memory
    }

    /// Set tempo memory mode
    pub fn set_tempo_memory(&mut self, enabled: bool) {
        self.settings.general.tempo_memory = enabled;
        self.memory.set_tempo_memory(enabled);
    }

    /// Get current quantize mode
    pub fn get_quantize_mode(&self) -> crate::settings::QuantizeMode {
        self.settings.general.quantize_mode
    }

    /// Set quantize mode
    pub fn set_quantize_mode(&mut self, mode: crate::settings::QuantizeMode) {
        self.settings.general.quantize_mode = mode;
        // Apply to audio engine when those methods are implemented
    }

    /// Get current undo mode
    pub fn get_undo_mode(&self) -> crate::settings::UndoMode {
        self.settings.general.undo_mode
    }

    /// Set undo mode
    pub fn set_undo_mode(&mut self, mode: crate::settings::UndoMode) {
        self.settings.general.undo_mode = mode;
        // Apply to audio engine undo system when implemented
    }

    /// Update backup timestamp
    pub fn update_backup_time(&mut self) {
        self.settings.backup.update_backup_time(self.system_time_ms);
    }

    /// Check if auto backup is due
    pub fn is_auto_backup_due(&self) -> bool {
        self.settings.backup.is_backup_due(self.system_time_ms)
    }
}

/// Backup operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupOperation {
    /// Save to USB drive
    SaveToUsb,
    /// Load from USB drive
    LoadFromUsb,
    /// Export via RC-505mk2 Manager
    ExportViaManager,
    /// Import via RC-505mk2 Manager
    ImportViaManager,
}

impl Default for LoopstationCore {
    fn default() -> Self {
        Self::new()
    }
}