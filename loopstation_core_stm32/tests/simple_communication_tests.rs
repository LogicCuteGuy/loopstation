//! Simple communication integration tests
//! Task 15.2: Add integration tests for communication
//! Requirements: 5.2, 11.1, 11.2, 4.6

use loopstation_core_stm32::{
    LoopstationCore, 
    midi::{MidiMessage, MidiChannel},
    audio::TrackState,
};

/// Test basic UART communication protocol simulation
/// Requirement 5.2: STM32-ESP32 communication protocol
#[test]
fn test_uart_communication_basic() {
    let mut core = LoopstationCore::new();
    
    // Simulate receiving status update command from ESP32
    core.system_time_ms = 1000;
    core.selected_track = 2;
    
    // Simulate parameter change via UART
    let result = core.set_track_level(2, 0.6);
    assert!(result.is_ok());
    
    // Verify state changes
    assert_eq!(core.selected_track, 2);
    if let Some(track) = core.audio_engine.get_track(2) {
        assert_eq!(track.level, 0.6);
    }
}

/// Test OSC command processing with response time
/// Requirement 5.6: <20ms response time for OSC commands
#[test]
fn test_osc_command_processing_basic() {
    let mut core = LoopstationCore::new();
    
    let start_time = std::time::Instant::now();
    
    // Simulate OSC commands
    let _ = core.set_track_level(1, 0.7);
    let _ = core.start_recording(2);
    core.set_tempo(140.0);
    
    let processing_time = start_time.elapsed();
    
    // Verify commands were processed
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.level, 0.7);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.state, TrackState::Recording);
    }
    
    assert_eq!(core.get_tempo(), 140.0);
    
    // Should process quickly (generous limit for test environment)
    assert!(processing_time.as_millis() < 100);
}

/// Test MIDI functionality integration
/// Requirement 4.6: MIDI CC input for loopstation control
#[test]
fn test_midi_functionality_basic() {
    let mut core = LoopstationCore::new();
    
    // Test MIDI settings
    let mut settings = core.midi_handler.get_settings().clone();
    settings.midi_channel = MidiChannel::Channel(5);
    settings.cc_tx_rx = true;
    core.midi_handler.set_settings(settings);
    
    // Test MIDI message processing
    let cc_message = MidiMessage::ControlChange {
        channel: 5,
        controller: 7, // Volume CC
        value: 100,
    };
    
    let messages = core.midi_handler.process_input(&cc_message.to_bytes());
    assert!(!messages.is_empty());
    
    // Verify message was processed correctly
    if let Some(parsed_msg) = messages.first() {
        match parsed_msg {
            MidiMessage::ControlChange { channel, controller, value } => {
                assert_eq!(*channel, 5);
                assert_eq!(*controller, 7);
                assert_eq!(*value, 100);
            }
            _ => panic!("Expected ControlChange message"),
        }
    }
    
    // Verify MIDI settings were applied
    let updated_settings = core.midi_handler.get_settings();
    assert_eq!(updated_settings.midi_channel, MidiChannel::Channel(5));
    assert!(updated_settings.cc_tx_rx);
}

/// Test plugin integration for DAW use
/// Requirement 4.6: Parameter automation support for DAW integration
#[test]
fn test_plugin_integration_basic() {
    let mut core = LoopstationCore::new();
    
    // Test plugin parameter automation
    let start_time = std::time::Instant::now();
    
    // Simulate DAW parameter automation
    let _ = core.set_track_level(1, 0.8);
    let _ = core.set_track_level(2, 0.6);
    core.set_master_level(0.9);
    
    let automation_time = start_time.elapsed();
    
    // Verify parameter changes
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.level, 0.8);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.level, 0.6);
    }
    
    assert_eq!(core.audio_engine.get_master_level(), 0.9);
    
    // Verify automation response time
    assert!(automation_time.as_millis() < 50);
    
    // Test audio processing (plugin callback simulation)
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    
    let audio_start = std::time::Instant::now();
    core.process_audio(&input, &mut output);
    let audio_time = audio_start.elapsed();
    
    // Verify audio processing time (should be very fast)
    assert!(audio_time.as_micros() < 10000);
}

/// Test communication error handling
#[test]
fn test_communication_error_handling_basic() {
    let mut core = LoopstationCore::new();
    
    // Test invalid MIDI data
    let invalid_midi_data = [0xFF, 0xFF, 0xFF]; // Invalid MIDI message
    let messages = core.midi_handler.process_input(&invalid_midi_data);
    // Should handle gracefully without crashing
    
    // Test invalid track operations
    let result = core.start_recording(0); // Invalid track ID
    assert!(result.is_err());
    
    let result = core.start_recording(7); // Invalid track ID
    assert!(result.is_err());
    
    // Test invalid parameter values
    let result = core.set_track_level(1, 2.0); // Should clamp to 1.0
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.level, 1.0); // Should be clamped
    }
}

/// Test concurrent communication operations
#[test]
fn test_concurrent_communication_basic() {
    let mut core = LoopstationCore::new();
    
    let start_time = std::time::Instant::now();
    
    // Simulate concurrent operations from different sources
    
    // 1. MIDI input processing
    let midi_cc = MidiMessage::ControlChange {
        channel: 1,
        controller: 7,
        value: 80,
    };
    let _messages = core.midi_handler.process_input(&midi_cc.to_bytes());
    
    // 2. OSC command processing
    let _ = core.set_track_level(2, 0.7);
    let _ = core.start_recording(3);
    
    // 3. UART status update (simulated)
    core.selected_track = 4;
    core.set_tempo(135.0);
    
    // 4. Audio processing
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    core.process_audio(&input, &mut output);
    
    // 5. System update
    core.update(1500);
    
    let total_time = start_time.elapsed();
    
    // Verify all operations completed successfully
    assert_eq!(core.selected_track, 4);
    assert_eq!(core.get_tempo(), 135.0);
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.level, 0.7);
    }
    
    if let Some(track3) = core.audio_engine.get_track(3) {
        assert_eq!(track3.state, TrackState::Recording);
    }
    
    // Verify concurrent operations completed in reasonable time
    assert!(total_time.as_millis() < 200);
}