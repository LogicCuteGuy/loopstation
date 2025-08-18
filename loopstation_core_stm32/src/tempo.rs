//! Tempo and rhythm system for the loopstation
//! 
//! This module provides comprehensive tempo control including:
//! - TAP TEMPO functionality with tempo detection
//! - Tempo reset and BPM management  
//! - Basic MIDI clock sync for external sequencers
//! - Beat position tracking for tempo-locked effects

use heapless::Vec;
use serde::{Deserialize, Serialize};

/// Maximum number of tap tempo samples to track
const MAX_TAP_SAMPLES: usize = 8;

/// Minimum valid BPM
const MIN_BPM: f32 = 60.0;

/// Maximum valid BPM  
const MAX_BPM: f32 = 200.0;

/// Default BPM
const DEFAULT_BPM: f32 = 120.0;

/// MIDI clock pulses per quarter note
const MIDI_CLOCK_PPQ: u32 = 24;

/// Tempo control system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoSystem {
    /// Current tempo in BPM
    pub bpm: f32,
    /// Tap tempo sample buffer (timestamps in milliseconds)
    tap_samples: Vec<u32, MAX_TAP_SAMPLES>,
    /// Last tap timestamp
    last_tap_time: u32,
    /// Number of valid tap samples
    tap_count: u8,
    /// MIDI clock sync enabled
    pub midi_sync_enabled: bool,
    /// External MIDI clock BPM (when syncing)
    external_bpm: f32,
    /// MIDI clock counter
    midi_clock_counter: u32,
    /// Last MIDI clock timestamp
    last_midi_clock_time: u32,
    /// Beat position (0.0-1.0 within current beat)
    beat_position: f32,
    /// Bar position (0.0-1.0 within current bar, assuming 4/4 time)
    bar_position: f32,
    /// Sample rate for timing calculations
    sample_rate: u32,
    /// Samples per beat at current tempo
    samples_per_beat: u32,
    /// Current sample position within beat
    beat_sample_position: u32,
}

impl TempoSystem {
    /// Create a new tempo system
    pub fn new(sample_rate: u32) -> Self {
        let mut system = Self {
            bpm: DEFAULT_BPM,
            tap_samples: Vec::new(),
            last_tap_time: 0,
            tap_count: 0,
            midi_sync_enabled: false,
            external_bpm: DEFAULT_BPM,
            midi_clock_counter: 0,
            last_midi_clock_time: 0,
            beat_position: 0.0,
            bar_position: 0.0,
            sample_rate,
            samples_per_beat: 0,
            beat_sample_position: 0,
        };
        
        system.update_timing_calculations();
        system
    }

    /// Set tempo in BPM
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(MIN_BPM, MAX_BPM);
        self.update_timing_calculations();
    }

    /// Get current tempo in BPM
    pub fn get_bpm(&self) -> f32 {
        if self.midi_sync_enabled {
            self.external_bpm
        } else {
            self.bpm
        }
    }

    /// Reset tempo to default
    pub fn reset_tempo(&mut self) {
        self.bpm = DEFAULT_BPM;
        self.clear_tap_history();
        self.update_timing_calculations();
    }

    /// Process tap tempo input
    pub fn tap_tempo(&mut self, timestamp_ms: u32) -> bool {
        let time_diff = timestamp_ms.saturating_sub(self.last_tap_time);
        
        // Valid tap range: 300ms to 1000ms (60-200 BPM)
        if time_diff >= 300 && time_diff <= 1000 && self.last_tap_time > 0 {
            // Add tap sample
            if self.tap_samples.push(time_diff).is_err() {
                // Buffer full, remove oldest sample
                self.tap_samples.remove(0);
                let _ = self.tap_samples.push(time_diff);
            }
            
            self.tap_count = self.tap_samples.len() as u8;
            
            // Calculate BPM if we have enough samples
            if self.tap_count >= 2 {
                let avg_interval = self.calculate_average_tap_interval();
                let calculated_bpm = 60000.0 / avg_interval;
                
                if calculated_bpm >= MIN_BPM && calculated_bpm <= MAX_BPM {
                    self.set_bpm(calculated_bpm);
                    self.last_tap_time = timestamp_ms;
                    return true; // Tempo updated
                }
            }
        } else if time_diff > 2000 {
            // Reset tap sequence if too much time has passed
            self.clear_tap_history();
        }
        
        self.last_tap_time = timestamp_ms;
        false // Tempo not updated
    }

    /// Calculate average tap interval from samples
    fn calculate_average_tap_interval(&self) -> f32 {
        if self.tap_samples.is_empty() {
            return 500.0; // Default 120 BPM interval
        }
        
        let sum: u32 = self.tap_samples.iter().sum();
        sum as f32 / self.tap_samples.len() as f32
    }

    /// Clear tap tempo history
    pub fn clear_tap_history(&mut self) {
        self.tap_samples.clear();
        self.tap_count = 0;
        self.last_tap_time = 0;
    }

    /// Enable/disable MIDI clock sync
    pub fn set_midi_sync(&mut self, enabled: bool) {
        self.midi_sync_enabled = enabled;
        if !enabled {
            // Reset MIDI clock state when disabling
            self.midi_clock_counter = 0;
            self.last_midi_clock_time = 0;
        }
    }

    /// Process MIDI clock message
    pub fn process_midi_clock(&mut self, timestamp_ms: u32) {
        if !self.midi_sync_enabled {
            return;
        }
        
        self.midi_clock_counter += 1;
        
        // Calculate tempo from MIDI clock timing
        if self.midi_clock_counter >= MIDI_CLOCK_PPQ && self.last_midi_clock_time > 0 {
            let time_diff = timestamp_ms.saturating_sub(self.last_midi_clock_time);
            
            if time_diff > 0 {
                // One quarter note completed, calculate BPM
                let quarter_note_ms = time_diff as f32 / (MIDI_CLOCK_PPQ as f32);
                let calculated_bpm = 60000.0 / (quarter_note_ms * 4.0); // 4 quarter notes per minute
                
                if calculated_bpm >= MIN_BPM && calculated_bpm <= MAX_BPM {
                    self.external_bpm = calculated_bpm;
                    self.update_timing_calculations();
                }
            }
            
            self.midi_clock_counter = 0;
        }
        
        if self.midi_clock_counter == 1 {
            self.last_midi_clock_time = timestamp_ms;
        }
    }

    /// Process MIDI start message
    pub fn process_midi_start(&mut self) {
        if self.midi_sync_enabled {
            self.reset_beat_position();
            self.midi_clock_counter = 0;
        }
    }

    /// Process MIDI stop message
    pub fn process_midi_stop(&mut self) {
        if self.midi_sync_enabled {
            self.reset_beat_position();
            self.midi_clock_counter = 0;
        }
    }

    /// Process MIDI continue message
    pub fn process_midi_continue(&mut self) {
        if self.midi_sync_enabled {
            // Continue from current position
            // Beat position is maintained
        }
    }

    /// Update timing calculations based on current BPM
    fn update_timing_calculations(&mut self) {
        let current_bpm = self.get_bpm();
        
        // Calculate samples per beat
        // 60 seconds/minute * sample_rate samples/second / BPM beats/minute
        self.samples_per_beat = ((60.0 * self.sample_rate as f32) / current_bpm) as u32;
    }

    /// Update beat position based on audio processing
    pub fn update_beat_position(&mut self, samples_processed: u32) {
        if self.midi_sync_enabled {
            // When MIDI synced, beat position is driven by MIDI clock
            return;
        }
        
        self.beat_sample_position += samples_processed;
        
        // Wrap around at beat boundary
        if self.beat_sample_position >= self.samples_per_beat {
            self.beat_sample_position %= self.samples_per_beat;
        }
        
        // Calculate normalized beat position (0.0-1.0)
        self.beat_position = self.beat_sample_position as f32 / self.samples_per_beat as f32;
        
        // Calculate bar position (assuming 4/4 time)
        let beats_per_bar = 4.0;
        let total_beat_position = (self.beat_sample_position as f32 / self.samples_per_beat as f32) 
            + (self.get_current_beat_number() as f32);
        self.bar_position = (total_beat_position % beats_per_bar) / beats_per_bar;
    }

    /// Get current beat position (0.0-1.0)
    pub fn get_beat_position(&self) -> f32 {
        self.beat_position
    }

    /// Get current bar position (0.0-1.0)
    pub fn get_bar_position(&self) -> f32 {
        self.bar_position
    }

    /// Get current beat number within bar (0-3 for 4/4 time)
    pub fn get_current_beat_number(&self) -> u8 {
        ((self.beat_position * 4.0) as u8) % 4
    }

    /// Check if we're at the start of a beat (within tolerance)
    pub fn is_beat_start(&self, tolerance: f32) -> bool {
        self.beat_position <= tolerance || self.beat_position >= (1.0 - tolerance)
    }

    /// Check if we're at the start of a bar (within tolerance)
    pub fn is_bar_start(&self, tolerance: f32) -> bool {
        self.bar_position <= tolerance || self.bar_position >= (1.0 - tolerance)
    }

    /// Reset beat position to start
    pub fn reset_beat_position(&mut self) {
        self.beat_position = 0.0;
        self.bar_position = 0.0;
        self.beat_sample_position = 0;
    }

    /// Get samples per beat at current tempo
    pub fn get_samples_per_beat(&self) -> u32 {
        self.samples_per_beat
    }

    /// Get milliseconds per beat at current tempo
    pub fn get_ms_per_beat(&self) -> f32 {
        60000.0 / self.get_bpm()
    }

    /// Get samples per bar (assuming 4/4 time)
    pub fn get_samples_per_bar(&self) -> u32 {
        self.samples_per_beat * 4
    }

    /// Calculate delay time in samples for tempo-synced effects
    pub fn get_tempo_synced_delay_samples(&self, note_division: NoteValue) -> u32 {
        let beat_samples = self.samples_per_beat as f32;
        
        match note_division {
            NoteValue::Whole => (beat_samples * 4.0) as u32,
            NoteValue::Half => (beat_samples * 2.0) as u32,
            NoteValue::Quarter => beat_samples as u32,
            NoteValue::Eighth => (beat_samples * 0.5) as u32,
            NoteValue::Sixteenth => (beat_samples * 0.25) as u32,
            NoteValue::ThirtySecond => (beat_samples * 0.125) as u32,
            NoteValue::QuarterTriplet => (beat_samples * 0.333) as u32,
            NoteValue::EighthTriplet => (beat_samples * 0.167) as u32,
            NoteValue::DottedQuarter => (beat_samples * 1.5) as u32,
            NoteValue::DottedEighth => (beat_samples * 0.75) as u32,
        }
    }

    /// Get tap tempo status for display
    pub fn get_tap_status(&self) -> TapStatus {
        TapStatus {
            tap_count: self.tap_count,
            is_active: self.tap_count > 0 && 
                      (self.last_tap_time > 0) && 
                      (self.last_tap_time + 2000 > self.last_tap_time), // Simplified check
            calculated_bpm: if self.tap_count >= 2 {
                Some(60000.0 / self.calculate_average_tap_interval())
            } else {
                None
            },
        }
    }

    /// Get MIDI sync status
    pub fn get_midi_sync_status(&self) -> MidiSyncStatus {
        MidiSyncStatus {
            enabled: self.midi_sync_enabled,
            external_bpm: if self.midi_sync_enabled { Some(self.external_bpm) } else { None },
            clock_counter: self.midi_clock_counter,
            is_receiving_clock: self.midi_sync_enabled && 
                               self.last_midi_clock_time > 0 &&
                               (self.last_midi_clock_time + 1000 > self.last_midi_clock_time), // Simplified check
        }
    }
}

/// Note values for tempo-synced effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteValue {
    /// Whole note (4 beats)
    Whole,
    /// Half note (2 beats)
    Half,
    /// Quarter note (1 beat)
    Quarter,
    /// Eighth note (1/2 beat)
    Eighth,
    /// Sixteenth note (1/4 beat)
    Sixteenth,
    /// Thirty-second note (1/8 beat)
    ThirtySecond,
    /// Quarter note triplet
    QuarterTriplet,
    /// Eighth note triplet
    EighthTriplet,
    /// Dotted quarter note (1.5 beats)
    DottedQuarter,
    /// Dotted eighth note (0.75 beats)
    DottedEighth,
}

impl Default for NoteValue {
    fn default() -> Self {
        NoteValue::Quarter
    }
}

/// Tap tempo status information
#[derive(Debug, Clone)]
pub struct TapStatus {
    /// Number of tap samples collected
    pub tap_count: u8,
    /// Whether tap tempo is currently active
    pub is_active: bool,
    /// Calculated BPM from taps (if available)
    pub calculated_bpm: Option<f32>,
}

/// MIDI sync status information
#[derive(Debug, Clone)]
pub struct MidiSyncStatus {
    /// Whether MIDI sync is enabled
    pub enabled: bool,
    /// External BPM from MIDI clock (if syncing)
    pub external_bpm: Option<f32>,
    /// Current MIDI clock counter
    pub clock_counter: u32,
    /// Whether MIDI clock is being received
    pub is_receiving_clock: bool,
}

/// Tempo-locked effect parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoLockedParameter {
    /// Note value for tempo sync
    pub note_value: NoteValue,
    /// Whether tempo sync is enabled
    pub tempo_sync_enabled: bool,
    /// Manual value (used when tempo sync is disabled)
    pub manual_value: f32,
}

impl TempoLockedParameter {
    /// Create new tempo-locked parameter
    pub fn new(note_value: NoteValue) -> Self {
        Self {
            note_value,
            tempo_sync_enabled: true,
            manual_value: 0.5,
        }
    }

    /// Get the current value based on tempo system
    pub fn get_value(&self, tempo_system: &TempoSystem) -> f32 {
        if self.tempo_sync_enabled {
            // Return tempo-synced value (normalized)
            let samples = tempo_system.get_tempo_synced_delay_samples(self.note_value);
            // Normalize to 0.0-1.0 range (this is effect-specific)
            (samples as f32 / tempo_system.get_samples_per_beat() as f32).clamp(0.0, 1.0)
        } else {
            self.manual_value
        }
    }

    /// Set manual value
    pub fn set_manual_value(&mut self, value: f32) {
        self.manual_value = value.clamp(0.0, 1.0);
    }

    /// Toggle tempo sync
    pub fn toggle_tempo_sync(&mut self) {
        self.tempo_sync_enabled = !self.tempo_sync_enabled;
    }
}

impl Default for TempoLockedParameter {
    fn default() -> Self {
        Self::new(NoteValue::Quarter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tempo_system_creation() {
        let tempo_system = TempoSystem::new(44100);
        assert_eq!(tempo_system.get_bpm(), DEFAULT_BPM);
        assert!(!tempo_system.midi_sync_enabled);
    }

    #[test]
    fn test_set_bpm() {
        let mut tempo_system = TempoSystem::new(44100);
        tempo_system.set_bpm(140.0);
        assert_eq!(tempo_system.get_bpm(), 140.0);
        
        // Test clamping
        tempo_system.set_bpm(300.0);
        assert_eq!(tempo_system.get_bpm(), MAX_BPM);
        
        tempo_system.set_bpm(30.0);
        assert_eq!(tempo_system.get_bpm(), MIN_BPM);
    }

    #[test]
    fn test_tap_tempo() {
        let mut tempo_system = TempoSystem::new(44100);
        
        // First tap
        assert!(!tempo_system.tap_tempo(1000));
        
        // Second tap (500ms later = 120 BPM)
        assert!(tempo_system.tap_tempo(1500));
        assert!((tempo_system.get_bpm() - 120.0).abs() < 1.0);
    }

    #[test]
    fn test_beat_position_tracking() {
        let mut tempo_system = TempoSystem::new(44100);
        tempo_system.set_bpm(120.0); // 120 BPM = 22050 samples per beat at 44.1kHz
        
        // Process half a beat
        tempo_system.update_beat_position(11025);
        assert!((tempo_system.get_beat_position() - 0.5).abs() < 0.01);
        
        // Process another half beat (should wrap to 0)
        tempo_system.update_beat_position(11025);
        assert!(tempo_system.get_beat_position() < 0.01);
    }

    #[test]
    fn test_tempo_synced_delay() {
        let tempo_system = TempoSystem::new(44100);
        // At 120 BPM, quarter note should be 22050 samples
        let quarter_samples = tempo_system.get_tempo_synced_delay_samples(NoteValue::Quarter);
        assert!((quarter_samples as f32 - 22050.0).abs() < 100.0); // Allow some tolerance
        
        let eighth_samples = tempo_system.get_tempo_synced_delay_samples(NoteValue::Eighth);
        assert_eq!(eighth_samples, quarter_samples / 2);
    }
}