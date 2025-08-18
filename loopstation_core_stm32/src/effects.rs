use heapless::Vec;
use serde::{Deserialize, Serialize};
use micromath::F32Ext;

/// Maximum number of effect slots per chain
pub const MAX_EFFECT_SLOTS: usize = 4;

/// Effect chain types corresponding to the 3-layer FX architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectChainType {
    /// Input FX - applied before recording (affects recorded audio)
    InputFX,
    /// Track FX - applied to individual track playback (post-recording)
    TrackFX,
    /// Master FX - applied to final output (affects all tracks)
    MasterFX,
}

/// Core effect types available in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    // Loop Management Effects
    Slicer,
    BeatRepeat,
    Reverse,
    
    // Time-Based Effects
    TapeEcho,
    SpaceReverb,
    T3Delay,
    Chorus,
    Flanger,
    
    // Dynamics Effects
    Compressor,
    NoiseSuppressor,
    Limiter,
    MultibandCompressor,
    
    // Filter Effects
    AutoWah,
    Isolator,
    DJFilter,
    MasteringEQ,
    
    // Pitch/Modulation Effects
    PitchShift,
    PitchCorrect,
    
    // Amp Simulation (COSM)
    JC120,
    Tweed,
    Metal,
    
    // Utility Effects
    Mixer,
    Sidechain,
}

impl EffectType {
    /// Get the display name for this effect type
    pub fn name(&self) -> &'static str {
        match self {
            EffectType::Slicer => "SLICER",
            EffectType::BeatRepeat => "BEAT REPEAT",
            EffectType::Reverse => "REVERSE",
            EffectType::TapeEcho => "TAPE ECHO",
            EffectType::SpaceReverb => "SPACE REVERB",
            EffectType::T3Delay => "T3 DELAY",
            EffectType::Chorus => "CHORUS",
            EffectType::Flanger => "FLANGER",
            EffectType::Compressor => "COMPRESSOR",
            EffectType::NoiseSuppressor => "NOISE SUPPRESSOR",
            EffectType::Limiter => "LIMITER",
            EffectType::MultibandCompressor => "MULTIBAND COMP",
            EffectType::AutoWah => "AUTO WAH",
            EffectType::Isolator => "ISOLATOR",
            EffectType::DJFilter => "DJ FILTER",
            EffectType::MasteringEQ => "MASTERING EQ",
            EffectType::PitchShift => "PITCH SHIFT",
            EffectType::PitchCorrect => "PITCH CORRECT",
            EffectType::JC120 => "JC-120",
            EffectType::Tweed => "TWEED",
            EffectType::Metal => "METAL",
            EffectType::Mixer => "MIXER",
            EffectType::Sidechain => "SIDECHAIN",
        }
    }

    /// Check if this effect supports MIDI tempo synchronization
    pub fn supports_tempo_sync(&self) -> bool {
        matches!(self, 
            EffectType::TapeEcho | 
            EffectType::T3Delay | 
            EffectType::BeatRepeat |
            EffectType::Slicer |
            EffectType::Chorus |
            EffectType::Flanger
        )
    }

    /// Get the default parameter count for this effect
    pub fn parameter_count(&self) -> usize {
        match self {
            EffectType::Slicer => 4,        // Rate, Depth, Attack, Release
            EffectType::BeatRepeat => 4,    // Rate, Gate, Mix, Feedback
            EffectType::Reverse => 2,       // Mix, Gate
            EffectType::TapeEcho => 4,      // Time, Feedback, Mix, Tone
            EffectType::SpaceReverb => 4,   // Time, Pre-delay, Mix, Tone
            EffectType::T3Delay => 4,       // Time, Feedback, Mix, Spread
            EffectType::Chorus => 4,        // Rate, Depth, Mix, Feedback
            EffectType::Flanger => 4,       // Rate, Depth, Mix, Feedback
            EffectType::Compressor => 4,    // Threshold, Ratio, Attack, Release
            EffectType::NoiseSuppressor => 2, // Threshold, Release
            EffectType::Limiter => 3,       // Threshold, Release, Output
            EffectType::MultibandCompressor => 4, // Low, Mid, High, Output
            EffectType::AutoWah => 4,       // Sensitivity, Frequency, Peak, Mix
            EffectType::Isolator => 3,      // Low, Mid, High
            EffectType::DJFilter => 2,      // Cutoff, Resonance
            EffectType::MasteringEQ => 4,   // Low, Low-Mid, High-Mid, High
            EffectType::PitchShift => 3,    // Pitch, Fine, Mix
            EffectType::PitchCorrect => 3,  // Key, Scale, Strength
            EffectType::JC120 => 4,         // Gain, Bass, Treble, Volume
            EffectType::Tweed => 4,         // Gain, Bass, Treble, Volume
            EffectType::Metal => 4,         // Gain, Bass, Treble, Volume
            EffectType::Mixer => 4,         // Input A, Input B, Balance, Output
            EffectType::Sidechain => 3,     // Threshold, Ratio, Mix
        }
    }
}

/// Effect parameter with value and metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectParameter {
    /// Parameter value (0.0 to 1.0 normalized)
    pub value: f32,
    /// Parameter name for display
    pub name: &'static str,
    /// Minimum value in actual units
    pub min: f32,
    /// Maximum value in actual units
    pub max: f32,
    /// Units for display (Hz, dB, ms, etc.)
    pub units: &'static str,
}

impl EffectParameter {
    /// Create a new effect parameter
    pub fn new(name: &'static str, min: f32, max: f32, units: &'static str) -> Self {
        Self {
            value: 0.5, // Default to middle value
            name,
            min,
            max,
            units,
        }
    }

    /// Get the actual parameter value in its units
    pub fn actual_value(&self) -> f32 {
        self.min + (self.max - self.min) * self.value
    }

    /// Set the parameter from an actual value
    pub fn set_actual_value(&mut self, actual: f32) {
        let clamped = actual.clamp(self.min, self.max);
        self.value = (clamped - self.min) / (self.max - self.min);
    }

    /// Set the normalized parameter value (0.0 to 1.0)
    pub fn set_normalized(&mut self, normalized: f32) {
        self.value = normalized.clamp(0.0, 1.0);
    }
}

/// Individual effect instance
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// Type of effect
    pub effect_type: EffectType,
    /// Effect parameters
    pub parameters: Vec<EffectParameter, 8>, // Max 8 parameters per effect
    /// Whether effect is enabled
    pub enabled: bool,
    /// Momentary mode (for FX button short press)
    pub momentary: bool,
    /// MIDI tempo synchronization enabled
    pub midi_sync: bool,
    /// Dry/wet mix (0.0 = dry, 1.0 = wet)
    pub dry_wet_mix: f32,
}

impl Effect {
    /// Create a new effect of the specified type
    pub fn new(effect_type: EffectType) -> Self {
        let mut effect = Self {
            effect_type,
            parameters: Vec::new(),
            enabled: false,
            momentary: false,
            midi_sync: false,
            dry_wet_mix: 0.5,
        };

        // Initialize default parameters based on effect type
        effect.init_default_parameters();
        effect
    }

    /// Initialize default parameters for this effect type
    fn init_default_parameters(&mut self) {
        self.parameters.clear();
        
        match self.effect_type {
            EffectType::Slicer => {
                let _ = self.parameters.push(EffectParameter::new("RATE", 0.1, 20.0, "Hz"));
                let _ = self.parameters.push(EffectParameter::new("DEPTH", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("ATTACK", 1.0, 100.0, "ms"));
                let _ = self.parameters.push(EffectParameter::new("RELEASE", 1.0, 1000.0, "ms"));
            },
            EffectType::TapeEcho => {
                let _ = self.parameters.push(EffectParameter::new("TIME", 1.0, 2000.0, "ms"));
                let _ = self.parameters.push(EffectParameter::new("FEEDBACK", 0.0, 95.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("MIX", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("TONE", -50.0, 50.0, ""));
            },
            EffectType::Compressor => {
                let _ = self.parameters.push(EffectParameter::new("THRESHOLD", -60.0, 0.0, "dB"));
                let _ = self.parameters.push(EffectParameter::new("RATIO", 1.0, 20.0, ":1"));
                let _ = self.parameters.push(EffectParameter::new("ATTACK", 0.1, 100.0, "ms"));
                let _ = self.parameters.push(EffectParameter::new("RELEASE", 1.0, 1000.0, "ms"));
            },
            EffectType::SpaceReverb => {
                let _ = self.parameters.push(EffectParameter::new("TIME", 0.1, 10.0, "s"));
                let _ = self.parameters.push(EffectParameter::new("PRE-DLY", 0.0, 200.0, "ms"));
                let _ = self.parameters.push(EffectParameter::new("MIX", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("TONE", -50.0, 50.0, ""));
            },
            EffectType::T3Delay => {
                let _ = self.parameters.push(EffectParameter::new("TIME", 1.0, 2000.0, "ms"));
                let _ = self.parameters.push(EffectParameter::new("FEEDBACK", 0.0, 95.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("MIX", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("SPREAD", 0.0, 100.0, "%"));
            },
            EffectType::MasteringEQ => {
                let _ = self.parameters.push(EffectParameter::new("LOW", -15.0, 15.0, "dB"));
                let _ = self.parameters.push(EffectParameter::new("LOW-MID", -15.0, 15.0, "dB"));
                let _ = self.parameters.push(EffectParameter::new("HI-MID", -15.0, 15.0, "dB"));
                let _ = self.parameters.push(EffectParameter::new("HIGH", -15.0, 15.0, "dB"));
            },
            EffectType::BeatRepeat => {
                let _ = self.parameters.push(EffectParameter::new("RATE", 0.25, 4.0, "x"));
                let _ = self.parameters.push(EffectParameter::new("GATE", 10.0, 90.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("MIX", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("FEEDBACK", 0.0, 95.0, "%"));
            },
            EffectType::Reverse => {
                let _ = self.parameters.push(EffectParameter::new("MIX", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("GATE", 10.0, 90.0, "%"));
            },
            EffectType::Chorus => {
                let _ = self.parameters.push(EffectParameter::new("RATE", 0.1, 10.0, "Hz"));
                let _ = self.parameters.push(EffectParameter::new("DEPTH", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("MIX", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("FEEDBACK", 0.0, 95.0, "%"));
            },
            EffectType::Flanger => {
                let _ = self.parameters.push(EffectParameter::new("RATE", 0.1, 10.0, "Hz"));
                let _ = self.parameters.push(EffectParameter::new("DEPTH", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("MIX", 0.0, 100.0, "%"));
                let _ = self.parameters.push(EffectParameter::new("FEEDBACK", 0.0, 95.0, "%"));
            },
            EffectType::NoiseSuppressor => {
                let _ = self.parameters.push(EffectParameter::new("THRESHOLD", -60.0, 0.0, "dB"));
                let _ = self.parameters.push(EffectParameter::new("RELEASE", 1.0, 1000.0, "ms"));
            },
            EffectType::Limiter => {
                let _ = self.parameters.push(EffectParameter::new("THRESHOLD", -20.0, 0.0, "dB"));
                let _ = self.parameters.push(EffectParameter::new("RELEASE", 1.0, 100.0, "ms"));
                let _ = self.parameters.push(EffectParameter::new("OUTPUT", -20.0, 20.0, "dB"));
            },
            // Add more effect parameter initializations as needed
            _ => {
                // Generic parameters for unspecified effects
                let _ = self.parameters.push(EffectParameter::new("PARAM1", 0.0, 100.0, ""));
                let _ = self.parameters.push(EffectParameter::new("PARAM2", 0.0, 100.0, ""));
            }
        }
    }

    /// Get parameter by index
    pub fn get_parameter(&self, index: usize) -> Option<&EffectParameter> {
        self.parameters.get(index)
    }

    /// Get mutable parameter by index
    pub fn get_parameter_mut(&mut self, index: usize) -> Option<&mut EffectParameter> {
        self.parameters.get_mut(index)
    }

    /// Set parameter value by index
    pub fn set_parameter(&mut self, index: usize, value: f32) {
        if let Some(param) = self.parameters.get_mut(index) {
            param.set_normalized(value);
        }
    }

    /// Set parameter by actual value (in its units)
    pub fn set_parameter_actual(&mut self, index: usize, actual_value: f32) {
        if let Some(param) = self.parameters.get_mut(index) {
            param.set_actual_value(actual_value);
        }
    }

    /// Get parameter actual value by index
    pub fn get_parameter_actual(&self, index: usize) -> Option<f32> {
        self.parameters.get(index).map(|p| p.actual_value())
    }

    /// Enable/disable the effect
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Toggle effect enabled state
    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Set momentary mode (for FX button short press)
    pub fn set_momentary(&mut self, momentary: bool) {
        self.momentary = momentary;
    }

    /// Set MIDI tempo synchronization
    pub fn set_midi_sync(&mut self, sync: bool) {
        if self.effect_type.supports_tempo_sync() {
            self.midi_sync = sync;
        }
    }

    /// Set dry/wet mix
    pub fn set_dry_wet_mix(&mut self, mix: f32) {
        self.dry_wet_mix = mix.clamp(0.0, 1.0);
    }

    /// Update effect parameters based on tempo (for tempo-synced effects)
    pub fn update_tempo(&mut self, bpm: f32) {
        if self.midi_sync && self.effect_type.supports_tempo_sync() {
            match self.effect_type {
                EffectType::TapeEcho | EffectType::T3Delay => {
                    // Sync delay time to tempo (quarter note = 60000ms / BPM)
                    let quarter_note_ms = 60000.0 / bpm;
                    if let Some(time_param) = self.get_parameter_mut(0) {
                        time_param.set_actual_value(quarter_note_ms);
                    }
                },
                EffectType::Chorus | EffectType::Flanger => {
                    // Sync LFO rate to tempo
                    let lfo_rate = bpm / 60.0; // 1 Hz at 60 BPM
                    if let Some(rate_param) = self.get_parameter_mut(0) {
                        rate_param.set_actual_value(lfo_rate);
                    }
                },
                _ => {} // Other effects don't need tempo sync
            }
        }
    }

    /// Update effect state (called from main loop)
    pub fn update(&mut self) {
        // Handle momentary effects
        if self.momentary {
            // Momentary effects are automatically disabled after processing
            // This would be handled by a timer in a full implementation
        }
        
        // Update internal effect state if needed
        // This is where effects would update their internal buffers, LFOs, etc.
    }

    /// Process audio through this effect
    pub fn process_audio(&mut self, input: &[f32], output: &mut [f32], sample_rate: f32) {
        if !self.enabled {
            // Effect bypassed - copy input to output
            let len = input.len().min(output.len());
            output[..len].copy_from_slice(&input[..len]);
            return;
        }

        match self.effect_type {
            EffectType::Compressor => self.process_compressor(input, output, sample_rate),
            EffectType::SpaceReverb => self.process_reverb(input, output, sample_rate),
            EffectType::TapeEcho => self.process_delay(input, output, sample_rate),
            EffectType::MasteringEQ => self.process_eq(input, output, sample_rate),
            _ => {
                // For unimplemented effects, apply dry/wet mix with input
                self.apply_dry_wet_mix(input, input, output);
            }
        }
    }

    /// Process compressor effect
    fn process_compressor(&mut self, input: &[f32], output: &mut [f32], sample_rate: f32) {
        let threshold = self.get_parameter(0).map(|p| p.actual_value()).unwrap_or(-20.0); // dB
        let ratio = self.get_parameter(1).map(|p| p.actual_value()).unwrap_or(4.0);
        let attack_ms = self.get_parameter(2).map(|p| p.actual_value()).unwrap_or(10.0);
        let release_ms = self.get_parameter(3).map(|p| p.actual_value()).unwrap_or(100.0);

        // Convert to linear values
        let threshold_linear = db_to_linear(threshold);
        let attack_coeff = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();

        let len = input.len().min(output.len());
        let mut envelope = 0.0f32;

        for i in 0..len {
            let input_sample = input[i];
            let input_level = input_sample.abs();

            // Envelope follower
            let target = if input_level > envelope { input_level } else { envelope };
            envelope = envelope + (target - envelope) * if input_level > envelope { 1.0 - attack_coeff } else { 1.0 - release_coeff };

            // Compression calculation
            let gain_reduction = if envelope > threshold_linear {
                let over_threshold = envelope / threshold_linear;
                let compressed = over_threshold.powf(1.0 / ratio);
                compressed / over_threshold
            } else {
                1.0
            };

            let processed = input_sample * gain_reduction;
            self.apply_dry_wet_mix(&[input_sample], &[processed], &mut output[i..i+1]);
        }
    }

    /// Process reverb effect
    fn process_reverb(&mut self, input: &[f32], output: &mut [f32], sample_rate: f32) {
        let time = self.get_parameter(0).map(|p| p.actual_value()).unwrap_or(2.0); // seconds
        let pre_delay = self.get_parameter(1).map(|p| p.actual_value()).unwrap_or(50.0); // ms
        let tone = self.get_parameter(3).map(|p| p.actual_value()).unwrap_or(0.0); // -50 to +50

        // Simple reverb using multiple delay lines (Schroeder reverb approximation)
        let delay_times = [
            (time * 0.03 * sample_rate) as usize,
            (time * 0.05 * sample_rate) as usize,
            (time * 0.07 * sample_rate) as usize,
            (time * 0.11 * sample_rate) as usize,
        ];

        let len = input.len().min(output.len());
        
        // For simplicity, we'll create a basic reverb effect
        // In a real implementation, this would use proper delay buffers
        for i in 0..len {
            let input_sample = input[i];
            
            // Simple reverb simulation with decay
            let decay = 0.3 * time / 10.0; // Approximate decay based on time parameter
            let reverb_sample = input_sample * decay;
            
            self.apply_dry_wet_mix(&[input_sample], &[reverb_sample], &mut output[i..i+1]);
        }
    }

    /// Process delay effect
    fn process_delay(&mut self, input: &[f32], output: &mut [f32], sample_rate: f32) {
        let delay_time = self.get_parameter(0).map(|p| p.actual_value()).unwrap_or(250.0); // ms
        let feedback = self.get_parameter(1).map(|p| p.actual_value()).unwrap_or(30.0) / 100.0; // %
        let tone = self.get_parameter(3).map(|p| p.actual_value()).unwrap_or(0.0); // -50 to +50

        let delay_samples = (delay_time * 0.001 * sample_rate) as usize;
        let len = input.len().min(output.len());

        // Simple delay implementation (in real implementation, would use circular buffer)
        for i in 0..len {
            let input_sample = input[i];
            
            // For this basic implementation, create a simple echo effect
            let delayed_sample = if i >= delay_samples {
                input[i - delay_samples] * feedback
            } else {
                0.0
            };
            
            let delay_output = input_sample + delayed_sample;
            self.apply_dry_wet_mix(&[input_sample], &[delay_output], &mut output[i..i+1]);
        }
    }

    /// Process EQ effect
    fn process_eq(&mut self, input: &[f32], output: &mut [f32], _sample_rate: f32) {
        let low_gain = self.get_parameter(0).map(|p| p.actual_value()).unwrap_or(0.0); // dB
        let low_mid_gain = self.get_parameter(1).map(|p| p.actual_value()).unwrap_or(0.0); // dB
        let high_mid_gain = self.get_parameter(2).map(|p| p.actual_value()).unwrap_or(0.0); // dB
        let high_gain = self.get_parameter(3).map(|p| p.actual_value()).unwrap_or(0.0); // dB

        // Convert dB to linear gain
        let low_linear = db_to_linear(low_gain);
        let low_mid_linear = db_to_linear(low_mid_gain);
        let high_mid_linear = db_to_linear(high_mid_gain);
        let high_linear = db_to_linear(high_gain);

        let len = input.len().min(output.len());

        // Simple EQ implementation (basic gain adjustment)
        // In a real implementation, this would use proper filter banks
        for i in 0..len {
            let input_sample = input[i];
            
            // Apply overall gain (simplified EQ)
            let avg_gain = (low_linear + low_mid_linear + high_mid_linear + high_linear) / 4.0;
            let eq_output = input_sample * avg_gain;
            
            self.apply_dry_wet_mix(&[input_sample], &[eq_output], &mut output[i..i+1]);
        }
    }

    /// Apply dry/wet mix to the processed audio
    fn apply_dry_wet_mix(&self, dry: &[f32], wet: &[f32], output: &mut [f32]) {
        let len = dry.len().min(wet.len()).min(output.len());
        let wet_amount = self.dry_wet_mix;
        let dry_amount = 1.0 - wet_amount;

        for i in 0..len {
            output[i] = dry[i] * dry_amount + wet[i] * wet_amount;
        }
    }
}

/// Effect chain containing up to 4 effects
#[derive(Debug, Clone, PartialEq)]
pub struct EffectChain {
    /// Effect slots (up to 4 per chain)
    pub slots: [Option<Effect>; MAX_EFFECT_SLOTS],
    /// Overall chain mix level
    pub mix_level: f32,
    /// Chain enabled/bypassed
    pub enabled: bool,
    /// Chain type (Input/Track/Master)
    pub chain_type: EffectChainType,
    /// FX Bank number (1-4) for effect presets
    pub fx_bank: u8,
}

impl EffectChain {
    /// Create a new effect chain of the specified type
    pub fn new(chain_type: EffectChainType) -> Self {
        Self {
            slots: [None, None, None, None],
            mix_level: 1.0,
            enabled: true,
            chain_type,
            fx_bank: 1,
        }
    }

    /// Create a new Input FX chain
    pub fn new_input_fx() -> Self {
        Self::new(EffectChainType::InputFX)
    }

    /// Create a new Track FX chain
    pub fn new_track_fx() -> Self {
        Self::new(EffectChainType::TrackFX)
    }

    /// Create a new Master FX chain
    pub fn new_master_fx() -> Self {
        Self::new(EffectChainType::MasterFX)
    }

    /// Add an effect to the first available slot
    pub fn add_effect(&mut self, effect: Effect) -> Result<usize, ()> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(effect);
                return Ok(i);
            }
        }
        Err(()) // No available slots
    }

    /// Set effect in specific slot
    pub fn set_effect(&mut self, slot_index: usize, effect: Option<Effect>) -> Result<(), ()> {
        if slot_index < MAX_EFFECT_SLOTS {
            self.slots[slot_index] = effect;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Get effect from specific slot
    pub fn get_effect(&self, slot_index: usize) -> Option<&Effect> {
        if slot_index < MAX_EFFECT_SLOTS {
            self.slots[slot_index].as_ref()
        } else {
            None
        }
    }

    /// Get mutable effect from specific slot
    pub fn get_effect_mut(&mut self, slot_index: usize) -> Option<&mut Effect> {
        if slot_index < MAX_EFFECT_SLOTS {
            self.slots[slot_index].as_mut()
        } else {
            None
        }
    }

    /// Remove effect from specific slot
    pub fn remove_effect(&mut self, slot_index: usize) -> Option<Effect> {
        if slot_index < MAX_EFFECT_SLOTS {
            self.slots[slot_index].take()
        } else {
            None
        }
    }

    /// Clear all effects from the chain
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
    }

    /// Get the number of active effects
    pub fn active_effect_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    /// Check if chain has any active effects
    pub fn has_effects(&self) -> bool {
        self.active_effect_count() > 0
    }

    /// Process audio through the entire effect chain
    pub fn process_audio(&mut self, input: &[f32], output: &mut [f32], sample_rate: f32) {
        if !self.enabled || !self.has_effects() {
            // Chain bypassed or no effects - copy input to output
            let len = input.len().min(output.len());
            output[..len].copy_from_slice(&input[..len]);
            return;
        }

        let len = input.len().min(output.len());
        
        // Create temporary buffers for processing chain
        let mut temp_buffer_1 = [0.0f32; 512]; // Max buffer size for embedded
        let mut temp_buffer_2 = [0.0f32; 512];
        let buffer_len = len.min(512);
        
        // Copy input to first temp buffer
        temp_buffer_1[..buffer_len].copy_from_slice(&input[..buffer_len]);
        
        let mut current_input = &temp_buffer_1[..buffer_len];
        let mut current_output = &mut temp_buffer_2[..buffer_len];
        let mut swap_buffers = false;

        // Process through each effect in the chain
        for slot in &mut self.slots {
            if let Some(effect) = slot {
                if effect.enabled {
                    // Process audio through this effect
                    effect.process_audio(current_input, current_output, sample_rate);
                    
                    // Swap buffers for next effect
                    if swap_buffers {
                        current_input = &temp_buffer_2[..buffer_len];
                        current_output = &mut temp_buffer_1[..buffer_len];
                    } else {
                        current_input = &temp_buffer_1[..buffer_len];
                        current_output = &mut temp_buffer_2[..buffer_len];
                    }
                    swap_buffers = !swap_buffers;
                }
            }
        }

        // Copy final result to output buffer
        if swap_buffers {
            output[..buffer_len].copy_from_slice(&temp_buffer_1[..buffer_len]);
        } else {
            output[..buffer_len].copy_from_slice(&temp_buffer_2[..buffer_len]);
        }

        // Apply chain mix level
        if self.mix_level != 1.0 {
            for i in 0..buffer_len {
                output[i] *= self.mix_level;
            }
        }
    }

    /// Set FX Bank (1-4) for effect presets
    pub fn set_fx_bank(&mut self, bank: u8) {
        self.fx_bank = bank.clamp(1, 4);
    }

    /// Get current FX Bank
    pub fn get_fx_bank(&self) -> u8 {
        self.fx_bank
    }

    /// Set effect parameter in specific slot
    pub fn set_effect_parameter(&mut self, slot_index: usize, param_index: usize, value: f32) -> Result<(), ()> {
        if let Some(effect) = self.get_effect_mut(slot_index) {
            effect.set_parameter(param_index, value);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Get effect parameter from specific slot
    pub fn get_effect_parameter(&self, slot_index: usize, param_index: usize) -> Option<f32> {
        self.get_effect(slot_index)?.get_parameter(param_index).map(|p| p.value)
    }

    /// Enable/disable effect in specific slot
    pub fn set_effect_enabled(&mut self, slot_index: usize, enabled: bool) -> Result<(), ()> {
        if let Some(effect) = self.get_effect_mut(slot_index) {
            effect.set_enabled(enabled);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Toggle effect in specific slot
    pub fn toggle_effect(&mut self, slot_index: usize) -> Result<bool, ()> {
        if let Some(effect) = self.get_effect_mut(slot_index) {
            effect.toggle_enabled();
            Ok(effect.enabled)
        } else {
            Err(())
        }
    }

    /// Set momentary mode for effect in specific slot
    pub fn set_effect_momentary(&mut self, slot_index: usize, momentary: bool) -> Result<(), ()> {
        if let Some(effect) = self.get_effect_mut(slot_index) {
            effect.set_momentary(momentary);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Update tempo for all tempo-synced effects in the chain
    pub fn update_tempo(&mut self, bpm: f32) {
        for slot in &mut self.slots {
            if let Some(effect) = slot {
                effect.update_tempo(bpm);
            }
        }
    }

    /// Update effect chain state (called from main loop)
    pub fn update(&mut self) {
        // Update individual effects
        for slot in &mut self.slots {
            if let Some(effect) = slot {
                effect.update();
            }
        }
    }

    /// Clear all effects from the chain
    pub fn clear_all_effects(&mut self) {
        self.clear();
    }

    /// Get effects array reference for direct access
    pub fn effects(&self) -> &[Option<Effect>; MAX_EFFECT_SLOTS] {
        &self.slots
    }

    /// Get mutable effects array reference for direct access
    pub fn effects_mut(&mut self) -> &mut [Option<Effect>; MAX_EFFECT_SLOTS] {
        &mut self.slots
    }

    /// Set chain mix level
    pub fn set_mix_level(&mut self, level: f32) {
        self.mix_level = level.clamp(0.0, 1.0);
    }

    /// Enable/disable entire chain
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Toggle entire chain
    pub fn toggle_enabled(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.enabled
    }
}

/// Convert decibels to linear gain
fn db_to_linear(db: f32) -> f32 {
    libm::powf(10.0, db / 20.0)
}

/// Convert linear gain to decibels
fn linear_to_db(linear: f32) -> f32 {
    20.0 * libm::log10f(linear)
}