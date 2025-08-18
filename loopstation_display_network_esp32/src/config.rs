use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub wifi_ssid: String,
    pub wifi_password: String,
    pub osc_port: u16,
    pub uart_baud_rate: u32,
    pub display_brightness: u8,
    pub auto_reconnect_interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wifi_ssid: "LoopstationWiFi".to_string(),
            wifi_password: "loopstation123".to_string(),
            osc_port: 8000,
            uart_baud_rate: 115200,
            display_brightness: 255,
            auto_reconnect_interval: 5, // seconds
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // For now, return default config
        // In future implementations, this could load from NVS storage
        // or configuration file
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<()> {
        // Placeholder for saving configuration to NVS
        // This will be implemented when persistent configuration is needed
        Ok(())
    }
}