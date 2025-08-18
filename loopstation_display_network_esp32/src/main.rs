use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::*;
use std::{sync::Arc, thread, time::Duration};

mod display;
mod network;
mod communication;
mod config;

use display::DisplayManager;
use network::NetworkManager;
use communication::CommunicationManager;
use config::Config;

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Starting Loopstation Display Network ESP32");

    // Initialize peripherals
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded: WiFi SSID: {}", config.wifi_ssid);

    // Initialize WiFi
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    // Initialize managers
    let display_manager = Arc::new(DisplayManager::new(peripherals.i2c0, peripherals.pins)?);
    let communication_manager = Arc::new(CommunicationManager::new(peripherals.uart1)?);
    let network_manager = Arc::new(NetworkManager::new());

    // Connect to WiFi with auto-reconnection
    connect_wifi(&mut wifi, &config)?;

    // Start STM32 communication status polling
    communication_manager.start_status_polling();
    info!("STM32 communication polling started");

    // Start display update thread
    let display_clone = display_manager.clone();
    let comm_clone = communication_manager.clone();
    thread::spawn(move || {
        display_update_loop(display_clone, comm_clone);
    });

    // Start network server
    let network_clone = network_manager.clone();
    let comm_clone = communication_manager.clone();
    thread::spawn(move || {
        if let Err(e) = network_clone.start_server(comm_clone) {
            error!("Network server error: {:?}", e);
        }
    });

    // Main loop - handle WiFi reconnection and system monitoring
    loop {
        if !wifi.is_connected()? {
            warn!("WiFi disconnected, attempting reconnection...");
            if let Err(e) = connect_wifi(&mut wifi, &config) {
                error!("WiFi reconnection failed: {:?}", e);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        }

        // System health check and status updates
        thread::sleep(Duration::from_secs(1));
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>, config: &Config) -> Result<()> {
    let wifi_configuration = Configuration::Client(ClientConfiguration {
        ssid: config.wifi_ssid.as_str().try_into()?,
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: config.wifi_password.as_str().try_into()?,
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(())
}

fn display_update_loop(
    display_manager: Arc<DisplayManager>,
    communication_manager: Arc<CommunicationManager>,
) {
    info!("Starting display update loop");
    
    loop {
        // Get latest system state from STM32
        if let Ok(mut state) = communication_manager.get_system_state() {
            // Add connection status to state
            let (connected, errors, since_heartbeat) = communication_manager.get_connection_status();
            state.network_connected = connected;
            
            // Log connection issues
            if !connected {
                warn!("STM32 connection lost - {} errors, last heartbeat {:?} ago", errors, since_heartbeat);
            }
            
            // Update display with current state
            if let Err(e) = display_manager.update_display(&state) {
                error!("Display update error: {:?}", e);
            }
        } else {
            warn!("Failed to get system state from STM32");
        }

        // 60 FPS display updates (16.67ms)
        thread::sleep(Duration::from_millis(16));
    }
}