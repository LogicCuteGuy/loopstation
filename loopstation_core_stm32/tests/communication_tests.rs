//! Integration tests for communication protocols
//! Requirements: 5.2, 11.1, 11.2, 4.6

use loopstation_core_stm32::{
    LoopstationCore, 
    midi::{MidiMessage, MidiChannel},
};

#[test]
fn test_loopstation_core_initialization() {
    let core = LoopstationCore::new();
    
    // Test initial state
    assert_eq!(core.selected_track, 1);
    assert_eq!(core.system_time_ms, 0);
    assert_eq!(core.get_tempo(), 120.0);
    assert_eq!(core.memory.current_memory, 1);
}

#[test]
fn test_midi_message_processing() {
    let mut core = LoopstationCore::new();
    
    // Test MIDI Control Change message processing through handler
    let cc_message = MidiMessage::ControlChange {
        channel: 1,
        controller: 7, // Volume CC
        value: 64,     // Mid-range
    };
    
    // Process through MIDI handler
    let messages = core.midi_handler.process_input(&cc_message.to_bytes());
    assert!(!messages.is_empty());
    
    // Verify message was parsed correctly
    if let Some(parsed_msg) = messages.first() {
        match parsed_msg {
            MidiMessage::ControlChange { channel, controller, value } => {
                assert_eq!(*channel, 1);
                assert_eq!(*controller, 7);
                assert_eq!(*value, 64);
            }
            _ => panic!("Expected ControlChange message"),
        }
    }
}

#[test]
fn test_track_operations() {
    let mut core = LoopstationCore::new();
    
    // Test track selection
    core.select_track(3);
    assert_eq!(core.get_selected_track(), 3);
    
    // Test track recording
    let result = core.start_recording(1);
    assert!(result.is_ok());
    
    // Test track stopping
    let result = core.stop_track(1);
    assert!(result.is_ok());
}

#[test]
fn test_tempo_operations() {
    let mut core = LoopstationCore::new();
    
    // Test tempo setting
    core.set_tempo(140.0);
    assert_eq!(core.get_tempo(), 140.0);
    
    // Test tap tempo
    core.tap_tempo();
    let tap_status = core.get_tap_status();
    assert!(tap_status.tap_count > 0);
}

#[test]
fn test_rhythm_operations() {
    let mut core = LoopstationCore::new();
    
    // Test rhythm control
    core.start_rhythm();
    assert!(core.rhythm_system.is_playing());
    
    core.stop_rhythm();
    assert!(!core.rhythm_system.is_playing());
    
    core.toggle_rhythm();
    assert!(core.rhythm_system.is_playing());
}

#[test]
fn test_audio_processing() {
    let mut core = LoopstationCore::new();
    
    // Test audio processing callback
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    
    // Should not crash
    core.process_audio(&input, &mut output);
}

#[test]
fn test_system_update() {
    let mut core = LoopstationCore::new();
    let time_ms = 5000;
    
    // Test system update
    core.update(time_ms);
    assert_eq!(core.system_time_ms, time_ms);
}

#[test]
fn test_midi_settings() {
    let mut core = LoopstationCore::new();
    
    // Test MIDI settings
    let mut settings = core.midi_handler.get_settings().clone();
    settings.midi_channel = MidiChannel::Channel(5);
    settings.cc_tx_rx = true;
    
    core.midi_handler.set_settings(settings);
    
    let updated_settings = core.midi_handler.get_settings();
    assert_eq!(updated_settings.midi_channel, MidiChannel::Channel(5));
    assert!(updated_settings.cc_tx_rx);
}

#[test]
fn test_effect_integration() {
    let mut core = LoopstationCore::new();
    
    // Add effect to input FX
    let mut compressor = loopstation_core_stm32::effects::Effect::new(
        loopstation_core_stm32::effects::EffectType::Compressor
    );
    compressor.set_enabled(true);
    let result = core.input_fx_mut().add_effect(compressor);
    assert!(result.is_ok());
    
    // Check effect was added
    assert_eq!(core.input_fx().active_effect_count(), 1);
}

#[test]
fn test_track_level_control() {
    let mut core = LoopstationCore::new();
    
    // Test track level setting
    let result = core.set_track_level(1, 0.7);
    assert!(result.is_ok());
    
    if let Some(track) = core.audio_engine.get_track(1) {
        assert_eq!(track.level, 0.7);
    }
}

#[test]
fn test_master_level_control() {
    let mut core = LoopstationCore::new();
    
    // Test master level setting
    core.set_master_level(0.8);
    assert_eq!(core.audio_engine.get_master_level(), 0.8);
}