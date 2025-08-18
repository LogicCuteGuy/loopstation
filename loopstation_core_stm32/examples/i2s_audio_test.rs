//! I2S Audio Interface Test Example
//! 
//! This example demonstrates the I2S audio interface implementation
//! for PCM1808/PCM5102A audio codecs with DMA streaming.
//! 
//! Task 3.2: I2S audio interface for PCM1808/PCM5102A

#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;
use loopstation_core_stm32::hal::init_global_hal;

#[entry]
fn main() -> ! {
    // Initialize the hardware abstraction layer
    let hal_result = init_global_hal();
    let mut hal = match hal_result {
        Ok(h) => h,
        Err(_) => {
            // Failed to initialize HAL - enter infinite loop
            loop {
                cortex_m::asm::nop();
            }
        }
    };

    // Start I2S audio streaming
    let stream_result = hal.start_audio_streaming();
    match stream_result {
        Ok(_) => {
            // Audio streaming started successfully
        },
        Err(_) => {
            // Failed to start audio streaming - enter infinite loop
            loop {
                cortex_m::asm::nop();
            }
        }
    }

    // Main processing loop
    loop {
        // Process audio callback manually (simulating interrupt)
        hal.process_audio_callback();
        
        // Test reading input channels
        let input_result = hal.read_audio_input_channels();
        if let Ok(input_channels) = input_result {
            // Create output channels array
            let mut output_channels = [0.0f32; 8];
            
            // Simple passthrough - copy input to output with volume reduction
            let mut i = 0;
            while i < 8 {
                output_channels[i] = input_channels[i] * 0.5;
                i += 1;
            }
            
            // Write to output channels
            let _ = hal.write_audio_output_channels(&output_channels);
        }

        // Test audio engine integration
        if let Some(engine) = hal.get_audio_engine_mut() {
            // Check engine status
            let _is_recording = engine.is_recording();
            let _is_playing = engine.is_playing();
            
            // Get statistics
            let stats = engine.get_stats();
            let _underruns = stats.underruns;
            let _overruns = stats.overruns;
        }

        // Small delay to prevent tight loop
        let mut delay_count = 0;
        while delay_count < 1000 {
            cortex_m::asm::nop();
            delay_count += 1;
        }
    }
}