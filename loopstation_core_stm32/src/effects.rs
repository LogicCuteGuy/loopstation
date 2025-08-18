use heapless::Vec;
use serde::{Deserialize, Serialize};

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

    /// Process audio through this effect (placeholder)
    pub fn process_audio(&mut self, _input: &[f32], _output: &mut [f32]) {
        // Audio processing implementation will be added in later tasks
        // For now, this is a placeholder
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

    /// Process audio through the entire effect chain (placeholder)
    pub fn process_audio(&mut self, _input: &[f32], _output: &mut [f32]) {
        // Effect chain processing implementation will be added in later tasks
        // This will process audio through each effect in sequence
    }

    /// Set FX Bank (1-4) for effect presets
    pub fn set_fx_bank(&mut self, bank: u8) {
        self.fx_bank = bank.clamp(1, 4);
    }

    /// Get current FX Bank
    pub fn get_fx_bank(&self) -> u8 {
        self.fx_bank
    }
}