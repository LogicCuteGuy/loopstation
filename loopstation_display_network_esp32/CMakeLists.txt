# ESP-IDF CMake build system alternative
# This file provides an ESP-IDF native C/C++ build option
# as an alternative to the Rust implementation

cmake_minimum_required(VERSION 3.16)

# Set the project name
set(PROJECT_NAME "loopstation_display_network_esp32")

# Include ESP-IDF build system
include($ENV{IDF_PATH}/tools/cmake/project.cmake)

# Define the project
project(${PROJECT_NAME})

# Note: This CMakeLists.txt is provided as a reference for ESP-IDF C/C++ development
# The main implementation uses Rust with esp-idf-svc
# To use this CMake build system, you would need to:
# 1. Install ESP-IDF framework
# 2. Create main/main.c with C implementation
# 3. Create component directories for display, network, communication modules
# 4. Run: idf.py build