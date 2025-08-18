# Implementation Plan

- [x] 1. Fix workspace configuration and set up core data structures

  - Fix workspace member names in root Cargo.toml to match actual directory names
  - Add proper dependencies to loopstation_core_stm32 for embedded development (cortex-m, stm32h7xx-hal, etc.)
  - Implement core data structures: Track, EffectChain, Project, BankSystem, MemorySystem
  - Create shared types module for cross-component communication
  - _Requirements: 11.1, 11.5, 1.1, 3.4, 8.1_

- [x] 2. Implement loopstation_core_stm32 basic structure
  - [x] 2.1 Create lib.rs with core module structure

    - Set up module organization (audio, effects, storage, controls, midi)
    - Implement basic Track struct with audio buffer management using heapless collections
    - Add TrackState enum (Stopped/Recording/Playing/Overdubbing/Muted) with state transitions
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

  - [x] 2.2 Implement Effect system foundation

    - Create EffectChain struct with 4 slots for Input/Track/Master FX layers
    - Define EffectType enum with core effects (start with 10-15 essential effects)
    - Add basic effect parameter management and wet/dry mix controls
    - _Requirements: 3.4, 3.5, 3.6, 3.7, 3.8_

  - [x] 2.3 Create Memory and Project management


    - Implement MemorySystem with 255 memory slots for project storage
    - Create Project struct with tracks, tempo, effects, and assignments
    - Add basic project serialization/deserialization structure
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 3. Fix STM32 HAL compilation issues and complete hardware abstraction layer













  - [x] 3.1 Fix HAL compilation errors and complete basic hardware setup



    - Fix DacOut trait import compilation error by importing stm32h7xx_hal::traits::dac::DacOut
    - Fix DAC initialization function call with correct number of arguments
    - Complete GPIO pin assignments and fix any remaining type mismatches for button matrix
    - Add missing interrupt handlers for audio timer and DMA
    - Test basic ADC/DAC functionality with simple audio passthrough
    - _Requirements: 1.3, 3.1, 3.2, 3.3, 2.1, 2.9, 2.24, 2.25, 2.28_



  - [x] 3.2 Implement I2S audio interface for PCM1808/PCM5102A



    - Add I2S peripheral configuration for 44.1kHz sample rate
    - Configure 4x PCM1808 ADCs for 8-channel input (MIC IN 1-4, INST IN 1-4)
    - Configure 4x PCM5102A DACs for 8-channel output (MAIN OUT, SUB OUT 1-4, PHONES)
    - Implement DMA-based I2S audio streaming with double buffering
    - Create audio callback integration with existing AudioEngine
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 11.1, 11.2, 11.3, 11.4, 11.5_

  - [x] 3.3 Implement PCF8575 I2C I/O expander driver for button matrix



    - Create PCF8575 driver for button matrix scanning with I2C communication
    - Implement button debouncing and gesture detection (short/long/double press)
    - Add interrupt-driven button response with 10ms response time requirement
    - Integrate with existing ControlSystem for button event processing
    - _Requirements: 2.1, 2.2, 2.3, 2.9, 2.10, 2.24, 2.25, 2.28_

  - [x] 3.4 Implement KY-040 rotary encoder and 74HC595 LED control



    - Create KY-040 rotary encoder driver with quadrature decoding and button detection
    - Implement 74HC595 shift register driver for LED matrix control
    - Add LED status updates synchronized with track and FX states
    - Integrate rotary encoder with menu navigation system
    - _Requirements: 2.1, 2.28, 6.1, 6.2, 7.2, 7.3_

- [x] 4. Create ESP32 display and network module project structure
  - [x] 4.1 Set up ESP32 project with basic framework
    - Create loopstation_display_network_esp32 project with ESP-IDF framework or Arduino framework
    - Set up basic project structure with main.cpp/main.rs and component organization
    - Configure build system and basic ESP32 initialization
    - Add WiFi connectivity foundation with auto-reconnection capability
    - Create basic project files (CMakeLists.txt or Cargo.toml depending on framework choice)
    - _Requirements: 7.1, 5.1, 5.2_

  - [x] 4.2 Implement 128x64 LCD display driver
    - Create display driver for 128x64 backlit LCD (SPI/I2C interface)
    - Implement basic graphics primitives (text, lines, rectangles)
    - Add display buffer management and refresh system
    - Create basic UI layout for track status display
    - _Requirements: 7.2, 7.3, 7.4, 7.5, 6.1, 6.2_

  - [x] 4.3 Create OSC network server foundation
    - Implement basic OSC server on port 8000 with UDP support
    - Add Bonjour/mDNS service advertisement for discovery
    - Create command parsing and response framework
    - Add basic command processing with <20ms response time target
    - _Requirements: 5.3, 5.4, 5.5, 5.6_

- [x] 5. Create STM32-ESP32 communication protocol
  - [x] 5.1 Implement UART communication protocol
    - Set up UART interface at 115200 baud between STM32 and ESP32
    - Create message protocol for display updates and control commands
    - Add command/response handling with error recovery
    - _Requirements: 11.1, 11.2_

  - [x] 5.2 Implement status synchronization
    - Create real-time status updates from STM32 to ESP32
    - Add parameter change notifications for display updates
    - Implement network command relay from ESP32 to STM32
    - _Requirements: 7.6, 7.7, 5.2_

- [x] 6. Transform PC plugin from basic gain to loopstation functionality
  - [x] 6.1 Replace gain plugin with loopstation parameter structure
    - Remove basic gain parameter and implement 6-track parameter structure
    - Add track-based parameters (volume, pan, record/play state for each track)
    - Create parameter groups for Input FX, Track FX, and Master FX chains
    - Integrate loopstation_core_stm32 as dependency for shared logic (already added to Cargo.toml)
    - _Requirements: 4.1, 4.2, 4.3_

  - [x] 6.2 Implement loopstation audio processing in plugin context
    - Create LoopstationCore instance in plugin for audio processing
    - Implement 6-track audio buffer management using host audio buffers
    - Add basic recording/playback functionality with VST3/CLAP compatibility
    - Connect plugin parameters to loopstation core controls
    - _Requirements: 4.4, 4.5, 1.1, 1.2_

  - [x] 6.3 Add MIDI control and DAW integration
    - Implement MIDI CC input for loopstation control using existing MidiHandler
    - Add parameter automation support for DAW integration
    - Create MIDI mapping for track control and effects using cc_mappings
    - Add project save/load integration with DAW serialization
    - _Requirements: 4.6, 2.25, 10.4_

- [x] 7. Implement audio effect processing algorithms
  - [x] 7.1 Create core effect processing framework
    - Replace placeholder Effect::process_audio and EffectChain::process_audio methods with real implementations
    - Add essential effects: Compressor, Reverb, Delay, EQ algorithms using micromath
    - Create effect parameter management and real-time control integration
    - Add wet/dry mix processing for each effect
    - _Requirements: 3.4, 3.5, 3.6, 3.7, 3.8_

  - [x] 7.2 Implement 3-layer effect processing pipeline
    - Connect Input FX layer to audio recording pipeline in AudioEngine
    - Connect Track FX layer to individual track playback processing
    - Connect Master FX layer to final output mixing in process_callback
    - Add effect chain processing to Track::process_audio method
    - _Requirements: 3.4, 3.5, 3.6, 3.20_

- [x] 8. Integrate control system with loopstation core
  - [x] 8.1 Connect control events to loopstation functions
    - Integrate ControlInterfaceHal with LoopstationCore in main update loop
    - Map button functions to actual loopstation operations (track control, effects, etc.)
    - Implement fader and knob control for track levels and effect parameters
    - Add expression pedal support for real-time parameter control
    - _Requirements: 2.1, 2.2, 2.3, 2.9, 2.10, 2.24, 2.25, 2.28_

  - [x] 8.2 Implement control assignment system
    - Connect FX button assignments to effect chain control
    - Add MIDI CC assignment processing for external control
    - Implement footswitch assignment functionality
    - Create control context switching for menu vs performance modes
    - _Requirements: 6.5, 6.21, 10.5, 10.6_

- [ ] 9. Implement MIDI I/O hardware integration
  - [ ] 9.1 Add MIDI input processing to STM32 HAL
    - Set up UART-based MIDI IN with Control Change and Note message support
    - Implement MIDI channel selection (1-16/OMNI) and message filtering
    - Add basic MIDI clock synchronization for tempo-locked effects
    - _Requirements: 2.25, 3.11, 10.4, 10.9_

  - [ ] 9.2 Create MIDI output functionality
    - Implement MIDI OUT via UART
    - Add Program Change output for memory slot switching
    - Create Control Change transmission for parameter updates
    - _Requirements: 10.10, 10.11_

- [ ] 10. Implement persistent storage and serialization
  - [ ] 10.1 Complete project serialization/deserialization implementation
    - Replace placeholder Project::serialize/deserialize with postcard implementation
    - Add proper save/load functionality for projects in MemorySystem
    - Implement storage error handling and recovery mechanisms
    - Add audio buffer serialization for project persistence
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

  - [ ] 10.2 Add persistent storage interface abstraction
    - Create storage abstraction trait for Flash memory (STM32) and file system (PC)
    - Implement auto-save functionality with VBAT backup protection
    - Add project export/import capabilities with WAV format support
    - Create storage space management and cleanup functionality
    - _Requirements: 8.11, 8.14, 8.12_

- [ ] 11. Implement menu system and navigation (ESP32)
  - [ ] 11.1 Create comprehensive menu structure
    - Implement main menu system with hierarchical navigation
    - Add core menus: CTL FUNC, Assign, Track, Input FX, Track FX, Master FX, Rhythm, Memory
    - Create menu state management and navigation logic with PAGE buttons
    - Add VALUE knob integration for parameter selection and editing
    - _Requirements: 6.1, 6.2, 6.3, 6.13, 6.14_

  - [ ] 11.2 Add EDIT button functionality and real-time parameter control
    - Implement real-time parameter editing with context-sensitive knob controls
    - Create track parameter editing (Volume, Pan, Play Mode, Quantize)
    - Add effect parameter editing with live control and knob LED feedback
    - Implement advanced parameter access with PAGE buttons in EDIT mode
    - _Requirements: 6.16, 6.17, 6.18, 6.19_

- [ ] 12. Implement system settings and configuration
  - [ ] 12.1 Create system settings data structures
    - Implement GENERAL settings (Tempo Memory, Quantize Mode, Undo Mode)
    - Add CLOCK settings (Clock Source, Sync Out, Rec Quantize)
    - Create MIDI settings (MIDI Channel, Local Control, PC Out, CC Tx/Rx)
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [ ] 12.2 Add control and utility settings
    - Implement CONTROL settings (CTL Func Assign, Foot SW Assign)
    - Create UTILITY settings (Factory Reset, Initialize)
    - Add basic backup/restore functionality
    - _Requirements: 10.5, 10.6, 10.7, 10.12, 10.13_

- [ ] 13. Implement undo/redo system
  - [ ] 13.1 Enhance action history management
    - Expand undo buffer for track operations (recording, overdubbing, clearing)
    - Add UNDO/REDO functionality with configurable undo modes
    - Create effect parameter change reversal system
    - _Requirements: 2.18, 2.19, 3.24_

- [ ] 14. Implement tempo and rhythm system
  - [ ] 14.1 Create tempo control system
    - Implement TAP TEMPO functionality with tempo detection
    - Add tempo reset and BPM management
    - Create basic MIDI clock sync for external sequencers
    - _Requirements: 2.20, 2.21, 3.11, 3.23_

  - [ ] 14.2 Add basic rhythm pattern support
    - Implement simple drum machine pattern playback
    - Add rhythm pattern configuration and selection
    - Create beat position tracking and tempo-locked effects
    - _Requirements: 7.9_

- [ ] 15. Create testing and validation suite
  - [ ] 15.1 Implement unit tests for core components
    - Write tests for Track audio buffer management and state transitions
    - Create tests for EffectChain processing and parameter control
    - Add tests for MemorySystem project save/load functionality
    - _Requirements: 1.1, 1.2, 1.3, 3.4, 8.2, 8.4_

  - [ ] 15.2 Add integration tests for communication
    - Test STM32-ESP32 UART communication protocol
    - Verify OSC network command processing and response times
    - Create tests for MIDI functionality and plugin integration
    - _Requirements: 5.2, 11.1, 11.2, 4.6_

- [ ] 16. Advanced features and modulation
  - [ ] 16.1 Add LFO and Step Sequencer system
    - Implement LFO system with multiple waveforms and tempo sync
    - Add Step Sequencer for parameter automation
    - Create modulation assignment matrix for parameter control
    - _Requirements: 3.13, 3.14, 3.15, 3.16, 3.17_

  - [ ] 16.2 Implement USB audio interface (STM32)
    - Create USB Type-B computer interface with multi-channel audio
    - Add DAW routing capabilities for track inputs/outputs
    - Implement zero-latency monitoring for DAW applications
    - _Requirements: 9.8, 9.9, 9.10, 9.18, 9.19_

- [ ] 17. Final system integration and polish
  - [ ] 17.1 Optimize performance and latency
    - Profile and optimize audio processing for target hardware
    - Ensure <5ms latency requirement across all processing chains
    - Verify 6-track simultaneous processing without dropouts
    - _Requirements: 1.3, 1.4, 3.1, 3.20_

  - [ ] 17.2 Complete system validation
    - Test hardware controls with 10ms response time requirement
    - Verify project compatibility between hardware and PC ecosystems
    - Validate core functionality and MIDI synchronization
    - _Requirements: 2.1, 4.5, 11.3, 11.4, 11.5_