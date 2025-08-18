use heapless::Vec;
use serde::{Deserialize, Serialize};
use crate::effects::EffectChain;

/// Maximum audio buffer size per track (1.5 hours at 44.1kHz stereo)
/// 1.5 hours * 60 min/hour * 60 sec/min * 44100 samples/sec * 2 channels
pub const MAX_TRACK_SAMPLES: usize = 476_280_000;

/// Circular audio buffer using heapless Vec for embedded systems
pub type AudioBuffer = Vec<f32, MAX_TRACK_SAMPLES>;

/// Track states for the loopstation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackState {
    /// Track is stopped, no playback or recording
    Stopped,
    /// Track is actively recording new audio
    Recording,
    /// Track is playing back recorded audio
    Playing,
    /// Track is overdubbing (recording over existing audio)
    Overdubbing,
    /// Track is muted (has audio but not playing)
    Muted,
}

impl TrackState {
    /// Check if the track is actively processing audio
    pub fn is_active(&self) -> bool {
        matches!(self, TrackState::Recording | TrackState::Playing | TrackState::Overdubbing)
    }

    /// Check if the track is recording (including overdubbing)
    pub fn is_recording(&self) -> bool {
        matches!(self, TrackState::Recording | TrackState::Overdubbing)
    }

    /// Check if the track is playing back audio
    pub fn is_playing(&self) -> bool {
        matches!(self, TrackState::Playing | TrackState::Overdubbing)
    }

    /// Transition to the next logical state based on current state and input
    pub fn transition(&mut self, action: TrackAction) {
        *self = match (*self, action) {
            // From Stopped
            (TrackState::Stopped, TrackAction::RecordPlay) => TrackState::Recording,
            (TrackState::Stopped, TrackAction::Stop) => TrackState::Stopped,
            (TrackState::Stopped, TrackAction::Mute) => TrackState::Stopped,
            
            // From Recording
            (TrackState::Recording, TrackAction::RecordPlay) => TrackState::Playing,
            (TrackState::Recording, TrackAction::Stop) => TrackState::Stopped,
            (TrackState::Recording, TrackAction::Mute) => TrackState::Muted,
            
            // From Playing
            (TrackState::Playing, TrackAction::RecordPlay) => TrackState::Overdubbing,
            (TrackState::Playing, TrackAction::Stop) => TrackState::Stopped,
            (TrackState::Playing, TrackAction::Mute) => TrackState::Muted,
            
            // From Overdubbing
            (TrackState::Overdubbing, TrackAction::RecordPlay) => TrackState::Playing,
            (TrackState::Overdubbing, TrackAction::Stop) => TrackState::Stopped,
            (TrackState::Overdubbing, TrackAction::Mute) => TrackState::Muted,
            
            // From Muted
            (TrackState::Muted, TrackAction::RecordPlay) => TrackState::Playing,
            (TrackState::Muted, TrackAction::Stop) => TrackState::Stopped,
            (TrackState::Muted, TrackAction::Mute) => TrackState::Playing,
            
            // Clear action always goes to Stopped
            (_, TrackAction::Clear) => TrackState::Stopped,
        };
    }
}

/// Actions that can be performed on a track
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackAction {
    /// Record/Play button press (context-dependent)
    RecordPlay,
    /// Stop button press
    Stop,
    /// Mute/unmute toggle
    Mute,
    /// Clear track content
    Clear,
}

/// Playback modes for tracks
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PlaybackMode {
    /// Normal forward playback
    Normal,
    /// Reverse playback
    Reverse,
    /// Half speed playback
    HalfSpeed,
    /// Double speed playback
    DoubleSpeed,
    /// Pitch shift (semitones, -24 to +24)
    PitchShift(f32),
}

impl Default for PlaybackMode {
    fn default() -> Self {
        PlaybackMode::Normal
    }
}

/// Input source routing for tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputSource {
    /// Microphone input (XLR)
    Mic(u8),  // 1-4
    /// Instrument input (1/4" phone)
    Inst(u8), // 1-4
    /// USB audio interface input
    USB(u8),  // Channel number
}

impl Default for InputSource {
    fn default() -> Self {
        InputSource::Mic(1)
    }
}

/// Fade settings for smooth track transitions
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FadeSettings {
    /// Fade in time in seconds
    pub fade_in_time: f32,
    /// Fade out time in seconds
    pub fade_out_time: f32,
    /// Whether fading is enabled
    pub enabled: bool,
}

impl Default for FadeSettings {
    fn default() -> Self {
        Self {
            fade_in_time: 0.01, // 10ms default
            fade_out_time: 0.01,
            enabled: true,
        }
    }
}

/// Audio snapshot for undo/redo functionality
#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    /// Snapshot of audio buffer state
    pub buffer_length: usize,
    /// Track state at time of snapshot
    pub state: TrackState,
    /// Timestamp of snapshot
    pub timestamp: u32,
}

/// Individual track in the loopstation
#[derive(Debug, Clone)]
pub struct Track {
    /// Track ID (1-6)
    pub id: u8,
    /// Audio buffer for stereo samples
    pub audio_buffer: AudioBuffer,
    /// Current track state
    pub state: TrackState,
    /// Track volume (0.0-1.0)
    pub level: f32,
    /// Track pan (-1.0 to 1.0, -1.0=left, 1.0=right)
    pub pan: f32,
    /// Length of recorded loop in samples
    pub loop_length: u32,
    /// Current playback position in samples
    pub play_position: u32,
    /// Current record position in samples
    pub record_position: u32,
    /// Quantization enabled
    pub quantize_enabled: bool,
    /// Input source routing
    pub input_source: InputSource,
    /// Playback mode
    pub playback_mode: PlaybackMode,
    /// Fade in/out settings
    pub fade_settings: FadeSettings,
    /// Undo buffer for track operations
    pub undo_buffer: Vec<AudioSnapshot, 32>, // Max 32 undo levels
    /// Whether track is selected for editing
    pub selected: bool,
    /// Track-specific effects chain (post-recording processing)
    pub track_fx: EffectChain,
}

impl Track {
    /// Create a new track with the given ID
    pub fn new(id: u8) -> Self {
        Self {
            id,
            audio_buffer: AudioBuffer::new(),
            state: TrackState::Stopped,
            level: 1.0,
            pan: 0.0,
            loop_length: 0,
            play_position: 0,
            record_position: 0,
            quantize_enabled: false,
            input_source: InputSource::default(),
            playback_mode: PlaybackMode::default(),
            fade_settings: FadeSettings::default(),
            undo_buffer: Vec::new(),
            selected: false,
            track_fx: EffectChain::new_track_fx(),
        }
    }

    /// Start recording on this track
    pub fn start_recording(&mut self) {
        self.state.transition(TrackAction::RecordPlay);
        if self.state == TrackState::Recording {
            self.record_position = 0;
        }
    }

    /// Start playback on this track
    pub fn start_playback(&mut self) {
        // Only start playback if track has content
        if self.loop_length > 0 {
            self.state.transition(TrackAction::RecordPlay);
            if self.state == TrackState::Playing {
                self.play_position = 0;
            }
        }
    }

    /// Stop recording/playback on this track
    pub fn stop(&mut self) {
        self.state.transition(TrackAction::Stop);
        self.play_position = 0;
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) {
        self.state.transition(TrackAction::Mute);
    }

    /// Undo last action on this track (placeholder)
    pub fn undo_last_action(&mut self) {
        // Placeholder for undo system implementation
        // This would restore the previous state from undo buffer
    }

    /// Redo last undone action on this track (placeholder)
    pub fn redo_last_action(&mut self) {
        // Placeholder for redo system implementation
        // This would restore the next state from redo buffer
    }

    /// Clear track content
    pub fn clear(&mut self) {
        // Save current state for undo
        if let Ok(_) = self.undo_buffer.push(AudioSnapshot {
            buffer_length: self.audio_buffer.len(),
            state: self.state,
            timestamp: 0, // TODO: Add proper timestamp
        }) {
            // Undo buffer saved successfully
        }

        self.state.transition(TrackAction::Clear);
        self.audio_buffer.clear();
        self.loop_length = 0;
        self.play_position = 0;
        self.record_position = 0;
    }

    /// Get the current audio level for display
    pub fn get_level(&self) -> f32 {
        self.level
    }

    /// Set track volume level
    pub fn set_level(&mut self, level: f32) {
        self.level = level.clamp(0.0, 1.0);
    }

    /// Set track pan position
    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan.clamp(-1.0, 1.0);
    }

    /// Check if track has recorded audio
    pub fn has_audio(&self) -> bool {
        !self.audio_buffer.is_empty()
    }

    /// Get track duration in seconds
    pub fn duration_seconds(&self, sample_rate: u32) -> f32 {
        self.loop_length as f32 / sample_rate as f32 / 2.0 // Stereo
    }

    /// Process audio for this track
    pub fn process_audio(&mut self, input: &[f32], output: &mut [f32], sample_rate: u32) {
        let buffer_len = input.len().min(output.len());
        
        // Temporary buffer for track processing before effects
        let mut temp_buffer = [0.0f32; 512]; // Max buffer size for embedded
        let temp_len = buffer_len.min(512);
        
        match self.state {
            TrackState::Recording => {
                self.record_audio(input, buffer_len);
                // During recording, pass input to temp buffer for monitoring
                temp_buffer[..temp_len].copy_from_slice(&input[..temp_len]);
            },
            TrackState::Playing => {
                self.playback_audio(&mut temp_buffer[..temp_len], temp_len);
            },
            TrackState::Overdubbing => {
                self.record_audio(input, buffer_len);
                self.playback_audio(&mut temp_buffer[..temp_len], temp_len);
            },
            TrackState::Stopped | TrackState::Muted => {
                // Output silence
                temp_buffer[..temp_len].fill(0.0);
            },
        }

        // Process through Track FX chain
        self.track_fx.process_audio(&temp_buffer[..temp_len], &mut output[..temp_len], sample_rate as f32);
    }

    /// Record audio into the track buffer
    fn record_audio(&mut self, input: &[f32], buffer_len: usize) {
        for i in 0..buffer_len {
            if self.record_position < MAX_TRACK_SAMPLES as u32 {
                // For overdubbing, mix with existing audio
                if self.state == TrackState::Overdubbing && (self.record_position as usize) < self.audio_buffer.len() {
                    if let Some(existing) = self.audio_buffer.get_mut(self.record_position as usize) {
                        *existing = (*existing + input[i]) * 0.5; // Simple mix
                    }
                } else {
                    // New recording
                    if self.audio_buffer.push(input[i]).is_err() {
                        // Buffer full, stop recording
                        self.state = TrackState::Playing;
                        break;
                    }
                }
                self.record_position += 1;
            } else {
                // Buffer full, stop recording
                self.state = TrackState::Playing;
                break;
            }
        }

        // Update loop length if we're recording new content
        if self.state == TrackState::Recording {
            self.loop_length = self.record_position as u32;
        }
    }

    /// Playback audio from the track buffer
    fn playback_audio(&mut self, output: &mut [f32], buffer_len: usize) {
        if self.loop_length == 0 {
            output[..buffer_len].fill(0.0);
            return;
        }

        for i in 0..buffer_len {
            let sample = if (self.play_position as usize) < self.audio_buffer.len() {
                self.audio_buffer[self.play_position as usize]
            } else {
                0.0
            };

            // Apply volume and pan
            let (left, right) = self.apply_pan_and_volume(sample);
            
            // For stereo output, interleave samples
            if i % 2 == 0 {
                output[i] = left;
            } else {
                output[i] = right;
            }

            // Advance playback position with looping
            self.play_position += 1;
            if self.play_position >= self.loop_length {
                self.play_position = 0; // Loop back to start
            }
        }
    }

    /// Apply pan and volume to a mono sample, returning (left, right)
    fn apply_pan_and_volume(&self, sample: f32) -> (f32, f32) {
        let volume_sample = sample * self.level;
        
        // Pan calculation: -1.0 = full left, 0.0 = center, 1.0 = full right
        let left_gain = if self.pan <= 0.0 { 1.0 } else { 1.0 - self.pan };
        let right_gain = if self.pan >= 0.0 { 1.0 } else { 1.0 + self.pan };
        
        (volume_sample * left_gain, volume_sample * right_gain)
    }
}

/// Audio processing engine for the loopstation
pub struct AudioEngine {
    /// Sample rate (44.1kHz as per requirements)
    pub sample_rate: u32,
    /// Audio buffer size for processing
    pub buffer_size: usize,
    /// All 6 tracks
    pub tracks: [Track; 6],
    /// Master volume level
    pub master_level: f32,
    /// Audio callback active flag
    pub callback_active: bool,
    /// Processing statistics
    pub stats: AudioStats,
    /// Input effects chain (pre-recording)
    pub input_fx: EffectChain,
    /// Master effects chain (final output)
    pub master_fx: EffectChain,
}

/// Audio processing statistics
#[derive(Debug, Default)]
pub struct AudioStats {
    /// Total samples processed
    pub samples_processed: u64,
    /// Number of buffer underruns
    pub underruns: u32,
    /// Number of buffer overruns  
    pub overruns: u32,
    /// Current CPU usage estimate (0.0-1.0)
    pub cpu_usage: f32,
}

impl AudioEngine {
    /// Create a new audio engine
    pub fn new(sample_rate: u32, buffer_size: usize) -> Self {
        Self {
            sample_rate,
            buffer_size,
            tracks: [
                Track::new(1), Track::new(2), Track::new(3),
                Track::new(4), Track::new(5), Track::new(6)
            ],
            master_level: 1.0,
            callback_active: false,
            stats: AudioStats::default(),
            input_fx: EffectChain::new_input_fx(),
            master_fx: EffectChain::new_master_fx(),
        }
    }

    /// Start the audio processing callback
    pub fn start_callback(&mut self) {
        self.callback_active = true;
    }

    /// Stop the audio processing callback
    pub fn stop_callback(&mut self) {
        self.callback_active = false;
    }

    /// Main audio processing callback - processes all 6 tracks
    /// This is called from the DMA interrupt at 44.1kHz
    /// 
    /// This method now implements the 3-layer FX architecture:
    /// 1. Input FX - applied to input before recording
    /// 2. Track FX - applied to individual track playback (handled in Track::process_audio)
    /// 3. Master FX - applied to final mixed output
    pub fn process_callback(&mut self, input_buffer: &[f32], output_buffer: &mut [f32]) {
        if !self.callback_active {
            output_buffer.fill(0.0);
            return;
        }

        let buffer_len = input_buffer.len().min(output_buffer.len());
        
        // Clear output buffer
        output_buffer[..buffer_len].fill(0.0);
        
        // Temporary buffers for processing
        let mut processed_input = [0.0f32; 512]; // Input after Input FX
        let mut track_output = [0.0f32; 512]; // Individual track output
        let mut mixed_output = [0.0f32; 512]; // Mixed tracks before Master FX
        let buffer_size = buffer_len.min(512);
        
        // Clear buffers
        processed_input[..buffer_size].fill(0.0);
        mixed_output[..buffer_size].fill(0.0);
        
        // Process each track
        for track in &mut self.tracks {
            if track.state.is_active() {
                // Clear track output buffer
                track_output[..buffer_size].fill(0.0);
                
                // For recording tracks, apply Input FX to the input signal first
                let track_input = if track.state.is_recording() {
                    // Apply Input FX to input signal (affects recorded audio)
                    // Input FX is shared across all recording tracks
                    self.input_fx.process_audio(&input_buffer[..buffer_size], &mut processed_input[..buffer_size], self.sample_rate as f32);
                    &processed_input[..buffer_size]
                } else {
                    // For playback-only tracks, use original input
                    &input_buffer[..buffer_size]
                };
                
                // Process this track (includes Track FX processing)
                track.process_audio(track_input, &mut track_output[..buffer_size], self.sample_rate);
                
                // Mix track output into main mix
                for i in 0..buffer_size {
                    mixed_output[i] += track_output[i];
                }
            }
        }

        // Apply master volume to mixed output
        for sample in &mut mixed_output[..buffer_size] {
            *sample *= self.master_level;
        }

        // Apply Master FX to final mixed output
        self.master_fx.process_audio(&mixed_output[..buffer_size], &mut output_buffer[..buffer_size], self.sample_rate as f32);

        // Update statistics
        self.stats.samples_processed += buffer_len as u64;
    }

    /// Get track by ID (1-6)
    pub fn get_track(&self, track_id: u8) -> Option<&Track> {
        if track_id >= 1 && track_id <= 6 {
            Some(&self.tracks[(track_id - 1) as usize])
        } else {
            None
        }
    }

    /// Get mutable track by ID (1-6)
    pub fn get_track_mut(&mut self, track_id: u8) -> Option<&mut Track> {
        if track_id >= 1 && track_id <= 6 {
            Some(&mut self.tracks[(track_id - 1) as usize])
        } else {
            None
        }
    }

    /// Start recording on a track
    pub fn start_recording(&mut self, track_id: u8) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.start_recording();
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Start playback on a track
    pub fn start_playback(&mut self, track_id: u8) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.start_playback();
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Stop recording/playback on a track
    pub fn stop_track(&mut self, track_id: u8) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.stop();
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Toggle mute on a track
    pub fn toggle_mute(&mut self, track_id: u8) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.toggle_mute();
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Clear a track
    pub fn clear_track(&mut self, track_id: u8) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.clear();
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Set track level
    pub fn set_track_level(&mut self, track_id: u8, level: f32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.set_level(level);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Set track pan
    pub fn set_track_pan(&mut self, track_id: u8, pan: f32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.set_pan(pan);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Set master level
    pub fn set_master_level(&mut self, level: f32) {
        self.master_level = level.clamp(0.0, 1.0);
    }

    /// Get current audio statistics
    pub fn get_stats(&self) -> &AudioStats {
        &self.stats
    }

    /// Check if any tracks are recording
    pub fn is_recording(&self) -> bool {
        self.tracks.iter().any(|track| track.state.is_recording())
    }

    /// Check if any tracks are playing
    pub fn is_playing(&self) -> bool {
        self.tracks.iter().any(|track| track.state.is_playing())
    }

    /// Update audio engine state (called from main loop)
    pub fn update(&mut self) {
        // Update track states, handle tempo changes, etc.
        for track in &mut self.tracks {
            // Update track-specific state if needed
        }
        
        // Update effect chains
        self.input_fx.update();
        self.master_fx.update();
        
        for track in &mut self.tracks {
            track.track_fx.update();
        }
    }

    /// Get total recording time across all tracks
    pub fn total_recording_time(&self) -> f32 {
        self.tracks.iter()
            .map(|track| track.duration_seconds(self.sample_rate))
            .sum()
    }

    /// Get input effects chain
    pub fn input_fx(&self) -> &EffectChain {
        &self.input_fx
    }

    /// Get mutable input effects chain
    pub fn input_fx_mut(&mut self) -> &mut EffectChain {
        &mut self.input_fx
    }

    /// Get master effects chain
    pub fn master_fx(&self) -> &EffectChain {
        &self.master_fx
    }

    /// Get mutable master effects chain
    pub fn master_fx_mut(&mut self) -> &mut EffectChain {
        &mut self.master_fx
    }

    /// Get track effects chain for a specific track
    pub fn track_fx(&self, track_id: u8) -> Option<&EffectChain> {
        if track_id >= 1 && track_id <= 6 {
            Some(&self.tracks[(track_id - 1) as usize].track_fx)
        } else {
            None
        }
    }

    /// Get mutable track effects chain for a specific track
    pub fn track_fx_mut(&mut self, track_id: u8) -> Option<&mut EffectChain> {
        if track_id >= 1 && track_id <= 6 {
            Some(&mut self.tracks[(track_id - 1) as usize].track_fx)
        } else {
            None
        }
    }

    /// Update tempo for all tempo-synced effects
    pub fn update_tempo(&mut self, bpm: f32) {
        self.input_fx.update_tempo(bpm);
        self.master_fx.update_tempo(bpm);
        
        for track in &mut self.tracks {
            track.track_fx.update_tempo(bpm);
        }
    }
}

/// Circular buffer for efficient audio processing
pub struct CircularBuffer<T> {
    buffer: Vec<T, MAX_TRACK_SAMPLES>,
    read_pos: usize,
    write_pos: usize,
    size: usize,
}

impl<T: Copy + Default> CircularBuffer<T> {
    /// Create a new circular buffer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            read_pos: 0,
            write_pos: 0,
            size: 0,
        }
    }

    /// Write data to the buffer
    pub fn write(&mut self, data: &[T]) -> usize {
        let mut written = 0;
        for &item in data {
            if self.size < MAX_TRACK_SAMPLES {
                if self.buffer.push(item).is_ok() {
                    self.write_pos = (self.write_pos + 1) % MAX_TRACK_SAMPLES;
                    self.size += 1;
                    written += 1;
                } else {
                    break;
                }
            } else {
                // Buffer full, overwrite oldest data
                if let Some(slot) = self.buffer.get_mut(self.write_pos) {
                    *slot = item;
                    self.write_pos = (self.write_pos + 1) % MAX_TRACK_SAMPLES;
                    self.read_pos = (self.read_pos + 1) % MAX_TRACK_SAMPLES;
                    written += 1;
                }
            }
        }
        written
    }

    /// Read data from the buffer
    pub fn read(&mut self, data: &mut [T]) -> usize {
        let mut read = 0;
        for slot in data {
            if self.size > 0 {
                if let Some(&item) = self.buffer.get(self.read_pos) {
                    *slot = item;
                    self.read_pos = (self.read_pos + 1) % self.buffer.len();
                    self.size -= 1;
                    read += 1;
                } else {
                    break;
                }
            } else {
                *slot = T::default();
            }
        }
        read
    }

    /// Get available space for writing
    pub fn available_write(&self) -> usize {
        MAX_TRACK_SAMPLES - self.size
    }

    /// Get available data for reading
    pub fn available_read(&self) -> usize {
        self.size
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.read_pos = 0;
        self.write_pos = 0;
        self.size = 0;
    }
}

/// Audio processing errors
#[derive(Debug)]
pub enum AudioError {
    InvalidTrack,
    BufferFull,
    BufferEmpty,
    InvalidSampleRate,
    CallbackNotActive,
}