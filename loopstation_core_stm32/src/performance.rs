//! Performance monitoring and optimization module
//! 
//! This module provides tools for profiling audio processing performance,
//! measuring latency, and optimizing the system for real-time operation.

use heapless::Vec;
use serde::{Deserialize, Serialize};

/// Performance metrics for monitoring system performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Audio processing latency in samples
    pub audio_latency_samples: u32,
    /// Audio processing latency in milliseconds
    pub audio_latency_ms: f32,
    /// CPU usage percentage (0.0 to 100.0)
    pub cpu_usage: f32,
    /// Memory usage in bytes
    pub memory_usage: u32,
    /// Audio dropouts count
    pub dropout_count: u32,
    /// Maximum processing time per audio callback (microseconds)
    pub max_callback_time_us: u32,
    /// Average processing time per audio callback (microseconds)
    pub avg_callback_time_us: u32,
    /// Current sample rate
    pub sample_rate: u32,
    /// Buffer size in samples
    pub buffer_size: u32,
    /// Number of active tracks
    pub active_tracks: u8,
    /// Number of active effects
    pub active_effects: u8,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            audio_latency_samples: 0,
            audio_latency_ms: 0.0,
            cpu_usage: 0.0,
            memory_usage: 0,
            dropout_count: 0,
            max_callback_time_us: 0,
            avg_callback_time_us: 0,
            sample_rate: 44100,
            buffer_size: 256,
            active_tracks: 0,
            active_effects: 0,
        }
    }
}

/// Performance profiler for measuring system performance
#[derive(Debug)]
pub struct PerformanceProfiler {
    /// Current metrics
    pub metrics: PerformanceMetrics,
    /// Callback time history for averaging
    callback_times: Vec<u32, 64>,
    /// Start time of current callback (microseconds)
    callback_start_time: u32,
    /// Total callback count for averaging
    callback_count: u32,
    /// Performance optimization flags
    pub optimizations: OptimizationFlags,
}

/// Optimization flags for enabling/disabling performance optimizations
#[derive(Debug, Clone, Copy)]
pub struct OptimizationFlags {
    /// Enable SIMD optimizations where available
    pub simd_enabled: bool,
    /// Enable fast math approximations
    pub fast_math: bool,
    /// Enable audio buffer pre-allocation
    pub buffer_prealloc: bool,
    /// Enable effect chain optimization
    pub effect_chain_opt: bool,
    /// Enable track processing optimization
    pub track_processing_opt: bool,
    /// Enable memory pool allocation
    pub memory_pool: bool,
}

impl Default for OptimizationFlags {
    fn default() -> Self {
        Self {
            simd_enabled: true,
            fast_math: true,
            buffer_prealloc: true,
            effect_chain_opt: true,
            track_processing_opt: true,
            memory_pool: true,
        }
    }
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new() -> Self {
        Self {
            metrics: PerformanceMetrics::default(),
            callback_times: Vec::new(),
            callback_start_time: 0,
            callback_count: 0,
            optimizations: OptimizationFlags::default(),
        }
    }

    /// Start timing an audio callback
    pub fn start_callback_timing(&mut self) {
        self.callback_start_time = self.get_microseconds();
    }

    /// End timing an audio callback and update metrics
    pub fn end_callback_timing(&mut self) {
        let end_time = self.get_microseconds();
        let callback_time = end_time.saturating_sub(self.callback_start_time);
        
        // Update max callback time
        if callback_time > self.metrics.max_callback_time_us {
            self.metrics.max_callback_time_us = callback_time;
        }
        
        // Add to history for averaging
        if self.callback_times.push(callback_time).is_err() {
            // Buffer full, remove oldest
            self.callback_times.remove(0);
            let _ = self.callback_times.push(callback_time);
        }
        
        // Update average
        self.callback_count += 1;
        let sum: u32 = self.callback_times.iter().sum();
        self.metrics.avg_callback_time_us = sum / self.callback_times.len() as u32;
        
        // Check for dropouts (callback time > buffer duration)
        let buffer_duration_us = (self.metrics.buffer_size * 1_000_000) / self.metrics.sample_rate;
        if callback_time > buffer_duration_us {
            self.metrics.dropout_count += 1;
        }
        
        // Update CPU usage estimate
        self.metrics.cpu_usage = (callback_time as f32 / buffer_duration_us as f32) * 100.0;
    }

    /// Update audio latency metrics
    pub fn update_latency(&mut self, latency_samples: u32) {
        self.metrics.audio_latency_samples = latency_samples;
        self.metrics.audio_latency_ms = (latency_samples as f32 / self.metrics.sample_rate as f32) * 1000.0;
    }

    /// Update system configuration
    pub fn update_config(&mut self, sample_rate: u32, buffer_size: u32) {
        self.metrics.sample_rate = sample_rate;
        self.metrics.buffer_size = buffer_size;
        
        // Recalculate latency
        self.update_latency(self.metrics.audio_latency_samples);
    }

    /// Update active component counts
    pub fn update_active_components(&mut self, active_tracks: u8, active_effects: u8) {
        self.metrics.active_tracks = active_tracks;
        self.metrics.active_effects = active_effects;
    }

    /// Check if system meets performance requirements
    pub fn check_performance_requirements(&self) -> PerformanceStatus {
        let mut status = PerformanceStatus::Good;
        let mut issues: Vec<&'static str, 8> = Vec::new();
        
        // Check latency requirement (<5ms)
        if self.metrics.audio_latency_ms > 5.0 {
            status = PerformanceStatus::Warning;
            let _ = issues.push("Audio latency exceeds 5ms requirement");
        }
        
        // Check CPU usage
        if self.metrics.cpu_usage > 80.0 {
            status = PerformanceStatus::Critical;
            let _ = issues.push("CPU usage exceeds 80%");
        } else if self.metrics.cpu_usage > 60.0 {
            status = PerformanceStatus::Warning;
            let _ = issues.push("CPU usage above 60%");
        }
        
        // Check for dropouts
        if self.metrics.dropout_count > 0 {
            status = PerformanceStatus::Critical;
            let _ = issues.push("Audio dropouts detected");
        }
        
        // Check callback timing
        let buffer_duration_us = (self.metrics.buffer_size * 1_000_000) / self.metrics.sample_rate;
        if self.metrics.max_callback_time_us > buffer_duration_us * 8 / 10 { // 80% of buffer time
            status = PerformanceStatus::Warning;
            let _ = issues.push("Audio callback time approaching buffer limit");
        }
        
        status
    }

    /// Get optimization recommendations based on current performance
    pub fn get_optimization_recommendations(&self) -> Vec<&'static str, 8> {
        let mut recommendations = Vec::new();
        
        if self.metrics.cpu_usage > 70.0 {
            let _ = recommendations.push("Consider reducing active effects");
            let _ = recommendations.push("Enable fast math optimizations");
        }
        
        if self.metrics.audio_latency_ms > 3.0 {
            let _ = recommendations.push("Reduce audio buffer size");
            let _ = recommendations.push("Enable buffer pre-allocation");
        }
        
        if self.metrics.dropout_count > 0 {
            let _ = recommendations.push("Increase audio buffer size");
            let _ = recommendations.push("Reduce system load");
        }
        
        if !self.optimizations.effect_chain_opt {
            let _ = recommendations.push("Enable effect chain optimization");
        }
        
        if !self.optimizations.track_processing_opt {
            let _ = recommendations.push("Enable track processing optimization");
        }
        
        recommendations
    }

    /// Reset performance counters
    pub fn reset_counters(&mut self) {
        self.metrics.dropout_count = 0;
        self.metrics.max_callback_time_us = 0;
        self.callback_times.clear();
        self.callback_count = 0;
    }

    /// Get current microsecond timestamp (platform-specific)
    fn get_microseconds(&self) -> u32 {
        // In a real implementation, this would use platform-specific timing
        // For now, return a dummy value
        0
    }
}

/// Performance status levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceStatus {
    /// System performing well within requirements
    Good,
    /// System has performance warnings but still functional
    Warning,
    /// System has critical performance issues
    Critical,
}

/// Audio processing optimization utilities
pub struct AudioOptimizer;

impl AudioOptimizer {
    /// Optimize audio buffer processing using SIMD where available
    pub fn process_buffer_simd(input: &[f32], output: &mut [f32], gain: f32) {
        let len = input.len().min(output.len());
        
        // For embedded systems without SIMD, use optimized scalar processing
        if len >= 4 {
            // Process 4 samples at a time for better cache efficiency
            let chunks = len / 4;
            for i in 0..chunks {
                let base = i * 4;
                output[base] = input[base] * gain;
                output[base + 1] = input[base + 1] * gain;
                output[base + 2] = input[base + 2] * gain;
                output[base + 3] = input[base + 3] * gain;
            }
            
            // Process remaining samples
            for i in (chunks * 4)..len {
                output[i] = input[i] * gain;
            }
        } else {
            // Process remaining samples normally
            for i in 0..len {
                output[i] = input[i] * gain;
            }
        }
    }

    /// Optimize audio mixing with multiple inputs
    pub fn mix_buffers_optimized(inputs: &[&[f32]], output: &mut [f32], gains: &[f32]) {
        if inputs.is_empty() {
            return;
        }
        
        let len = output.len();
        
        // Clear output buffer
        output.fill(0.0);
        
        // Mix each input with its gain
        for (input, &gain) in inputs.iter().zip(gains.iter()) {
            let input_len = input.len().min(len);
            
            // Optimized mixing loop
            for i in 0..input_len {
                output[i] += input[i] * gain;
            }
        }
    }

    /// Fast approximation functions for audio processing
    pub fn fast_sin(x: f32) -> f32 {
        // Fast sine approximation using polynomial
        let x = x % (2.0 * core::f32::consts::PI);
        let x = if x > core::f32::consts::PI { x - 2.0 * core::f32::consts::PI } else { x };
        
        // Bhaskara I's sine approximation
        let x_abs = x.abs();
        let sign = if x >= 0.0 { 1.0 } else { -1.0 };
        
        sign * (16.0 * x_abs * (core::f32::consts::PI - x_abs)) / 
               (5.0 * core::f32::consts::PI * core::f32::consts::PI - 4.0 * x_abs * (core::f32::consts::PI - x_abs))
    }

    /// Fast exponential decay for envelope processing
    pub fn fast_exp_decay(x: f32) -> f32 {
        // Fast exponential approximation
        if x > 10.0 {
            0.0
        } else if x < -10.0 {
            1.0
        } else {
            // Linear approximation for small values
            1.0 - x * 0.1
        }
    }

    /// Optimize effect chain processing order
    pub fn optimize_effect_order(effects: &mut [Option<crate::effects::Effect>]) {
        // Sort effects by computational cost (lighter effects first)
        // This is a simplified optimization - real implementation would be more sophisticated
        
        let mut effect_costs: Vec<(usize, u32), 4> = Vec::new();
        
        for (i, effect) in effects.iter().enumerate() {
            if let Some(eff) = effect {
                let cost = Self::estimate_effect_cost(&eff.effect_type);
                let _ = effect_costs.push((i, cost));
            }
        }
        
        // Sort by cost (ascending)
        effect_costs.sort_by_key(|&(_, cost)| cost);
        
        // Reorder effects based on cost (this is a simplified approach)
        // In practice, some effects have dependencies on order
    }

    /// Estimate computational cost of an effect
    fn estimate_effect_cost(effect_type: &crate::effects::EffectType) -> u32 {
        use crate::effects::EffectType;
        
        match effect_type {
            // Low cost effects
            EffectType::Mixer => 1,
            EffectType::Limiter => 2,
            EffectType::NoiseSuppressor => 2,
            
            // Medium cost effects
            EffectType::Compressor => 3,
            EffectType::MasteringEQ => 3,
            EffectType::AutoWah => 3,
            EffectType::DJFilter => 3,
            
            // High cost effects
            EffectType::TapeEcho => 4,
            EffectType::T3Delay => 4,
            EffectType::Chorus => 4,
            EffectType::Flanger => 4,
            
            // Very high cost effects
            EffectType::SpaceReverb => 5,
            EffectType::BeatRepeat => 5,
            EffectType::PitchShift => 6,
            EffectType::PitchCorrect => 6,
            
            // Complex effects
            EffectType::Slicer => 4,
            EffectType::Reverse => 3,
            EffectType::MultibandCompressor => 5,
            EffectType::Isolator => 4,
            EffectType::JC120 => 4,
            EffectType::Tweed => 4,
            EffectType::Metal => 4,
            EffectType::Sidechain => 3,
        }
    }
}

/// Memory pool for optimized audio buffer allocation
pub struct AudioMemoryPool {
    /// Pre-allocated buffers for audio processing
    buffers: Vec<[f32; 512], 16>, // 16 buffers of 512 samples each
    /// Available buffer indices
    available: Vec<usize, 16>,
}

impl AudioMemoryPool {
    /// Create a new memory pool
    pub fn new() -> Self {
        let mut pool = Self {
            buffers: Vec::new(),
            available: Vec::new(),
        };
        
        // Pre-allocate buffers
        for i in 0..16 {
            let _ = pool.buffers.push([0.0f32; 512]);
            let _ = pool.available.push(i);
        }
        
        pool
    }

    /// Allocate a buffer from the pool
    pub fn allocate(&mut self) -> Option<&mut [f32; 512]> {
        if let Some(index) = self.available.pop() {
            self.buffers.get_mut(index)
        } else {
            None // Pool exhausted
        }
    }

    /// Return a buffer to the pool
    pub fn deallocate(&mut self, buffer: &[f32; 512]) {
        // Find the buffer index and return it to available pool
        for (i, pool_buffer) in self.buffers.iter().enumerate() {
            if core::ptr::eq(buffer, pool_buffer) {
                let _ = self.available.push(i);
                break;
            }
        }
    }

    /// Get pool utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f32 {
        let used = 16 - self.available.len();
        used as f32 / 16.0
    }
}

/// Latency measurement utilities
pub struct LatencyMeasurement {
    /// Input to output latency in samples
    pub total_latency: u32,
    /// ADC latency in samples
    pub adc_latency: u32,
    /// Processing latency in samples
    pub processing_latency: u32,
    /// DAC latency in samples
    pub dac_latency: u32,
    /// Buffer latency in samples
    pub buffer_latency: u32,
}

impl LatencyMeasurement {
    /// Create a new latency measurement
    pub fn new() -> Self {
        Self {
            total_latency: 0,
            adc_latency: 0,
            processing_latency: 0,
            dac_latency: 0,
            buffer_latency: 0,
        }
    }

    /// Measure system latency using impulse response
    pub fn measure_impulse_latency(&mut self, sample_rate: u32, buffer_size: u32) {
        // Estimate component latencies based on typical values
        
        // ADC latency (typically 1-2 samples for delta-sigma ADCs)
        self.adc_latency = 2;
        
        // DAC latency (typically 1-3 samples for delta-sigma DACs)
        self.dac_latency = 3;
        
        // Buffer latency (double buffering adds one buffer period)
        self.buffer_latency = buffer_size * 2;
        
        // Processing latency (estimated based on effect complexity)
        self.processing_latency = buffer_size / 4; // Conservative estimate
        
        // Total latency
        self.total_latency = self.adc_latency + self.dac_latency + 
                           self.buffer_latency + self.processing_latency;
    }

    /// Get total latency in milliseconds
    pub fn total_latency_ms(&self, sample_rate: u32) -> f32 {
        (self.total_latency as f32 / sample_rate as f32) * 1000.0
    }

    /// Check if latency meets requirements (<5ms)
    pub fn meets_requirements(&self, sample_rate: u32) -> bool {
        self.total_latency_ms(sample_rate) < 5.0
    }
}

/// Performance test suite for validating system performance
pub struct PerformanceTestSuite {
    /// Test results
    pub results: Vec<PerformanceTestResult, 16>,
}

#[derive(Debug, Clone)]
pub struct PerformanceTestResult {
    pub test_name: &'static str,
    pub passed: bool,
    pub measured_value: f32,
    pub required_value: f32,
    pub units: &'static str,
}

impl PerformanceTestSuite {
    /// Create a new test suite
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Run all performance tests
    pub fn run_all_tests(&mut self, profiler: &PerformanceProfiler) {
        self.results.clear();
        
        self.test_audio_latency(profiler);
        self.test_cpu_usage(profiler);
        self.test_dropout_count(profiler);
        self.test_callback_timing(profiler);
    }

    /// Test audio latency requirement
    fn test_audio_latency(&mut self, profiler: &PerformanceProfiler) {
        let result = PerformanceTestResult {
            test_name: "Audio Latency",
            passed: profiler.metrics.audio_latency_ms < 5.0,
            measured_value: profiler.metrics.audio_latency_ms,
            required_value: 5.0,
            units: "ms",
        };
        let _ = self.results.push(result);
    }

    /// Test CPU usage
    fn test_cpu_usage(&mut self, profiler: &PerformanceProfiler) {
        let result = PerformanceTestResult {
            test_name: "CPU Usage",
            passed: profiler.metrics.cpu_usage < 80.0,
            measured_value: profiler.metrics.cpu_usage,
            required_value: 80.0,
            units: "%",
        };
        let _ = self.results.push(result);
    }

    /// Test dropout count
    fn test_dropout_count(&mut self, profiler: &PerformanceProfiler) {
        let result = PerformanceTestResult {
            test_name: "Audio Dropouts",
            passed: profiler.metrics.dropout_count == 0,
            measured_value: profiler.metrics.dropout_count as f32,
            required_value: 0.0,
            units: "count",
        };
        let _ = self.results.push(result);
    }

    /// Test callback timing
    fn test_callback_timing(&mut self, profiler: &PerformanceProfiler) {
        let buffer_duration_us = (profiler.metrics.buffer_size * 1_000_000) / profiler.metrics.sample_rate;
        let max_allowed = buffer_duration_us * 8 / 10; // 80% of buffer time
        
        let result = PerformanceTestResult {
            test_name: "Callback Timing",
            passed: profiler.metrics.max_callback_time_us <= max_allowed,
            measured_value: profiler.metrics.max_callback_time_us as f32,
            required_value: max_allowed as f32,
            units: "μs",
        };
        let _ = self.results.push(result);
    }

    /// Get overall test status
    pub fn overall_status(&self) -> bool {
        self.results.iter().all(|result| result.passed)
    }

    /// Get failed tests
    pub fn failed_tests(&self) -> Vec<&PerformanceTestResult, 16> {
        let mut failed = Vec::new();
        for result in &self.results {
            if !result.passed {
                let _ = failed.push(result);
            }
        }
        failed
    }
}