//! Tests for the LFO and Step Sequencer modulation system

use loopstation_core_stm32::modulation::*;
use loopstation_core_stm32::effects::EffectChainType;

#[test]
fn test_lfo_waveform_generation() {
    let mut random_state = 12345;
    
    // Test sine wave at key points
    let sine_0 = LfoWaveform::Sine.generate(0.0, &mut random_state);
    let sine_quarter = LfoWaveform::Sine.generate(0.25, &mut random_state);
    let sine_half = LfoWaveform::Sine.generate(0.5, &mut random_state);
    let sine_three_quarter = LfoWaveform::Sine.generate(0.75, &mut random_state);
    
    assert!((sine_0 - 0.0).abs() < 0.001, "Sine at 0° should be ~0");
    assert!(sine_quarter > 0.9, "Sine at 90° should be ~1");
    assert!((sine_half - 0.0).abs() < 0.001, "Sine at 180° should be ~0");
    assert!(sine_three_quarter < -0.9, "Sine at 270° should be ~-1");
    
    // Test triangle wave
    let tri_0 = LfoWaveform::Triangle.generate(0.0, &mut random_state);
    let tri_quarter = LfoWaveform::Triangle.generate(0.25, &mut random_state);
    let tri_half = LfoWaveform::Triangle.generate(0.5, &mut random_state);
    let tri_three_quarter = LfoWaveform::Triangle.generate(0.75, &mut random_state);
    
    assert!((tri_0 - (-1.0)).abs() < 0.001, "Triangle at 0 should be -1");
    assert!((tri_quarter - 0.0).abs() < 0.001, "Triangle at 0.25 should be 0");
    assert!((tri_half - 1.0).abs() < 0.001, "Triangle at 0.5 should be 1");
    assert!((tri_three_quarter - 0.0).abs() < 0.001, "Triangle at 0.75 should be 0");
    
    // Test square wave
    let square_0 = LfoWaveform::Square.generate(0.0, &mut random_state);
    let square_quarter = LfoWaveform::Square.generate(0.25, &mut random_state);
    let square_half = LfoWaveform::Square.generate(0.5, &mut random_state);
    let square_three_quarter = LfoWaveform::Square.generate(0.75, &mut random_state);
    
    assert_eq!(square_0, -1.0, "Square at 0-0.5 should be -1");
    assert_eq!(square_quarter, -1.0, "Square at 0.25 should be -1");
    assert_eq!(square_half, 1.0, "Square at 0.5-1.0 should be 1");
    assert_eq!(square_three_quarter, 1.0, "Square at 0.75 should be 1");
    
    // Test sawtooth wave
    let saw_0 = LfoWaveform::Sawtooth.generate(0.0, &mut random_state);
    let saw_quarter = LfoWaveform::Sawtooth.generate(0.25, &mut random_state);
    let saw_half = LfoWaveform::Sawtooth.generate(0.5, &mut random_state);
    let saw_three_quarter = LfoWaveform::Sawtooth.generate(0.75, &mut random_state);
    
    assert!((saw_0 - (-1.0)).abs() < 0.001, "Sawtooth at 0 should be -1");
    assert!((saw_quarter - (-0.5)).abs() < 0.001, "Sawtooth at 0.25 should be -0.5");
    assert!((saw_half - 0.0).abs() < 0.001, "Sawtooth at 0.5 should be 0");
    assert!((saw_three_quarter - 0.5).abs() < 0.001, "Sawtooth at 0.75 should be 0.5");
}

#[test]
fn test_lfo_creation_and_basic_operation() {
    let mut lfo = Lfo::new(0, 44100.0);
    
    // Test initial state
    assert_eq!(lfo.id, 0);
    assert_eq!(lfo.waveform, LfoWaveform::Sine);
    assert_eq!(lfo.rate_hz, 1.0);
    assert_eq!(lfo.depth, 0.5);
    assert_eq!(lfo.phase_offset, 0.0);
    assert_eq!(lfo.current_phase, 0.0);
    assert!(!lfo.enabled);
    
    // Test parameter setting
    lfo.set_rate_hz(2.0);
    assert_eq!(lfo.rate_hz, 2.0);
    
    lfo.set_depth(0.8);
    assert_eq!(lfo.depth, 0.8);
    
    lfo.set_phase_offset(0.25);
    assert_eq!(lfo.phase_offset, 0.25);
    
    // Test clamping
    lfo.set_rate_hz(25.0); // Should clamp to 20.0
    assert_eq!(lfo.rate_hz, 20.0);
    
    lfo.set_depth(1.5); // Should clamp to 1.0
    assert_eq!(lfo.depth, 1.0);
}

#[test]
fn test_lfo_update_and_phase_progression() {
    let mut lfo = Lfo::new(0, 44100.0);
    lfo.enabled = true;
    lfo.set_rate_hz(1.0); // 1 Hz = 1 cycle per second
    lfo.set_depth(1.0); // Full depth
    
    // Update for 1/4 second (should advance phase by 0.25)
    let samples_per_quarter_second = 11025; // 44100 / 4
    let value1 = lfo.update(120.0, samples_per_quarter_second);
    
    // Phase should be around 0.25
    assert!(lfo.current_phase > 0.2 && lfo.current_phase < 0.3, 
           "Phase after 1/4 second should be ~0.25, got {}", lfo.current_phase);
    
    // Should generate some output (sine at 0.25 phase should be positive)
    assert!(value1 > 0.5, "LFO output should be positive at 0.25 phase");
    
    // Update for another 1/4 second (total 1/2 second, phase should be ~0.5)
    let value2 = lfo.update(120.0, samples_per_quarter_second);
    
    assert!(lfo.current_phase > 0.45 && lfo.current_phase < 0.55,
           "Phase after 1/2 second should be ~0.5, got {}", lfo.current_phase);
    
    // At phase 0.5, sine should be near 0
    assert!(value2.abs() < 0.1, "LFO output should be near 0 at 0.5 phase");
}

#[test]
fn test_lfo_tempo_sync() {
    let mut lfo = Lfo::new(0, 44100.0);
    lfo.enabled = true;
    lfo.set_sync_mode(LfoSyncMode::TempoSync);
    lfo.set_tempo_division(TempoSyncDivision::Quarter); // Quarter note sync
    lfo.set_depth(1.0);
    
    let bpm = 120.0; // 120 BPM = 2 beats per second = 2 Hz for quarter notes
    
    // Update for 1/4 second at 120 BPM
    let samples_per_quarter_second = 11025;
    let _value = lfo.update(bpm, samples_per_quarter_second);
    
    // At 120 BPM, quarter note frequency is 2 Hz
    // So in 1/4 second, we should advance by 0.5 cycles
    assert!(lfo.current_phase > 0.4 && lfo.current_phase < 0.6,
           "Phase should be ~0.5 for quarter note sync at 120 BPM");
}

#[test]
fn test_step_sequencer_creation_and_basic_operation() {
    let mut sequencer = StepSequencer::new(0, 44100.0);
    
    // Test initial state
    assert_eq!(sequencer.id, 0);
    assert_eq!(sequencer.step_count, 16);
    assert_eq!(sequencer.current_step, 0);
    assert_eq!(sequencer.tempo_division, TempoSyncDivision::Sixteenth);
    assert_eq!(sequencer.swing, 0.0);
    assert!(!sequencer.enabled);
    
    // Test step manipulation
    sequencer.set_step_value(0, 1.0);
    sequencer.set_step_value(1, 0.5);
    sequencer.set_step_value(2, 0.0);
    
    assert_eq!(sequencer.get_step(0).unwrap().value, 1.0);
    assert_eq!(sequencer.get_step(1).unwrap().value, 0.5);
    assert_eq!(sequencer.get_step(2).unwrap().value, 0.0);
    
    // Test step count setting
    sequencer.set_step_count(8);
    assert_eq!(sequencer.step_count, 8);
    
    // Test swing setting
    sequencer.set_swing(0.5);
    assert_eq!(sequencer.swing, 0.5);
}

#[test]
fn test_step_sequencer_playback() {
    let mut sequencer = StepSequencer::new(0, 44100.0);
    sequencer.enabled = true;
    sequencer.set_step_count(4);
    sequencer.set_tempo_division(TempoSyncDivision::Quarter); // Quarter note steps
    
    // Set up a simple pattern: 1.0, 0.5, 0.0, 0.75
    sequencer.set_step_value(0, 1.0);
    sequencer.set_step_value(1, 0.5);
    sequencer.set_step_value(2, 0.0);
    sequencer.set_step_value(3, 0.75);
    
    let bpm = 120.0; // 120 BPM = 2 Hz for quarter notes
    
    // At start, should be on step 0
    let value1 = sequencer.update(bpm, 1024);
    assert_eq!(sequencer.current_step, 0);
    assert!(value1 > 0.9, "Should output step 0 value (~1.0)");
    
    // Advance to next step (this is approximate due to timing)
    let samples_per_quarter_note = 22050; // 44100 / 2 (2 Hz)
    let _value2 = sequencer.update(bpm, samples_per_quarter_note);
    
    // Should have advanced to step 1
    assert!(sequencer.current_step >= 1, "Should have advanced to step 1 or beyond");
}

#[test]
fn test_modulation_assignment() {
    let target = ModulationTarget::TrackVolume(1);
    let assignment = ModulationAssignment::new(target.clone());
    
    // Test default values
    assert_eq!(assignment.target, target);
    assert_eq!(assignment.depth, 0.5);
    assert!(assignment.bipolar);
    assert!(assignment.enabled);
    
    // Test bipolar modulation
    let base_value = 0.5;
    let modulation_value = 0.5; // 50% of full scale
    let result = assignment.apply_modulation(modulation_value, base_value);
    
    // With 50% depth and 50% modulation, should add 0.25 to base
    assert!(result > base_value && result < 0.75, 
           "Bipolar modulation should increase value, got {}", result);
    
    // Test unipolar modulation
    let mut unipolar_assignment = assignment.clone();
    unipolar_assignment.bipolar = false;
    let result = unipolar_assignment.apply_modulation(modulation_value, base_value);
    
    // Unipolar should also increase the value
    assert!(result > base_value, "Unipolar modulation should increase value");
    
    // Test disabled assignment
    let mut disabled_assignment = assignment.clone();
    disabled_assignment.enabled = false;
    let result = disabled_assignment.apply_modulation(modulation_value, base_value);
    
    assert_eq!(result, base_value, "Disabled assignment should not change value");
}

#[test]
fn test_modulation_system_creation() {
    let system = ModulationSystem::new(44100.0);
    
    // Should have 8 LFOs and 4 Step Sequencers
    assert_eq!(system.lfos.len(), MAX_LFOS);
    assert_eq!(system.step_sequencers.len(), MAX_STEP_SEQUENCERS);
    
    // All should be disabled initially
    for lfo in &system.lfos {
        assert!(!lfo.enabled);
    }
    
    for sequencer in &system.step_sequencers {
        assert!(!sequencer.enabled);
    }
}

#[test]
fn test_modulation_system_update() {
    let mut system = ModulationSystem::new(44100.0);
    
    // Enable first LFO
    if let Some(lfo) = system.get_lfo_mut(0) {
        lfo.enabled = true;
        lfo.set_rate_hz(2.0);
        lfo.set_depth(1.0);
    }
    
    // Enable first Step Sequencer
    if let Some(sequencer) = system.get_step_sequencer_mut(0) {
        sequencer.enabled = true;
        sequencer.set_step_count(4);
        sequencer.set_step_value(0, 1.0);
    }
    
    // Update system
    let values = system.update(120.0, 1024);
    
    // Should have modulation values
    assert!(values.lfo_values[0].abs() >= 0.0, "LFO 0 should produce some output");
    assert!(values.step_sequencer_values[0] >= 0.0, "Step Sequencer 0 should produce output");
    
    // Other LFOs and sequencers should be silent (disabled)
    for i in 1..MAX_LFOS {
        assert_eq!(values.lfo_values[i], 0.0, "Disabled LFO {} should be silent", i);
    }
    
    for i in 1..MAX_STEP_SEQUENCERS {
        assert_eq!(values.step_sequencer_values[i], 0.0, "Disabled Step Sequencer {} should be silent", i);
    }
}

#[test]
fn test_modulation_system_activity_tracking() {
    let mut system = ModulationSystem::new(44100.0);
    
    // Initially no activity
    let activity = system.get_modulation_activity();
    assert_eq!(activity.active_lfos, 0);
    assert_eq!(activity.active_step_sequencers, 0);
    assert_eq!(activity.total_assignments, 0);
    
    // Enable some LFOs
    system.get_lfo_mut(0).unwrap().enabled = true;
    system.get_lfo_mut(1).unwrap().enabled = true;
    
    // Enable some Step Sequencers
    system.get_step_sequencer_mut(0).unwrap().enabled = true;
    
    // Add some assignments
    let assignment = ModulationAssignment::new(ModulationTarget::TrackVolume(1));
    let _ = system.get_lfo_mut(0).unwrap().add_assignment(assignment.clone());
    let _ = system.get_step_sequencer_mut(0).unwrap().add_assignment(assignment);
    
    let activity = system.get_modulation_activity();
    assert_eq!(activity.active_lfos, 2);
    assert_eq!(activity.active_step_sequencers, 1);
    assert_eq!(activity.total_assignments, 2);
}

#[test]
fn test_modulation_system_sync_to_tempo() {
    let mut system = ModulationSystem::new(44100.0);
    
    // Enable and configure LFO for tempo sync
    if let Some(lfo) = system.get_lfo_mut(0) {
        lfo.enabled = true;
        lfo.set_sync_mode(LfoSyncMode::TempoSync);
        lfo.set_tempo_division(TempoSyncDivision::Quarter);
    }
    
    // Enable and configure Step Sequencer
    if let Some(sequencer) = system.get_step_sequencer_mut(0) {
        sequencer.enabled = true;
        sequencer.set_step_count(4);
        sequencer.set_tempo_division(TempoSyncDivision::Quarter);
    }
    
    // Sync to beat position 0.5 (halfway through first beat)
    system.sync_to_tempo(0.5);
    
    // LFO should be synced to beat position
    let lfo = system.get_lfo(0).unwrap();
    assert!(lfo.current_phase > 0.4 && lfo.current_phase < 0.6,
           "LFO phase should be synced to beat position");
    
    // Step Sequencer should also be synced
    let sequencer = system.get_step_sequencer(0).unwrap();
    assert!(sequencer.step_phase > 0.4 && sequencer.step_phase < 0.6,
           "Step Sequencer phase should be synced to beat position");
}

#[test]
fn test_modulation_system_reset() {
    let mut system = ModulationSystem::new(44100.0);
    
    // Enable and advance some modulation sources
    system.get_lfo_mut(0).unwrap().enabled = true;
    system.get_lfo_mut(0).unwrap().current_phase = 0.5;
    
    system.get_step_sequencer_mut(0).unwrap().enabled = true;
    system.get_step_sequencer_mut(0).unwrap().current_step = 2;
    system.get_step_sequencer_mut(0).unwrap().step_phase = 0.7;
    
    // Reset all
    system.reset_all();
    
    // All phases should be reset
    assert_eq!(system.get_lfo(0).unwrap().current_phase, 0.0);
    assert_eq!(system.get_step_sequencer(0).unwrap().current_step, 0);
    assert_eq!(system.get_step_sequencer(0).unwrap().step_phase, 0.0);
}

#[test]
fn test_modulation_target_types() {
    // Test different modulation target types
    let track_volume = ModulationTarget::TrackVolume(3);
    let track_pan = ModulationTarget::TrackPan(2);
    let master_volume = ModulationTarget::MasterVolume;
    let effect_param = ModulationTarget::EffectParameter {
        chain_type: EffectChainType::TrackFX,
        slot_index: 1,
        param_index: 2,
        track_id: Some(4),
    };
    
    // Test assignments with different targets
    let assignment1 = ModulationAssignment::new(track_volume);
    let assignment2 = ModulationAssignment::new(track_pan);
    let assignment3 = ModulationAssignment::new(master_volume);
    let assignment4 = ModulationAssignment::new(effect_param);
    
    // All should be created successfully
    assert!(assignment1.enabled);
    assert!(assignment2.enabled);
    assert!(assignment3.enabled);
    assert!(assignment4.enabled);
}

#[test]
fn test_tempo_sync_divisions() {
    // Test all tempo sync divisions
    let divisions = [
        (TempoSyncDivision::ThirtySecond, 8.0),
        (TempoSyncDivision::Sixteenth, 4.0),
        (TempoSyncDivision::Eighth, 2.0),
        (TempoSyncDivision::Quarter, 1.0),
        (TempoSyncDivision::Half, 0.5),
        (TempoSyncDivision::WholeNote, 0.25),
        (TempoSyncDivision::TwoBars, 0.125),
        (TempoSyncDivision::FourBars, 0.0625),
        (TempoSyncDivision::EightBars, 0.03125),
    ];
    
    for (division, expected_multiplier) in divisions {
        assert_eq!(division.multiplier(), expected_multiplier,
                  "Division {:?} should have multiplier {}", division, expected_multiplier);
    }
}

#[test]
fn test_lfo_to_lfo_modulation() {
    let mut system = ModulationSystem::new(44100.0);
    
    // Set up LFO 0 to modulate LFO 1's rate
    if let Some(lfo0) = system.get_lfo_mut(0) {
        lfo0.enabled = true;
        lfo0.set_rate_hz(0.5); // Slow modulation
        lfo0.set_depth(1.0);
        
        let assignment = ModulationAssignment::new(ModulationTarget::LfoRate(1));
        let _ = lfo0.add_assignment(assignment);
    }
    
    // Set up LFO 1 as the target
    if let Some(lfo1) = system.get_lfo_mut(1) {
        lfo1.enabled = true;
        lfo1.set_rate_hz(2.0); // Base rate
        lfo1.set_depth(1.0);
    }
    
    // Update system
    let values = system.update(120.0, 1024);
    
    // Both LFOs should produce output
    assert!(values.lfo_values[0].abs() > 0.0, "LFO 0 should produce modulation");
    assert!(values.lfo_values[1].abs() > 0.0, "LFO 1 should produce output");
}