//! Test example for the enhanced undo/redo system
//! 
//! This example demonstrates the comprehensive undo/redo functionality
//! for track operations and effect parameter changes.

use loopstation_core_stm32::{
    LoopstationCore, 
    settings::UndoMode,
    effects::{EffectChainType, EffectType, Effect},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Enhanced Undo/Redo System");
    println!("=================================");

    // Create loopstation core
    let mut loopstation = LoopstationCore::new();
    
    // Set undo mode to All (most comprehensive)
    loopstation.set_undo_mode(UndoMode::All);
    println!("Undo mode set to: {:?}", loopstation.get_undo_mode());

    // Test track operations undo/redo
    test_track_operations(&mut loopstation)?;
    
    // Test effect parameter undo/redo
    test_effect_parameters(&mut loopstation)?;
    
    println!("\n✅ All undo/redo tests passed!");
    Ok(())
}

fn test_track_operations(loopstation: &mut LoopstationCore) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎵 Testing Track Operations Undo/Redo");
    
    let track_id = 1;
    
    // Initial state
    println!("Initial undo count: {}", loopstation.get_undo_count());
    assert_eq!(loopstation.get_undo_count(), 0);
    
    // Test track level changes
    println!("Setting track level to 0.8...");
    loopstation.set_track_level(track_id, 0.8)?;
    
    println!("Undo count after level change: {}", loopstation.get_undo_count());
    assert_eq!(loopstation.get_undo_count(), 1);
    
    // Verify level was set
    if let Some(track) = loopstation.audio_engine().get_track(track_id) {
        assert!((track.level - 0.8).abs() < 0.001);
        println!("✓ Track level set correctly: {}", track.level);
    }
    
    // Test undo
    println!("Undoing track level change...");
    let undo_success = loopstation.audio_engine_mut().undo_track_action(track_id, loopstation.get_undo_mode());
    assert!(undo_success);
    
    // Verify level was restored
    if let Some(track) = loopstation.audio_engine().get_track(track_id) {
        assert!((track.level - 1.0).abs() < 0.001); // Should be back to default 1.0
        println!("✓ Track level undone correctly: {}", track.level);
    }
    
    // Test redo
    println!("Redoing track level change...");
    let redo_success = loopstation.audio_engine_mut().redo_track_action(track_id, loopstation.get_undo_mode());
    assert!(redo_success);
    
    // Verify level was restored to 0.8
    if let Some(track) = loopstation.audio_engine().get_track(track_id) {
        assert!((track.level - 0.8).abs() < 0.001);
        println!("✓ Track level redone correctly: {}", track.level);
    }
    
    // Test track mute
    println!("Testing track mute undo/redo...");
    loopstation.toggle_mute(track_id)?;
    
    if let Some(track) = loopstation.audio_engine().get_track(track_id) {
        println!("✓ Track muted, state: {:?}", track.state);
    }
    
    // Undo mute
    let undo_success = loopstation.audio_engine_mut().undo_track_action(track_id, loopstation.get_undo_mode());
    assert!(undo_success);
    
    if let Some(track) = loopstation.audio_engine().get_track(track_id) {
        println!("✓ Track mute undone, state: {:?}", track.state);
    }
    
    println!("✅ Track operations undo/redo tests passed!");
    Ok(())
}

fn test_effect_parameters(loopstation: &mut LoopstationCore) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎛️ Testing Effect Parameter Undo/Redo");
    
    // Add a compressor effect to Input FX
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    
    let input_fx = loopstation.input_fx_mut();
    input_fx.add_effect(compressor)?;
    
    println!("Added compressor to Input FX");
    
    // Test parameter change
    let slot_index = 0;
    let param_index = 0; // Threshold parameter
    let new_value = 0.3; // -20dB threshold
    
    println!("Setting compressor threshold to {}...", new_value);
    
    // Get initial parameter history count
    let initial_undo_count = input_fx.parameter_undo_count();
    println!("Initial parameter undo count: {}", initial_undo_count);
    
    // Set parameter (this should add to undo history)
    input_fx.set_effect_parameter(slot_index, param_index, new_value, 1000, None)?;
    
    // Check undo count increased
    let after_change_undo_count = input_fx.parameter_undo_count();
    println!("Parameter undo count after change: {}", after_change_undo_count);
    assert_eq!(after_change_undo_count, initial_undo_count + 1);
    
    // Verify parameter was set
    if let Some(effect) = input_fx.get_effect(slot_index) {
        if let Some(param) = effect.get_parameter(param_index) {
            assert!((param.value - new_value).abs() < 0.001);
            println!("✓ Parameter set correctly: {}", param.value);
        }
    }
    
    // Test undo
    println!("Undoing parameter change...");
    let undo_success = input_fx.undo_parameter_change();
    assert!(undo_success);
    
    // Verify parameter was restored
    if let Some(effect) = input_fx.get_effect(slot_index) {
        if let Some(param) = effect.get_parameter(param_index) {
            assert!((param.value - 0.5).abs() < 0.001); // Should be back to default 0.5
            println!("✓ Parameter undone correctly: {}", param.value);
        }
    }
    
    // Test redo
    println!("Redoing parameter change...");
    let redo_success = input_fx.redo_parameter_change();
    assert!(redo_success);
    
    // Verify parameter was restored to new_value
    if let Some(effect) = input_fx.get_effect(slot_index) {
        if let Some(param) = effect.get_parameter(param_index) {
            assert!((param.value - new_value).abs() < 0.001);
            println!("✓ Parameter redone correctly: {}", param.value);
        }
    }
    
    println!("✅ Effect parameter undo/redo tests passed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_modes() {
        let mut loopstation = LoopstationCore::new();
        
        // Test different undo modes
        loopstation.set_undo_mode(UndoMode::RecordOnly);
        assert_eq!(loopstation.get_undo_mode(), UndoMode::RecordOnly);
        
        loopstation.set_undo_mode(UndoMode::RecordAndPlay);
        assert_eq!(loopstation.get_undo_mode(), UndoMode::RecordAndPlay);
        
        loopstation.set_undo_mode(UndoMode::All);
        assert_eq!(loopstation.get_undo_mode(), UndoMode::All);
    }
    
    #[test]
    fn test_undo_buffer_limits() {
        let mut loopstation = LoopstationCore::new();
        loopstation.set_undo_mode(UndoMode::All);
        
        let track_id = 1;
        
        // Fill up the undo buffer
        for i in 0..20 { // More than the 16 buffer limit
            let level = 0.1 + (i as f32 * 0.01);
            let _ = loopstation.set_track_level(track_id, level);
        }
        
        // Should not exceed buffer limit
        let undo_count = loopstation.get_undo_count();
        assert!(undo_count <= 16);
        println!("Undo buffer correctly limited to {} entries", undo_count);
    }
    
    #[test]
    fn test_clear_undo_history() {
        let mut loopstation = LoopstationCore::new();
        loopstation.set_undo_mode(UndoMode::All);
        
        // Make some changes
        let _ = loopstation.set_track_level(1, 0.5);
        let _ = loopstation.set_track_level(2, 0.7);
        
        assert!(loopstation.get_undo_count() > 0);
        
        // Clear history
        loopstation.clear_undo_history();
        
        assert_eq!(loopstation.get_undo_count(), 0);
        assert_eq!(loopstation.get_redo_count(), 0);
        println!("✓ Undo history cleared successfully");
    }
}