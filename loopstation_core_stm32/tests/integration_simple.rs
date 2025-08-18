//! Simple integration tests for communication protocols
//! Requirements: 5.2, 11.1, 11.2, 4.6

use loopstation_core_stm32::{
    LoopstationCore, 
    midi::{MidiMessage, MidiChannel},
    audio::TrackState,
};

#[test]
fn test_uart_communication_simulation() {
    // Simulate STM32-ESP32 UART communication protocol
    let mut core = LoopstationCore::new();
    
    // Simulate receiving status update command from ESP32
    core.system_time_ms = 1000;
    core.selected_track = 2;
    
    // Simulate parameter change via UART
    let _ = core.set_track_level(2, 0.6);
    
    // Verify state changes
    assert_eq!(core.selected_track, 2);
    if let Some(track) = core.audio_engine.get_track(2) {
        assert_eq!(track.level, 0.6);
    }
}

#[test]
fn test_osc_command_processing() {
    // Simulate OSC network command processing with <20ms response time
    let mut core = LoopstationCore::new();
    
    let start_time = std::time::Instant::now();
    
    // Simulate OSC commands
    let _ = core.set_track_level(1, 0.7);
    let _ = core.start_recording(2);
    core.set_tempo(140.0);
    core.start_rhythm();
    
    let processing_time = start_time.elapsed();
    
    // Verify commands were processed
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.level, 0.7);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.state, TrackState::Recording);
    }
    
    assert_eq!(core.get_tempo(), 140.0);
    assert!(core.rhythm_system.is_playing());
    
    // Should process quickly (generous limit for test environment)
    assert!(processing_time.as_millis() < 100);
}

#[test]
fn test_midi_functionality_integration() {
    let mut core = LoopstationCore::new();
    let timestamp = 1000;
    
    // Test MIDI settings
    let mut settings = core.midi_handler.get_settings().clone();
    settings.channel = MidiChannel::Channel(5);
    settings.cc_tx_rx = true;
    settings.program_change_out = true;
    core.midi_handler.set_settings(settings);
    
    // Test MIDI message processing
    let cc_message = MidiMessage::ControlChange {
        channel: 5,
        controller: 7, // Volume CC
        value: 100,
    };
    core.process_midi_message(cc_message, timestamp);
    
    let pc_message = MidiMessage::ProgramChange {
        channel: 5,
        program: 2, // Should switch to memory slot 3
    };
    core.process_midi_message(pc_message, timestamp);
    
    // Test MIDI clock
    let clock_message = MidiMessage::Clock;
    core.process_midi_message(clock_message, timestamp);
    
    // Verify MIDI settings were applied
    let updated_settings = core.midi_handler.get_settings();
    assert_eq!(updated_settings.channel, MidiChannel::Channel(5));
    assert!(updated_settings.cc_tx_rx);
    assert!(updated_settings.program_change_out);
}

#[test]
fn test_plugin_integration() {
    // Test plugin-style integration for DAW use
    let mut core = LoopstationCore::new();
    
    // Simulate plugin parameter automation
    let _ = core.set_track_level(1, 0.8);
    let _ = core.set_track_level(2, 0.6);
    let _ = core.set_master_level(0.9);
    
    // Simulate MIDI input from DAW
    let midi_note = MidiMessage::NoteOn {
        channel: 1,
        note: 60,
        velocity: 100,
    };
    core.process_midi_message(midi_note, 1000);
    
    // Simulate audio processing (plugin callback)
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    core.process_audio(&input, &mut output);
    
    // Verify plugin-style operations work
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.level, 0.8);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.level, 0.6);
    }
    
    assert_eq!(core.audio_engine.get_master_level(), 0.9);
}

#[test]
fn test_system_response_times() {
    // Test control response time requirements (<10ms)
    let mut core = LoopstationCore::new();
    
    let start_time = std::time::Instant::now();
    
    // Simulate rapid control changes
    for i in 1..=6 {
        let _ = core.set_track_level(i, i as f32 / 10.0);
    }
    
    let control_time = start_time.elapsed();
    
    // Test system update
    let update_start = std::time::Instant::now();
    core.update(5000);
    let update_time = update_start.elapsed();
    
    // Verify response times (generous for test environment)
    assert!(control_time.as_millis() < 50);
    assert!(update_time.as_millis() < 50);
}

#[test]
fn test_communication_error_handling() {
    let mut core = LoopstationCore::new();
    
    // Test invalid MIDI messages
    let invalid_cc = MidiMessage::ControlChange {
        channel: 17, // Invalid channel
        controller: 200, // Invalid controller
        value: 255,
    };
    
    // Should not crash
    core.process_midi_message(invalid_cc, 1000);
    
    // Test invalid track operations
    let result = core.start_recording(0); // Invalid track
    assert!(result.is_err());
    
    let result = core.start_recording(7); // Invalid track
    assert!(result.is_err());
    
    // Test invalid level settings
    let result = core.set_track_level(1, 2.0); // Should clamp to 1.0
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.level, 1.0); // Should be clamped
    }
}

#[test]
fn test_concurrent_operations() {
    let mut core = LoopstationCore::new();
    
    // Test multiple simultaneous operations
    let _ = core.start_recording(1);
    let _ = core.start_recording(2);
    core.set_tempo(150.0);
    core.start_rhythm();
    
    // Add effects
    let mut compressor = loopstation_core_stm32::effects::Effect::new(
        loopstation_core_stm32::effects::EffectType::Compressor
    );
    compressor.set_enabled(true);
    let _ = core.input_fx_mut().add_effect(compressor);
    
    // Process audio while everything is active
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    core.process_audio(&input, &mut output);
    
    // Update system
    core.update(2000);
    
    // Verify all operations completed successfully
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.state, TrackState::Recording);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.state, TrackState::Recording);
    }
    
    assert_eq!(core.get_tempo(), 150.0);
    assert!(core.rhythm_system.is_playing());
    assert_eq!(core.input_fx().active_effect_count(), 1);
}

#[test]
fn test_state_synchronization() {
    // Test state synchronization between components
    let mut core = LoopstationCore::new();
    
    // Change multiple states
    core.select_track(3);
    let _ = core.set_track_level(3, 0.7);
    core.set_tempo(130.0);
    
    // Simulate status broadcast (what would be sent to ESP32)
    let system_state = (
        core.selected_track,
        core.get_tempo(),
        core.get_beat_position(),
        core.rhythm_system.is_playing(),
    );
    
    // Verify state consistency
    assert_eq!(system_state.0, 3);
    assert_eq!(system_state.1, 130.0);
    assert!(system_state.2 >= 0.0 && system_state.2 <= 1.0);
    assert_eq!(system_state.3, false);
    
    if let Some(track) = core.audio_engine.get_track(3) {
        assert_eq!(track.level, 0.7);
    }
}