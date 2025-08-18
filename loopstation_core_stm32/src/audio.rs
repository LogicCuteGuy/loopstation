use heapless::Vec;
use serde::{Deserialize, Serialize};
use crate::effects::EffectChain;

/// Audio buffer size for processing (matches HAL definition)
pub const AUDIO_BUFFER_SIZE: usize = 256;

/// Circular buffer implementation for audio data with serialization support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularBuffer<T> {
    /// Internal buffer storage
    buffer: Vec<T, MAX_TRACK_SAMPLES>,
    /// Write position in the buffer
    write_pos: usize,
    /// Read position in the buffer
    read_pos: usize,
    /// Whether the buffer has wrapped around
    wrapped: bool,
}

impl<T: Clone + Default> CircularBuffer<T> {
    /// Create a new circular buffer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            write_pos: 0,
            read_pos: 0,
            wrapped: false,
        }
    }

    /// Write data to the buffer
    pub fn write(&mut self, data: &[T]) {
        for item in data {
            if self.buffer.len() < MAX_TRACK_SAMPLES {
                let _ = self.buffer.push(item.clone());
                self.write_pos = self.buffer.len();
            } else {
                self.buffer[self.write_pos] = item.clone();
                self.write_pos = (self.write_pos + 1) % MAX_TRACK_SAMPLES;
                if self.write_pos == 0 {
                    self.wrapped = true;
                }
            }
        }
    }

    /// Read data from the buffer
    pub fn read(&mut self, output: &mut [T]) -> usize {
        let mut read_count = 0;
        for item in output.iter_mut() {
            if self.read_pos < self.buffer.len() {
                *item = self.buffer[self.read_pos].clone();
                self.read_pos = (self.read_pos + 1) % self.buffer.len();
                read_count += 1;
            } else {
                break;
            }
        }
        read_count
    }

    /// Get the current length of valid data
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_pos = 0;
        self.read_pos = 0;
        self.wrapped = false;
    }

    /// Get a slice of the buffer data
    pub fn as_slice(&self) -> &[T] {
        &self.buffer
    }

    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        MAX_TRACK_SAMPLES
    }

    /// Push a single item to the buffer
    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.buffer.len() < MAX_TRACK_SAMPLES {
            self.buffer.push(item)
        } else {
            Err(item)
        }
    }

    /// Get mutable reference to item at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.buffer.get_mut(index)
    }

    /// Get iterator over buffer contents
    pub fn iter(&self) -> core::slice::Iter<T> {
        self.buffer.iter()
    }
}

impl<T> core::ops::Index<usize> for CircularBuffer<T> {
    type Output = T;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.buffer[index]
    }
}

/// Maximum audio buffer size per track (1.5 hours at 44.1kHz stereo)
/// 1.5 hours * 60 min/hour * 60 sec/min * 44100 samples/sec * 2 channels
pub const MAX_TRACK_SAMPLES: usize = 476_280_000;

/// Circular audio buffer using heapless Vec for embedded systems
pub type AudioBuffer = CircularBuffer<f32>;

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

/// Action types that can be undone/redone
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UndoableAction {
    /// Track recording started
    StartRecording { track_id: u8 },
    /// Track recording stopped
    StopRecording { track_id: u8, buffer_length: usize },
    /// Track overdubbing started
    StartOverdubbing { track_id: u8 },
    /// Track overdubbing stopped
    StopOverdubbing { track_id: u8, buffer_length: usize },
    /// Track cleared
    ClearTrack { track_id: u8, buffer_length: usize, previous_state: TrackState },
    /// Track level changed
    SetTrackLevel { track_id: u8, previous_level: f32, new_level: f32 },
    /// Track pan changed
    SetTrackPan { track_id: u8, previous_pan: f32, new_pan: f32 },
    /// Track muted/unmuted
    ToggleMute { track_id: u8, previous_state: TrackState },
}

/// Audio snapshot for undo/redo functionality with compressed audio data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSnapshot {
    /// Action that was performed
    pub action: UndoableAction,
    /// Timestamp of snapshot
    pub timestamp: u32,
    /// Track state before the action
    pub previous_state: TrackState,
    /// Track state after the action
    pub new_state: TrackState,
}

/// Individual track in the loopstation
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Undo buffer for track operations (configurable size based on undo mode)
    pub undo_buffer: Vec<AudioSnapshot, 16>, // Max 16 undo levels
    /// Redo buffer for track operations
    pub redo_buffer: Vec<AudioSnapshot, 16>, // Max 16 redo levels
    /// Current undo position in the buffer
    pub undo_position: usize,
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
            redo_buffer: Vec::new(),
            undo_position: 0,
            selected: false,
            track_fx: EffectChain::new_track_fx(),
        }
    }

    /// Start recording on this track
    pub fn start_recording(&mut self, timestamp: u32) {
        // Add undo action before changing state
        self.add_undo_action(UndoableAction::StartRecording { track_id: self.id }, timestamp);
        
        self.state.transition(TrackAction::RecordPlay);
        if self.state == TrackState::Recording {
            self.record_position = 0;
        }
    }

    /// Start playback on this track
    pub fn start_playback(&mut self, timestamp: u32) {
        // Only start playback if track has content
        if self.loop_length > 0 {
            // Add undo action for overdubbing if transitioning from playing to overdubbing
            if self.state == TrackState::Playing {
                self.add_undo_action(UndoableAction::StartOverdubbing { track_id: self.id }, timestamp);
            }
            
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
    pub fn toggle_mute(&mut self, timestamp: u32) {
        let previous_state = self.state;
        self.add_undo_action(UndoableAction::ToggleMute { 
            track_id: self.id, 
            previous_state 
        }, timestamp);
        
        self.state.transition(TrackAction::Mute);
    }

    /// Add an action to the undo buffer
    pub fn add_undo_action(&mut self, action: UndoableAction, timestamp: u32) {
        let snapshot = AudioSnapshot {
            action: action.clone(),
            timestamp,
            previous_state: self.state,
            new_state: self.state, // Will be updated after action is performed
        };

        // Clear redo buffer when new action is added
        self.redo_buffer.clear();

        // Add to undo buffer
        if self.undo_buffer.push(snapshot).is_err() {
            // Buffer full, remove oldest entry
            self.undo_buffer.remove(0);
            let _ = self.undo_buffer.push(AudioSnapshot {
                action,
                timestamp,
                previous_state: self.state,
                new_state: self.state,
            });
        }

        self.undo_position = self.undo_buffer.len();
    }

    /// Undo last action on this track
    pub fn undo_last_action(&mut self, undo_mode: crate::settings::UndoMode) -> bool {
        if self.undo_position == 0 || self.undo_buffer.is_empty() {
            return false; // Nothing to undo
        }

        self.undo_position -= 1;
        let snapshot = &self.undo_buffer[self.undo_position];

        // Check if this action type is allowed by the undo mode
        if !self.is_action_undoable(&snapshot.action, undo_mode) {
            self.undo_position += 1; // Restore position
            return false;
        }

        // Perform the undo operation
        match &snapshot.action {
            UndoableAction::StartRecording { .. } => {
                // Undo start recording - stop recording and clear any recorded data
                if self.state == TrackState::Recording {
                    self.state = TrackState::Stopped;
                    // Clear any audio recorded since start
                    if self.record_position > 0 {
                        // Truncate buffer to remove recorded audio
                        while self.audio_buffer.len() > (self.loop_length as usize) {
                            self.audio_buffer.buffer.pop();
                        }
                        self.record_position = 0;
                    }
                }
            },
            UndoableAction::StopRecording { buffer_length, .. } => {
                // Undo stop recording - truncate buffer to previous length and return to recording state
                while self.audio_buffer.len() > *buffer_length {
                    self.audio_buffer.buffer.pop();
                }
                self.state = TrackState::Recording;
                self.loop_length = *buffer_length as u32;
            },
            UndoableAction::StartOverdubbing { .. } => {
                // Undo start overdubbing - return to playing state
                if self.state == TrackState::Overdubbing {
                    self.state = TrackState::Playing;
                }
            },
            UndoableAction::StopOverdubbing { buffer_length, .. } => {
                // Undo stop overdubbing - restore buffer to previous length
                while self.audio_buffer.len() > *buffer_length {
                    self.audio_buffer.buffer.pop();
                }
                self.state = TrackState::Overdubbing;
                self.loop_length = *buffer_length as u32;
            },
            UndoableAction::ClearTrack { buffer_length, previous_state, .. } => {
                // Undo clear track - restore previous state (audio cannot be fully restored without storage)
                self.state = *previous_state;
                self.loop_length = *buffer_length as u32;
                self.play_position = 0;
                self.record_position = 0;
                // Note: Audio data cannot be fully restored without additional storage
                // This is a limitation of the lightweight undo system
            },
            UndoableAction::SetTrackLevel { previous_level, .. } => {
                // Undo level change
                self.level = *previous_level;
            },
            UndoableAction::SetTrackPan { previous_pan, .. } => {
                // Undo pan change
                self.pan = *previous_pan;
            },
            UndoableAction::ToggleMute { previous_state, .. } => {
                // Undo mute toggle
                self.state = *previous_state;
            },
        }

        // Move the undone action to redo buffer
        if let Some(undone_action) = self.undo_buffer.get(self.undo_position).cloned() {
            if self.redo_buffer.push(undone_action).is_err() {
                // Redo buffer full, remove oldest
                self.redo_buffer.remove(0);
                let _ = self.redo_buffer.push(self.undo_buffer[self.undo_position].clone());
            }
        }

        true
    }

    /// Redo last undone action on this track
    pub fn redo_last_action(&mut self, undo_mode: crate::settings::UndoMode) -> bool {
        if self.redo_buffer.is_empty() {
            return false; // Nothing to redo
        }

        let snapshot = self.redo_buffer.pop().unwrap();

        // Check if this action type is allowed by the undo mode
        if !self.is_action_undoable(&snapshot.action, undo_mode) {
            // Put it back
            let _ = self.redo_buffer.push(snapshot);
            return false;
        }

        // Perform the redo operation (opposite of undo)
        match &snapshot.action {
            UndoableAction::StartRecording { .. } => {
                // Redo start recording
                self.state = TrackState::Recording;
                self.record_position = 0;
            },
            UndoableAction::StopRecording { .. } => {
                // Redo stop recording
                self.state = TrackState::Playing;
            },
            UndoableAction::StartOverdubbing { .. } => {
                // Redo start overdubbing
                self.state = TrackState::Overdubbing;
            },
            UndoableAction::StopOverdubbing { .. } => {
                // Redo stop overdubbing - return to playing state
                self.state = TrackState::Playing;
            },
            UndoableAction::ClearTrack { .. } => {
                // Redo clear track
                self.audio_buffer.clear();
                self.state = TrackState::Stopped;
                self.loop_length = 0;
                self.play_position = 0;
                self.record_position = 0;
            },
            UndoableAction::SetTrackLevel { new_level, .. } => {
                // Redo level change
                self.level = *new_level;
            },
            UndoableAction::SetTrackPan { new_pan, .. } => {
                // Redo pan change
                self.pan = *new_pan;
            },
            UndoableAction::ToggleMute { .. } => {
                // Redo mute toggle - use the new state from snapshot
                self.state = snapshot.new_state;
            },
        }

        // Move back to undo buffer
        if self.undo_buffer.push(snapshot).is_err() {
            // Undo buffer full, remove oldest
            self.undo_buffer.remove(0);
            let _ = self.undo_buffer.push(self.redo_buffer[self.redo_buffer.len() - 1].clone());
        }

        self.undo_position = self.undo_buffer.len();
        true
    }

    /// Check if an action type is undoable based on the undo mode
    fn is_action_undoable(&self, action: &UndoableAction, undo_mode: crate::settings::UndoMode) -> bool {
        use crate::settings::UndoMode;
        
        match undo_mode {
            UndoMode::RecordOnly => {
                matches!(action, 
                    UndoableAction::StartRecording { .. } |
                    UndoableAction::StopRecording { .. } |
                    UndoableAction::StartOverdubbing { .. } |
                    UndoableAction::StopOverdubbing { .. } |
                    UndoableAction::ClearTrack { .. }
                )
            },
            UndoMode::RecordAndPlay => {
                matches!(action, 
                    UndoableAction::StartRecording { .. } |
                    UndoableAction::StopRecording { .. } |
                    UndoableAction::StartOverdubbing { .. } |
                    UndoableAction::StopOverdubbing { .. } |
                    UndoableAction::ClearTrack { .. } |
                    UndoableAction::ToggleMute { .. }
                )
            },
            UndoMode::All => true, // All actions are undoable
        }
    }

    /// Get the number of available undo actions
    pub fn undo_count(&self) -> usize {
        self.undo_position
    }

    /// Get the number of available redo actions
    pub fn redo_count(&self) -> usize {
        self.redo_buffer.len()
    }

    /// Clear all undo/redo history
    pub fn clear_undo_history(&mut self) {
        self.undo_buffer.clear();
        self.redo_buffer.clear();
        self.undo_position = 0;
    }

    /// Clear track content
    pub fn clear(&mut self, timestamp: u32) {
        let buffer_length = self.audio_buffer.len();
        let previous_state = self.state;
        
        self.add_undo_action(UndoableAction::ClearTrack { 
            track_id: self.id, 
            buffer_length,
            previous_state
        }, timestamp);

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
    pub fn set_level(&mut self, level: f32, timestamp: u32) {
        let previous_level = self.level;
        let new_level = level.clamp(0.0, 1.0);
        
        // Only add undo action if level actually changed
        if (previous_level - new_level).abs() > 0.001 {
            self.add_undo_action(UndoableAction::SetTrackLevel { 
                track_id: self.id, 
                previous_level,
                new_level
            }, timestamp);
        }
        
        self.level = new_level;
    }

    /// Set track pan position
    pub fn set_pan(&mut self, pan: f32, timestamp: u32) {
        let previous_pan = self.pan;
        let new_pan = pan.clamp(-1.0, 1.0);
        
        // Only add undo action if pan actually changed
        if (previous_pan - new_pan).abs() > 0.001 {
            self.add_undo_action(UndoableAction::SetTrackPan { 
                track_id: self.id, 
                previous_pan,
                new_pan
            }, timestamp);
        }
        
        self.pan = new_pan;
    }

    /// Check if track has recorded audio
    pub fn has_audio(&self) -> bool {
        !self.audio_buffer.is_empty()
    }

    /// Get track duration in seconds
    pub fn duration_seconds(&self, sample_rate: u32) -> f32 {
        self.loop_length as f32 / sample_rate as f32 / 2.0 // Stereo
    }

    /// Export audio buffer as WAV format data
    pub fn export_wav_data(&self, sample_rate: u32) -> Result<Vec<u8, 65536>, &'static str> {
        if self.audio_buffer.is_empty() {
            return Err("No audio data to export");
        }

        // Simple WAV header creation (44 bytes)
        let mut wav_data = Vec::new();
        
        // WAV header
        let audio_data = self.audio_buffer.as_slice();
        let data_size = audio_data.len() * 4; // 4 bytes per f32 sample
        let file_size = 36 + data_size;
        
        // RIFF header
        let _ = wav_data.extend_from_slice(b"RIFF");
        let _ = wav_data.extend_from_slice(&(file_size as u32).to_le_bytes());
        let _ = wav_data.extend_from_slice(b"WAVE");
        
        // fmt chunk
        let _ = wav_data.extend_from_slice(b"fmt ");
        let _ = wav_data.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        let _ = wav_data.extend_from_slice(&3u16.to_le_bytes()); // IEEE float format
        let _ = wav_data.extend_from_slice(&2u16.to_le_bytes()); // stereo
        let _ = wav_data.extend_from_slice(&sample_rate.to_le_bytes());
        let _ = wav_data.extend_from_slice(&(sample_rate * 2 * 4).to_le_bytes()); // byte rate
        let _ = wav_data.extend_from_slice(&8u16.to_le_bytes()); // block align
        let _ = wav_data.extend_from_slice(&32u16.to_le_bytes()); // bits per sample
        
        // data chunk
        let _ = wav_data.extend_from_slice(b"data");
        let _ = wav_data.extend_from_slice(&(data_size as u32).to_le_bytes());
        
        // Audio data (convert f32 to bytes)
        for sample in audio_data {
            let _ = wav_data.extend_from_slice(&sample.to_le_bytes());
        }
        
        Ok(wav_data)
    }

    /// Import audio data from WAV format
    pub fn import_wav_data(&mut self, wav_data: &[u8]) -> Result<(), &'static str> {
        if wav_data.len() < 44 {
            return Err("Invalid WAV file - too small");
        }

        // Simple WAV parsing (skip header validation for now)
        let data_start = 44; // Standard WAV header size
        let audio_bytes = &wav_data[data_start..];
        
        // Clear existing audio
        self.clear(0); // Use timestamp 0 for import operations
        
        // Convert bytes back to f32 samples
        let mut samples: Vec<f32, 65536> = Vec::new();
        for chunk in audio_bytes.chunks_exact(4) {
            if let Ok(bytes) = chunk.try_into() {
                let sample = f32::from_le_bytes(bytes);
                if samples.push(sample).is_err() {
                    break; // Buffer full
                }
            }
        }
        
        // Write samples to audio buffer
        self.audio_buffer.write(&samples);
        self.loop_length = samples.len() as u32;
        
        Ok(())
    }

    /// Create compressed audio snapshot for undo system
    pub fn create_audio_snapshot(&self, action: UndoableAction, timestamp: u32) -> AudioSnapshot {
        AudioSnapshot {
            action,
            timestamp,
            previous_state: self.state,
            new_state: self.state,
        }
    }

    /// Restore from audio snapshot
    pub fn restore_from_snapshot(&mut self, snapshot: &AudioSnapshot) {
        // In a full implementation, this would restore the actual audio data
        // For now, we just restore the metadata
        self.state = snapshot.previous_state;
        // Audio restoration would require storing compressed audio data
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
    pub fn record_audio(&mut self, input: &[f32], buffer_len: usize) {
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
    pub fn playback_audio(&self, output: &mut [f32], buffer_len: usize) {
        if self.loop_length == 0 {
            output[..buffer_len].fill(0.0);
            return;
        }

        let mut current_position = self.play_position;

        for i in 0..buffer_len {
            let sample = if (current_position as usize) < self.audio_buffer.len() {
                self.audio_buffer[current_position as usize]
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
            current_position += 1;
            if current_position >= self.loop_length {
                current_position = 0; // Loop back to start
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
    pub fn start_recording(&mut self, track_id: u8, timestamp: u32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.start_recording(timestamp);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Start playback on a track
    pub fn start_playback(&mut self, track_id: u8, timestamp: u32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.start_playback(timestamp);
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
    pub fn toggle_mute(&mut self, track_id: u8, timestamp: u32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.toggle_mute(timestamp);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Clear a track
    pub fn clear_track(&mut self, track_id: u8, timestamp: u32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.clear(timestamp);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Set track level
    pub fn set_track_level(&mut self, track_id: u8, level: f32, timestamp: u32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.set_level(level, timestamp);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Set track pan
    pub fn set_track_pan(&mut self, track_id: u8, pan: f32, timestamp: u32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.set_pan(pan, timestamp);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Set master level
    pub fn set_master_level(&mut self, level: f32) {
        self.master_level = level.clamp(0.0, 1.0);
    }

    /// Get master level
    pub fn get_master_level(&self) -> f32 {
        self.master_level
    }

    /// Set track level internally (for modulation, without triggering MIDI)
    pub fn set_track_level_internal(&mut self, track_id: u8, level: f32) -> Result<(), AudioError> {
        if let Some(track) = self.get_track_mut(track_id) {
            track.level = level.clamp(0.0, 1.0);
            Ok(())
        } else {
            Err(AudioError::InvalidTrack)
        }
    }

    /// Set master level internally (for modulation, without triggering MIDI)
    pub fn set_master_level_internal(&mut self, level: f32) {
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

    /// Get the number of active tracks (recording, playing, or overdubbing)
    pub fn get_active_track_count(&self) -> u8 {
        self.tracks.iter()
            .filter(|track| track.state.is_active())
            .count() as u8
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

    /// Undo last action on selected track
    pub fn undo_track_action(&mut self, track_id: u8, undo_mode: crate::settings::UndoMode) -> bool {
        if let Some(track) = self.get_track_mut(track_id) {
            track.undo_last_action(undo_mode)
        } else {
            false
        }
    }

    /// Redo last undone action on selected track
    pub fn redo_track_action(&mut self, track_id: u8, undo_mode: crate::settings::UndoMode) -> bool {
        if let Some(track) = self.get_track_mut(track_id) {
            track.redo_last_action(undo_mode)
        } else {
            false
        }
    }

    /// Undo last effect parameter change
    pub fn undo_effect_parameter(&mut self, chain_type: crate::effects::EffectChainType, track_id: Option<u8>) -> bool {
        match chain_type {
            crate::effects::EffectChainType::InputFX => {
                self.input_fx.undo_parameter_change()
            },
            crate::effects::EffectChainType::MasterFX => {
                self.master_fx.undo_parameter_change()
            },
            crate::effects::EffectChainType::TrackFX => {
                if let Some(track_id) = track_id {
                    if let Some(track_fx) = self.track_fx_mut(track_id) {
                        track_fx.undo_parameter_change()
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
        }
    }

    /// Redo last undone effect parameter change
    pub fn redo_effect_parameter(&mut self, chain_type: crate::effects::EffectChainType, track_id: Option<u8>) -> bool {
        match chain_type {
            crate::effects::EffectChainType::InputFX => {
                self.input_fx.redo_parameter_change()
            },
            crate::effects::EffectChainType::MasterFX => {
                self.master_fx.redo_parameter_change()
            },
            crate::effects::EffectChainType::TrackFX => {
                if let Some(track_id) = track_id {
                    if let Some(track_fx) = self.track_fx_mut(track_id) {
                        track_fx.redo_parameter_change()
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
        }
    }

    /// Get undo count for a specific track
    pub fn get_track_undo_count(&self, track_id: u8) -> usize {
        if let Some(track) = self.get_track(track_id) {
            track.undo_count()
        } else {
            0
        }
    }

    /// Get redo count for a specific track
    pub fn get_track_redo_count(&self, track_id: u8) -> usize {
        if let Some(track) = self.get_track(track_id) {
            track.redo_count()
        } else {
            0
        }
    }

    /// Clear all undo/redo history for all tracks and effects
    pub fn clear_all_undo_history(&mut self) {
        for track in &mut self.tracks {
            track.clear_undo_history();
        }
        self.input_fx.clear_parameter_history();
        self.master_fx.clear_parameter_history();
    }

    /// Update tempo for all tempo-synced effects
    pub fn update_tempo(&mut self, bpm: f32) {
        self.input_fx.update_tempo(bpm);
        self.master_fx.update_tempo(bpm);
        
        for track in &mut self.tracks {
            track.track_fx.update_tempo(bpm);
        }
    }

    /// Get current tempo from effects (returns the tempo from input FX chain)
    pub fn get_current_tempo(&self) -> f32 {
        self.input_fx.get_current_tempo()
    }

    /// Process USB audio input for track recording
    /// Routes USB input channels to track recording based on DAW routing configuration
    /// Requirement 11.9: 16-channel DAW routing
    pub fn process_usb_input_for_recording(&mut self, usb_inputs: &[[f32; AUDIO_BUFFER_SIZE]; 16], daw_routing: &crate::hal::DawRoutingConfig) {
        for track_id in 1..=6 {
            let track_index = (track_id - 1) as usize;
            
            // Check if this track should record from USB input
            if let Some(usb_channel) = daw_routing.track_input_routing[track_index] {
                if (usb_channel as usize) < 16 {
                    let track = &mut self.tracks[track_index];
                    
                    // Only process if track is recording
                    if track.state.is_recording() {
                        // Use USB input as the audio source for recording
                        let usb_audio = &usb_inputs[usb_channel as usize];
                        
                        // Convert mono USB input to stereo for track recording
                        let mut stereo_input = [0.0f32; AUDIO_BUFFER_SIZE * 2];
                        for i in 0..AUDIO_BUFFER_SIZE {
                            stereo_input[i * 2] = usb_audio[i];     // Left
                            stereo_input[i * 2 + 1] = usb_audio[i]; // Right (duplicate mono)
                        }
                        
                        // Record the USB input into the track
                        track.record_audio(&stereo_input, AUDIO_BUFFER_SIZE * 2);
                    }
                }
            }
        }
    }

    /// Generate USB audio output for DAW
    /// Routes track outputs and master output to USB channels based on DAW routing
    /// Requirement 11.9: 16-channel DAW routing with individual track outputs
    pub fn generate_usb_output(&self, daw_routing: &crate::hal::DawRoutingConfig) -> [[f32; AUDIO_BUFFER_SIZE]; 16] {
        let mut usb_outputs = [[0.0f32; AUDIO_BUFFER_SIZE]; 16];
        
        // Route individual tracks to USB output channels
        for track_id in 1..=6 {
            let track_index = (track_id - 1) as usize;
            let track = &self.tracks[track_index];
            
            // Only output if track is playing and routing is configured
            if track.state.is_playing() {
                if let Some((left_ch, right_ch)) = daw_routing.track_output_routing[track_index] {
                    if (left_ch as usize) < 16 && (right_ch as usize) < 16 {
                        // Generate track audio output
                        let mut track_output = [0.0f32; AUDIO_BUFFER_SIZE * 2];
                        track.playback_audio(&mut track_output, AUDIO_BUFFER_SIZE * 2);
                        
                        // Apply track level and pan
                        let level = track.level;
                        let pan = track.pan;
                        let left_gain = level * (1.0 - pan.max(0.0));
                        let right_gain = level * (1.0 + pan.min(0.0));
                        
                        // Route to USB channels (convert stereo to separate channels)
                        for i in 0..AUDIO_BUFFER_SIZE {
                            usb_outputs[left_ch as usize][i] = track_output[i * 2] * left_gain;
                            usb_outputs[right_ch as usize][i] = track_output[i * 2 + 1] * right_gain;
                        }
                    }
                }
            }
        }
        
        // Route master output to USB channels
        let (master_left, master_right) = daw_routing.master_output_routing;
        if (master_left as usize) < 16 && (master_right as usize) < 16 {
            // Generate master mix
            let mut master_output = [0.0f32; AUDIO_BUFFER_SIZE * 2];
            self.generate_master_output(&mut master_output);
            
            // Route master to USB channels
            for i in 0..AUDIO_BUFFER_SIZE {
                usb_outputs[master_left as usize][i] = master_output[i * 2];     // Left
                usb_outputs[master_right as usize][i] = master_output[i * 2 + 1]; // Right
            }
        }
        
        usb_outputs
    }

    /// Process zero-latency monitoring for USB inputs
    /// Routes USB inputs directly to hardware outputs for monitoring
    /// Requirement 11.19: Zero-latency monitoring for DAW applications
    pub fn process_zero_latency_monitoring(&self, usb_inputs: &[[f32; AUDIO_BUFFER_SIZE]; 16], daw_routing: &crate::hal::DawRoutingConfig, hardware_outputs: &mut [[f32; AUDIO_BUFFER_SIZE]; 8]) {
        // Route USB inputs to hardware outputs for zero-latency monitoring
        for (usb_channel, hardware_output_opt) in daw_routing.input_monitoring_routing.iter().enumerate() {
            if let Some(hardware_output_ch) = hardware_output_opt {
                if usb_channel < 16 && (*hardware_output_ch as usize) < 8 {
                    // Mix USB input into hardware output for monitoring
                    for i in 0..AUDIO_BUFFER_SIZE {
                        hardware_outputs[*hardware_output_ch as usize][i] += usb_inputs[usb_channel][i] * 0.5; // Reduce level for monitoring
                    }
                }
            }
        }
    }

    /// Set USB input source for a track
    /// Configures a track to record from a specific USB input channel
    /// Requirement 11.9: DAW routing capabilities for track inputs
    pub fn set_track_usb_input(&mut self, track_id: u8, usb_channel: Option<u8>) -> Result<(), AudioError> {
        if track_id < 1 || track_id > 6 {
            return Err(AudioError::InvalidTrack);
        }
        
        let track = &mut self.tracks[(track_id - 1) as usize];
        
        // Set input source to USB channel
        if let Some(channel) = usb_channel {
            if channel < 16 {
                track.input_source = crate::audio::InputSource::USB(channel);
            }
        } else {
            // Reset to default microphone input
            track.input_source = crate::audio::InputSource::Mic(track_id);
        }
        
        Ok(())
    }

    /// Get USB input source for a track
    pub fn get_track_usb_input(&self, track_id: u8) -> Option<u8> {
        if track_id < 1 || track_id > 6 {
            return None;
        }
        
        let track = &self.tracks[(track_id - 1) as usize];
        
        match track.input_source {
            crate::audio::InputSource::USB(channel) => Some(channel),
            _ => None,
        }
    }

    /// Generate master output mix for USB routing
    fn generate_master_output(&self, output: &mut [f32]) {
        // Clear output buffer
        output.fill(0.0);
        
        // Mix all playing tracks
        for track in &self.tracks {
            if track.state.is_playing() {
                let mut track_output = [0.0f32; 1024]; // Fixed size buffer for no_std
                let buffer_len = output.len().min(1024);
                track.playback_audio(&mut track_output[..buffer_len], buffer_len);
                
                // Apply track level and pan, then mix into master
                let level = track.level;
                let pan = track.pan;
                
                for i in 0..(buffer_len / 2) {
                    let left_gain = level * (1.0 - pan.max(0.0));
                    let right_gain = level * (1.0 + pan.min(0.0));
                    
                    output[i * 2] += track_output[i * 2] * left_gain;
                    output[i * 2 + 1] += track_output[i * 2 + 1] * right_gain;
                }
            }
        }
        
        // Apply master effects (create a copy for input to avoid borrowing issues)
        // Note: In a real implementation, we would need a mutable reference to master_fx
        // For now, we skip the master effects processing in this const context
        // A proper implementation would require restructuring to avoid the borrowing conflict
    }

    /// Start track recording (wrapper for USB audio integration)
    pub fn start_track_recording(&mut self, track_id: u8, timestamp: u32) -> Result<(), AudioError> {
        if track_id < 1 || track_id > 6 {
            return Err(AudioError::InvalidTrack);
        }
        
        let track = &mut self.tracks[(track_id - 1) as usize];
        track.start_recording(timestamp);
        Ok(())
    }

    /// Stop track recording (wrapper for USB audio integration)
    pub fn stop_track_recording(&mut self, track_id: u8) -> Result<(), AudioError> {
        if track_id < 1 || track_id > 6 {
            return Err(AudioError::InvalidTrack);
        }
        
        let track = &mut self.tracks[(track_id - 1) as usize];
        track.stop();
        Ok(())
    }

    /// Start track playback (wrapper for USB audio integration)
    pub fn start_track_playback(&mut self, track_id: u8) -> Result<(), AudioError> {
        if track_id < 1 || track_id > 6 {
            return Err(AudioError::InvalidTrack);
        }
        
        let track = &mut self.tracks[(track_id - 1) as usize];
        track.start_playback(0); // Use timestamp 0 for now
        Ok(())
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