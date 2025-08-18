//! Performance optimization and latency test
//! 
//! This example demonstrates the performance profiling and optimization
//! capabilities of the loopstation system.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

use loopstation_core_stm32::*;

#[cfg(not(feature = "std"))]
use panic_halt as _;

#[cfg(not(feature = "std"))]
use cortex_m_rt::entry;

#[cfg(feature = "std")]
fn main() {
    run_performance_test();
}

#[cfg(not(feature = "std"))]
#[entry]
fn main() -> ! {
    run_performance_test();
    loop {}
}

fn run_performance_test() {
    println!("=== Loopstation Performance Optimization Test ===");
    
    // Create loopstation core
    let mut loopstation = LoopstationCore::new();
    
    // Test 1: Baseline performance measurement
    println!("\n1. Baseline Performance Measurement");
    test_baseline_performance(&mut loopstation);
    
    // Test 2: Latency measurement and validation
    println!("\n2. Latency Measurement and Validation");
    test_latency_requirements(&mut loopstation);
    
    // Test 3: Load testing with multiple tracks and effects
    println!("\n3. Load Testing with Multiple Tracks and Effects");
    test_system_load(&mut loopstation);
    
    // Test 4: Performance optimization testing
    println!("\n4. Performance Optimization Testing");
    test_performance_optimizations(&mut loopstation);
    
    // Test 5: Real-time performance validation
    println!("\n5. Real-time Performance Validation");
    test_realtime_performance(&mut loopstation);
    
    // Test 6: Memory pool efficiency
    println!("\n6. Memory Pool Efficiency Test");
    test_memory_pool_efficiency(&mut loopstation);
    
    println!("\n=== Performance Test Complete ===");
}

fn test_baseline_performance(loopstation: &mut LoopstationCore) {
    println!("Testing baseline system performance...");
    
    // Reset performance counters
    loopstation.reset_performance_counters();
    
    // Simulate audio processing for baseline measurement
    let input = [0.0f32; 256];
    let mut output = [0.0f32; 256];
    
    // Process several audio buffers to get stable measurements
    for _ in 0..100 {
        loopstation.process_audio(&input, &mut output);
    }
    
    let metrics = loopstation.get_performance_metrics();
    println!("  Sample Rate: {} Hz", metrics.sample_rate);
    println!("  Buffer Size: {} samples", metrics.buffer_size);
    println!("  Audio Latency: {:.2} ms", metrics.audio_latency_ms);
    println!("  CPU Usage: {:.1}%", metrics.cpu_usage);
    println!("  Max Callback Time: {} μs", metrics.max_callback_time_us);
    println!("  Avg Callback Time: {} μs", metrics.avg_callback_time_us);
    println!("  Dropout Count: {}", metrics.dropout_count);
    
    // Check performance status
    let status = loopstation.check_performance_requirements();
    println!("  Performance Status: {:?}", status);
    
    if status != PerformanceStatus::Good {
        let recommendations = loopstation.get_optimization_recommendations();
        println!("  Recommendations:");
        for rec in &recommendations {
            println!("    - {}", rec);
        }
    }
}

fn test_latency_requirements(loopstation: &mut LoopstationCore) {
    println!("Testing latency requirements (<5ms)...");
    
    let total_latency = loopstation.get_total_latency_ms();
    let meets_requirements = loopstation.latency_meets_requirements();
    
    println!("  Total System Latency: {:.2} ms", total_latency);
    println!("  Meets <5ms Requirement: {}", meets_requirements);
    
    if !meets_requirements {
        println!("  ❌ LATENCY REQUIREMENT FAILED");
        println!("  Consider reducing buffer size or optimizing processing");
    } else {
        println!("  ✅ Latency requirement met");
    }
    
    // Test different buffer sizes
    println!("  Testing different buffer sizes:");
    for buffer_size in [128, 256, 512, 1024] {
        loopstation.update_performance_config(44100, buffer_size);
        let latency = loopstation.get_total_latency_ms();
        let meets_req = loopstation.latency_meets_requirements();
        println!("    Buffer {}: {:.2} ms ({})", 
                buffer_size, latency, if meets_req { "✅" } else { "❌" });
    }
    
    // Restore original configuration
    loopstation.update_performance_config(44100, 256);
}

fn test_system_load(loopstation: &mut LoopstationCore) {
    println!("Testing system load with multiple tracks and effects...");
    
    // Start recording on multiple tracks
    for track_id in 1..=6 {
        let _ = loopstation.start_recording(track_id);
        println!("  Started recording on track {}", track_id);
    }
    
    // Add effects to Input FX chain
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    let _ = loopstation.input_fx_mut().add_effect(compressor);
    
    let mut reverb = Effect::new(EffectType::SpaceReverb);
    reverb.set_enabled(true);
    let _ = loopstation.input_fx_mut().add_effect(reverb);
    
    // Add effects to Master FX chain
    let mut eq = Effect::new(EffectType::MasteringEQ);
    eq.set_enabled(true);
    let _ = loopstation.master_fx_mut().add_effect(eq);
    
    let mut limiter = Effect::new(EffectType::Limiter);
    limiter.set_enabled(true);
    let _ = loopstation.master_fx_mut().add_effect(limiter);
    
    // Add effects to each track
    for track_id in 1..=6 {
        if let Some(track_fx) = loopstation.track_fx_mut(track_id) {
            let mut delay = Effect::new(EffectType::TapeEcho);
            delay.set_enabled(true);
            let _ = track_fx.add_effect(delay);
        }
    }
    
    println!("  Added effects to all chains");
    
    // Reset performance counters and test under load
    loopstation.reset_performance_counters();
    
    let input = [0.1f32; 256]; // Non-zero input to trigger processing
    let mut output = [0.0f32; 256];
    
    // Process audio under full load
    for _ in 0..200 {
        loopstation.process_audio(&input, &mut output);
    }
    
    let metrics = loopstation.get_performance_metrics();
    println!("  Load Test Results:");
    println!("    Active Tracks: {}", metrics.active_tracks);
    println!("    Active Effects: {}", metrics.active_effects);
    println!("    CPU Usage: {:.1}%", metrics.cpu_usage);
    println!("    Max Callback Time: {} μs", metrics.max_callback_time_us);
    println!("    Dropout Count: {}", metrics.dropout_count);
    
    let status = loopstation.check_performance_requirements();
    match status {
        PerformanceStatus::Good => println!("    ✅ System handles full load well"),
        PerformanceStatus::Warning => println!("    ⚠️  System under stress but functional"),
        PerformanceStatus::Critical => println!("    ❌ System overloaded - dropouts detected"),
    }
}

fn test_performance_optimizations(loopstation: &mut LoopstationCore) {
    println!("Testing performance optimizations...");
    
    // Test without optimizations
    let mut flags = OptimizationFlags::default();
    flags.simd_enabled = false;
    flags.fast_math = false;
    flags.effect_chain_opt = false;
    flags.track_processing_opt = false;
    loopstation.set_optimization_flags(flags);
    
    loopstation.reset_performance_counters();
    
    let input = [0.1f32; 256];
    let mut output = [0.0f32; 256];
    
    for _ in 0..100 {
        loopstation.process_audio(&input, &mut output);
    }
    
    let unoptimized_cpu = loopstation.get_performance_metrics().cpu_usage;
    let unoptimized_time = loopstation.get_performance_metrics().avg_callback_time_us;
    
    println!("  Without optimizations:");
    println!("    CPU Usage: {:.1}%", unoptimized_cpu);
    println!("    Avg Callback Time: {} μs", unoptimized_time);
    
    // Test with all optimizations enabled
    let optimized_flags = OptimizationFlags::default(); // All optimizations enabled
    loopstation.set_optimization_flags(optimized_flags);
    
    loopstation.reset_performance_counters();
    
    for _ in 0..100 {
        loopstation.process_audio(&input, &mut output);
    }
    
    let optimized_cpu = loopstation.get_performance_metrics().cpu_usage;
    let optimized_time = loopstation.get_performance_metrics().avg_callback_time_us;
    
    println!("  With optimizations:");
    println!("    CPU Usage: {:.1}%", optimized_cpu);
    println!("    Avg Callback Time: {} μs", optimized_time);
    
    let cpu_improvement = ((unoptimized_cpu - optimized_cpu) / unoptimized_cpu) * 100.0;
    let time_improvement = ((unoptimized_time as f32 - optimized_time as f32) / unoptimized_time as f32) * 100.0;
    
    println!("  Performance Improvement:");
    println!("    CPU Usage: {:.1}% reduction", cpu_improvement);
    println!("    Callback Time: {:.1}% reduction", time_improvement);
    
    if cpu_improvement > 5.0 {
        println!("    ✅ Significant performance improvement achieved");
    } else {
        println!("    ⚠️  Minimal performance improvement");
    }
}

fn test_realtime_performance(loopstation: &mut LoopstationCore) {
    println!("Testing real-time performance validation...");
    
    // Run comprehensive performance test suite
    let test_suite = loopstation.run_performance_tests();
    
    println!("  Performance Test Results:");
    for result in &test_suite.results {
        let status = if result.passed { "✅" } else { "❌" };
        println!("    {} {}: {:.2} {} (required: {:.2} {})", 
                status, result.test_name, result.measured_value, 
                result.units, result.required_value, result.units);
    }
    
    let overall_pass = test_suite.overall_status();
    println!("  Overall Status: {}", if overall_pass { "✅ PASS" } else { "❌ FAIL" });
    
    if !overall_pass {
        let failed_tests = test_suite.failed_tests();
        println!("  Failed Tests:");
        for test in &failed_tests {
            println!("    - {}: {:.2} {} (required: {:.2} {})", 
                    test.test_name, test.measured_value, test.units,
                    test.required_value, test.units);
        }
    }
}

fn test_memory_pool_efficiency(loopstation: &mut LoopstationCore) {
    println!("Testing memory pool efficiency...");
    
    let initial_utilization = loopstation.get_memory_pool_utilization();
    println!("  Initial pool utilization: {:.1}%", initial_utilization * 100.0);
    
    // Simulate heavy memory usage
    let input = [0.1f32; 256];
    let mut output = [0.0f32; 256];
    
    // Process many buffers to stress memory allocation
    for i in 0..1000 {
        loopstation.process_audio(&input, &mut output);
        
        if i % 100 == 0 {
            let utilization = loopstation.get_memory_pool_utilization();
            println!("    After {} buffers: {:.1}% utilization", 
                    i, utilization * 100.0);
        }
    }
    
    let final_utilization = loopstation.get_memory_pool_utilization();
    println!("  Final pool utilization: {:.1}%", final_utilization * 100.0);
    
    if final_utilization < 0.8 {
        println!("  ✅ Memory pool operating efficiently");
    } else {
        println!("  ⚠️  Memory pool under stress");
    }
}

// Helper function for printing (no-op in no_std)
#[cfg(not(feature = "std"))]
macro_rules! println {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "std")]
use std::println;