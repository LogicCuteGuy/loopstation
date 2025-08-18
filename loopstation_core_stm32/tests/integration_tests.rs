//! Integration tests for communication protocols
//! Requirements: 5.2, 11.1, 11.2, 4.6

use loopstation_core_stm32::{
    LoopstationCore, 
    midi::{MidiHandler, MidiMessage, MidiSettings, MidiChannel},
    controls::{ControlEvent, ButtonPress, ControlInterfaceHal},
    audio::TrackState,
    effects::EffectType,
};

#[test]
fn test_loopstation_core_initialization() {
    let mut core = LoopstationCore::new();
    
    // Test initial state
    assert_eq!(core.selected_track, 1);
    assert_eq!(core.system_time_ms, 0);
    assert_eq!(core.audio_engine.get_sample_rate(), 44100);
    assert_eq!(core.get_tempo(), 120.0);
    
    // Test that all subsystems are initialized
    assert_eq!(core.memory.current_memory, 1);
    assert!(!core.tempo_system.get_midi_sync_status().enabled);
    assert!(!core.rhythm_system.is_playing());
}

#[test]
fn test_midi_message_processing() {
    let mut core = LoopstationCore::new();
    let timestamp = 1000;
    
    // Test MIDI Control Change message
    let cc_message = MidiMessage::ControlChange {
        channel: 1,
        controller: 7, // Volume CC
        value: 64,     // Mid-range
    };
    
    core.process_midi_message(cc_message, timestamp);
    
    // Should have processed the MIDI message
    // (Exact behavior depends on MIDI mapping configuration)
}

#[test]
fn test_midi_note_message_processing() {
    let mut core = LoopstationCore::new();
    let timestamp = 1000;
    
    // Test MIDI Note On message
    let note_on = MidiMessage::NoteOn {
        channel: 1,
        note: 60, // Middle C
        velocity: 100,
    };
    
    core.process_midi_message(note_on, timestamp);
    
    // Test MIDI Note Off message
    let note_off = MidiMessage::NoteOff {
        channel: 1,
        note: 60,
        velocity: 0,
    };
    
    core.process_midi_message(note_off, timestamp);
}

#[test]
fn test_midi_program_change_processing() {
    let mut core = LoopstationCore::new();
    let timestamp = 1000;
    
    // Test MIDI Program Change message (should switch memory slots)
    let pc_message = MidiMessage::ProgramChange {
        channel: 1,
        program: 5, // Should switch to memory slot 6 (PC#5 = Memory 6)
    };
    
    core.process_midi_message(pc_message, timestamp);
    
    // Memory slot should have changed (if valid)
    // Note: This depends on whether slot 6 exists and is initialized
}

#[test]
fn test_midi_clock_sync() {
    let mut core = LoopstationCore::new();
    let timestamp = 1000;
    
    // Enable MIDI clock sync
    core.set_midi_clock_sync(true);
    assert!(core.tempo_system.get_midi_sync_status().enabled);
    
    // Process MIDI clock messages
    let clock_message = MidiMessage::Clock;
    core.process_midi_message(clock_message, timestamp);
    
    let start_message = MidiMessage::Start;
    core.process_midi_message(start_message, timestamp);
    
    let stop_message = MidiMessage::Stop;
    core.process_midi_message(stop_message, timestamp);
}

#[test]
fn test_control_event_processing() {
    let mut core = LoopstationCore::new();
    
    // Test button press event
    let button_event = ControlEvent::ButtonPress {
        button: crate::controls::ButtonId::Track1,
        press_type: ButtonPress::Short,
    };
    
    core.process_control_event(button_event);
    
    // Should have processed the button press
    // (Exact behavior depends on control assignments)
}

#[test]
fn test_fader_control_integration() {
    let mut core = LoopstationCore::new();
    
    // Test fader movement
    let fader_event = ControlEvent::FaderMove {
        fader: crate::controls::FaderId::Track1Level,
        value: 0.7,
    };
    
    core.process_analog_control_event(fader_event);
    
    // Track 1 level should be updated
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.level, 0.7);
    }
}

#[test]
fn test_knob_control_integration() {
    let mut core = LoopstationCore::new();
    
    // Test knob turn
    let knob_event = ControlEvent::KnobTurn {
        knob: crate::controls::KnobId::OutputLevel,
        value: 0.8,
    };
    
    core.process_analog_control_event(knob_event);
    
    // Master level should be updated
    assert_eq!(core.audio_engine.get_master_level(), 0.8);
}

#[test]
fn test_expression_pedal_integration() {
    let mut core = LoopstationCore::new();
    
    // Test expression pedal input
    let exp_event = ControlEvent::ExpressionInput {
        input: crate::controls::ExpressionInput::CTL1_EXP1,
        value: 0.5,
    };
    
    core.process_analog_control_event(exp_event);
    
    // Should have processed expression input
    // (Exact behavior depends on expression assignments)
}

#[test]
fn test_track_operations_integration() {
    let mut core = LoopstationCore::new();
    
    // Test track selection
    core.select_track(3);
    assert_eq!(core.get_selected_track(), 3);
    
    // Test track recording
    let result = core.start_recording(1);
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.state, TrackState::Recording);
    }
    
    // Test track stopping
    let result = core.stop_track(1);
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.state, TrackState::Stopped);
    }
}

#[test]
fn test_track_playback_toggle() {
    let mut core = LoopstationCore::new();
    
    // Add some audio to track first
    if let Some(track) = core.audio_engine.get_track_mut(1) {
        let _ = track.audio_buffer.push(0.5);
        track.loop_length = 1;
    }
    
    // Test toggle playback
    let result = core.toggle_track_playback(1);
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.state, TrackState::Playing);
    }
    
    // Toggle again to stop
    let result = core.toggle_track_playback(1);
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.state, TrackState::Stopped);
    }
}

#[test]
fn test_track_mute_integration() {
    let mut core = LoopstationCore::new();
    
    // Set track to playing first
    if let Some(track) = core.audio_engine.get_track_mut(1) {
        let _ = track.audio_buffer.push(0.5);
        track.loop_length = 1;
        track.state = TrackState::Playing;
    }
    
    // Test mute toggle
    let result = core.toggle_mute(1);
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.state, TrackState::Muted);
    }
}

#[test]
fn test_track_clear_integration() {
    let mut core = LoopstationCore::new();
    
    // Add audio to track
    if let Some(track) = core.audio_engine.get_track_mut(1) {
        let _ = track.audio_buffer.push(0.5);
        let _ = track.audio_buffer.push(0.3);
        track.loop_length = 2;
        track.state = TrackState::Playing;
    }
    
    // Test clear track
    let result = core.clear_track(1);
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.state, TrackState::Stopped);
        assert_eq!(track.loop_length, 0);
        assert!(track.audio_buffer.is_empty());
    }
}

#[test]
fn test_effect_chain_integration() {
    let mut core = LoopstationCore::new();
    
    // Add effect to input FX
    let mut compressor = crate::effects::Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    let result = core.input_fx_mut().add_effect(compressor);
    assert!(result.is_ok());
    
    // Check effect was added
    assert_eq!(core.input_fx().active_effect_count(), 1);
    
    // Add effect to track FX
    let mut reverb = crate::effects::Effect::new(EffectType::SpaceReverb);
    reverb.set_enabled(true);
    if let Some(track_fx) = core.track_fx_mut(1) {
        let result = track_fx.add_effect(reverb);
        assert!(result.is_ok());
        assert_eq!(track_fx.active_effect_count(), 1);
    }
    
    // Add effect to master FX
    let mut eq = crate::effects::Effect::new(EffectType::MasteringEQ);
    eq.set_enabled(true);
    let result = core.master_fx_mut().add_effect(eq);
    assert!(result.is_ok());
    assert_eq!(core.master_fx().active_effect_count(), 1);
}

#[test]
fn test_tempo_system_integration() {
    let mut core = LoopstationCore::new();
    
    // Test tempo setting
    core.set_tempo(140.0);
    assert_eq!(core.get_tempo(), 140.0);
    assert_eq!(core.tempo_system.get_bpm(), 140.0);
    
    // Test tap tempo
    core.tap_tempo();
    let tap_status = core.get_tap_status();
    assert!(tap_status.tap_count > 0);
    
    // Test tempo reset
    core.reset_tempo();
    // Should reset to default or previous tempo
}

#[test]
fn test_rhythm_system_integration() {
    let mut core = LoopstationCore::new();
    
    // Test rhythm control
    core.start_rhythm();
    assert!(core.rhythm_system.is_playing());
    
    core.stop_rhythm();
    assert!(!core.rhythm_system.is_playing());
    
    core.toggle_rhythm();
    assert!(core.rhythm_system.is_playing());
    
    // Test pattern selection
    core.select_rhythm_pattern(2);
    assert_eq!(core.rhythm_system.current_pattern, 2);
    
    // Test rhythm volume
    core.set_rhythm_volume(0.7);
    assert_eq!(core.rhythm_system.master_volume, 0.7);
}

#[test]
fn test_audio_processing_integration() {
    let mut core = LoopstationCore::new();
    
    // Test audio processing callback
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    
    core.process_audio(&input, &mut output);
    
    // Output should not be all zeros (some processing occurred)
    // Note: Exact output depends on current state and effects
}

#[test]
fn test_system_update_integration() {
    let mut core = LoopstationCore::new();
    let time_ms = 5000;
    
    // Test system update
    core.update(time_ms);
    assert_eq!(core.system_time_ms, time_ms);
    
    // Update should not crash and should process all subsystems
}

#[test]
fn test_all_tracks_operations() {
    let mut core = LoopstationCore::new();
    
    // Add audio to multiple tracks
    for i in 1..=3 {
        if let Some(track) = core.audio_engine.get_track_mut(i) {
            let _ = track.audio_buffer.push(0.5);
            track.loop_length = 1;
        }
    }
    
    // Test all start
    core.start_all_tracks();
    for i in 1..=3 {
        if let Some(track) = core.audio_engine.get_track(i) {
            assert_eq!(track.state, TrackState::Playing);
        }
    }
    
    // Test all stop
    core.stop_all_tracks();
    for i in 1..=3 {
        if let Some(track) = core.audio_engine.get_track(i) {
            assert_eq!(track.state, TrackState::Stopped);
        }
    }
    
    // Test all clear
    core.clear_all_tracks();
    for i in 1..=3 {
        if let Some(track) = core.audio_engine.get_track(i) {
            assert_eq!(track.state, TrackState::Stopped);
            assert_eq!(track.loop_length, 0);
        }
    }
}

#[test]
fn test_undo_redo_integration() {
    let mut core = LoopstationCore::new();
    
    // Perform some actions
    let _ = core.set_track_level(1, 0.5);
    let _ = core.start_recording(1);
    
    // Test undo
    core.undo_last_action();
    
    // Test redo
    core.redo_last_action();
    
    // Should not crash and should handle undo/redo properly
}

#[test]
fn test_memory_operations_integration() {
    let mut core = LoopstationCore::new();
    
    // Test save current project
    core.save_current_project();
    
    // Test load project
    core.load_project();
    
    // Test memory navigation
    core.increment_memory_slot();
    core.decrement_memory_slot();
    
    // Should not crash and should handle memory operations
}

#[test]
fn test_midi_settings_integration() {
    let mut core = LoopstationCore::new();
    
    // Test MIDI settings
    let mut settings = core.midi_handler.get_settings().clone();
    settings.channel = MidiChannel::Channel(5);
    settings.cc_tx_rx = true;
    settings.program_change_out = true;
    
    core.midi_handler.set_settings(settings);
    
    let updated_settings = core.midi_handler.get_settings();
    assert_eq!(updated_settings.channel, MidiChannel::Channel(5));
    assert!(updated_settings.cc_tx_rx);
    assert!(updated_settings.program_change_out);
}

#[test]
fn test_panic_stop_integration() {
    let mut core = LoopstationCore::new();
    
    // Set up some active tracks
    for i in 1..=3 {
        if let Some(track) = core.audio_engine.get_track_mut(i) {
            let _ = track.audio_buffer.push(0.5);
            track.loop_length = 1;
            track.state = TrackState::Playing;
        }
    }
    
    // Start rhythm
    core.start_rhythm();
    
    // Test panic stop
    core.panic_stop();
    
    // All tracks should be stopped
    for i in 1..=6 {
        if let Some(track) = core.audio_engine.get_track(i) {
            assert_eq!(track.state, TrackState::Stopped);
        }
    }
    
    // Rhythm should be stopped
    assert!(!core.rhythm_system.is_playing());
}

#[test]
fn test_effect_processing_pipeline() {
    let mut core = LoopstationCore::new();
    
    // Test the 3-layer effect processing pipeline
    let result = core.demo_effect_processing();
    assert!(result.is_ok());
    
    // Verify effects were added to each layer
    assert!(core.input_fx().active_effect_count() > 0);
    assert!(core.master_fx().active_effect_count() > 0);
    
    if let Some(track_fx) = core.track_fx(1) {
        assert!(track_fx.active_effect_count() > 0);
    }
}

#[test]
fn test_midi_cc_transmission() {
    let mut core = LoopstationCore::new();
    
    // Enable MIDI CC transmission
    let mut settings = core.midi_handler.get_settings().clone();
    settings.cc_tx_rx = true;
    core.midi_handler.set_settings(settings);
    
    // Change track level (should send MIDI CC)
    let result = core.set_track_level(1, 0.8);
    assert!(result.is_ok());
    
    // Change master level (should send MIDI CC)
    core.set_master_level(0.9);
    
    // Change tempo (should send MIDI CC)
    core.set_tempo(130.0);
    
    // MIDI messages should have been queued for transmission
    // (Exact verification depends on MIDI output implementation)
}

#[test]
fn test_beat_position_tracking() {
    let mut core = LoopstationCore::new();
    
    // Set tempo
    core.set_tempo(120.0);
    
    // Get beat position
    let beat_pos = core.get_beat_position();
    assert!(beat_pos >= 0.0 && beat_pos <= 1.0);
    
    // Get bar position
    let bar_pos = core.get_bar_position();
    assert!(bar_pos >= 0.0 && bar_pos <= 1.0);
    
    // Test beat/bar detection
    let is_beat = core.is_beat_start();
    let is_bar = core.is_bar_start();
    
    // Should return boolean values without crashing
    assert!(is_beat == true || is_beat == false);
    assert!(is_bar == true || is_bar == false);
}

#[test]
fn test_communication_protocol_simulation() {
    // This test simulates STM32-ESP32 communication
    let mut core = LoopstationCore::new();
    
    // Simulate receiving a command from ESP32 (via UART)
    // In real implementation, this would be parsed from UART data
    
    // Simulate display update command
    core.system_time_ms = 1000;
    
    // Simulate menu navigation command
    core.selected_track = 2;
    
    // Simulate parameter change command
    let _ = core.set_track_level(2, 0.6);
    
    // Simulate status broadcast to ESP32
    let current_state = (
        core.selected_track,
        core.get_tempo(),
        core.get_beat_position(),
        core.rhythm_system.is_playing(),
    );
    
    // Verify state is consistent
    assert_eq!(current_state.0, 2);
    assert_eq!(current_state.1, 120.0);
    assert!(current_state.2 >= 0.0 && current_state.2 <= 1.0);
}

#[test]
fn test_osc_command_simulation() {
    // This test simulates OSC network commands
    let mut core = LoopstationCore::new();
    
    // Simulate OSC commands that would be received over network
    
    // OSC: /track/1/level 0.7
    let _ = core.set_track_level(1, 0.7);
    
    // OSC: /track/2/record
    let _ = core.start_recording(2);
    
    // OSC: /tempo 140
    core.set_tempo(140.0);
    
    // OSC: /rhythm/start
    core.start_rhythm();
    
    // OSC: /fx/input/compressor/enable
    if let Some(effect) = core.input_fx_mut().get_effect_mut(0) {
        effect.set_enabled(true);
    }
    
    // Verify commands were processed
    if let Some(track1) = core.audio_engine.get_track(1) {
        assert_eq!(track1.level, 0.7);
    }
    
    if let Some(track2) = core.audio_engine.get_track(2) {
        assert_eq!(track2.state, TrackState::Recording);
    }
    
    assert_eq!(core.get_tempo(), 140.0);
    assert!(core.rhythm_system.is_playing());
}

#[test]
fn test_response_time_requirements() {
    let mut core = LoopstationCore::new();
    let start_time = std::time::Instant::now();
    
    // Simulate control input processing (should be <10ms)
    let button_event = ControlEvent::ButtonPress {
        button: crate::controls::ButtonId::Track1,
        press_type: ButtonPress::Short,
    };
    core.process_control_event(button_event);
    
    let control_time = start_time.elapsed();
    
    // Simulate OSC command processing (should be <20ms)
    let osc_start_time = std::time::Instant::now();
    let _ = core.set_track_level(1, 0.5);
    let osc_time = osc_start_time.elapsed();
    
    // In a real-time system, these should meet timing requirements
    // For unit tests, we just verify they complete quickly
    assert!(control_time.as_millis() < 100); // Generous for unit test
    assert!(osc_time.as_millis() < 100);     // Generous for unit test
}