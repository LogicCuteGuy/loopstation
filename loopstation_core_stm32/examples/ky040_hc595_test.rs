//! KY-040 Rotary Encoder and 74HC595 LED Controller Test
//!
//! This example demonstrates the KY-040 rotary encoder driver and 74HC595 LED controller
//! functionality for menu navigation and status indication.
//!
//! Hardware connections:
//! - KY-040 Rotary Encoder:
//!   - CLK (A phase) -> PA0
//!   - DT (B phase) -> PA1  
//!   - SW (button) -> PA2
//!   - VCC -> 3.3V
//!   - GND -> GND
//!
//! - 74HC595 LED Controller:
//!   - SER/DS (Data) -> PB0
//!   - SRCLK/SHCP (Clock) -> PB1
//!   - RCLK/STCP (Latch) -> PB2
//!   - OE (Output Enable) -> PB3 (active low)
//!   - VCC -> 5V
//!   - GND -> GND
//!   - Q0-Q7 -> LEDs with current limiting resistors

#![no_main]
#![no_std]

use panic_halt as _;
use cortex_m_rt::entry;
use loopstation_core_stm32::hal::{HardwareHal, LedId, LedCommand, RotaryEvent};

#[entry]
fn main() -> ! {
    // Initialize hardware abstraction layer
    let mut hal = match HardwareHal::init() {
        Ok(hal) => hal,
        Err(_) => panic!("Failed to initialize HAL"),
    };

    // Test LED controller - light up LEDs in sequence
    test_led_controller(&mut hal);

    // Test rotary encoder and LED interaction
    test_rotary_encoder_with_leds(&mut hal);

    // Main loop - demonstrate rotary encoder and LED synchronization
    let mut timestamp = 0u32;
    let mut led_pattern = 0u8;
    let mut last_encoder_position = 0i32;

    loop {
        timestamp += 1;

        // Update rotary encoder
        let encoder_events = hal.update_rotary_encoder(timestamp);
        
        // Process encoder events
        for event in encoder_events {
            match event {
                RotaryEvent::Clockwise => {
                    // Rotate LED pattern clockwise
                    led_pattern = (led_pattern + 1) % 6;
                    update_track_leds(&mut hal, led_pattern);
                }
                RotaryEvent::CounterClockwise => {
                    // Rotate LED pattern counter-clockwise
                    led_pattern = if led_pattern == 0 { 5 } else { led_pattern - 1 };
                    update_track_leds(&mut hal, led_pattern);
                }
                RotaryEvent::ButtonPress => {
                    // Toggle all FX LEDs on button press
                    toggle_fx_leds(&mut hal);
                }
                RotaryEvent::ButtonRelease => {
                    // Clear all LEDs on button release
                    let _ = hal.clear_all_leds();
                }
            }
        }

        // Check for encoder position changes
        let current_position = hal.get_rotary_position();
        if current_position != last_encoder_position {
            last_encoder_position = current_position;
            
            // Update tempo LED based on encoder position
            let tempo_on = (current_position % 2) == 0;
            let _ = hal.set_led(LedId::Tempo, if tempo_on { LedCommand::On } else { LedCommand::Off });
            let _ = hal.update_leds();
        }

        // Sync LEDs with system state periodically
        if timestamp % 10000 == 0 {
            let _ = hal.sync_leds_with_system_state();
        }

        // Small delay to prevent overwhelming the system
        for _ in 0..1000 {
            cortex_m::asm::nop();
        }
    }
}

/// Test LED controller by lighting up LEDs in sequence
fn test_led_controller(hal: &mut HardwareHal) {
    // Clear all LEDs first
    let _ = hal.clear_all_leds();

    // Test track LEDs
    for track_id in 1..=6 {
        let _ = hal.set_led(LedId::Track(track_id), LedCommand::On);
        let _ = hal.update_leds();
        
        // Delay
        for _ in 0..100000 {
            cortex_m::asm::nop();
        }
        
        let _ = hal.set_led(LedId::Track(track_id), LedCommand::Off);
    }

    // Test FX LEDs
    for fx_id in 1..=5 {
        let _ = hal.set_led(LedId::FX(fx_id), LedCommand::On);
        let _ = hal.update_leds();
        
        // Delay
        for _ in 0..100000 {
            cortex_m::asm::nop();
        }
        
        let _ = hal.set_led(LedId::FX(fx_id), LedCommand::Off);
    }

    // Test transport LEDs
    let transport_leds = [LedId::Play, LedId::Stop, LedId::Rec];
    for led_id in transport_leds.iter() {
        let _ = hal.set_led(*led_id, LedCommand::On);
        let _ = hal.update_leds();
        
        // Delay
        for _ in 0..100000 {
            cortex_m::asm::nop();
        }
        
        let _ = hal.set_led(*led_id, LedCommand::Off);
    }

    // Final update
    let _ = hal.update_leds();
}

/// Test rotary encoder with LED feedback
fn test_rotary_encoder_with_leds(hal: &mut HardwareHal) {
    let mut test_timestamp = 0u32;
    let mut test_duration = 0;

    // Test for a limited time
    while test_duration < 1000000 {
        test_timestamp += 1;
        test_duration += 1;

        // Update encoder
        let events = hal.update_rotary_encoder(test_timestamp);
        
        // Light up LEDs based on encoder events
        for event in events {
            match event {
                RotaryEvent::Clockwise => {
                    let _ = hal.set_led(LedId::PageRight, LedCommand::On);
                    let _ = hal.update_leds();
                }
                RotaryEvent::CounterClockwise => {
                    let _ = hal.set_led(LedId::PageLeft, LedCommand::On);
                    let _ = hal.update_leds();
                }
                RotaryEvent::ButtonPress => {
                    let _ = hal.set_led(LedId::Menu, LedCommand::On);
                    let _ = hal.update_leds();
                }
                RotaryEvent::ButtonRelease => {
                    // Clear navigation LEDs
                    let _ = hal.set_led(LedId::PageLeft, LedCommand::Off);
                    let _ = hal.set_led(LedId::PageRight, LedCommand::Off);
                    let _ = hal.set_led(LedId::Menu, LedCommand::Off);
                    let _ = hal.update_leds();
                }
            }
        }

        // Small delay
        for _ in 0..100 {
            cortex_m::asm::nop();
        }
    }
}

/// Update track LEDs in a pattern
fn update_track_leds(hal: &mut HardwareHal, active_track: u8) {
    // Clear all track LEDs first
    for track_id in 1..=6 {
        let _ = hal.set_led(LedId::Track(track_id), LedCommand::Off);
    }

    // Light up the active track
    if active_track < 6 {
        let _ = hal.set_led(LedId::Track(active_track + 1), LedCommand::On);
    }

    let _ = hal.update_leds();
}

/// Toggle all FX LEDs
fn toggle_fx_leds(hal: &mut HardwareHal) {
    for fx_id in 1..=5 {
        // Get current state and toggle
        if let Ok(current_state) = hal.get_led_state(LedId::FX(fx_id)) {
            let new_command = if current_state { LedCommand::Off } else { LedCommand::On };
            let _ = hal.set_led(LedId::FX(fx_id), new_command);
        }
    }
    let _ = hal.update_leds();
}