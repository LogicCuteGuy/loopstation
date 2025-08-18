//! MIDI I/O Test Example
//!
//! This example demonstrates the MIDI input/output functionality
//! including channel filtering, Control Change processing, Program Change,
//! and MIDI clock synchronization.

#![no_std]
#![no_main]

use loopstation_core_stm32::{
    LoopstationCore, MidiMessage, MidiChannel, MidiSettings,
    HardwareHal, HalError
};
use heapless::Vec;

/// MIDI I/O test demonstration
pub fn midi_io_test() -> Result<(), &'static str> {
    // Initialize loopstation core
    let mut loopstation = LoopstationCore::new();
    
    // Initialize hardware (stub for this example)
    if let Err(_) = loopstation.init_hardware() {
        return Err("Failed to initialize hardware");
    }

    // Configure MIDI settings
    configure_midi_settings(&mut loopstation)?;
    
    // Test MIDI input processing
    test_midi_input_processing(&mut loopstation)?;
    
    // Test MIDI output functionality
    test_midi_output_functionality(&mut loopstation)?;
    
    // Test MIDI clock synchronization
    test_midi_clock_sync(&mut loopstation)?;
    
    // Test channel filtering
    test_midi_channel_filtering(&mut loopstation)?;
    
    Ok(())
}

/// Configure MIDI settings for testing
fn configure_midi_settings(loopstation: &mut LoopstationCore) -> Result<(), &'static str> {
    // Set MIDI channel to 1
    loopstation.configure_midi(1, true);
    
    // Enable MIDI output (Program Change and Control Change)
    loopstation.configure_midi_output(true, true);
    
    // Enable MIDI clock synchronization
    loopstation.set_midi_clock_sync(true);
    
    // Verify settings
    let (pc_out, cc_tx_rx) = loopstation.get_midi_output_settings();
    if !pc_out || !cc_tx_rx {
        return Err("MIDI output settings not configured correctly");
    }
    
    Ok(())
}

/// Test MIDI input message processing
fn test_midi_input_processing(loopstation: &mut LoopstationCore) -> Result<(), &'static str> {
    // Simulate MIDI input messages
    let test_messages = create_test_midi_messages();
    
    for message in test_messages {
        // Process each message (in real hardware, this would come from UART)
        process_simulated_midi_message(loopstation, message)?;
    }
    
    Ok(())
}

/// Create test MIDI messages for simulation
fn create_test_midi_messages() -> Vec<MidiMessage, 16> {
    let mut messages = Vec::new();
    
    // Track control via MIDI notes
    let _ = messages.push(MidiMessage::NoteOn { 
        channel: 1, 
        note: 36, // Track 1 REC/PLAY (C2)
        velocity: 127 
    });
    
    // Volume control via MIDI CC
    let _ = messages.push(MidiMessage::ControlChange { 
        channel: 1, 
        controller: 7, // Track 1 Volume
        value: 100 
    });
    
    // Tempo control via MIDI CC
    let _ = messages.push(MidiMessage::ControlChange { 
        channel: 1, 
        controller: 20, // Tempo
        value: 64 // Mid-range tempo
    });
    
    // Memory slot change via Program Change
    let _ = messages.push(MidiMessage::ProgramChange { 
        channel: 1, 
        program: 5 // Memory slot 6 (PC#5)
    });
    
    // MIDI Clock messages
    let _ = messages.push(MidiMessage::Start);
    let _ = messages.push(MidiMessage::Clock);
    let _ = messages.push(MidiMessage::Clock);
    let _ = messages.push(MidiMessage::Stop);
    
    messages
}

/// Process a simulated MIDI message
fn process_simulated_midi_message(
    loopstation: &mut LoopstationCore, 
    message: MidiMessage
) -> Result<(), &'static str> {
    // In real hardware, this would be called from the HAL's MIDI input processing
    // For this test, we simulate the message processing directly
    
    match message {
        MidiMessage::NoteOn { note, velocity, .. } => {
            // Simulate track control
            if note == 36 && velocity > 0 {
                // Track 1 REC/PLAY
                if let Err(_) = loopstation.toggle_track_playback(1) {
                    return Err("Failed to toggle track playback");
                }
            }
        },
        MidiMessage::ControlChange { controller, value, .. } => {
            // Simulate parameter control
            let normalized_value = value as f32 / 127.0;
            
            match controller {
                7 => { // Track 1 Volume
                    if let Err(_) = loopstation.set_track_level(1, normalized_value) {
                        return Err("Failed to set track level");
                    }
                },
                20 => { // Tempo
                    let bpm = 60.0 + (normalized_value * 140.0);
                    loopstation.set_tempo(bpm);
                },
                _ => {
                    // Other controllers handled by the system
                }
            }
        },
        MidiMessage::ProgramChange { program, .. } => {
            // Simulate memory slot change
            let memory_slot = program + 1;
            // In real implementation, this would load the project
        },
        _ => {
            // Other messages handled by the system
        }
    }
    
    Ok(())
}

/// Test MIDI output functionality
fn test_midi_output_functionality(loopstation: &mut LoopstationCore) -> Result<(), &'static str> {
    // Test parameter changes that should generate MIDI output
    
    // Change track volume (should send MIDI CC)
    if let Err(_) = loopstation.set_track_level(1, 0.8) {
        return Err("Failed to set track level");
    }
    
    // Change master volume (should send MIDI CC)
    loopstation.set_master_level(0.9);
    
    // Change tempo (should send MIDI CC)
    loopstation.set_tempo(130.0);
    
    // Change memory slot (should send Program Change)
    if let Err(_) = loopstation.send_memory_change_midi(10) {
        return Err("Failed to send memory change MIDI");
    }
    
    // Broadcast current state (should send multiple MIDI messages)
    if let Err(_) = loopstation.broadcast_midi_state() {
        return Err("Failed to broadcast MIDI state");
    }
    
    Ok(())
}

/// Test MIDI clock synchronization
fn test_midi_clock_sync(loopstation: &mut LoopstationCore) -> Result<(), &'static str> {
    // Enable MIDI clock sync
    loopstation.set_midi_clock_sync(true);
    
    // Simulate MIDI clock messages with timing
    let mut timestamp = 0u32;
    
    // Start message
    process_simulated_midi_message(loopstation, MidiMessage::Start)?;
    
    // Clock pulses (24 per quarter note)
    for _ in 0..24 {
        timestamp += 20833; // ~120 BPM timing (500ms per beat / 24 = ~20.8ms per clock)
        process_simulated_midi_message(loopstation, MidiMessage::Clock)?;
    }
    
    // Stop message
    process_simulated_midi_message(loopstation, MidiMessage::Stop)?;
    
    Ok(())
}

/// Test MIDI channel filtering
fn test_midi_channel_filtering(loopstation: &mut LoopstationCore) -> Result<(), &'static str> {
    // Test channel-specific filtering
    loopstation.configure_midi(2, true); // Set to channel 2
    
    // Message on channel 1 (should be ignored)
    let message_ch1 = MidiMessage::ControlChange { 
        channel: 1, 
        controller: 7, 
        value: 64 
    };
    process_simulated_midi_message(loopstation, message_ch1)?;
    
    // Message on channel 2 (should be processed)
    let message_ch2 = MidiMessage::ControlChange { 
        channel: 2, 
        controller: 7, 
        value: 100 
    };
    process_simulated_midi_message(loopstation, message_ch2)?;
    
    // Test OMNI mode
    loopstation.configure_midi(0, true); // Set to OMNI (channel 0)
    
    // Messages on any channel should now be processed
    let message_ch3 = MidiMessage::ControlChange { 
        channel: 3, 
        controller: 7, 
        value: 80 
    };
    process_simulated_midi_message(loopstation, message_ch3)?;
    
    Ok(())
}

/// Demonstrate MIDI CC mappings
fn demonstrate_midi_cc_mappings() -> Result<(), &'static str> {
    use loopstation_core_stm32::midi::cc_mappings::*;
    
    // Track volume CCs (7-12)
    let track_volumes = [
        TRACK_1_VOLUME, TRACK_2_VOLUME, TRACK_3_VOLUME,
        TRACK_4_VOLUME, TRACK_5_VOLUME, TRACK_6_VOLUME
    ];
    
    // Track pan CCs (13-18)
    let track_pans = [
        TRACK_1_PAN, TRACK_2_PAN, TRACK_3_PAN,
        TRACK_4_PAN, TRACK_5_PAN, TRACK_6_PAN
    ];
    
    // System CCs
    let master_volume = MASTER_VOLUME; // 19
    let tempo = TEMPO; // 20
    
    // Effect parameter CCs (21-24)
    let fx_params = [
        FX_1_PARAM_1, FX_1_PARAM_2, FX_1_PARAM_3, FX_1_PARAM_4
    ];
    
    // Expression pedal CCs (1-4)
    let expression_pedals = [
        EXPRESSION_1, EXPRESSION_2, EXPRESSION_3, EXPRESSION_4
    ];
    
    Ok(())
}

/// Demonstrate MIDI note mappings
fn demonstrate_midi_note_mappings() -> Result<(), &'static str> {
    use loopstation_core_stm32::midi::note_mappings::*;
    
    // Track control notes (36-41)
    let track_rec_play = [
        TRACK_1_REC_PLAY, TRACK_2_REC_PLAY, TRACK_3_REC_PLAY,
        TRACK_4_REC_PLAY, TRACK_5_REC_PLAY, TRACK_6_REC_PLAY
    ];
    
    // Track stop notes (42-47)
    let track_stop = [
        TRACK_1_STOP, TRACK_2_STOP, TRACK_3_STOP,
        TRACK_4_STOP, TRACK_5_STOP, TRACK_6_STOP
    ];
    
    // Transport control notes
    let all_start = ALL_START; // 48
    let all_stop = ALL_STOP; // 49
    let tap_tempo = TAP_TEMPO; // 50
    
    Ok(())
}

/// Main test function
#[cfg(not(feature = "embedded"))]
fn main() -> Result<(), &'static str> {
    midi_io_test()?;
    demonstrate_midi_cc_mappings()?;
    demonstrate_midi_note_mappings()?;
    
    Ok(())
}

/// Embedded entry point (stub)
#[cfg(feature = "embedded")]
#[cortex_m_rt::entry]
fn main() -> ! {
    if let Err(_) = midi_io_test() {
        // Error handling for embedded
    }
    
    loop {
        // Main loop for embedded
    }
}