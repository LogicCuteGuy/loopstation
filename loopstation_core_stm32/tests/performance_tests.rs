//! Performance module tests
//! 
//! Tests for performance profiling, optimization, and latency measurement.

use loopstation_core_stm32::*;

#[test]
fn test_performance_profiler_creation() {
    let profiler = PerformanceProfiler::new();
    
    assert_eq!(profiler.metrics.sample_rate, 44100);
    assert_eq!(profiler.metrics.buffer_size, 256);
    assert_eq!(profiler.metrics.cpu_usage, 0.0);
    assert_eq!(profiler.metrics.dropout_count, 0);
}

#[test]
fn test_performance_profiler_config_update() {
    let mut profiler = PerformanceProfiler::new();
    
    profiler.update_config(48000, 512);
    
    assert_eq!(profiler.metrics.sample_rate, 48000);
    assert_eq!(profiler.metrics.buffer_size, 512);
}

#[test]
fn test_performance_profiler_callback_timing() {
    let mut profiler = PerformanceProfiler::new();
    
    // Simulate callback timing
    profiler.start_callback_timing();
    // Simulate some processing time
    profiler.callback_start_time = 0; // Mock start time
    profiler.end_callback_timing();
    
    // Should have recorded timing data
    assert!(profiler.callback_times.len() > 0);
}

#[test]
fn test_performance_profiler_latency_update() {
    let mut profiler = PerformanceProfiler::new();
    
    profiler.update_latency(256);
    
    assert_eq!(profiler.metrics.audio_latency_samples, 256);
    // At 44.1kHz, 256 samples = ~5.8ms
    assert!((profiler.metrics.audio_latency_ms - 5.8).abs() < 0.1);
}

#[test]
fn test_performance_profiler_active_components() {
    let mut profiler = PerformanceProfiler::new();
    
    profiler.update_active_components(6, 12);
    
    assert_eq!(profiler.metrics.active_tracks, 6);
    assert_eq!(profiler.metrics.active_effects, 12);
}

#[test]
fn test_performance_requirements_check() {
    let mut profiler = PerformanceProfiler::new();
    
    // Test good performance
    profiler.metrics.audio_latency_ms = 3.0;
    profiler.metrics.cpu_usage = 50.0;
    profiler.metrics.dropout_count = 0;
    
    let status = profiler.check_performance_requirements();
    assert_eq!(status, PerformanceStatus::Good);
    
    // Test warning conditions
    profiler.metrics.cpu_usage = 70.0;
    let status = profiler.check_performance_requirements();
    assert_eq!(status, PerformanceStatus::Warning);
    
    // Test critical conditions
    profiler.metrics.dropout_count = 1;
    let status = profiler.check_performance_requirements();
    assert_eq!(status, PerformanceStatus::Critical);
}

#[test]
fn test_performance_optimization_recommendations() {
    let mut profiler = PerformanceProfiler::new();
    
    // Set conditions that should trigger recommendations
    profiler.metrics.cpu_usage = 75.0;
    profiler.metrics.audio_latency_ms = 4.0;
    profiler.optimizations.effect_chain_opt = false;
    
    let recommendations = profiler.get_optimization_recommendations();
    
    assert!(recommendations.len() > 0);
    // Should recommend reducing effects and enabling optimizations
    assert!(recommendations.iter().any(|&rec| rec.contains("effects")));
}

#[test]
fn test_audio_optimizer_buffer_processing() {
    let input = [1.0, 2.0, 3.0, 4.0];
    let mut output = [0.0; 4];
    let gain = 0.5;
    
    AudioOptimizer::process_buffer_simd(&input, &mut output, gain);
    
    assert_eq!(output, [0.5, 1.0, 1.5, 2.0]);
}

#[test]
fn test_audio_optimizer_buffer_mixing() {
    let input1 = [1.0, 2.0, 3.0, 4.0];
    let input2 = [0.5, 1.0, 1.5, 2.0];
    let inputs = [&input1[..], &input2[..]];
    let gains = [1.0, 0.5];
    let mut output = [0.0; 4];
    
    AudioOptimizer::mix_buffers_optimized(&inputs, &mut output, &gains);
    
    // Expected: input1 * 1.0 + input2 * 0.5
    assert_eq!(output, [1.25, 2.5, 3.75, 5.0]);
}

#[test]
fn test_audio_optimizer_fast_math() {
    // Test fast sine approximation
    let result = AudioOptimizer::fast_sin(0.0);
    assert!((result - 0.0).abs() < 0.1);
    
    let result = AudioOptimizer::fast_sin(std::f32::consts::PI / 2.0);
    assert!((result - 1.0).abs() < 0.1);
    
    // Test fast exponential decay
    let result = AudioOptimizer::fast_exp_decay(0.0);
    assert!((result - 1.0).abs() < 0.1);
    
    let result = AudioOptimizer::fast_exp_decay(5.0);
    assert!(result < 1.0);
}

#[test]
fn test_audio_optimizer_effect_cost_estimation() {
    // Test that different effects have appropriate cost estimates
    let mixer_cost = AudioOptimizer::estimate_effect_cost(&EffectType::Mixer);
    let reverb_cost = AudioOptimizer::estimate_effect_cost(&EffectType::SpaceReverb);
    let pitch_cost = AudioOptimizer::estimate_effect_cost(&EffectType::PitchShift);
    
    // Reverb should be more expensive than mixer
    assert!(reverb_cost > mixer_cost);
    
    // Pitch shift should be most expensive
    assert!(pitch_cost >= reverb_cost);
}

#[test]
fn test_audio_memory_pool() {
    let mut pool = AudioMemoryPool::new();
    
    // Test initial state
    assert_eq!(pool.utilization(), 0.0);
    
    // Allocate some buffers
    let _buffer1 = pool.allocate();
    let _buffer2 = pool.allocate();
    
    // Utilization should increase
    assert!(pool.utilization() > 0.0);
    assert!(pool.utilization() <= 1.0);
}

#[test]
fn test_latency_measurement() {
    let mut latency = LatencyMeasurement::new();
    
    latency.measure_impulse_latency(44100, 256);
    
    // Should have measured some latency components
    assert!(latency.total_latency > 0);
    assert!(latency.adc_latency > 0);
    assert!(latency.dac_latency > 0);
    assert!(latency.buffer_latency > 0);
    
    // Total should be sum of components
    let expected_total = latency.adc_latency + latency.dac_latency + 
                        latency.buffer_latency + latency.processing_latency;
    assert_eq!(latency.total_latency, expected_total);
}

#[test]
fn test_latency_measurement_requirements() {
    let mut latency = LatencyMeasurement::new();
    
    // Test with small buffer (should meet requirements)
    latency.measure_impulse_latency(44100, 128);
    assert!(latency.meets_requirements(44100));
    
    // Test with large buffer (might not meet requirements)
    latency.measure_impulse_latency(44100, 2048);
    let latency_ms = latency.total_latency_ms(44100);
    
    if latency_ms >= 5.0 {
        assert!(!latency.meets_requirements(44100));
    }
}

#[test]
fn test_performance_test_suite() {
    let mut profiler = PerformanceProfiler::new();
    
    // Set up good performance metrics
    profiler.metrics.audio_latency_ms = 3.0;
    profiler.metrics.cpu_usage = 50.0;
    profiler.metrics.dropout_count = 0;
    profiler.metrics.max_callback_time_us = 1000;
    profiler.update_config(44100, 256);
    
    let mut test_suite = PerformanceTestSuite::new();
    test_suite.run_all_tests(&profiler);
    
    // Should have run multiple tests
    assert!(test_suite.results.len() >= 4);
    
    // All tests should pass with good metrics
    assert!(test_suite.overall_status());
    
    // No failed tests
    let failed = test_suite.failed_tests();
    assert_eq!(failed.len(), 0);
}

#[test]
fn test_performance_test_suite_failures() {
    let mut profiler = PerformanceProfiler::new();
    
    // Set up poor performance metrics
    profiler.metrics.audio_latency_ms = 8.0; // Exceeds 5ms requirement
    profiler.metrics.cpu_usage = 90.0; // Exceeds 80% requirement
    profiler.metrics.dropout_count = 5; // Has dropouts
    profiler.update_config(44100, 256);
    
    let mut test_suite = PerformanceTestSuite::new();
    test_suite.run_all_tests(&profiler);
    
    // Should have failures
    assert!(!test_suite.overall_status());
    
    // Should have failed tests
    let failed = test_suite.failed_tests();
    assert!(failed.len() > 0);
}

#[test]
fn test_loopstation_core_performance_integration() {
    let mut loopstation = LoopstationCore::new();
    
    // Test initial performance state
    let metrics = loopstation.get_performance_metrics();
    assert_eq!(metrics.sample_rate, 44100);
    assert_eq!(metrics.buffer_size, 256);
    
    // Test performance status check
    let status = loopstation.check_performance_requirements();
    assert_eq!(status, PerformanceStatus::Good);
    
    // Test latency requirements
    assert!(loopstation.latency_meets_requirements());
    
    // Test memory pool utilization
    let utilization = loopstation.get_memory_pool_utilization();
    assert_eq!(utilization, 0.0); // Should be empty initially
}

#[test]
fn test_loopstation_core_performance_under_load() {
    let mut loopstation = LoopstationCore::new();
    
    // Add some load to the system
    for track_id in 1..=3 {
        let _ = loopstation.start_recording(track_id);
    }
    
    // Add some effects
    let mut effect = Effect::new(EffectType::Compressor);
    effect.set_enabled(true);
    let _ = loopstation.input_fx_mut().add_effect(effect);
    
    // Process some audio to generate performance data
    let input = [0.1f32; 256];
    let mut output = [0.0f32; 256];
    
    for _ in 0..10 {
        loopstation.process_audio(&input, &mut output);
    }
    
    // Check that performance metrics are updated
    let metrics = loopstation.get_performance_metrics();
    assert!(metrics.active_tracks > 0);
    assert!(metrics.active_effects > 0);
}

#[test]
fn test_optimization_flags() {
    let mut loopstation = LoopstationCore::new();
    
    // Test setting optimization flags
    let mut flags = OptimizationFlags::default();
    flags.simd_enabled = false;
    flags.fast_math = false;
    
    loopstation.set_optimization_flags(flags);
    
    // Flags should be applied
    assert!(!loopstation.performance_profiler.optimizations.simd_enabled);
    assert!(!loopstation.performance_profiler.optimizations.fast_math);
}

#[test]
fn test_performance_config_update() {
    let mut loopstation = LoopstationCore::new();
    
    // Update performance configuration
    loopstation.update_performance_config(48000, 512);
    
    let metrics = loopstation.get_performance_metrics();
    assert_eq!(metrics.sample_rate, 48000);
    assert_eq!(metrics.buffer_size, 512);
    
    // Latency should be recalculated
    let latency_ms = loopstation.get_total_latency_ms();
    assert!(latency_ms > 0.0);
}