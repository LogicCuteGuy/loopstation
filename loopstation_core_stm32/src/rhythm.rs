//! Rhythm system for the loopstation
//! 
//! This module provides basic rhythm pattern support including:
//! - Simple drum machine pattern playback
//! - Rhythm pattern configuration and selection
//! - Beat position tracking and tempo-locked effects

use heapless::Vec;
use serde::{Deserialize, Serialize};
use crate::tempo::TempoSystem;

#[cfg(not(feature = "std"))]
use micromath::F32Ext;

/// Maximum number of rhythm patterns
const MAX_RHYTHM_PATTERNS: usize = 16;

/// Maximum number of steps per pattern
const MAX_PATTERN_STEPS: usize = 16;

/// Maximum number of drum sounds
const MAX_DRUM_SOUNDS: usize = 8;

/// Drum sound types for rhythm patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrumSound {
    /// Kick drum
    Kick,
    /// Snare drum
    Snare,
    /// Closed hi-hat
    HiHatClosed,
    /// Open hi-hat
    HiHatOpen,
    /// Crash cymbal
    Crash,
    /// Ride cymbal
    Ride,
    /// Tom 1 (high)
    Tom1,
    /// Tom 2 (low)
    Tom2,
}

impl DrumSound {
    /// Get all available drum sounds
    pub fn all() -> [DrumSound; MAX_DRUM_SOUNDS] {
        [
            DrumSound::Kick,
            DrumSound::Snare,
            DrumSound::HiHatClosed,
            DrumSound::HiHatOpen,
            DrumSound::Crash,
            DrumSound::Ride,
            DrumSound::Tom1,
            DrumSound::Tom2,
        ]
    }

    /// Get the name of the drum sound
    pub fn name(&self) -> &'static str {
        match self {
            DrumSound::Kick => "Kick",
            DrumSound::Snare => "Snare",
            DrumSound::HiHatClosed => "HH Closed",
            DrumSound::HiHatOpen => "HH Open",
            DrumSound::Crash => "Crash",
            DrumSound::Ride => "Ride",
            DrumSound::Tom1 => "Tom 1",
            DrumSound::Tom2 => "Tom 2",
        }
    }

    /// Get the default velocity for this drum sound
    pub fn default_velocity(&self) -> f32 {
        match self {
            DrumSound::Kick => 0.8,
            DrumSound::Snare => 0.7,
            DrumSound::HiHatClosed => 0.5,
            DrumSound::HiHatOpen => 0.6,
            DrumSound::Crash => 0.9,
            DrumSound::Ride => 0.6,
            DrumSound::Tom1 => 0.7,
            DrumSound::Tom2 => 0.7,
        }
    }
}

/// A single step in a rhythm pattern
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PatternStep {
    /// Whether this step is active
    pub active: bool,
    /// Velocity (0.0-1.0)
    pub velocity: f32,
    /// Accent (additional velocity boost)
    pub accent: bool,
    /// Flam (slight timing offset)
    pub flam: bool,
}

impl PatternStep {
    /// Create a new inactive step
    pub fn new() -> Self {
        Self {
            active: false,
            velocity: 0.7,
            accent: false,
            flam: false,
        }
    }

    /// Create an active step with default velocity
    pub fn active() -> Self {
        Self {
            active: true,
            velocity: 0.7,
            accent: false,
            flam: false,
        }
    }

    /// Create an accented step
    pub fn accented() -> Self {
        Self {
            active: true,
            velocity: 0.9,
            accent: true,
            flam: false,
        }
    }

    /// Get the effective velocity including accent
    pub fn effective_velocity(&self) -> f32 {
        if self.accent {
            (self.velocity * 1.3).min(1.0)
        } else {
            self.velocity
        }
    }
}

impl Default for PatternStep {
    fn default() -> Self {
        Self::new()
    }
}

/// A drum track within a rhythm pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumTrack {
    /// The drum sound for this track
    pub sound: DrumSound,
    /// Pattern steps for this track
    pub steps: [PatternStep; MAX_PATTERN_STEPS],
    /// Track volume (0.0-1.0)
    pub volume: f32,
    /// Track pan (-1.0 to 1.0)
    pub pan: f32,
    /// Track mute state
    pub muted: bool,
}

impl DrumTrack {
    /// Create a new drum track
    pub fn new(sound: DrumSound) -> Self {
        Self {
            sound,
            steps: [PatternStep::default(); MAX_PATTERN_STEPS],
            volume: sound.default_velocity(),
            pan: 0.0,
            muted: false,
        }
    }

    /// Set a step as active
    pub fn set_step(&mut self, step: usize, active: bool) {
        if step < MAX_PATTERN_STEPS {
            self.steps[step].active = active;
        }
    }

    /// Set step velocity
    pub fn set_step_velocity(&mut self, step: usize, velocity: f32) {
        if step < MAX_PATTERN_STEPS {
            self.steps[step].velocity = velocity.clamp(0.0, 1.0);
        }
    }

    /// Toggle step accent
    pub fn toggle_step_accent(&mut self, step: usize) {
        if step < MAX_PATTERN_STEPS {
            self.steps[step].accent = !self.steps[step].accent;
        }
    }

    /// Clear all steps
    pub fn clear(&mut self) {
        for step in &mut self.steps {
            *step = PatternStep::default();
        }
    }

    /// Get step at position
    pub fn get_step(&self, step: usize) -> Option<&PatternStep> {
        self.steps.get(step)
    }
}

/// A complete rhythm pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhythmPattern {
    /// Pattern name
    pub name: heapless::String<32>,
    /// Number of steps in the pattern (1-16)
    pub length: u8,
    /// Time signature numerator (beats per bar)
    pub time_signature_num: u8,
    /// Time signature denominator (note value)
    pub time_signature_den: u8,
    /// Swing amount (0.0 = straight, 1.0 = full swing)
    pub swing: f32,
    /// Pattern volume (0.0-1.0)
    pub volume: f32,
    /// Drum tracks
    pub tracks: [DrumTrack; MAX_DRUM_SOUNDS],
}

impl RhythmPattern {
    /// Create a new empty rhythm pattern
    pub fn new(name: &str) -> Self {
        let mut pattern_name = heapless::String::new();
        let _ = pattern_name.push_str(name);

        let drum_sounds = DrumSound::all();
        let tracks = [
            DrumTrack::new(drum_sounds[0]),
            DrumTrack::new(drum_sounds[1]),
            DrumTrack::new(drum_sounds[2]),
            DrumTrack::new(drum_sounds[3]),
            DrumTrack::new(drum_sounds[4]),
            DrumTrack::new(drum_sounds[5]),
            DrumTrack::new(drum_sounds[6]),
            DrumTrack::new(drum_sounds[7]),
        ];

        Self {
            name: pattern_name,
            length: 16,
            time_signature_num: 4,
            time_signature_den: 4,
            swing: 0.0,
            volume: 0.8,
            tracks,
        }
    }

    /// Create a basic 4/4 rock pattern
    pub fn rock_pattern() -> Self {
        let mut pattern = Self::new("Rock");
        
        // Kick on 1 and 3
        pattern.tracks[0].set_step(0, true);  // Beat 1
        pattern.tracks[0].set_step(8, true);  // Beat 3
        
        // Snare on 2 and 4
        pattern.tracks[1].set_step(4, true);  // Beat 2
        pattern.tracks[1].set_step(12, true); // Beat 4
        
        // Hi-hat on every 8th note
        for i in 0..16 {
            pattern.tracks[2].set_step(i, true);
            pattern.tracks[2].set_step_velocity(i, if i % 4 == 0 { 0.7 } else { 0.4 });
        }
        
        pattern
    }

    /// Create a basic 4/4 pop pattern
    pub fn pop_pattern() -> Self {
        let mut pattern = Self::new("Pop");
        
        // Kick on 1, 3, and 4+
        pattern.tracks[0].set_step(0, true);  // Beat 1
        pattern.tracks[0].set_step(8, true);  // Beat 3
        pattern.tracks[0].set_step(14, true); // Beat 4+
        
        // Snare on 2 and 4
        pattern.tracks[1].set_step(4, true);  // Beat 2
        pattern.tracks[1].set_step(12, true); // Beat 4
        
        // Hi-hat on off-beats
        for i in (1..16).step_by(2) {
            pattern.tracks[2].set_step(i, true);
            pattern.tracks[2].set_step_velocity(i, 0.5);
        }
        
        pattern
    }

    /// Create a basic 4/4 funk pattern
    pub fn funk_pattern() -> Self {
        let mut pattern = Self::new("Funk");
        
        // Kick with syncopation
        pattern.tracks[0].set_step(0, true);  // Beat 1
        pattern.tracks[0].set_step(6, true);  // 1e+a
        pattern.tracks[0].set_step(10, true); // 3e
        
        // Snare on 2 and 4 with ghost notes
        pattern.tracks[1].set_step(4, true);  // Beat 2
        pattern.tracks[1].set_step(12, true); // Beat 4
        pattern.tracks[1].set_step(2, true);  // Ghost note
        pattern.tracks[1].set_step_velocity(2, 0.3);
        pattern.tracks[1].set_step(14, true); // Ghost note
        pattern.tracks[1].set_step_velocity(14, 0.3);
        
        // Hi-hat pattern
        let hihat_pattern = [true, false, true, true, false, true, false, true,
                           true, false, true, true, false, true, false, true];
        for (i, &active) in hihat_pattern.iter().enumerate() {
            pattern.tracks[2].set_step(i, active);
            pattern.tracks[2].set_step_velocity(i, if i % 4 == 0 { 0.6 } else { 0.4 });
        }
        
        pattern
    }

    /// Get the drum track for a specific sound
    pub fn get_track_mut(&mut self, sound: DrumSound) -> Option<&mut DrumTrack> {
        self.tracks.iter_mut().find(|track| track.sound == sound)
    }

    /// Get the drum track for a specific sound (immutable)
    pub fn get_track(&self, sound: DrumSound) -> Option<&DrumTrack> {
        self.tracks.iter().find(|track| track.sound == sound)
    }

    /// Clear the entire pattern
    pub fn clear(&mut self) {
        for track in &mut self.tracks {
            track.clear();
        }
    }

    /// Get pattern duration in beats
    pub fn duration_beats(&self) -> f32 {
        self.length as f32 / 4.0 // Assuming 16th note steps
    }
}

/// Rhythm system managing pattern playback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhythmSystem {
    /// Available rhythm patterns
    pub patterns: Vec<RhythmPattern, MAX_RHYTHM_PATTERNS>,
    /// Currently selected pattern index
    pub current_pattern: usize,
    /// Whether rhythm playback is enabled
    pub enabled: bool,
    /// Current step position in the pattern
    pub current_step: usize,
    /// Sub-step position for swing timing (0.0-1.0)
    pub sub_step_position: f32,
    /// Sample rate for timing calculations
    sample_rate: u32,
    /// Samples per step at current tempo
    samples_per_step: u32,
    /// Current sample position within step
    step_sample_position: u32,
    /// Master rhythm volume
    pub master_volume: f32,
}

impl RhythmSystem {
    /// Create a new rhythm system
    pub fn new(sample_rate: u32) -> Self {
        let mut system = Self {
            patterns: Vec::new(),
            current_pattern: 0,
            enabled: false,
            current_step: 0,
            sub_step_position: 0.0,
            sample_rate,
            samples_per_step: 0,
            step_sample_position: 0,
            master_volume: 0.7,
        };

        // Add default patterns
        let _ = system.patterns.push(RhythmPattern::rock_pattern());
        let _ = system.patterns.push(RhythmPattern::pop_pattern());
        let _ = system.patterns.push(RhythmPattern::funk_pattern());

        system.update_timing_calculations(120.0); // Default tempo
        system
    }

    /// Update timing calculations based on tempo
    pub fn update_timing_calculations(&mut self, bpm: f32) {
        // Calculate samples per 16th note step
        // 60 seconds/minute * sample_rate samples/second / (BPM beats/minute * 4 steps/beat)
        self.samples_per_step = ((60.0 * self.sample_rate as f32) / (bpm * 4.0)) as u32;
    }

    /// Start rhythm playback
    pub fn start(&mut self) {
        self.enabled = true;
        self.current_step = 0;
        self.sub_step_position = 0.0;
        self.step_sample_position = 0;
    }

    /// Stop rhythm playback
    pub fn stop(&mut self) {
        self.enabled = false;
        self.current_step = 0;
        self.sub_step_position = 0.0;
        self.step_sample_position = 0;
    }

    /// Toggle rhythm playback
    pub fn toggle(&mut self) {
        if self.enabled {
            self.stop();
        } else {
            self.start();
        }
    }

    /// Select a rhythm pattern
    pub fn select_pattern(&mut self, pattern_index: usize) {
        if pattern_index < self.patterns.len() {
            self.current_pattern = pattern_index;
            // Reset position when changing patterns
            self.current_step = 0;
            self.sub_step_position = 0.0;
            self.step_sample_position = 0;
        }
    }

    /// Get the currently selected pattern
    pub fn get_current_pattern(&self) -> Option<&RhythmPattern> {
        self.patterns.get(self.current_pattern)
    }

    /// Get the currently selected pattern (mutable)
    pub fn get_current_pattern_mut(&mut self) -> Option<&mut RhythmPattern> {
        self.patterns.get_mut(self.current_pattern)
    }

    /// Add a new rhythm pattern
    pub fn add_pattern(&mut self, pattern: RhythmPattern) -> Result<(), ()> {
        self.patterns.push(pattern).map_err(|_| ())
    }

    /// Process rhythm system and generate trigger events
    pub fn process(&mut self, samples_to_process: u32, tempo_system: &TempoSystem) -> Vec<RhythmTrigger, 16> {
        let mut triggers = Vec::new();

        if !self.enabled {
            return triggers;
        }

        // Update timing based on current tempo
        self.update_timing_calculations(tempo_system.get_bpm());

        let pattern_length = match self.get_current_pattern() {
            Some(p) => p.length,
            None => return triggers,
        };

        // Process each sample
        for _ in 0..samples_to_process {
            // Check if we've reached a new step
            if self.step_sample_position == 0 {
                // Generate triggers for this step
                if let Some(pattern) = self.get_current_pattern() {
                    self.generate_step_triggers(&mut triggers, pattern);
                }
            }

            self.step_sample_position += 1;

            // Check for step boundary
            if self.step_sample_position >= self.samples_per_step {
                self.current_step = (self.current_step + 1) % (pattern_length as usize);
                self.step_sample_position = 0;
                self.sub_step_position = 0.0;
            }
        }

        triggers
    }

    /// Generate triggers for the current step
    fn generate_step_triggers(&self, triggers: &mut Vec<RhythmTrigger, 16>, pattern: &RhythmPattern) {
        if self.current_step >= pattern.length as usize {
            return;
        }

        for track in &pattern.tracks {
            if track.muted {
                continue;
            }

            if let Some(step) = track.get_step(self.current_step) {
                if step.active {
                    let trigger = RhythmTrigger {
                        sound: track.sound,
                        velocity: step.effective_velocity() * track.volume * pattern.volume * self.master_volume,
                        pan: track.pan,
                        step: self.current_step,
                        timing_offset: if step.flam { -0.01 } else { 0.0 }, // 10ms flam
                    };

                    if triggers.push(trigger).is_err() {
                        break; // Trigger buffer full
                    }
                }
            }
        }
    }



    /// Sync rhythm to tempo system beat position
    pub fn sync_to_tempo(&mut self, tempo_system: &TempoSystem) {
        if !self.enabled {
            return;
        }

        let pattern = match self.get_current_pattern() {
            Some(p) => p,
            None => return,
        };

        // Calculate step position based on beat position
        let beat_position = tempo_system.get_beat_position();
        let steps_per_beat = 4.0; // 16th notes per quarter note
        let total_steps = pattern.length as f32;
        
        let step_position = (beat_position * steps_per_beat) % total_steps;
        self.current_step = step_position as usize;
        self.sub_step_position = step_position.fract();
        
        // Update sample position within step
        self.step_sample_position = (self.sub_step_position * self.samples_per_step as f32) as u32;
    }

    /// Get current step position for display
    pub fn get_current_step(&self) -> usize {
        self.current_step
    }

    /// Get pattern names for selection
    pub fn get_pattern_names(&self) -> Vec<&str, MAX_RHYTHM_PATTERNS> {
        let mut names = Vec::new();
        for pattern in &self.patterns {
            if names.push(pattern.name.as_str()).is_err() {
                break;
            }
        }
        names
    }

    /// Check if rhythm is currently playing
    pub fn is_playing(&self) -> bool {
        self.enabled
    }

    /// Get rhythm status for display
    pub fn get_status(&self) -> RhythmStatus {
        let mut pattern_name = heapless::String::new();
        if let Some(pattern) = self.get_current_pattern() {
            let _ = pattern_name.push_str(&pattern.name);
        } else {
            let _ = pattern_name.push_str("None");
        }

        RhythmStatus {
            enabled: self.enabled,
            current_pattern: self.current_pattern,
            current_step: self.current_step,
            pattern_name,
            master_volume: self.master_volume,
        }
    }
}

/// A rhythm trigger event
#[derive(Debug, Clone, Copy)]
pub struct RhythmTrigger {
    /// The drum sound to trigger
    pub sound: DrumSound,
    /// Trigger velocity (0.0-1.0)
    pub velocity: f32,
    /// Pan position (-1.0 to 1.0)
    pub pan: f32,
    /// Step number that triggered this
    pub step: usize,
    /// Timing offset in seconds (for flams, etc.)
    pub timing_offset: f32,
}

/// Rhythm system status for display
#[derive(Debug, Clone)]
pub struct RhythmStatus {
    /// Whether rhythm is enabled
    pub enabled: bool,
    /// Current pattern index
    pub current_pattern: usize,
    /// Current step position
    pub current_step: usize,
    /// Current pattern name
    pub pattern_name: heapless::String<32>,
    /// Master volume
    pub master_volume: f32,
}

/// Simple drum synthesizer for rhythm playback
#[derive(Debug, Clone)]
pub struct DrumSynthesizer {
    /// Sample rate
    sample_rate: u32,
    /// Active drum voices
    voices: Vec<DrumVoice, 16>,
}

/// A single drum voice
#[derive(Debug, Clone, Copy)]
struct DrumVoice {
    /// The drum sound being played
    sound: DrumSound,
    /// Current phase/position
    phase: f32,
    /// Velocity
    velocity: f32,
    /// Pan position
    pan: f32,
    /// Remaining samples to play
    remaining_samples: u32,
    /// Voice active flag
    active: bool,
}

impl DrumSynthesizer {
    /// Create a new drum synthesizer
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            voices: Vec::new(),
        }
    }

    /// Trigger a drum sound
    pub fn trigger(&mut self, trigger: RhythmTrigger) {
        // Find an inactive voice or reuse the oldest one
        let voice_index = self.voices.iter().position(|v| !v.active)
            .unwrap_or_else(|| {
                // All voices active, find oldest (simplified - just use first)
                0
            });

        let duration_samples = self.get_drum_duration_samples(trigger.sound);
        
        let voice = DrumVoice {
            sound: trigger.sound,
            phase: 0.0,
            velocity: trigger.velocity,
            pan: trigger.pan,
            remaining_samples: duration_samples,
            active: true,
        };

        if voice_index < self.voices.len() {
            self.voices[voice_index] = voice;
        } else if self.voices.push(voice).is_err() {
            // Voice buffer full, replace first voice
            if !self.voices.is_empty() {
                self.voices[0] = voice;
            }
        }
    }

    /// Process audio and generate drum sounds
    pub fn process_audio(&mut self, output: &mut [f32]) {
        // Clear output buffer
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        // Process each active voice
        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }

            for (_i, output_sample) in output.iter_mut().enumerate() {
                if voice.remaining_samples == 0 {
                    voice.active = false;
                    break;
                }

                let drum_sample = Self::generate_drum_sample_static(voice, self.sample_rate);
                *output_sample += drum_sample * voice.velocity;

                voice.phase += 1.0;
                voice.remaining_samples -= 1;
            }
        }

        // Apply simple limiting to prevent clipping
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    /// Generate a single sample for a drum voice
    fn generate_drum_sample_static(voice: &DrumVoice, sample_rate: u32) -> f32 {
        let t = voice.phase / sample_rate as f32;
        let envelope = Self::get_drum_envelope_static(voice.sound, t);

        match voice.sound {
            DrumSound::Kick => {
                // Simple kick: sine wave with pitch envelope
                let freq = 60.0 * (1.0 - t * 8.0).max(0.1);
                (t * freq * 2.0 * core::f32::consts::PI).sin() * envelope
            },
            DrumSound::Snare => {
                // Simple snare: noise + tone
                let noise = (voice.phase * 0.1).sin() * 0.7; // Simplified noise
                let tone = (t * 200.0 * 2.0 * core::f32::consts::PI).sin() * 0.3;
                (noise + tone) * envelope
            },
            DrumSound::HiHatClosed => {
                // Simple hi-hat: filtered noise
                let noise = (voice.phase * 0.05).sin(); // Simplified noise
                noise * envelope * 0.5
            },
            DrumSound::HiHatOpen => {
                // Open hi-hat: longer decay
                let noise = (voice.phase * 0.05).sin(); // Simplified noise
                noise * envelope * 0.6
            },
            DrumSound::Crash => {
                // Crash: bright noise with long decay
                let noise = (voice.phase * 0.02).sin(); // Simplified noise
                noise * envelope * 0.8
            },
            DrumSound::Ride => {
                // Ride: metallic tone
                let freq = 800.0;
                (t * freq * 2.0 * core::f32::consts::PI).sin() * envelope * 0.4
            },
            DrumSound::Tom1 => {
                // Tom: pitched drum
                let freq = 150.0 * (1.0 - t * 2.0).max(0.3);
                (t * freq * 2.0 * core::f32::consts::PI).sin() * envelope
            },
            DrumSound::Tom2 => {
                // Lower tom
                let freq = 100.0 * (1.0 - t * 2.0).max(0.3);
                (t * freq * 2.0 * core::f32::consts::PI).sin() * envelope
            },
        }
    }

    /// Get envelope for drum sound
    fn get_drum_envelope_static(sound: DrumSound, t: f32) -> f32 {
        match sound {
            DrumSound::Kick => {
                // Quick attack, medium decay
                if t < 0.01 {
                    t / 0.01
                } else {
                    (-t * 8.0).exp()
                }
            },
            DrumSound::Snare => {
                // Quick attack, quick decay
                if t < 0.005 {
                    t / 0.005
                } else {
                    (-t * 15.0).exp()
                }
            },
            DrumSound::HiHatClosed => {
                // Very quick decay
                (-t * 25.0).exp()
            },
            DrumSound::HiHatOpen => {
                // Longer decay than closed
                (-t * 8.0).exp()
            },
            DrumSound::Crash => {
                // Long decay
                (-t * 2.0).exp()
            },
            DrumSound::Ride => {
                // Medium decay
                (-t * 5.0).exp()
            },
            DrumSound::Tom1 | DrumSound::Tom2 => {
                // Medium attack and decay
                if t < 0.01 {
                    t / 0.01
                } else {
                    (-t * 6.0).exp()
                }
            },
        }
    }

    /// Get duration in samples for a drum sound
    fn get_drum_duration_samples(&self, sound: DrumSound) -> u32 {
        let duration_seconds = match sound {
            DrumSound::Kick => 0.5,
            DrumSound::Snare => 0.3,
            DrumSound::HiHatClosed => 0.1,
            DrumSound::HiHatOpen => 0.8,
            DrumSound::Crash => 2.0,
            DrumSound::Ride => 1.0,
            DrumSound::Tom1 => 0.6,
            DrumSound::Tom2 => 0.8,
        };
        
        (duration_seconds * self.sample_rate as f32) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rhythm_pattern_creation() {
        let pattern = RhythmPattern::rock_pattern();
        assert_eq!(pattern.name.as_str(), "Rock");
        assert_eq!(pattern.length, 16);
        
        // Check kick pattern
        assert!(pattern.tracks[0].get_step(0).unwrap().active); // Beat 1
        assert!(pattern.tracks[0].get_step(8).unwrap().active); // Beat 3
    }

    #[test]
    fn test_rhythm_system() {
        let mut rhythm_system = RhythmSystem::new(44100);
        assert!(!rhythm_system.is_playing());
        
        rhythm_system.start();
        assert!(rhythm_system.is_playing());
        
        rhythm_system.stop();
        assert!(!rhythm_system.is_playing());
    }

    #[test]
    fn test_drum_synthesizer() {
        let mut synth = DrumSynthesizer::new(44100);
        
        let trigger = RhythmTrigger {
            sound: DrumSound::Kick,
            velocity: 0.8,
            pan: 0.0,
            step: 0,
            timing_offset: 0.0,
        };
        
        synth.trigger(trigger);
        
        let mut output = [0.0f32; 256];
        synth.process_audio(&mut output);
        
        // Should have generated some audio
        assert!(output.iter().any(|&sample| sample.abs() > 0.0));
    }

    #[test]
    fn test_pattern_step() {
        let mut step = PatternStep::new();
        assert!(!step.active);
        
        step.active = true;
        step.accent = true;
        step.velocity = 0.7;
        
        assert!(step.effective_velocity() > step.velocity);
    }
}