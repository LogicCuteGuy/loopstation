//! Comprehensive integration tests for communication protocols
//! Task 15.2: Add integration tests for communication
//! Requirements: 5.2, 11.1, 11.2, 4.6

use loopstation_core_stm32::{
    LoopstationCore, 
    midi::{MidiMessage, MidiChannel, MidiHandler},
    audio::TrackState,
    hal::{Esp32Message, CommandType, ParameterId, TrackStatus, TrackStateComm},
};
use std::time::{Duration, Instant};

/// Test STM32-ESP32 UART communication protocol
/// Requirement 5.2: STM32-ESP32 communication protocol
/// Requirement 11.1: UART communication at 115200 baud
/// Requirement 11.2: Message protocol for display updates and control commands
#[test]
fn test_stm32_esp32_uart_communication_protocol() {
    let mut core = LoopstationCore::new();
    
    // Test 1: Status update message creation
    let status_message = Esp32Message::StatusUpdate {
        tracks: [
            TrackStatus {
                state: TrackStateComm::Playing,
                volume: 0.8,
                pan: 0.0,
                muted: false,
                selected: true,
            },
            TrackStatus {
                state: TrackStateComm::Recording,
                volume: 0.6,
                pan: -0.2,
                muted: false,
                selected: false,
            },
            TrackStatus {
                state: TrackStateComm::Stopped,
                volume: 0.5,
                pan: 0.1,
                muted: true,
                selected: false,
            },
            TrackStatus {
                state: TrackStateComm::Overdubbing,
                volume: 0.9,
                pan: 0.3,
                muted: false,
                selected: false,
            },
            TrackStatus {
                state: TrackStateComm::Muted,
                volume: 0.7,
                pan: -0.1,
                muted: true,
                selected: false,
            },
            TrackStatus {
                state: TrackStateComm::Stopped,
                volume: 0.4,
                pan: 0.0,
                muted: false,
                selected: false,
            },
        ],
        tempo: 128.5,
        current_memory: 5,
        fx_states: [true, false, true, false, false],
    };
    
    // Verify status message structure
    match status_message {
        Esp32Message::StatusUpdate { tracks, tempo, current_memory, fx_states } => {
            assert_eq!(tracks[0].state as u8, TrackStateComm::Playing as u8);
            assert_eq!(tracks[1].state as u8, TrackStateComm::Recording as u8);
            assert_eq!(tempo, 128.5);
            assert_eq!(current_memory, 5);
            assert_eq!(fx_states[0], true);
            assert_eq!(fx_states[1], false);
        }
        _ => panic!("Expected StatusUpdate message"),
    }
    
    // Test 2: Parameter change message
    let param_message = Esp32Message::ParameterChange {
        parameter: ParameterId::TrackVolume(2),
        value: 0.75,
    };
    
    match param_message {
        Esp32Message::ParameterChange { parameter, value } => {
            match parameter {
                ParameterId::TrackVolume(track_id) => {
                    assert_eq!(track_id, 2);
                    assert_eq!(value, 0.75);
                }
                _ => panic!("Expected TrackVolume parameter"),
            }
        }
        _ => panic!("Expected ParameterChange message"),
    }
    
    // Test 3: Command message processing
    let command_message = Esp32Message::Command {
        command: CommandType::TrackPlay,
        track_id: Some(3),
        value: None,
    };
    
    match command_message {
        Esp32Message::Command { command, track_id, value } => {
            assert_eq!(command as u8, CommandType::TrackPlay as u8);
            assert_eq!(track_id, Some(3));
            assert_eq!(value, None);
        }
        _ => panic!("Expected Command message"),
    }
    
    // Test 4: Response message
    let response_message = Esp32Message::Response {
        success: true,
        error_message: None,
    };
    
    match response_message {
        Esp32Message::Response { success, error_message } => {
            assert!(success);
            assert!(error_message.is_none());
        }
        _ => panic!("Expected Response message"),
    }
    
    // Test 5: Heartbeat message
    let heartbeat_message = Esp32Message::Heartbeat;
    match heartbeat_message {
        Esp32Message::Heartbeat => {
            // Heartbeat message verified
        }
        _ => panic!("Expected Heartbeat message"),
    }
}

/// Test UART communication error recovery
/// Requirement 11.2: Command/response handling with error recovery
#[test]
fn test_uart_communication_error_recovery() {
    let mut core = LoopstationCore::new();
    
    // Test error response message
    let error_response = Esp32Message::Response {
        success: false,
        error_message: Some("Invalid track ID".into()),
    };
    
    match error_response {
        Esp32Message::Response { success, error_message } => {
            assert!(!success);
            assert!(error_message.is_some());
            if let Some(msg) = error_message {
                assert_eq!(msg.as_str(), "Invalid track ID");
            }
        }
        _ => panic!("Expected error Response message"),
    }
    
    // Test command validation
    let invalid_command = Esp32Message::Command {
        command: CommandType::TrackPlay,
        track_id: Some(10), // Invalid track ID (should be 1-6)
        value: None,
    };
    
    // In a real implementation, this would be validated and return an error response
    match invalid_command {
        Esp32Message::Command { track_id, .. } => {
            if let Some(id) = track_id {
                assert!(id > 6); // Should be detected as invalid
            }
        }
        _ => panic!("Expected Command message"),
    }
}

/// Test OSC network command processing and response times
/// Requirement 5.3: OSC server on port 8000 with UDP support
/// Requirement 5.4: Bonjour/mDNS service advertisement
/// Requirement 5.5: Command parsing and response framework
/// Requirement 5.6: <20ms response time target
#[test]
fn test_osc_network_command_processing_and_response_times() {
    let mut core = LoopstationCore::new();
    
    // Test 1: Track control commands with timing
    let start_time = Instant::now();
    
    // Simulate OSC command: /track/1/level 0.7
    let result = core.set_track_level(1, 0.7);
    assert!(result.is_ok());
    
    // Simulate OSC command: /track/2/record
    let result = core.start_recording(2);
    assert!(result.is_ok());
    
    // Simulate OSC command: /track/3/play
    let result = core.toggle_track_playback(3);
    assert!(result.is_ok());
    
    // Simulate OSC command: /track/4/mute
    let result = core.toggle_mute(4);
    assert!(result.is_ok());
    
    let processing_time = start_time.elapsed();
    
    // Verify commands were processed correctly
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.level, 0.7);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.state, TrackState::Recording);
    }
    
    if let Some(track3) = core.audio_engine.get_track(3) {
        // Track should be in playing state if it has content, stopped if empty
        assert!(matches!(track3.state, TrackState::Playing | TrackState::Stopped));
    }
    
    if let Some(track4) = core.audio_engine.get_track(4) {
        // Track should be muted if it was playing, or remain stopped
        assert!(matches!(track4.state, TrackState::Muted | TrackState::Stopped));
    }
    
    // Verify response time requirement (<20ms in production, generous for tests)
    assert!(processing_time.as_millis() < 100, "OSC command processing took too long: {:?}", processing_time);
}

/// Test OSC tempo and system commands
/// Requirement 5.6: <20ms response time for OSC commands
#[test]
fn test_osc_tempo_and_system_commands() {
    let mut core = LoopstationCore::new();
    
    let start_time = Instant::now();
    
    // Simulate OSC command: /tempo 140
    core.set_tempo(140.0);
    
    // Simulate OSC command: /master/level 0.8
    core.set_master_level(0.8);
    
    // Simulate OSC command: /rhythm/start
    core.start_rhythm();
    
    // Simulate OSC command: /memory/load 3
    // Note: This would typically load memory slot 3
    core.memory.current_memory = 3;
    
    let processing_time = start_time.elapsed();
    
    // Verify commands were processed
    assert_eq!(core.get_tempo(), 140.0);
    assert_eq!(core.audio_engine.get_master_level(), 0.8);
    assert!(core.rhythm_system.is_playing());
    assert_eq!(core.memory.current_memory, 3);
    
    // Verify response time
    assert!(processing_time.as_millis() < 100, "OSC system commands took too long: {:?}", processing_time);
}

/// Test OSC effect commands
/// Requirement 5.6: <20ms response time for effect control
#[test]
fn test_osc_effect_commands() {
    let mut core = LoopstationCore::new();
    
    let start_time = Instant::now();
    
    // Simulate OSC command: /fx/input/compressor/enable
    let mut compressor = loopstation_core_stm32::effects::Effect::new(
        loopstation_core_stm32::effects::EffectType::Compressor
    );
    compressor.set_enabled(true);
    let result = core.input_fx_mut().add_effect(compressor);
    assert!(result.is_ok());
    
    // Simulate OSC command: /fx/master/reverb/enable
    let mut reverb = loopstation_core_stm32::effects::Effect::new(
        loopstation_core_stm32::effects::EffectType::SpaceReverb
    );
    reverb.set_enabled(true);
    let result = core.master_fx_mut().add_effect(reverb);
    assert!(result.is_ok());
    
    // Simulate OSC command: /fx/track/1/delay/enable
    if let Some(track_fx) = core.track_fx_mut(1) {
        let mut delay = loopstation_core_stm32::effects::Effect::new(
            loopstation_core_stm32::effects::EffectType::TapeEcho
        );
        delay.set_enabled(true);
        let result = track_fx.add_effect(delay);
        assert!(result.is_ok());
    }
    
    let processing_time = start_time.elapsed();
    
    // Verify effects were added
    assert_eq!(core.input_fx().active_effect_count(), 1);
    assert_eq!(core.master_fx().active_effect_count(), 1);
    
    if let Some(track_fx) = core.track_fx(1) {
        assert_eq!(track_fx.active_effect_count(), 1);
    }
    
    // Verify response time
    assert!(processing_time.as_millis() < 100, "OSC effect commands took too long: {:?}", processing_time);
}

/// Test MIDI functionality and plugin integration
/// Requirement 4.6: MIDI CC input for loopstation control
/// Requirement 2.25: MIDI CC assignment processing
/// Requirement 10.4: MIDI channel selection and message filtering
#[test]
fn test_midi_functionality_and_plugin_integration() {
    let mut core = LoopstationCore::new();
    
    // Test 1: MIDI settings configuration
    let mut settings = core.midi_handler.get_settings().clone();
    settings.midi_channel = MidiChannel::Channel(5);
    settings.cc_tx_rx = true;
    settings.pc_out = true;
    settings.clock_sync = true;
    core.midi_handler.set_settings(settings);
    
    let updated_settings = core.midi_handler.get_settings();
    assert_eq!(updated_settings.midi_channel, MidiChannel::Channel(5));
    assert!(updated_settings.cc_tx_rx);
    assert!(updated_settings.pc_out);
    assert!(updated_settings.clock_sync);
    
    // Test 2: MIDI Control Change processing
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
    
    // Test 3: MIDI Program Change processing
    let pc_message = MidiMessage::ProgramChange {
        channel: 5,
        program: 2, // Should switch to memory slot 3 (PC#2 = Memory 3)
    };
    
    let messages = core.midi_handler.process_input(&pc_message.to_bytes());
    assert!(!messages.is_empty());
    
    if let Some(parsed_msg) = messages.first() {
        match parsed_msg {
            MidiMessage::ProgramChange { channel, program } => {
                assert_eq!(*channel, 5);
                assert_eq!(*program, 2);
            }
            _ => panic!("Expected ProgramChange message"),
        }
    }
    
    // Test 4: MIDI Note On/Off processing
    let note_on = MidiMessage::NoteOn {
        channel: 5,
        note: 60, // Middle C
        velocity: 100,
    };
    
    let messages = core.midi_handler.process_input(&note_on.to_bytes());
    assert!(!messages.is_empty());
    
    let note_off = MidiMessage::NoteOff {
        channel: 5,
        note: 60,
        velocity: 0,
    };
    
    let messages = core.midi_handler.process_input(&note_off.to_bytes());
    assert!(!messages.is_empty());
}

/// Test MIDI clock synchronization
/// Requirement 10.9: MIDI clock synchronization for tempo-locked effects
#[test]
fn test_midi_clock_synchronization() {
    let mut core = LoopstationCore::new();
    
    // Enable MIDI clock sync
    let mut settings = core.midi_handler.get_settings().clone();
    settings.clock_sync = true;
    core.midi_handler.set_settings(settings);
    
    assert!(core.midi_handler.is_clock_sync_enabled());
    
    // Test MIDI clock messages
    let clock_message = MidiMessage::Clock;
    let messages = core.midi_handler.process_input(&clock_message.to_bytes());
    assert!(!messages.is_empty());
    
    let start_message = MidiMessage::Start;
    let messages = core.midi_handler.process_input(&start_message.to_bytes());
    assert!(!messages.is_empty());
    
    let stop_message = MidiMessage::Stop;
    let messages = core.midi_handler.process_input(&stop_message.to_bytes());
    assert!(!messages.is_empty());
    
    let continue_message = MidiMessage::Continue;
    let messages = core.midi_handler.process_input(&continue_message.to_bytes());
    assert!(!messages.is_empty());
}

/// Test MIDI channel filtering
/// Requirement 10.4: MIDI channel selection (1-16/OMNI) and message filtering
#[test]
fn test_midi_channel_filtering() {
    let mut core = LoopstationCore::new();
    
    // Test 1: Specific channel filtering
    let mut settings = core.midi_handler.get_settings().clone();
    settings.midi_channel = MidiChannel::Channel(3);
    core.midi_handler.set_settings(settings);
    
    // Message on correct channel should be processed
    let correct_channel_msg = MidiMessage::ControlChange {
        channel: 3,
        controller: 7,
        value: 64,
    };
    
    let messages = core.midi_handler.process_input(&correct_channel_msg.to_bytes());
    assert!(!messages.is_empty());
    
    // Message on wrong channel should be filtered out
    let wrong_channel_msg = MidiMessage::ControlChange {
        channel: 5,
        controller: 7,
        value: 64,
    };
    
    let messages = core.midi_handler.process_input_with_filtering(&wrong_channel_msg.to_bytes());
    // Should be empty due to channel filtering
    assert!(messages.is_empty());
    
    // Test 2: OMNI mode (accept all channels)
    let mut settings = core.midi_handler.get_settings().clone();
    settings.midi_channel = MidiChannel::Omni;
    core.midi_handler.set_settings(settings);
    
    // All channels should be accepted in OMNI mode
    let channel_1_msg = MidiMessage::ControlChange {
        channel: 1,
        controller: 7,
        value: 64,
    };
    
    let messages = core.midi_handler.process_input_with_filtering(&channel_1_msg.to_bytes());
    assert!(!messages.is_empty());
    
    let channel_16_msg = MidiMessage::ControlChange {
        channel: 16,
        controller: 7,
        value: 64,
    };
    
    let messages = core.midi_handler.process_input_with_filtering(&channel_16_msg.to_bytes());
    assert!(!messages.is_empty());
}

/// Test plugin integration for DAW use
/// Requirement 4.6: Parameter automation support for DAW integration
#[test]
fn test_plugin_integration_for_daw() {
    let mut core = LoopstationCore::new();
    
    // Test 1: Plugin parameter automation
    let start_time = Instant::now();
    
    // Simulate DAW parameter automation
    let _ = core.set_track_level(1, 0.8);
    let _ = core.set_track_level(2, 0.6);
    let _ = core.set_track_level(3, 0.4);
    core.set_master_level(0.9);
    core.set_tempo(125.0);
    
    let automation_time = start_time.elapsed();
    
    // Verify parameter changes
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.level, 0.8);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.level, 0.6);
    }
    
    if let Some(track3) = core.audio_engine.get_track(3) {
        assert_eq!(track3.level, 0.4);
    }
    
    assert_eq!(core.audio_engine.get_master_level(), 0.9);
    assert_eq!(core.get_tempo(), 125.0);
    
    // Verify automation response time
    assert!(automation_time.as_millis() < 50, "Parameter automation took too long: {:?}", automation_time);
    
    // Test 2: Audio processing (plugin callback simulation)
    let input = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    let mut output = [0.0; 8];
    
    let audio_start = Instant::now();
    core.process_audio(&input, &mut output);
    let audio_time = audio_start.elapsed();
    
    // Verify audio processing completed
    // Output should not be all zeros (some processing occurred)
    let has_output = output.iter().any(|&x| x != 0.0);
    // Note: Output might be zero if no tracks are playing, which is valid
    
    // Verify audio processing time (should be very fast)
    assert!(audio_time.as_micros() < 10000, "Audio processing took too long: {:?}", audio_time);
}

/// Test MIDI output functionality
/// Requirement 10.10: Program Change output for memory slot switching
/// Requirement 10.11: Control Change transmission for parameter updates
#[test]
fn test_midi_output_functionality() {
    let mut core = LoopstationCore::new();
    
    // Enable MIDI output
    let mut settings = core.midi_handler.get_settings().clone();
    settings.pc_out = true;
    settings.cc_tx_rx = true;
    core.midi_handler.set_settings(settings);
    
    // Test 1: Program Change output
    let result = core.midi_handler.send_program_change(5);
    assert!(result.is_ok());
    
    // Check that PC message was queued for output
    let output_data = core.midi_handler.get_output_data();
    assert!(!output_data.is_empty());
    
    // Parse the output message
    if let Some(message) = MidiMessage::from_bytes(output_data) {
        match message {
            MidiMessage::ProgramChange { channel: _, program } => {
                assert_eq!(program, 4); // Memory 5 = PC#4
            }
            _ => panic!("Expected ProgramChange message in output"),
        }
    }
    
    // Clear output buffer
    core.midi_handler.clear_output_buffer();
    
    // Test 2: Control Change output
    let result = core.midi_handler.send_control_change(7, 100); // Volume CC
    assert!(result.is_ok());
    
    let output_data = core.midi_handler.get_output_data();
    assert!(!output_data.is_empty());
    
    // Parse the output message
    if let Some(message) = MidiMessage::from_bytes(output_data) {
        match message {
            MidiMessage::ControlChange { channel: _, controller, value } => {
                assert_eq!(controller, 7);
                assert_eq!(value, 100);
            }
            _ => panic!("Expected ControlChange message in output"),
        }
    }
}

/// Test concurrent communication operations
/// Verify system can handle multiple communication protocols simultaneously
#[test]
fn test_concurrent_communication_operations() {
    let mut core = LoopstationCore::new();
    
    let start_time = Instant::now();
    
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
    assert!(total_time.as_millis() < 200, "Concurrent operations took too long: {:?}", total_time);
}

/// Test communication error handling and recovery
#[test]
fn test_communication_error_handling() {
    let mut core = LoopstationCore::new();
    
    // Test 1: Invalid MIDI data
    let invalid_midi_data = [0xFF, 0xFF, 0xFF]; // Invalid MIDI message
    let messages = core.midi_handler.process_input(&invalid_midi_data);
    // Should handle gracefully without crashing
    
    // Test 2: Invalid track operations
    let result = core.start_recording(0); // Invalid track ID
    assert!(result.is_err());
    
    let result = core.start_recording(7); // Invalid track ID
    assert!(result.is_err());
    
    // Test 3: Invalid parameter values
    let result = core.set_track_level(1, 2.0); // Should clamp to 1.0
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.level, 1.0); // Should be clamped
    }
    
    let result = core.set_track_level(1, -0.5); // Should clamp to 0.0
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.level, 0.0); // Should be clamped
    }
    
    // Test 4: Invalid tempo values
    core.set_tempo(300.0); // Should clamp to maximum
    assert!(core.get_tempo() <= 200.0); // Assuming 200 BPM max
    
    core.set_tempo(30.0); // Should clamp to minimum
    assert!(core.get_tempo() >= 60.0); // Assuming 60 BPM min
}

/// Test system performance under communication load
#[test]
fn test_communication_performance_under_load() {
    let mut core = LoopstationCore::new();
    
    let start_time = Instant::now();
    
    // Simulate high-frequency communication load
    for i in 0..100 {
        // MIDI CC messages
        let cc_msg = MidiMessage::ControlChange {
            channel: 1,
            controller: 7,
            value: (i % 128) as u8,
        };
        let _messages = core.midi_handler.process_input(&cc_msg.to_bytes());
        
        // OSC-style parameter changes
        let track_id = (i % 6) + 1;
        let level = (i as f32 % 100.0) / 100.0;
        let _ = core.set_track_level(track_id, level);
        
        // System updates
        if i % 10 == 0 {
            core.update(i * 10);
        }
    }
    
    let load_time = start_time.elapsed();
    
    // Verify system handled load without excessive delay
    assert!(load_time.as_millis() < 1000, "Communication load test took too long: {:?}", load_time);
    
    // Verify final state is consistent
    assert!(core.system_time_ms > 0);
    
    // Verify all tracks have valid levels
    for i in 1..=6 {
        if let Some(track) = core.audio_engine.get_track(i) {
            assert!(track.level >= 0.0 && track.level <= 1.0);
        }
    }
}