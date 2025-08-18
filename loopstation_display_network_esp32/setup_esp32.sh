#!/bin/bash

# ESP32 Development Environment Setup Script
# This script sets up the ESP32 Rust development environment

echo "Setting up ESP32 Rust development environment..."

# Install ESP32 Rust toolchain
echo "Installing ESP32 Rust toolchain..."
cargo install espup
espup install

# Install additional tools
echo "Installing additional ESP32 tools..."
cargo install ldproxy
cargo install espflash
cargo install cargo-espflash

# Add ESP32 target
echo "Adding ESP32 target..."
rustup target add xtensa-esp32-espidf

# Source the ESP environment
echo "Setting up ESP environment..."
if [ -f "$HOME/export-esp.sh" ]; then
    source "$HOME/export-esp.sh"
    echo "ESP environment sourced from $HOME/export-esp.sh"
else
    echo "Warning: $HOME/export-esp.sh not found. Run 'espup install' first."
fi

echo "ESP32 setup complete!"
echo ""
echo "To build and flash the ESP32 project:"
echo "1. Source the ESP environment: . \$HOME/export-esp.sh"
echo "2. Build: cargo build"
echo "3. Flash: cargo run"
echo ""
echo "Note: Make sure your ESP32 device is connected via USB."