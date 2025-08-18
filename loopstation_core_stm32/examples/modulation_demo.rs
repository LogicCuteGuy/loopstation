//! Demonstration of the LFO and Step Sequencer modulation system
//! 
//! This example shows how to:
//! - Create and configure LFOs with different waveforms
//! - Set up Step Sequencers with custom patterns
//! - Assign modulation to various parameters
//! - Integrate modulation with the loopstation core

use loopstation_core_stm32::*;

fn main() {
    println!("=== LFO and Step Sequencer Modulation System Demo ===\n");

    // Create a loopstation core instance
    let mut loopstation = LoopstationCore::new();
    
    // Demo 1: LFO Configuration
    demo_lfo_configuration(&mut loopstation);
    
    // Demo 2: Step Sequencer Configuration
    demo_step_sequencer_configuration(&mut loopstation);
    
    // Demo 3: Modulation Assignment
    demo_modulation_assignment(&mut loopstation);
    
    // Demo 4: Real-time Modulation Processing
    demo_real_time_processing(&mut loopstation);
    
    println!("Demo completed successfully!");
}

fn demo_lfo_configuration(loopstation: &mut LoopstationCore) {
    println!("--- Demo 1: LFO Configuration ---");
    
    // Configure LFO 0 for track volume modulation
    loopstation.set_lfo_enabled(0, true);
    loopstation.set_lfo_waveform(0, LfoWaveform::Sine);
    loopstation.set_lfo_rate(0, 0.5); // 0.5 Hz - slow modulation
    loopstation.set_lfo_depth(0, 0.3); // 30% depth
    loopstation.set_lfo_sync_mode(0, LfoSyncMode::FreeRunning);
    
    println!("✓ LFO 0 configured: Sine wave, 0.5 Hz, 30% depth, free-running");
    
    // Configure LFO 1 for tempo-synced effect modulation
    loopstation.set_lfo_enabled(1, true);
    loopstation.set_lfo_waveform(1, LfoWaveform::Triangle);
    loopstation.set_lfo_tempo_division(1, TempoSyncDivision::Quarter);
    loopstation.set_lfo_depth(1, 0.5); // 50% depth
    loopstation.set_lfo_sync_mode(1, LfoSyncMode::TempoSync);
    
    println!("✓ LFO 1 configured: Triangle wave, quarter note sync, 50% depth");
    
    // Configure LFO 2 for random modulation
    loopstation.set_lfo_enabled(2, true);
    loopstation.set_lfo_waveform(2, LfoWaveform::Random);
    loopstation.set_lfo_rate(2, 2.0); // 2 Hz - faster random changes
    loopstation.set_lfo_depth(2, 0.2); // 20% depth for subtle randomness
    loopstation.set_lfo_phase_offset(2, 0.25); // 90 degree phase offset
    
    println!("✓ LFO 2 configured: Random wave, 2 Hz, 20% depth, 90° phase offset");
    
    println!();
}

fn demo_step_sequencer_configuration(loopstation: &mut LoopstationCore) {
    println!("--- Demo 2: Step Sequencer Configuration ---");
    
    // Configure Step Sequencer 0 with a rhythmic pattern
    loopstation.set_step_sequencer_enabled(0, true);
    loopstation.set_step_sequencer_length(0, 8); // 8-step pattern
    loopstation.set_step_sequencer_tempo_division(0, TempoSyncDivision::Sixteenth);
    loopstation.set_step_sequencer_swing(0, 0.1); // 10% swing
    
    // Create a rhythmic pattern: strong-weak-medium-off-strong-weak-medium-off
    let pattern = [1.0, 0.3, 0.6, 0.0, 1.0, 0.3, 0.6, 0.0];
    let gates = [0.8, 0.4, 0.6, 0.0, 0.8, 0.4, 0.6, 0.0];
    
    for (i, (&value, &gate)) in pattern.iter().zip(gates.iter()).enumerate() {
        loopstation.set_step_sequencer_step_value(0, i as u8, value);
        loopstation.set_step_sequencer_step_gate(0, i as u8, gate);
        loopstation.set_step_sequencer_step_velocity(0, i as u8, 1.0);
        loopstation.set_step_sequencer_step_enabled(0, i as u8, value > 0.0);
    }
    
    println!("✓ Step Sequencer 0 configured: 8-step rhythmic pattern, 16th note sync, 10% swing");
    
    // Configure Step Sequencer 1 with a melodic pattern
    loopstation.set_step_sequencer_enabled(1, true);
    loopstation.set_step_sequencer_length(1, 4); // 4-step pattern
    loopstation.set_step_sequencer_tempo_division(1, TempoSyncDivision::Eighth);
    
    // Create a melodic pattern for filter cutoff modulation
    let melodic_pattern = [0.2, 0.8, 0.5, 0.9];
    for (i, &value) in melodic_pattern.iter().enumerate() {
        loopstation.set_step_sequencer_step_value(1, i as u8, value);
        loopstation.set_step_sequencer_step_gate(1, i as u8, 0.7);
        loopstation.set_step_sequencer_step_velocity(1, i as u8, 0.8);
        loopstation.set_step_sequencer_step_enabled(1, i as u8, true);
    }
    
    println!("✓ Step Sequencer 1 configured: 4-step melodic pattern, 8th note sync");
    
    println!();
}

fn demo_modulation_assignment(loopstation: &mut LoopstationCore) {
    println!("--- Demo 3: Modulation Assignment ---");
    
    // Assign LFO 0 to modulate Track 1 volume
    let track1_volume_assignment = ModulationAssignment {
        target: ModulationTarget::TrackVolume(1),
        depth: 0.4, // 40% modulation depth
        bipolar: true, // Bipolar modulation (-1 to +1)
        enabled: true,
    };
    
    if let Ok(()) = loopstation.add_lfo_assignment(0, track1_volume_assignment) {
        println!("✓ LFO 0 assigned to Track 1 volume (40% depth, bipolar)");
    }
    
    // Assign LFO 1 to modulate effect parameters
    let effect_param_assignment = ModulationAssignment {
        target: ModulationTarget::EffectParameter {
            chain_type: crate::effects::EffectChainType::TrackFX,
            slot_index: 0,
            param_index: 0, // First parameter of first effect
            track_id: Some(1),
        },
        depth: 0.6, // 60% modulation depth
        bipolar: false, // Unipolar modulation (0 to +1)
        enabled: true,
    };
    
    if let Ok(()) = loopstation.add_lfo_assignment(1, effect_param_assignment) {
        println!("✓ LFO 1 assigned to Track 1 FX parameter (60% depth, unipolar)");
    }
    
    // Assign Step Sequencer 0 to modulate master volume
    let master_volume_assignment = ModulationAssignment {
        target: ModulationTarget::MasterVolume,
        depth: 0.2, // 20% modulation depth for subtle rhythmic pumping
        bipolar: false, // Unipolar to avoid complete silence
        enabled: true,
    };
    
    if let Ok(()) = loopstation.add_step_sequencer_assignment(0, master_volume_assignment) {
        println!("✓ Step Sequencer 0 assigned to Master Volume (20% depth, unipolar)");
    }
    
    // Assign LFO 2 (random) to modulate Track 2 pan
    let track2_pan_assignment = ModulationAssignment {
        target: ModulationTarget::TrackPan(2),
        depth: 0.3, // 30% random pan modulation
        bipolar: true, // Full left-right range
        enabled: true,
    };
    
    if let Ok(()) = loopstation.add_lfo_assignment(2, track2_pan_assignment) {
        println!("✓ LFO 2 (random) assigned to Track 2 pan (30% depth, bipolar)");
    }
    
    println!();
}

fn demo_real_time_processing(loopstation: &mut LoopstationCore) {
    println!("--- Demo 4: Real-time Modulation Processing ---");
    
    // Set tempo for tempo-synced modulation
    loopstation.set_tempo(120.0); // 120 BPM
    println!("✓ Tempo set to 120 BPM for tempo-synced modulation");
    
    // Simulate several update cycles to show modulation in action
    println!("\nSimulating modulation over time:");
    
    for cycle in 0..10 {
        let time_ms = cycle * 100; // 100ms per cycle
        
        // Update the loopstation (this processes modulation internally)
        loopstation.update(time_ms);
        
        // Get modulation activity status
        let activity = loopstation.get_modulation_activity();
        
        // Get current values for display
        let lfo0 = loopstation.get_lfo(0);
        let lfo1 = loopstation.get_lfo(1);
        let seq0 = loopstation.get_step_sequencer(0);
        
        if let (Some(lfo0), Some(lfo1), Some(seq0)) = (lfo0, lfo1, seq0) {
            println!(
                "Cycle {}: LFO0 phase={:.2}, LFO1 phase={:.2}, Seq0 step={}, Active: {}L/{}S, {}A",
                cycle,
                lfo0.current_phase,
                lfo1.current_phase,
                seq0.current_step,
                activity.active_lfos,
                activity.active_step_sequencers,
                activity.total_assignments
            );
        }
    }
    
    // Demonstrate tempo sync
    println!("\n✓ Demonstrating tempo sync:");
    loopstation.modulation_system_mut().sync_to_tempo(0.5); // Sync to middle of beat
    println!("  Synced all modulation sources to beat position 0.5");
    
    // Demonstrate reset
    println!("\n✓ Demonstrating modulation reset:");
    loopstation.reset_modulation();
    println!("  All modulation sources reset to initial phase");
    
    // Show final activity status
    let final_activity = loopstation.get_modulation_activity();
    println!(
        "\nFinal modulation status: {} active LFOs, {} active Step Sequencers, {} total assignments",
        final_activity.active_lfos,
        final_activity.active_step_sequencers,
        final_activity.total_assignments
    );
    
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modulation_demo() {
        // This test ensures the demo runs without panicking
        main();
    }

    #[test]
    fn test_lfo_waveform_accuracy() {
        let mut system = ModulationSystem::new(44100.0);
        
        // Test sine wave at known points
        if let Some(lfo) = system.get_lfo_mut(0) {
            lfo.enabled = true;
            lfo.waveform = LfoWaveform::Sine;
            lfo.set_rate_hz(1.0);
            lfo.set_depth(1.0);
            
            // Test at quarter phase (should be near 1.0)
            lfo.current_phase = 0.25;
            let value = lfo.update(120.0, 0); // No time advance
            assert!(value > 0.9, "Sine at 0.25 phase should be near 1.0, got {}", value);
        }
    }

    #[test]
    fn test_step_sequencer_pattern() {
        let mut system = ModulationSystem::new(44100.0);
        
        if let Some(seq) = system.get_step_sequencer_mut(0) {
            seq.enabled = true;
            seq.set_step_count(4);
            
            // Set up a test pattern
            seq.set_step_value(0, 1.0);
            seq.set_step_value(1, 0.5);
            seq.set_step_value(2, 0.0);
            seq.set_step_value(3, 0.75);
            
            // Test that we get the expected values
            assert_eq!(seq.get_step(0).unwrap().value, 1.0);
            assert_eq!(seq.get_step(1).unwrap().value, 0.5);
            assert_eq!(seq.get_step(2).unwrap().value, 0.0);
            assert_eq!(seq.get_step(3).unwrap().value, 0.75);
        }
    }

    #[test]
    fn test_modulation_assignment_application() {
        let assignment = ModulationAssignment::new(ModulationTarget::TrackVolume(1));
        
        // Test bipolar modulation
        let result = assignment.apply_modulation(0.5, 0.5); // 50% modulation on 50% base
        assert!(result > 0.5 && result < 0.75, "Bipolar modulation should increase value");
        
        // Test that disabled assignment doesn't change value
        let mut disabled = assignment.clone();
        disabled.enabled = false;
        let result = disabled.apply_modulation(0.5, 0.5);
        assert_eq!(result, 0.5, "Disabled assignment should not change value");
    }
}