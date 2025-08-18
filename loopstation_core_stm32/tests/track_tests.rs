//! Unit tests for Track audio buffer management and state transitions
//! Requirements: 1.1, 1.2, 1.3

use loopstation_core_stm32::audio::{Track, TrackState, TrackAction, UndoableAction};
use loopstation_core_stm32::settings::UndoMode;

#[test]
fn test_track_creation() {
    let track = Track::new(1);
    
    assert_eq!(track.id, 1);
    assert_eq!(track.state, TrackState::Stopped);
    assert_eq!(track.level, 1.0);
    assert_eq!(track.pan, 0.0);
    assert_eq!(track.loop_length, 0);
    assert_eq!(track.play_position, 0);
    assert_eq!(track.record_position, 0);
    assert!(!track.quantize_enabled);
    assert!(!track.selected);
    assert!(track.audio_buffer.is_empty());
}

#[test]
fn test_track_state_transitions() {
    let mut track = Track::new(1);
    
    // Test initial state
    assert_eq!(track.state, TrackState::Stopped);
    
    // Test transition from Stopped to Recording
    track.state.transition(TrackAction::RecordPlay);
    assert_eq!(track.state, TrackState::Recording);
    
    // Test transition from Recording to Playing
    track.state.transition(TrackAction::RecordPlay);
    assert_eq!(track.state, TrackState::Playing);
    
    // Test transition from Playing to Overdubbing
    track.state.transition(TrackAction::RecordPlay);
    assert_eq!(track.state, TrackState::Overdubbing);
    
    // Test transition from Overdubbing to Playing
    track.state.transition(TrackAction::RecordPlay);
    assert_eq!(track.state, TrackState::Playing);
    
    // Test mute transition
    track.state.transition(TrackAction::Mute);
    assert_eq!(track.state, TrackState::Muted);
    
    // Test unmute transition
    track.state.transition(TrackAction::Mute);
    assert_eq!(track.state, TrackState::Playing);
    
    // Test stop transition
    track.state.transition(TrackAction::Stop);
    assert_eq!(track.state, TrackState::Stopped);
    
    // Test clear transition (always goes to Stopped)
    track.state = TrackState::Playing;
    track.state.transition(TrackAction::Clear);
    assert_eq!(track.state, TrackState::Stopped);
}

#[test]
fn test_track_state_queries() {
    let mut state = TrackState::Stopped;
    assert!(!state.is_active());
    assert!(!state.is_recording());
    assert!(!state.is_playing());
    
    state = TrackState::Recording;
    assert!(state.is_active());
    assert!(state.is_recording());
    assert!(!state.is_playing());
    
    state = TrackState::Playing;
    assert!(state.is_active());
    assert!(!state.is_recording());
    assert!(state.is_playing());
    
    state = TrackState::Overdubbing;
    assert!(state.is_active());
    assert!(state.is_recording());
    assert!(state.is_playing());
    
    state = TrackState::Muted;
    assert!(!state.is_active());
    assert!(!state.is_recording());
    assert!(!state.is_playing());
}

#[test]
fn test_track_recording() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Start recording
    track.start_recording(timestamp);
    assert_eq!(track.state, TrackState::Recording);
    assert_eq!(track.record_position, 0);
    
    // Check undo action was added
    assert_eq!(track.undo_buffer.len(), 1);
    if let Some(action) = track.undo_buffer.get(0) {
        assert!(matches!(action.action, UndoableAction::StartRecording { track_id: 1 }));
    }
}

#[test]
fn test_track_playback() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Add some audio data first
    track.loop_length = 1000;
    let _ = track.audio_buffer.push(0.5);
    
    // Start playback
    track.start_playback(timestamp);
    assert_eq!(track.state, TrackState::Playing);
    assert_eq!(track.play_position, 0);
}

#[test]
fn test_track_playback_without_audio() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Try to start playback without audio
    track.start_playback(timestamp);
    // Should remain stopped since no audio
    assert_eq!(track.state, TrackState::Stopped);
}

#[test]
fn test_track_stop() {
    let mut track = Track::new(1);
    
    // Set to playing state
    track.state = TrackState::Playing;
    track.play_position = 500;
    
    // Stop track
    track.stop();
    assert_eq!(track.state, TrackState::Stopped);
    assert_eq!(track.play_position, 0);
}

#[test]
fn test_track_mute_toggle() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Set to playing state
    track.state = TrackState::Playing;
    
    // Toggle mute
    track.toggle_mute(timestamp);
    assert_eq!(track.state, TrackState::Muted);
    
    // Check undo action was added
    assert_eq!(track.undo_buffer.len(), 1);
    if let Some(action) = track.undo_buffer.get(0) {
        assert!(matches!(action.action, UndoableAction::ToggleMute { 
            track_id: 1, 
            previous_state: TrackState::Playing 
        }));
    }
}

#[test]
fn test_track_clear() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Add some audio data
    let _ = track.audio_buffer.push(0.5);
    let _ = track.audio_buffer.push(0.3);
    track.loop_length = 2;
    track.state = TrackState::Playing;
    
    // Clear track
    track.clear(timestamp);
    assert_eq!(track.state, TrackState::Stopped);
    assert_eq!(track.loop_length, 0);
    assert_eq!(track.play_position, 0);
    assert_eq!(track.record_position, 0);
    assert!(track.audio_buffer.is_empty());
    
    // Check undo action was added
    assert_eq!(track.undo_buffer.len(), 1);
    if let Some(action) = track.undo_buffer.get(0) {
        assert!(matches!(action.action, UndoableAction::ClearTrack { 
            track_id: 1, 
            buffer_length: 2,
            previous_state: TrackState::Playing
        }));
    }
}

#[test]
fn test_track_level_control() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Test setting valid level
    track.set_level(0.5, timestamp);
    assert_eq!(track.level, 0.5);
    
    // Test clamping high value
    track.set_level(1.5, timestamp);
    assert_eq!(track.level, 1.0);
    
    // Test clamping low value
    track.set_level(-0.5, timestamp);
    assert_eq!(track.level, 0.0);
    
    // Check undo actions were added for significant changes
    assert!(track.undo_buffer.len() > 0);
}

#[test]
fn test_track_pan_control() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Test setting valid pan
    track.set_pan(0.5, timestamp);
    assert_eq!(track.pan, 0.5);
    
    // Test clamping high value
    track.set_pan(1.5, timestamp);
    assert_eq!(track.pan, 1.0);
    
    // Test clamping low value
    track.set_pan(-1.5, timestamp);
    assert_eq!(track.pan, -1.0);
    
    // Check undo actions were added for significant changes
    assert!(track.undo_buffer.len() > 0);
}

#[test]
fn test_track_undo_redo() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Perform some actions
    track.set_level(0.5, timestamp);
    track.set_pan(0.3, timestamp + 100);
    track.start_recording(timestamp + 200);
    
    // Check we have undo actions
    assert_eq!(track.undo_buffer.len(), 3);
    assert_eq!(track.undo_count(), 3);
    assert_eq!(track.redo_count(), 0);
    
    // Undo last action (start recording)
    let result = track.undo_last_action(UndoMode::All);
    assert!(result);
    assert_eq!(track.state, TrackState::Stopped);
    assert_eq!(track.undo_count(), 2);
    assert_eq!(track.redo_count(), 1);
    
    // Undo pan change
    let result = track.undo_last_action(UndoMode::All);
    assert!(result);
    assert_eq!(track.pan, 0.0); // Back to default
    assert_eq!(track.undo_count(), 1);
    assert_eq!(track.redo_count(), 2);
    
    // Redo pan change
    let result = track.redo_last_action(UndoMode::All);
    assert!(result);
    assert_eq!(track.pan, 0.3);
    assert_eq!(track.undo_count(), 2);
    assert_eq!(track.redo_count(), 1);
}

#[test]
fn test_track_undo_modes() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Add different types of actions
    track.start_recording(timestamp);
    track.set_level(0.5, timestamp + 100);
    
    // Test RecordOnly mode - should only undo recording actions
    let result = track.undo_last_action(UndoMode::RecordOnly);
    assert!(!result); // Level change not undoable in RecordOnly mode
    
    // Move to recording action
    track.undo_position = 1; // Point to recording action
    let result = track.undo_last_action(UndoMode::RecordOnly);
    assert!(result); // Recording action should be undoable
    
    // Test All mode - should undo any action
    track.set_level(0.7, timestamp + 200);
    let result = track.undo_last_action(UndoMode::All);
    assert!(result); // Level change should be undoable in All mode
}

#[test]
fn test_track_audio_buffer_management() {
    let mut track = Track::new(1);
    
    // Test buffer operations
    assert!(track.audio_buffer.is_empty());
    assert_eq!(track.audio_buffer.len(), 0);
    
    // Add some audio data
    let _ = track.audio_buffer.push(0.1);
    let _ = track.audio_buffer.push(0.2);
    let _ = track.audio_buffer.push(0.3);
    
    assert!(!track.audio_buffer.is_empty());
    assert_eq!(track.audio_buffer.len(), 3);
    
    // Test buffer access
    assert_eq!(track.audio_buffer[0], 0.1);
    assert_eq!(track.audio_buffer[1], 0.2);
    assert_eq!(track.audio_buffer[2], 0.3);
    
    // Test buffer clearing
    track.audio_buffer.clear();
    assert!(track.audio_buffer.is_empty());
    assert_eq!(track.audio_buffer.len(), 0);
}

#[test]
fn test_track_has_audio() {
    let mut track = Track::new(1);
    
    // Initially no audio
    assert!(!track.has_audio());
    
    // Add audio
    let _ = track.audio_buffer.push(0.5);
    assert!(track.has_audio());
    
    // Clear audio
    track.audio_buffer.clear();
    assert!(!track.has_audio());
}

#[test]
fn test_track_duration_calculation() {
    let mut track = Track::new(1);
    let sample_rate = 44100;
    
    // No audio
    assert_eq!(track.duration_seconds(sample_rate), 0.0);
    
    // Set loop length (stereo samples)
    track.loop_length = 88200; // 1 second at 44.1kHz stereo
    let duration = track.duration_seconds(sample_rate);
    assert!((duration - 1.0).abs() < 0.01); // Should be approximately 1 second
}

#[test]
fn test_track_undo_history_management() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Fill undo buffer to capacity
    for i in 0..20 {
        track.set_level(i as f32 / 20.0, timestamp + i * 100);
    }
    
    // Buffer should be limited to 16 entries
    assert!(track.undo_buffer.len() <= 16);
    
    // Clear history
    track.clear_undo_history();
    assert_eq!(track.undo_buffer.len(), 0);
    assert_eq!(track.redo_buffer.len(), 0);
    assert_eq!(track.undo_position, 0);
}

#[test]
fn test_track_process_audio() {
    let mut track = Track::new(1);
    let sample_rate = 44100;
    
    // Test with stopped track
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    track.process_audio(&input, &mut output, sample_rate);
    
    // Stopped track should output silence
    assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
    
    // Test with muted track
    track.state = TrackState::Muted;
    track.process_audio(&input, &mut output, sample_rate);
    
    // Muted track should output silence
    assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_circular_buffer_operations() {
    use loopstation_core_stm32::audio::CircularBuffer;
    
    let mut buffer: CircularBuffer<f32> = CircularBuffer::new();
    
    // Test initial state
    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);
    
    // Test writing data
    let data = [0.1, 0.2, 0.3];
    buffer.write(&data);
    assert_eq!(buffer.len(), 3);
    assert!(!buffer.is_empty());
    
    // Test reading data
    let mut output = [0.0; 3];
    let read_count = buffer.read(&mut output);
    assert_eq!(read_count, 3);
    assert_eq!(output, [0.1, 0.2, 0.3]);
    
    // Test clearing
    buffer.clear();
    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);
}

#[test]
fn test_circular_buffer_capacity() {
    use loopstation_core_stm32::audio::{CircularBuffer, MAX_TRACK_SAMPLES};
    
    let buffer: CircularBuffer<f32> = CircularBuffer::new();
    assert_eq!(buffer.capacity(), MAX_TRACK_SAMPLES);
}

#[test]
fn test_track_serialization_compatibility() {
    let track = Track::new(1);
    
    // Test that track can be serialized (for project save/load)
    let serialized = serde_json_core::to_string::<_, 65536>(&track);
    assert!(serialized.is_ok());
    
    // Test deserialization
    if let Ok((json_str, _)) = serialized {
        let deserialized: Result<Track, _> = serde_json_core::from_str(&json_str);
        assert!(deserialized.is_ok());
        
        if let Ok(deserialized_track) = deserialized {
            assert_eq!(deserialized_track.id, track.id);
            assert_eq!(deserialized_track.state, track.state);
            assert_eq!(deserialized_track.level, track.level);
        }
    }
}