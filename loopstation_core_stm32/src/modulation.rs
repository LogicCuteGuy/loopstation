//! LFO and Step Sequencer modulation system
//! 
//! This module provides LFO generators and Step Sequencers for parameter automation
//! and dynamic movement in the loopstation system.

use heapless::Vec;
use serde::{Deserialize, Serialize};
use micromath::F32Ext;

/// Maximum number of LFOs per system
pub const MAX_LFOS: usize = 8;

/// Maximum number of Step Sequencers per system
pub const MAX_STEP_SEQUENCERS: usize = 4;

/// Maximum steps per Step Sequencer
pub const MAX_STEPS: usize = 16;

/// Maximum modulation assignments per LFO/Step Sequencer
pub const MAX_ASSIGNMENTS: usize = 8;

/// LFO waveform types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoWaveform {
    Sine,
    Triangle,
    Square,
    Sawtooth,
    Random,
}

impl LfoWaveform {
    /// Get the display name for this waveform
    pub fn name(&self) -> &'static str {
        match self {
            LfoWaveform::Sine => "SINE",
            LfoWaveform::Triangle => "TRIANGLE",
            LfoWaveform::Square => "SQUARE",
            LfoWaveform::Sawtooth => "SAWTOOTH",
            LfoWaveform::Random => "RANDOM",
        }
    }

    /// Generate waveform value at given phase (0.0-1.0)
    pub fn generate(&self, phase: f32, random_state: &mut u32) -> f32 {
        match self {
            LfoWaveform::Sine => {
                (phase * 2.0 * core::f32::consts::PI).sin()
            },
            LfoWaveform::Triangle => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            },
            LfoWaveform::Square => {
                if phase < 0.5 { -1.0 } else { 1.0 }
            },
            LfoWaveform::Sawtooth => {
                2.0 * phase - 1.0
            },
            LfoWaveform::Random => {
                // Simple linear congruential generator for random values
                *random_state = random_state.wrapping_mul(1103515245).wrapping_add(12345);
                let normalized = (*random_state as f32) / (u32::MAX as f32);
                2.0 * normalized - 1.0
            },
        }
    }
}

/// LFO sync mode for tempo synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoSyncMode {
    /// Free-running mode (Hz)
    FreeRunning,
    /// Tempo synchronized mode
    TempoSync,
}

/// Tempo sync divisions for LFO
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TempoSyncDivision {
    /// 1/32 note
    ThirtySecond,
    /// 1/16 note
    Sixteenth,
    /// 1/8 note
    Eighth,
    /// 1/4 note
    Quarter,
    /// 1/2 note
    Half,
    /// 1 bar
    WholeNote,
    /// 2 bars
    TwoBars,
    /// 4 bars
    FourBars,
    /// 8 bars
    EightBars,
}

impl TempoSyncDivision {
    /// Get the display name for this division
    pub fn name(&self) -> &'static str {
        match self {
            TempoSyncDivision::ThirtySecond => "1/32",
            TempoSyncDivision::Sixteenth => "1/16",
            TempoSyncDivision::Eighth => "1/8",
            TempoSyncDivision::Quarter => "1/4",
            TempoSyncDivision::Half => "1/2",
            TempoSyncDivision::WholeNote => "1 BAR",
            TempoSyncDivision::TwoBars => "2 BARS",
            TempoSyncDivision::FourBars => "4 BARS",
            TempoSyncDivision::EightBars => "8 BARS",
        }
    }

    /// Get the multiplier for this division (relative to quarter note)
    pub fn multiplier(&self) -> f32 {
        match self {
            TempoSyncDivision::ThirtySecond => 8.0,
            TempoSyncDivision::Sixteenth => 4.0,
            TempoSyncDivision::Eighth => 2.0,
            TempoSyncDivision::Quarter => 1.0,
            TempoSyncDivision::Half => 0.5,
            TempoSyncDivision::WholeNote => 0.25,
            TempoSyncDivision::TwoBars => 0.125,
            TempoSyncDivision::FourBars => 0.0625,
            TempoSyncDivision::EightBars => 0.03125,
        }
    }
}

/// Modulation target for LFO and Step Sequencer assignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModulationTarget {
    /// Track volume (track_id)
    TrackVolume(u8),
    /// Track pan (track_id)
    TrackPan(u8),
    /// Master volume
    MasterVolume,
    /// Effect parameter (chain_type, slot_index, param_index, track_id)
    EffectParameter {
        chain_type: crate::effects::EffectChainType,
        slot_index: u8,
        param_index: u8,
        track_id: Option<u8>,
    },
    /// Filter cutoff frequency
    FilterCutoff,
    /// Filter resonance
    FilterResonance,
    /// LFO rate (for LFO-to-LFO modulation)
    LfoRate(u8),
    /// LFO depth (for LFO-to-LFO modulation)
    LfoDepth(u8),
}

/// Modulation assignment linking a modulation source to a target
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModulationAssignment {
    /// Target parameter to modulate
    pub target: ModulationTarget,
    /// Modulation depth (0.0-1.0)
    pub depth: f32,
    /// Bipolar modulation (-1.0 to +1.0) or unipolar (0.0 to +1.0)
    pub bipolar: bool,
    /// Enabled state
    pub enabled: bool,
}

impl ModulationAssignment {
    /// Create a new modulation assignment
    pub fn new(target: ModulationTarget) -> Self {
        Self {
            target,
            depth: 0.5,
            bipolar: true,
            enabled: true,
        }
    }

    /// Apply modulation value to the target parameter
    pub fn apply_modulation(&self, modulation_value: f32, base_value: f32) -> f32 {
        if !self.enabled {
            return base_value;
        }

        let scaled_modulation = if self.bipolar {
            modulation_value * self.depth
        } else {
            (modulation_value + 1.0) * 0.5 * self.depth
        };

        (base_value + scaled_modulation).clamp(0.0, 1.0)
    }
}

/// LFO (Low Frequency Oscillator) generator
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lfo {
    /// LFO ID (0-7)
    pub id: u8,
    /// Waveform type
    pub waveform: LfoWaveform,
    /// Rate in Hz (free-running mode) or tempo sync division
    pub rate_hz: f32,
    /// Tempo sync division (when in tempo sync mode)
    pub tempo_division: TempoSyncDivision,
    /// Sync mode (free-running or tempo sync)
    pub sync_mode: LfoSyncMode,
    /// Depth (0.0-1.0)
    pub depth: f32,
    /// Phase offset (0.0-1.0)
    pub phase_offset: f32,
    /// Current phase (0.0-1.0)
    pub current_phase: f32,
    /// Random state for random waveform
    pub random_state: u32,
    /// Enabled state
    pub enabled: bool,
    /// Modulation assignments
    pub assignments: Vec<ModulationAssignment, MAX_ASSIGNMENTS>,
    /// Sample rate for phase calculation
    sample_rate: f32,
}

impl Lfo {
    /// Create a new LFO
    pub fn new(id: u8, sample_rate: f32) -> Self {
        Self {
            id,
            waveform: LfoWaveform::Sine,
            rate_hz: 1.0,
            tempo_division: TempoSyncDivision::Quarter,
            sync_mode: LfoSyncMode::FreeRunning,
            depth: 0.5,
            phase_offset: 0.0,
            current_phase: 0.0,
            random_state: 12345 + id as u32, // Different seed per LFO
            enabled: false,
            assignments: Vec::new(),
            sample_rate,
        }
    }

    /// Set LFO rate in Hz (free-running mode)
    pub fn set_rate_hz(&mut self, rate: f32) {
        self.rate_hz = rate.clamp(0.1, 20.0);
    }

    /// Set tempo sync division
    pub fn set_tempo_division(&mut self, division: TempoSyncDivision) {
        self.tempo_division = division;
    }

    /// Set sync mode
    pub fn set_sync_mode(&mut self, mode: LfoSyncMode) {
        self.sync_mode = mode;
    }

    /// Set depth (0.0-1.0)
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 1.0);
    }

    /// Set phase offset (0.0-1.0)
    pub fn set_phase_offset(&mut self, offset: f32) {
        self.phase_offset = offset.clamp(0.0, 1.0);
    }

    /// Add modulation assignment
    pub fn add_assignment(&mut self, assignment: ModulationAssignment) -> Result<(), ()> {
        self.assignments.push(assignment).map_err(|_| ())
    }

    /// Remove modulation assignment by index
    pub fn remove_assignment(&mut self, index: usize) -> Option<ModulationAssignment> {
        if index < self.assignments.len() {
            Some(self.assignments.swap_remove(index))
        } else {
            None
        }
    }

    /// Update LFO phase and generate output value
    pub fn update(&mut self, tempo_bpm: f32, samples_per_update: u32) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        // Calculate phase increment based on sync mode
        let frequency = match self.sync_mode {
            LfoSyncMode::FreeRunning => self.rate_hz,
            LfoSyncMode::TempoSync => {
                // Convert tempo to frequency based on division
                let quarter_note_freq = tempo_bpm / 60.0;
                quarter_note_freq * self.tempo_division.multiplier()
            }
        };

        let phase_increment = frequency * (samples_per_update as f32) / self.sample_rate;
        self.current_phase = (self.current_phase + phase_increment) % 1.0;

        // Apply phase offset
        let offset_phase = (self.current_phase + self.phase_offset) % 1.0;

        // Generate waveform value
        let raw_value = self.waveform.generate(offset_phase, &mut self.random_state);

        // Apply depth
        raw_value * self.depth
    }

    /// Reset LFO phase
    pub fn reset_phase(&mut self) {
        self.current_phase = 0.0;
    }

    /// Sync to tempo system beat position
    pub fn sync_to_beat(&mut self, beat_position: f32) {
        if self.sync_mode == LfoSyncMode::TempoSync {
            // Sync phase to beat position based on tempo division
            let cycles_per_beat = self.tempo_division.multiplier();
            self.current_phase = (beat_position * cycles_per_beat) % 1.0;
        }
    }
}

/// Step in a Step Sequencer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequencerStep {
    /// Step value (0.0-1.0)
    pub value: f32,
    /// Gate length (0.0-1.0, 1.0 = full step)
    pub gate_length: f32,
    /// Velocity/intensity (0.0-1.0)
    pub velocity: f32,
    /// Step enabled
    pub enabled: bool,
}

impl SequencerStep {
    /// Create a new sequencer step
    pub fn new() -> Self {
        Self {
            value: 0.5,
            gate_length: 0.5,
            velocity: 1.0,
            enabled: true,
        }
    }
}

impl Default for SequencerStep {
    fn default() -> Self {
        Self::new()
    }
}

/// Step Sequencer for parameter automation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepSequencer {
    /// Sequencer ID (0-3)
    pub id: u8,
    /// Steps in the sequence
    pub steps: Vec<SequencerStep, MAX_STEPS>,
    /// Number of active steps (1-16)
    pub step_count: u8,
    /// Current step position (0-15)
    pub current_step: u8,
    /// Tempo sync division
    pub tempo_division: TempoSyncDivision,
    /// Swing amount (0.0-1.0)
    pub swing: f32,
    /// Current step phase (0.0-1.0)
    pub step_phase: f32,
    /// Enabled state
    pub enabled: bool,
    /// Modulation assignments
    pub assignments: Vec<ModulationAssignment, MAX_ASSIGNMENTS>,
    /// Sample rate for timing calculation
    sample_rate: f32,
}

impl StepSequencer {
    /// Create a new Step Sequencer
    pub fn new(id: u8, sample_rate: f32) -> Self {
        let mut sequencer = Self {
            id,
            steps: Vec::new(),
            step_count: 16,
            current_step: 0,
            tempo_division: TempoSyncDivision::Sixteenth,
            swing: 0.0,
            step_phase: 0.0,
            enabled: false,
            assignments: Vec::new(),
            sample_rate,
        };

        // Initialize with default steps
        for _ in 0..MAX_STEPS {
            let _ = sequencer.steps.push(SequencerStep::new());
        }

        sequencer
    }

    /// Set number of active steps (1-16)
    pub fn set_step_count(&mut self, count: u8) {
        self.step_count = count.clamp(1, MAX_STEPS as u8);
        if self.current_step >= self.step_count {
            self.current_step = 0;
        }
    }

    /// Set tempo division
    pub fn set_tempo_division(&mut self, division: TempoSyncDivision) {
        self.tempo_division = division;
    }

    /// Set swing amount (0.0-1.0)
    pub fn set_swing(&mut self, swing: f32) {
        self.swing = swing.clamp(0.0, 1.0);
    }

    /// Get step by index
    pub fn get_step(&self, index: u8) -> Option<&SequencerStep> {
        self.steps.get(index as usize)
    }

    /// Get mutable step by index
    pub fn get_step_mut(&mut self, index: u8) -> Option<&mut SequencerStep> {
        self.steps.get_mut(index as usize)
    }

    /// Set step value
    pub fn set_step_value(&mut self, index: u8, value: f32) {
        if let Some(step) = self.get_step_mut(index) {
            step.value = value.clamp(0.0, 1.0);
        }
    }

    /// Set step gate length
    pub fn set_step_gate(&mut self, index: u8, gate: f32) {
        if let Some(step) = self.get_step_mut(index) {
            step.gate_length = gate.clamp(0.0, 1.0);
        }
    }

    /// Set step velocity
    pub fn set_step_velocity(&mut self, index: u8, velocity: f32) {
        if let Some(step) = self.get_step_mut(index) {
            step.velocity = velocity.clamp(0.0, 1.0);
        }
    }

    /// Enable/disable step
    pub fn set_step_enabled(&mut self, index: u8, enabled: bool) {
        if let Some(step) = self.get_step_mut(index) {
            step.enabled = enabled;
        }
    }

    /// Add modulation assignment
    pub fn add_assignment(&mut self, assignment: ModulationAssignment) -> Result<(), ()> {
        self.assignments.push(assignment).map_err(|_| ())
    }

    /// Remove modulation assignment by index
    pub fn remove_assignment(&mut self, index: usize) -> Option<ModulationAssignment> {
        if index < self.assignments.len() {
            Some(self.assignments.swap_remove(index))
        } else {
            None
        }
    }

    /// Update Step Sequencer and generate output value
    pub fn update(&mut self, tempo_bpm: f32, samples_per_update: u32) -> f32 {
        if !self.enabled || self.step_count == 0 {
            return 0.0;
        }

        // Calculate step frequency based on tempo and division
        let quarter_note_freq = tempo_bpm / 60.0;
        let step_freq = quarter_note_freq * self.tempo_division.multiplier();
        
        // Calculate phase increment
        let phase_increment = step_freq * (samples_per_update as f32) / self.sample_rate;
        self.step_phase += phase_increment;

        // Check for step advance
        if self.step_phase >= 1.0 {
            self.step_phase -= 1.0;
            self.current_step = (self.current_step + 1) % self.step_count;
        }

        // Get current step
        if let Some(step) = self.get_step(self.current_step) {
            if step.enabled {
                // Apply swing timing
                let swing_offset = if self.current_step % 2 == 1 {
                    self.swing * 0.5 // Delay odd steps
                } else {
                    0.0
                };

                let adjusted_phase = (self.step_phase + swing_offset).clamp(0.0, 1.0);

                // Check if we're within the gate time
                if adjusted_phase < step.gate_length {
                    // Generate step output value
                    step.value * step.velocity
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Reset sequencer to first step
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.step_phase = 0.0;
    }

    /// Sync to tempo system beat position
    pub fn sync_to_beat(&mut self, beat_position: f32) {
        // Calculate which step we should be on based on beat position
        let steps_per_beat = self.tempo_division.multiplier();
        let total_step_position = beat_position * steps_per_beat * (self.step_count as f32);
        
        self.current_step = (total_step_position as u8) % self.step_count;
        self.step_phase = total_step_position.fract();
    }
}

/// Modulation system managing LFOs and Step Sequencers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModulationSystem {
    /// LFO generators
    pub lfos: Vec<Lfo, MAX_LFOS>,
    /// Step Sequencers
    pub step_sequencers: Vec<StepSequencer, MAX_STEP_SEQUENCERS>,
    /// Sample rate
    sample_rate: f32,
}

impl ModulationSystem {
    /// Create a new modulation system
    pub fn new(sample_rate: f32) -> Self {
        let mut system = Self {
            lfos: Vec::new(),
            step_sequencers: Vec::new(),
            sample_rate,
        };

        // Initialize LFOs
        for i in 0..MAX_LFOS {
            let _ = system.lfos.push(Lfo::new(i as u8, sample_rate));
        }

        // Initialize Step Sequencers
        for i in 0..MAX_STEP_SEQUENCERS {
            let _ = system.step_sequencers.push(StepSequencer::new(i as u8, sample_rate));
        }

        system
    }

    /// Get LFO by ID
    pub fn get_lfo(&self, id: u8) -> Option<&Lfo> {
        self.lfos.get(id as usize)
    }

    /// Get mutable LFO by ID
    pub fn get_lfo_mut(&mut self, id: u8) -> Option<&mut Lfo> {
        self.lfos.get_mut(id as usize)
    }

    /// Get Step Sequencer by ID
    pub fn get_step_sequencer(&self, id: u8) -> Option<&StepSequencer> {
        self.step_sequencers.get(id as usize)
    }

    /// Get mutable Step Sequencer by ID
    pub fn get_step_sequencer_mut(&mut self, id: u8) -> Option<&mut StepSequencer> {
        self.step_sequencers.get_mut(id as usize)
    }

    /// Update all modulation sources and return modulation values
    pub fn update(&mut self, tempo_bpm: f32, samples_per_update: u32) -> ModulationValues {
        let mut values = ModulationValues::new();

        // Update LFOs
        for lfo in &mut self.lfos {
            let value = lfo.update(tempo_bpm, samples_per_update);
            values.lfo_values[lfo.id as usize] = value;
        }

        // Update Step Sequencers
        for sequencer in &mut self.step_sequencers {
            let value = sequencer.update(tempo_bpm, samples_per_update);
            values.step_sequencer_values[sequencer.id as usize] = value;
        }

        values
    }

    /// Sync all modulation sources to tempo system
    pub fn sync_to_tempo(&mut self, beat_position: f32) {
        // Sync LFOs
        for lfo in &mut self.lfos {
            lfo.sync_to_beat(beat_position);
        }

        // Sync Step Sequencers
        for sequencer in &mut self.step_sequencers {
            sequencer.sync_to_beat(beat_position);
        }
    }

    /// Reset all modulation sources
    pub fn reset_all(&mut self) {
        for lfo in &mut self.lfos {
            lfo.reset_phase();
        }

        for sequencer in &mut self.step_sequencers {
            sequencer.reset();
        }
    }

    /// Apply modulation to a target parameter
    pub fn apply_modulation(&self, target: &ModulationTarget, base_value: f32, values: &ModulationValues) -> f32 {
        let mut result = base_value;

        // Apply LFO modulation
        for lfo in &self.lfos {
            if lfo.enabled {
                for assignment in &lfo.assignments {
                    if assignment.target == *target {
                        let lfo_value = values.lfo_values[lfo.id as usize];
                        result = assignment.apply_modulation(lfo_value, result);
                    }
                }
            }
        }

        // Apply Step Sequencer modulation
        for sequencer in &self.step_sequencers {
            if sequencer.enabled {
                for assignment in &sequencer.assignments {
                    if assignment.target == *target {
                        let seq_value = values.step_sequencer_values[sequencer.id as usize];
                        result = assignment.apply_modulation(seq_value, result);
                    }
                }
            }
        }

        result
    }

    /// Get modulation activity status for display
    pub fn get_modulation_activity(&self) -> ModulationActivity {
        ModulationActivity {
            active_lfos: self.lfos.iter().filter(|lfo| lfo.enabled).count() as u8,
            active_step_sequencers: self.step_sequencers.iter().filter(|seq| seq.enabled).count() as u8,
            total_assignments: self.lfos.iter().map(|lfo| lfo.assignments.len()).sum::<usize>() +
                              self.step_sequencers.iter().map(|seq| seq.assignments.len()).sum::<usize>(),
        }
    }
}

/// Modulation values from all sources
#[derive(Debug, Clone, PartialEq)]
pub struct ModulationValues {
    /// LFO output values (-1.0 to +1.0)
    pub lfo_values: [f32; MAX_LFOS],
    /// Step Sequencer output values (0.0 to +1.0)
    pub step_sequencer_values: [f32; MAX_STEP_SEQUENCERS],
}

impl ModulationValues {
    /// Create new modulation values (all zeros)
    pub fn new() -> Self {
        Self {
            lfo_values: [0.0; MAX_LFOS],
            step_sequencer_values: [0.0; MAX_STEP_SEQUENCERS],
        }
    }
}

/// Modulation activity status for display
#[derive(Debug, Clone, PartialEq)]
pub struct ModulationActivity {
    /// Number of active LFOs
    pub active_lfos: u8,
    /// Number of active Step Sequencers
    pub active_step_sequencers: u8,
    /// Total number of modulation assignments
    pub total_assignments: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfo_waveforms() {
        let mut random_state = 12345;
        
        // Test sine wave
        let sine_0 = LfoWaveform::Sine.generate(0.0, &mut random_state);
        let sine_quarter = LfoWaveform::Sine.generate(0.25, &mut random_state);
        let sine_half = LfoWaveform::Sine.generate(0.5, &mut random_state);
        
        assert!((sine_0 - 0.0).abs() < 0.001);
        assert!(sine_quarter > 0.9);
        assert!((sine_half - 0.0).abs() < 0.001);
        
        // Test triangle wave
        let tri_0 = LfoWaveform::Triangle.generate(0.0, &mut random_state);
        let tri_quarter = LfoWaveform::Triangle.generate(0.25, &mut random_state);
        let tri_half = LfoWaveform::Triangle.generate(0.5, &mut random_state);
        
        assert!((tri_0 - (-1.0)).abs() < 0.001);
        assert!((tri_quarter - 0.0).abs() < 0.001);
        assert!((tri_half - 1.0).abs() < 0.001);
        
        // Test square wave
        let square_0 = LfoWaveform::Square.generate(0.0, &mut random_state);
        let square_half = LfoWaveform::Square.generate(0.5, &mut random_state);
        
        assert_eq!(square_0, -1.0);
        assert_eq!(square_half, 1.0);
    }

    #[test]
    fn test_lfo_creation_and_update() {
        let mut lfo = Lfo::new(0, 44100.0);
        lfo.enabled = true;
        lfo.set_rate_hz(1.0); // 1 Hz
        
        // Update for 1/4 second (should advance phase by 0.25)
        let value = lfo.update(120.0, 11025); // 1/4 second at 44.1kHz
        
        assert!(lfo.current_phase > 0.2 && lfo.current_phase < 0.3);
        assert!(value.abs() > 0.0); // Should generate some output
    }

    #[test]
    fn test_step_sequencer_creation() {
        let mut sequencer = StepSequencer::new(0, 44100.0);
        sequencer.enabled = true;
        
        // Set some step values
        sequencer.set_step_value(0, 1.0);
        sequencer.set_step_value(1, 0.5);
        sequencer.set_step_value(2, 0.0);
        
        assert_eq!(sequencer.get_step(0).unwrap().value, 1.0);
        assert_eq!(sequencer.get_step(1).unwrap().value, 0.5);
        assert_eq!(sequencer.get_step(2).unwrap().value, 0.0);
    }

    #[test]
    fn test_modulation_assignment() {
        let target = ModulationTarget::TrackVolume(1);
        let assignment = ModulationAssignment::new(target);
        
        // Test bipolar modulation
        let result = assignment.apply_modulation(0.5, 0.5); // 50% modulation on 50% base
        assert!(result > 0.5 && result < 0.75);
        
        // Test unipolar modulation
        let mut unipolar_assignment = assignment.clone();
        unipolar_assignment.bipolar = false;
        let result = unipolar_assignment.apply_modulation(0.5, 0.5);
        assert!(result > 0.5 && result < 0.75);
    }

    #[test]
    fn test_modulation_system() {
        let mut system = ModulationSystem::new(44100.0);
        
        // Enable first LFO
        if let Some(lfo) = system.get_lfo_mut(0) {
            lfo.enabled = true;
            lfo.set_rate_hz(2.0);
        }
        
        // Enable first Step Sequencer
        if let Some(sequencer) = system.get_step_sequencer_mut(0) {
            sequencer.enabled = true;
            sequencer.set_step_count(4);
        }
        
        let values = system.update(120.0, 1024);
        
        // Should have some modulation values
        assert!(values.lfo_values[0].abs() >= 0.0);
        assert!(values.step_sequencer_values[0] >= 0.0);
        
        let activity = system.get_modulation_activity();
        assert_eq!(activity.active_lfos, 1);
        assert_eq!(activity.active_step_sequencers, 1);
    }
}