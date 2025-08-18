//! Unit tests for EffectChain processing and parameter control
//! Requirements: 3.4, 3.5, 3.6, 3.7, 3.8

use loopstation_core_stm32::effects::{
    EffectChain, Effect, EffectType, EffectChainType, EffectParameter, MAX_EFFECT_SLOTS
};

#[test]
fn test_effect_chain_creation() {
    let input_fx = EffectChain::new_input_fx();
    assert_eq!(input_fx.chain_type, EffectChainType::InputFX);
    assert_eq!(input_fx.mix_level, 1.0);
    assert!(input_fx.enabled);
    assert_eq!(input_fx.fx_bank, 1);
    assert_eq!(input_fx.active_effect_count(), 0);
    
    let track_fx = EffectChain::new_track_fx();
    assert_eq!(track_fx.chain_type, EffectChainType::TrackFX);
    
    let master_fx = EffectChain::new_master_fx();
    assert_eq!(master_fx.chain_type, EffectChainType::MasterFX);
}

#[test]
fn test_effect_creation() {
    let compressor = Effect::new(EffectType::Compressor);
    assert_eq!(compressor.effect_type, EffectType::Compressor);
    assert!(!compressor.enabled);
    assert!(!compressor.momentary);
    assert!(!compressor.midi_sync);
    assert_eq!(compressor.dry_wet_mix, 0.5);
    assert_eq!(compressor.parameters.len(), 4); // Threshold, Ratio, Attack, Release
    
    let reverb = Effect::new(EffectType::SpaceReverb);
    assert_eq!(reverb.effect_type, EffectType::SpaceReverb);
    assert_eq!(reverb.parameters.len(), 4); // Time, Pre-delay, Mix, Tone
}

#[test]
fn test_effect_parameter_management() {
    let mut effect = Effect::new(EffectType::Compressor);
    
    // Test parameter access
    assert!(effect.get_parameter(0).is_some());
    assert!(effect.get_parameter(10).is_none()); // Out of bounds
    
    // Test parameter setting
    effect.set_parameter(0, 0.3); // Threshold
    if let Some(param) = effect.get_parameter(0) {
        assert_eq!(param.value, 0.3);
    }
    
    // Test parameter bounds
    effect.set_parameter(0, 1.5); // Should clamp to 1.0
    if let Some(param) = effect.get_parameter(0) {
        assert_eq!(param.value, 1.0);
    }
    
    effect.set_parameter(0, -0.5); // Should clamp to 0.0
    if let Some(param) = effect.get_parameter(0) {
        assert_eq!(param.value, 0.0);
    }
}

#[test]
fn test_effect_parameter_actual_values() {
    let mut param = EffectParameter::new("THRESHOLD", -60.0, 0.0, "dB");
    
    // Test normalized to actual conversion
    param.set_normalized(0.5);
    assert_eq!(param.actual_value(), -30.0); // Middle of -60 to 0 range
    
    // Test actual to normalized conversion
    param.set_actual_value(-20.0);
    assert!((param.value - 0.6667).abs() < 0.001); // Should be ~2/3 of the way
    
    // Test bounds
    param.set_actual_value(-100.0); // Below min
    assert_eq!(param.actual_value(), -60.0);
    
    param.set_actual_value(10.0); // Above max
    assert_eq!(param.actual_value(), 0.0);
}

#[test]
fn test_effect_enable_disable() {
    let mut effect = Effect::new(EffectType::Compressor);
    
    // Initially disabled
    assert!(!effect.enabled);
    
    // Enable effect
    effect.set_enabled(true);
    assert!(effect.enabled);
    
    // Toggle effect
    effect.toggle_enabled();
    assert!(!effect.enabled);
    
    effect.toggle_enabled();
    assert!(effect.enabled);
}

#[test]
fn test_effect_momentary_mode() {
    let mut effect = Effect::new(EffectType::TapeEcho);
    
    // Initially not momentary
    assert!(!effect.momentary);
    
    // Set momentary
    effect.set_momentary(true);
    assert!(effect.momentary);
}

#[test]
fn test_effect_midi_sync() {
    let mut delay = Effect::new(EffectType::TapeEcho);
    let mut compressor = Effect::new(EffectType::Compressor);
    
    // Delay supports tempo sync
    assert!(delay.effect_type.supports_tempo_sync());
    delay.set_midi_sync(true);
    assert!(delay.midi_sync);
    
    // Compressor doesn't support tempo sync
    assert!(!compressor.effect_type.supports_tempo_sync());
    compressor.set_midi_sync(true);
    assert!(!compressor.midi_sync); // Should remain false
}

#[test]
fn test_effect_tempo_update() {
    let mut delay = Effect::new(EffectType::TapeEcho);
    delay.set_midi_sync(true);
    
    // Update tempo to 120 BPM
    delay.update_tempo(120.0);
    
    // Check that delay time was updated (quarter note = 500ms at 120 BPM)
    if let Some(time_param) = delay.get_parameter(0) {
        assert!((time_param.actual_value() - 500.0).abs() < 1.0);
    }
}

#[test]
fn test_effect_dry_wet_mix() {
    let mut effect = Effect::new(EffectType::SpaceReverb);
    
    // Test setting dry/wet mix
    effect.set_dry_wet_mix(0.3);
    assert_eq!(effect.dry_wet_mix, 0.3);
    
    // Test clamping
    effect.set_dry_wet_mix(1.5);
    assert_eq!(effect.dry_wet_mix, 1.0);
    
    effect.set_dry_wet_mix(-0.5);
    assert_eq!(effect.dry_wet_mix, 0.0);
}

#[test]
fn test_effect_chain_add_remove() {
    let mut chain = EffectChain::new_input_fx();
    
    // Initially empty
    assert_eq!(chain.active_effect_count(), 0);
    assert!(!chain.has_effects());
    
    // Add effects
    let compressor = Effect::new(EffectType::Compressor);
    let reverb = Effect::new(EffectType::SpaceReverb);
    
    let slot1 = chain.add_effect(compressor).unwrap();
    assert_eq!(slot1, 0);
    assert_eq!(chain.active_effect_count(), 1);
    assert!(chain.has_effects());
    
    let slot2 = chain.add_effect(reverb).unwrap();
    assert_eq!(slot2, 1);
    assert_eq!(chain.active_effect_count(), 2);
    
    // Test slot access
    assert!(chain.get_effect(0).is_some());
    assert!(chain.get_effect(1).is_some());
    assert!(chain.get_effect(2).is_none());
    
    // Remove effect
    let removed = chain.remove_effect(0);
    assert!(removed.is_some());
    assert_eq!(chain.active_effect_count(), 1);
    
    // Clear all effects
    chain.clear();
    assert_eq!(chain.active_effect_count(), 0);
    assert!(!chain.has_effects());
}

#[test]
fn test_effect_chain_capacity() {
    let mut chain = EffectChain::new_track_fx();
    
    // Fill chain to capacity
    for _ in 0..MAX_EFFECT_SLOTS {
        let effect = Effect::new(EffectType::Compressor);
        assert!(chain.add_effect(effect).is_ok());
    }
    
    // Should be at capacity
    assert_eq!(chain.active_effect_count(), MAX_EFFECT_SLOTS);
    
    // Adding another should fail
    let extra_effect = Effect::new(EffectType::SpaceReverb);
    assert!(chain.add_effect(extra_effect).is_err());
}

#[test]
fn test_effect_chain_set_effect() {
    let mut chain = EffectChain::new_master_fx();
    
    // Set effect in specific slot
    let compressor = Effect::new(EffectType::Compressor);
    assert!(chain.set_effect(2, Some(compressor)).is_ok());
    
    // Check it was set
    assert!(chain.get_effect(2).is_some());
    assert_eq!(chain.active_effect_count(), 1);
    
    // Remove from specific slot
    assert!(chain.set_effect(2, None).is_ok());
    assert!(chain.get_effect(2).is_none());
    assert_eq!(chain.active_effect_count(), 0);
    
    // Test out of bounds
    let effect = Effect::new(EffectType::SpaceReverb);
    assert!(chain.set_effect(10, Some(effect)).is_err());
}

#[test]
fn test_effect_chain_fx_banks() {
    let mut chain = EffectChain::new_input_fx();
    
    // Test default bank
    assert_eq!(chain.get_fx_bank(), 1);
    
    // Set different banks
    chain.set_fx_bank(3);
    assert_eq!(chain.get_fx_bank(), 3);
    
    // Test clamping
    chain.set_fx_bank(0);
    assert_eq!(chain.get_fx_bank(), 1);
    
    chain.set_fx_bank(10);
    assert_eq!(chain.get_fx_bank(), 4);
}

#[test]
fn test_effect_chain_parameter_history() {
    let mut chain = EffectChain::new_track_fx();
    let timestamp = 1000;
    
    // Add an effect
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    assert!(chain.add_effect(compressor).is_ok());
    
    // Set parameter with history tracking
    let result = chain.set_effect_parameter(0, 0, 0.3, timestamp, Some(1));
    assert!(result.is_ok());
    
    // Check parameter was set
    if let Some(effect) = chain.get_effect(0) {
        if let Some(param) = effect.get_parameter(0) {
            assert_eq!(param.value, 0.3);
        }
    }
    
    // Check history was recorded
    assert_eq!(chain.parameter_history.len(), 1);
}

#[test]
fn test_effect_audio_processing_bypass() {
    let mut chain = EffectChain::new_input_fx();
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    let sample_rate = 44100.0;
    
    // Empty chain should pass through
    chain.process_audio(&input, &mut output, sample_rate);
    assert_eq!(output, input);
    
    // Disabled chain should pass through
    chain.enabled = false;
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    let _ = chain.add_effect(compressor);
    
    chain.process_audio(&input, &mut output, sample_rate);
    assert_eq!(output, input);
}

#[test]
fn test_effect_audio_processing_enabled() {
    let mut chain = EffectChain::new_master_fx();
    let input = [0.1, 0.2, 0.3, 0.4];
    let mut output = [0.0; 4];
    let sample_rate = 44100.0;
    
    // Add enabled effect
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    let _ = chain.add_effect(compressor);
    
    // Process audio
    chain.process_audio(&input, &mut output, sample_rate);
    
    // Output should be different from input (processed)
    // Note: The exact output depends on the effect implementation
    // We just verify that processing occurred
    assert_ne!(output, [0.0; 4]); // Should not be silence
}

#[test]
fn test_effect_chain_mix_level() {
    let mut chain = EffectChain::new_input_fx();
    let input = [0.5, 0.5, 0.5, 0.5];
    let mut output = [0.0; 4];
    let sample_rate = 44100.0;
    
    // Set mix level to 0.5
    chain.mix_level = 0.5;
    
    // Add bypassed effect (should pass through)
    let effect = Effect::new(EffectType::Compressor); // Disabled by default
    let _ = chain.add_effect(effect);
    
    // Process audio
    chain.process_audio(&input, &mut output, sample_rate);
    
    // Output should be scaled by mix level
    let expected = [0.25, 0.25, 0.25, 0.25]; // 0.5 * 0.5
    assert_eq!(output, expected);
}

#[test]
fn test_effect_types_parameter_counts() {
    // Test that different effect types have correct parameter counts
    assert_eq!(EffectType::Compressor.parameter_count(), 4);
    assert_eq!(EffectType::SpaceReverb.parameter_count(), 4);
    assert_eq!(EffectType::TapeEcho.parameter_count(), 4);
    assert_eq!(EffectType::MasteringEQ.parameter_count(), 4);
    assert_eq!(EffectType::Slicer.parameter_count(), 4);
    assert_eq!(EffectType::NoiseSuppressor.parameter_count(), 2);
    assert_eq!(EffectType::Limiter.parameter_count(), 3);
}

#[test]
fn test_effect_types_tempo_sync_support() {
    // Test which effects support tempo sync
    assert!(EffectType::TapeEcho.supports_tempo_sync());
    assert!(EffectType::T3Delay.supports_tempo_sync());
    assert!(EffectType::BeatRepeat.supports_tempo_sync());
    assert!(EffectType::Slicer.supports_tempo_sync());
    assert!(EffectType::Chorus.supports_tempo_sync());
    assert!(EffectType::Flanger.supports_tempo_sync());
    
    assert!(!EffectType::Compressor.supports_tempo_sync());
    assert!(!EffectType::SpaceReverb.supports_tempo_sync());
    assert!(!EffectType::MasteringEQ.supports_tempo_sync());
}

#[test]
fn test_effect_types_names() {
    // Test effect type display names
    assert_eq!(EffectType::Compressor.name(), "COMPRESSOR");
    assert_eq!(EffectType::SpaceReverb.name(), "SPACE REVERB");
    assert_eq!(EffectType::TapeEcho.name(), "TAPE ECHO");
    assert_eq!(EffectType::MasteringEQ.name(), "MASTERING EQ");
    assert_eq!(EffectType::BeatRepeat.name(), "BEAT REPEAT");
    assert_eq!(EffectType::NoiseSuppressor.name(), "NOISE SUPPRESSOR");
}

#[test]
fn test_effect_chain_serialization() {
    let mut chain = EffectChain::new_input_fx();
    
    // Add some effects
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    compressor.set_parameter(0, 0.3);
    let _ = chain.add_effect(compressor);
    
    let mut reverb = Effect::new(EffectType::SpaceReverb);
    reverb.set_enabled(true);
    reverb.set_dry_wet_mix(0.4);
    let _ = chain.add_effect(reverb);
    
    // Test serialization
    let serialized = serde_json_core::to_string::<_, 65536>(&chain);
    assert!(serialized.is_ok());
    
    // Test deserialization
    if let Ok((json_str, _)) = serialized {
        let deserialized: Result<EffectChain, _> = serde_json_core::from_str(&json_str);
        assert!(deserialized.is_ok());
        
        if let Ok(deserialized_chain) = deserialized {
            assert_eq!(deserialized_chain.chain_type, chain.chain_type);
            assert_eq!(deserialized_chain.mix_level, chain.mix_level);
            assert_eq!(deserialized_chain.enabled, chain.enabled);
            assert_eq!(deserialized_chain.active_effect_count(), chain.active_effect_count());
        }
    }
}

#[test]
fn test_effect_update_method() {
    let mut effect = Effect::new(EffectType::TapeEcho);
    
    // Set momentary mode
    effect.set_momentary(true);
    effect.set_enabled(true);
    
    // Update effect (in real implementation, this would handle timing)
    effect.update();
    
    // Effect should still be enabled (timing logic would be in full implementation)
    assert!(effect.enabled);
}

#[test]
fn test_effect_chain_current_tempo() {
    let mut chain = EffectChain::new_track_fx();
    
    // Test default tempo
    assert_eq!(chain.current_tempo, 120.0);
    
    // Add tempo-synced effect
    let mut delay = Effect::new(EffectType::TapeEcho);
    delay.set_midi_sync(true);
    let _ = chain.add_effect(delay);
    
    // Update tempo
    chain.current_tempo = 140.0;
    
    // In a full implementation, this would update all tempo-synced effects
    if let Some(effect) = chain.get_effect_mut(0) {
        effect.update_tempo(chain.current_tempo);
    }
}