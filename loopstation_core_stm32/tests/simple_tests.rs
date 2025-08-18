//! Simple unit tests for core components
//! Requirements: 1.1, 1.2, 1.3, 3.4, 8.2, 8.4

use loopstation_core_stm32::audio::{Track, TrackState, TrackAction};
use loopstation_core_stm32::effects::{EffectChain, Effect, EffectType, EffectChainType};
use loopstation_core_stm32::storage::{MemorySystem, Project};

#[test]
fn test_track_basic_operations() {
    let mut track = Track::new(1);
    
    // Test initial state
    assert_eq!(track.id, 1);
    assert_eq!(track.state, TrackState::Stopped);
    assert_eq!(track.level, 1.0);
    assert!(!track.has_audio());
    
    // Test state transitions
    track.state.transition(TrackAction::RecordPlay);
    assert_eq!(track.state, TrackState::Recording);
    
    track.state.transition(TrackAction::RecordPlay);
    assert_eq!(track.state, TrackState::Playing);
    
    track.state.transition(TrackAction::Stop);
    assert_eq!(track.state, TrackState::Stopped);
}

#[test]
fn test_track_audio_buffer() {
    let mut track = Track::new(1);
    
    // Add audio data
    let _ = track.audio_buffer.push(0.5);
    let _ = track.audio_buffer.push(0.3);
    
    assert!(track.has_audio());
    assert_eq!(track.audio_buffer.len(), 2);
    assert_eq!(track.audio_buffer[0], 0.5);
    assert_eq!(track.audio_buffer[1], 0.3);
    
    // Clear audio
    track.audio_buffer.clear();
    assert!(!track.has_audio());
    assert_eq!(track.audio_buffer.len(), 0);
}

#[test]
fn test_track_level_control() {
    let mut track = Track::new(1);
    let timestamp = 1000;
    
    // Test setting level
    track.set_level(0.7, timestamp);
    assert_eq!(track.level, 0.7);
    
    // Test clamping
    track.set_level(1.5, timestamp);
    assert_eq!(track.level, 1.0);
    
    track.set_level(-0.5, timestamp);
    assert_eq!(track.level, 0.0);
}

#[test]
fn test_effect_chain_basic_operations() {
    let mut chain = EffectChain::new_input_fx();
    
    // Test initial state
    assert_eq!(chain.chain_type, EffectChainType::InputFX);
    assert_eq!(chain.active_effect_count(), 0);
    assert!(!chain.has_effects());
    
    // Add effect
    let compressor = Effect::new(EffectType::Compressor);
    let result = chain.add_effect(compressor);
    assert!(result.is_ok());
    assert_eq!(chain.active_effect_count(), 1);
    assert!(chain.has_effects());
    
    // Remove effect
    let removed = chain.remove_effect(0);
    assert!(removed.is_some());
    assert_eq!(chain.active_effect_count(), 0);
}

#[test]
fn test_effect_parameter_control() {
    let mut effect = Effect::new(EffectType::Compressor);
    
    // Test parameter access
    assert!(effect.get_parameter(0).is_some());
    assert!(effect.get_parameter(10).is_none());
    
    // Test parameter setting
    effect.set_parameter(0, 0.5);
    if let Some(param) = effect.get_parameter(0) {
        assert_eq!(param.value, 0.5);
    }
    
    // Test enable/disable
    assert!(!effect.enabled);
    effect.set_enabled(true);
    assert!(effect.enabled);
}

#[test]
fn test_memory_system_basic_operations() {
    let mut memory = MemorySystem::new();
    
    // Test initial state
    assert_eq!(memory.current_memory, 1);
    assert!(!memory.is_slot_empty(1));
    assert!(memory.is_slot_empty(2));
    
    // Test slot switching
    let result = memory.switch_to_slot(5);
    assert!(result.is_ok());
    assert_eq!(memory.current_memory, 5);
    
    // Test slot initialization
    let result = memory.initialize_slot(5);
    assert!(result.is_ok());
    assert!(!memory.is_slot_empty(5));
}

#[test]
fn test_project_basic_operations() {
    let mut project = Project::new(3);
    
    // Test initial state
    assert_eq!(project.memory_slot, 3);
    assert_eq!(project.get_name(), "NEW PROJECT");
    assert_eq!(project.tempo, 120.0);
    assert!(!project.has_audio());
    
    // Test name setting
    project.set_name("Test Project");
    assert_eq!(project.get_name(), "Test Project");
    
    // Test audio detection
    let _ = project.tracks[0].audio_buffer.push(0.5);
    assert!(project.has_audio());
}

#[test]
fn test_project_save_load() {
    let mut memory = MemorySystem::new();
    
    // Create project
    let mut project = Project::new(2);
    project.set_name("Save Test");
    project.tempo = 140.0;
    
    // Save project
    let result = memory.save_project(2, project);
    assert!(result.is_ok());
    
    // Load project
    let loaded = memory.load_project(2);
    assert!(loaded.is_ok());
    
    let loaded_project = loaded.unwrap();
    assert_eq!(loaded_project.get_name(), "Save Test");
    assert_eq!(loaded_project.tempo, 140.0);
}

#[test]
fn test_effect_types() {
    // Test effect type properties
    assert_eq!(EffectType::Compressor.name(), "COMPRESSOR");
    assert_eq!(EffectType::Compressor.parameter_count(), 4);
    assert!(!EffectType::Compressor.supports_tempo_sync());
    
    assert_eq!(EffectType::TapeEcho.name(), "TAPE ECHO");
    assert_eq!(EffectType::TapeEcho.parameter_count(), 4);
    assert!(EffectType::TapeEcho.supports_tempo_sync());
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
}

#[test]
fn test_circular_buffer() {
    use loopstation_core_stm32::audio::CircularBuffer;
    
    let mut buffer: CircularBuffer<f32> = CircularBuffer::new();
    
    // Test initial state
    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);
    
    // Test writing
    let data = [0.1, 0.2, 0.3];
    buffer.write(&data);
    assert_eq!(buffer.len(), 3);
    
    // Test reading
    let mut output = [0.0; 3];
    let read_count = buffer.read(&mut output);
    assert_eq!(read_count, 3);
    assert_eq!(output, [0.1, 0.2, 0.3]);
}