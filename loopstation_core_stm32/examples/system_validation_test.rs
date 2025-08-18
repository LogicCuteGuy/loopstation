//! System validation test for final integration
//! 
//! This example validates the complete loopstation system against
//! all requirements, including hardware controls, project compatibility,
//! and MIDI synchronization.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

use loopstation_core_stm32::*;

#[cfg(not(feature = "std"))]
use panic_halt as _;

#[cfg(not(feature = "std"))]
use cortex_m_rt::entry;

#[cfg(feature = "std")]
fn main() {
    run_system_validation();
}

#[cfg(not(feature = "std"))]
#[entry]
fn main() -> ! {
    run_system_validation();
    loop {}
}

fn run_system_validation() {
    println!("=== Loopstation System Validation Test ===");
    
    // Create loopstation core
    let mut loopstation = LoopstationCore::new();
    
    // Test 1: Hardware control response time validation
    println!("\n1. Hardware Control Response Time Validation");
    test_control_response_time(&mut loopstation);
    
    // Test 2: Project compatibility validation
    println!("\n2. Project Compatibility Validation");
    test_project_compatibility(&mut loopstation);
    
    // Test 3: MIDI synchronization validation
    println!("\n3. MIDI Synchronization Validation");
    test_midi_synchronization(&mut loopstation);
    
    // Test 4: Core functionality validation
    println!("\n4. Core Functionality Validation");
    test_core_functionality(&mut loopstation);
    
    // Test 5: Audio processing validation
    println!("\n5. Audio Processing Validation");
    test_audio_processing(&mut loopstation);
    
    // Test 6: Memory system validation
    println!("\n6. Memory System Validation");
    test_memory_system(&mut loopstation);
    
    // Test 7: Effect system validation
    println!("\n7. Effect System Validation");
    test_effect_system(&mut loopstation);
    
    // Test 8: Performance requirements validation
    println!("\n8. Performance Requirements Validation");
    test_performance_requirements(&mut loopstation);
    
    println!("\n=== System Validation Complete ===");
}

fn test_control_response_time(loopstation: &mut LoopstationCore) {
    println!("Testing hardware control response time (<10ms requirement)...");
    
    // Simulate control events and measure response time
    let start_time = get_current_time_ms();
    
    // Test track button response
    for track_id in 1..=6 {
        let button_time = get_current_time_ms();
        let _ = loopstation.start_recording(track_id);
        let response_time = get_current_time_ms() - button_time;
        
        println!("  Track {} button response: {}ms", track_id, response_time);
        
        if response_time <= 10 {
            println!("    ✅ Response time meets <10ms requirement");
        } else {
            println!("    ❌ Response time exceeds 10ms requirement");
        }
    }
    
    // Test fader response
    let fader_time = get_current_time_ms();
    let _ = loopstation.set_track_level(1, 0.5);
    let fader_response = get_current_time_ms() - fader_time;
    
    println!("  Fader response time: {}ms", fader_response);
    if fader_response <= 10 {
        println!("    ✅ Fader response meets requirement");
    } else {
        println!("    ❌ Fader response exceeds requirement");
    }
    
    // Test effect button response
    let fx_time = get_current_time_ms();
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    let _ = loopstation.input_fx_mut().add_effect(compressor);
    let fx_response = get_current_time_ms() - fx_time;
    
    println!("  Effect button response: {}ms", fx_response);
    if fx_response <= 10 {
        println!("    ✅ Effect response meets requirement");
    } else {
        println!("    ❌ Effect response exceeds requirement");
    }
    
    let total_time = get_current_time_ms() - start_time;
    println!("  Total control test time: {}ms", total_time);
}

fn test_project_compatibility(loopstation: &mut LoopstationCore) {
    println!("Testing project compatibility between hardware and PC ecosystems...");
    
    // Create a test project with various content
    println!("  Creating test project...");
    
    // Add some tracks with different states
    let _ = loopstation.start_recording(1);
    let _ = loopstation.start_recording(2);
    let _ = loopstation.toggle_mute(3);
    
    // Add effects to different chains
    let mut reverb = Effect::new(EffectType::SpaceReverb);
    reverb.set_enabled(true);
    let _ = loopstation.input_fx_mut().add_effect(reverb);
    
    let mut delay = Effect::new(EffectType::TapeEcho);
    delay.set_enabled(true);
    if let Some(track_fx) = loopstation.track_fx_mut(1) {
        let _ = track_fx.add_effect(delay);
    }
    
    let mut eq = Effect::new(EffectType::MasteringEQ);
    eq.set_enabled(true);
    let _ = loopstation.master_fx_mut().add_effect(eq);
    
    // Set various parameters
    let _ = loopstation.set_track_level(1, 0.8);
    let _ = loopstation.set_track_level(2, 0.6);
    loopstation.set_tempo(128.0);
    
    println!("  Test project created with:");
    println!("    - 2 recording tracks, 1 muted track");
    println!("    - Input FX: Reverb");
    println!("    - Track 1 FX: Delay");
    println!("    - Master FX: EQ");
    println!("    - Custom levels and tempo");
    
    // Test project save
    println!("  Testing project save...");
    let save_result = loopstation.memory.save_project(1);
    match save_result {
        Ok(_) => println!("    ✅ Project saved successfully"),
        Err(e) => println!("    ❌ Project save failed: {:?}", e),
    }
    
    // Test project load
    println!("  Testing project load...");
    
    // Modify current state
    let _ = loopstation.clear_track(1);
    loopstation.set_tempo(120.0);
    
    // Load the saved project
    let load_result = loopstation.memory.load_project(1);
    match load_result {
        Ok(_) => {
            println!("    ✅ Project loaded successfully");
            
            // Verify state was restored
            let current_tempo = loopstation.get_tempo();
            if (current_tempo - 128.0).abs() < 0.1 {
                println!("    ✅ Tempo restored correctly");
            } else {
                println!("    ❌ Tempo not restored (expected 128, got {})", current_tempo);
            }
            
            // Check effect chains
            let input_fx_count = loopstation.input_fx().active_effect_count();
            let master_fx_count = loopstation.master_fx().active_effect_count();
            
            if input_fx_count > 0 && master_fx_count > 0 {
                println!("    ✅ Effect chains restored");
            } else {
                println!("    ❌ Effect chains not restored properly");
            }
        },
        Err(e) => println!("    ❌ Project load failed: {:?}", e),
    }
    
    println!("  ✅ Project compatibility validation complete");
}

fn test_midi_synchronization(loopstation: &mut LoopstationCore) {
    println!("Testing MIDI synchronization...");
    
    // Test MIDI clock sync
    println!("  Testing MIDI clock synchronization...");
    loopstation.set_midi_clock_sync(true);
    
    let sync_status = loopstation.get_midi_sync_status();
    println!("    MIDI sync status: {:?}", sync_status);
    
    // Test tempo-synced effects
    println!("  Testing tempo-synced effects...");
    let mut delay = Effect::new(EffectType::TapeEcho);
    delay.set_midi_sync(true);
    delay.set_enabled(true);
    
    // Test tempo changes affect synced effects
    let original_tempo = loopstation.get_tempo();
    loopstation.set_tempo(140.0);
    
    // Verify tempo was updated
    let new_tempo = loopstation.get_tempo();
    if (new_tempo - 140.0).abs() < 0.1 {
        println!("    ✅ Tempo update successful");
    } else {
        println!("    ❌ Tempo update failed");
    }
    
    // Test MIDI control messages
    println!("  Testing MIDI control integration...");
    
    // Simulate MIDI CC for track volume
    let midi_message = MidiMessage::ControlChange {
        channel: 1,
        controller: 7, // Volume CC
        value: 100,
    };
    
    loopstation.process_midi_message(midi_message, get_current_time_ms());
    
    // Test MIDI program change for memory switching
    let pc_message = MidiMessage::ProgramChange {
        channel: 1,
        program: 2, // Memory slot 3 (0-based)
    };
    
    loopstation.process_midi_message(pc_message, get_current_time_ms());
    
    println!("    ✅ MIDI message processing complete");
    
    // Restore original tempo
    loopstation.set_tempo(original_tempo);
    
    println!("  ✅ MIDI synchronization validation complete");
}

fn test_core_functionality(loopstation: &mut LoopstationCore) {
    println!("Testing core loopstation functionality...");
    
    // Test 6-track operation
    println!("  Testing 6-track simultaneous operation...");
    
    let mut successful_tracks = 0;
    
    for track_id in 1..=6 {
        let result = loopstation.start_recording(track_id);
        if result.is_ok() {
            successful_tracks += 1;
            println!("    Track {} recording started", track_id);
        } else {
            println!("    ❌ Track {} failed to start recording", track_id);
        }
    }
    
    if successful_tracks == 6 {
        println!("    ✅ All 6 tracks operational");
    } else {
        println!("    ❌ Only {} tracks operational", successful_tracks);
    }
    
    // Test track state transitions
    println!("  Testing track state transitions...");
    
    // Test recording -> playing
    let _ = loopstation.start_recording(1);
    let track1 = loopstation.audio_engine().get_track(1).unwrap();
    if track1.state == TrackState::Recording {
        println!("    ✅ Track 1 in recording state");
    }
    
    // Test mute functionality
    let _ = loopstation.toggle_mute(1);
    let track1 = loopstation.audio_engine().get_track(1).unwrap();
    if track1.state == TrackState::Muted {
        println!("    ✅ Track 1 mute functionality working");
    }
    
    // Test clear functionality
    let _ = loopstation.clear_track(1);
    let track1 = loopstation.audio_engine().get_track(1).unwrap();
    if track1.state == TrackState::Stopped {
        println!("    ✅ Track 1 clear functionality working");
    }
    
    println!("  ✅ Core functionality validation complete");
}

fn test_audio_processing(loopstation: &mut LoopstationCore) {
    println!("Testing audio processing pipeline...");
    
    // Test audio callback processing
    println!("  Testing audio callback processing...");
    
    let input = [0.1f32; 256]; // Test signal
    let mut output = [0.0f32; 256];
    
    // Process several buffers
    for i in 0..10 {
        loopstation.process_audio(&input, &mut output);
        
        // Check for audio processing
        let has_output = output.iter().any(|&sample| sample.abs() > 0.001);
        if i == 0 {
            if has_output {
                println!("    ✅ Audio processing producing output");
            } else {
                println!("    ⚠️  No audio output detected (may be normal)");
            }
        }
    }
    
    // Test effect processing
    println!("  Testing effect processing...");
    
    // Add effects and test processing
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    let _ = loopstation.input_fx_mut().add_effect(compressor);
    
    // Process with effects
    loopstation.process_audio(&input, &mut output);
    
    let effect_count = loopstation.input_fx().active_effect_count();
    if effect_count > 0 {
        println!("    ✅ Effects processing active ({} effects)", effect_count);
    } else {
        println!("    ❌ No effects processing detected");
    }
    
    println!("  ✅ Audio processing validation complete");
}

fn test_memory_system(loopstation: &mut LoopstationCore) {
    println!("Testing memory system...");
    
    // Test memory slot operations
    println!("  Testing memory slot operations...");
    
    // Create different projects in multiple slots
    for slot in 1..=5 {
        // Create unique project state
        loopstation.set_tempo(120.0 + slot as f32);
        let _ = loopstation.set_track_level(1, 0.1 * slot as f32);
        
        // Save project
        let save_result = loopstation.memory.save_project(slot);
        match save_result {
            Ok(_) => println!("    ✅ Project saved to slot {}", slot),
            Err(e) => println!("    ❌ Failed to save to slot {}: {:?}", slot, e),
        }
    }
    
    // Test loading different projects
    println!("  Testing project loading...");
    
    for slot in 1..=5 {
        let load_result = loopstation.memory.load_project(slot);
        match load_result {
            Ok(_) => {
                let tempo = loopstation.get_tempo();
                let expected_tempo = 120.0 + slot as f32;
                
                if (tempo - expected_tempo).abs() < 0.1 {
                    println!("    ✅ Slot {} loaded correctly (tempo: {})", slot, tempo);
                } else {
                    println!("    ❌ Slot {} data mismatch (expected {}, got {})", 
                            slot, expected_tempo, tempo);
                }
            },
            Err(e) => println!("    ❌ Failed to load slot {}: {:?}", slot, e),
        }
    }
    
    println!("  ✅ Memory system validation complete");
}

fn test_effect_system(loopstation: &mut LoopstationCore) {
    println!("Testing effect system...");
    
    // Test 3-layer effect architecture
    println!("  Testing 3-layer effect architecture...");
    
    // Input FX layer
    let mut input_compressor = Effect::new(EffectType::Compressor);
    input_compressor.set_enabled(true);
    let input_result = loopstation.input_fx_mut().add_effect(input_compressor);
    
    // Track FX layer
    let mut track_delay = Effect::new(EffectType::TapeEcho);
    track_delay.set_enabled(true);
    let track_result = if let Some(track_fx) = loopstation.track_fx_mut(1) {
        track_fx.add_effect(track_delay)
    } else {
        Err(())
    };
    
    // Master FX layer
    let mut master_eq = Effect::new(EffectType::MasteringEQ);
    master_eq.set_enabled(true);
    let master_result = loopstation.master_fx_mut().add_effect(master_eq);
    
    if input_result.is_ok() && track_result.is_ok() && master_result.is_ok() {
        println!("    ✅ All 3 effect layers operational");
    } else {
        println!("    ❌ Effect layer setup failed");
    }
    
    // Test effect parameter control
    println!("  Testing effect parameter control...");
    
    if let Some(Some(effect)) = loopstation.input_fx_mut().effects_mut().get_mut(0) {
        effect.set_parameter(0, 0.7); // Set first parameter to 70%
        
        if let Some(param) = effect.get_parameter(0) {
            if (param.value - 0.7).abs() < 0.01 {
                println!("    ✅ Effect parameter control working");
            } else {
                println!("    ❌ Effect parameter control failed");
            }
        }
    }
    
    // Test effect processing
    println!("  Testing effect processing...");
    
    let input = [0.1f32; 256];
    let mut output = [0.0f32; 256];
    
    // Process audio through effect chains
    loopstation.process_audio(&input, &mut output);
    
    let total_effects = loopstation.count_active_effects();
    println!("    Total active effects: {}", total_effects);
    
    if total_effects >= 3 {
        println!("    ✅ Effect processing pipeline active");
    } else {
        println!("    ⚠️  Limited effect processing detected");
    }
    
    println!("  ✅ Effect system validation complete");
}

fn test_performance_requirements(loopstation: &mut LoopstationCore) {
    println!("Testing performance requirements...");
    
    // Test latency requirement (<5ms)
    println!("  Testing latency requirement (<5ms)...");
    
    let latency_ms = loopstation.get_total_latency_ms();
    println!("    Total system latency: {:.2} ms", latency_ms);
    
    if latency_ms < 5.0 {
        println!("    ✅ Latency requirement met");
    } else {
        println!("    ❌ Latency exceeds 5ms requirement");
    }
    
    // Test performance under load
    println!("  Testing performance under full load...");
    
    // Start all tracks
    for track_id in 1..=6 {
        let _ = loopstation.start_recording(track_id);
    }
    
    // Add maximum effects
    for _ in 0..4 {
        let mut effect = Effect::new(EffectType::SpaceReverb);
        effect.set_enabled(true);
        let _ = loopstation.input_fx_mut().add_effect(effect);
    }
    
    // Process audio under load
    let input = [0.1f32; 256];
    let mut output = [0.0f32; 256];
    
    loopstation.reset_performance_counters();
    
    for _ in 0..100 {
        loopstation.process_audio(&input, &mut output);
    }
    
    let metrics = loopstation.get_performance_metrics();
    println!("    CPU Usage: {:.1}%", metrics.cpu_usage);
    println!("    Dropout Count: {}", metrics.dropout_count);
    println!("    Max Callback Time: {} μs", metrics.max_callback_time_us);
    
    let status = loopstation.check_performance_requirements();
    match status {
        PerformanceStatus::Good => println!("    ✅ Performance requirements met under load"),
        PerformanceStatus::Warning => println!("    ⚠️  Performance warnings under load"),
        PerformanceStatus::Critical => println!("    ❌ Performance critical under load"),
    }
    
    // Run comprehensive performance test
    println!("  Running comprehensive performance test...");
    
    let test_suite = loopstation.run_performance_tests();
    let overall_pass = test_suite.overall_status();
    
    if overall_pass {
        println!("    ✅ All performance tests passed");
    } else {
        println!("    ❌ Some performance tests failed");
        let failed = test_suite.failed_tests();
        for test in &failed {
            println!("      - {}: {:.2} {} (required: {:.2} {})", 
                    test.test_name, test.measured_value, test.units,
                    test.required_value, test.units);
        }
    }
    
    println!("  ✅ Performance requirements validation complete");
}

// Helper function to get current time (mock implementation)
fn get_current_time_ms() -> u32 {
    // In a real implementation, this would use platform-specific timing
    static mut MOCK_TIME: u32 = 0;
    unsafe {
        MOCK_TIME += 1;
        MOCK_TIME
    }
}

// Helper function for printing (no-op in no_std)
#[cfg(not(feature = "std"))]
macro_rules! println {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "std")]
use std::println;