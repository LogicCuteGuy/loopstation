# ESP32 Development Environment Setup Script for Windows
Write-Host "Setting up ESP32 Rust development environment..."

# Install ESP32 Rust toolchain
Write-Host "Installing ESP32 Rust toolchain..."
cargo install espup
espup install

# Install additional tools
Write-Host "Installing additional ESP32 tools..."
cargo install ldproxy
cargo install espflash
cargo install cargo-espflash

# Add ESP32 target
Write-Host "Adding ESP32 target..."
rustup target add xtensa-esp32-espidf

# Set ESP-IDF environment variables
Write-Host "Setting up ESP-IDF environment..."
$env:ESP_IDF_TOOLS_INSTALL_DIR = "$env:USERPROFILE\.espressif"
$env:ESP_IDF_VERSION = "v4.4.5"
