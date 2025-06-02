use crate::run_task;
use embassy_executor::Spawner;
use embassy_stm32::Config;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize STM32 hardware
    let _p = embassy_stm32::init(Config::default());

    // Start async task
    spawner.spawn(run_task()).unwrap();
}

// Defmt logging setup
#[defmt::global_logger]
struct Logger;

// Panic handler
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

static mut LOGGER: Logger = Logger;