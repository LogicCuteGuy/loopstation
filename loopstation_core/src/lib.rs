#![cfg_attr(target_os = "none", no_std)]

use embassy_time::Timer;
use log::info;

// Shared async task
pub async fn run_task() {
    loop {
        info!("tick");
        Timer::after_secs(1).await;
    }
}

#[cfg(target_os = "none")]
pub mod stm32;