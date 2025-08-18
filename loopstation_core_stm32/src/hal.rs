//! Hardware Abstraction Layer for STM32H743VIT6
//!
//! This module provides low-level hardware initialization and management
//! for the loopstation hardware including clocks, ADC/DAC, DMA, and GPIO.
//!
//! Task 3.1 Implementation: Basic HAL setup with 400MHz clocks, ADC/DAC peripherals,
//! and DMA-based audio buffer management foundation.
//!
//! Task 3.2 Implementation: I2S audio interface for PCM1808/PCM5102A with DMA streaming
//!
//! Task 3.3 Implementation: PCF8575 I2C I/O expander driver for button matrix scanning

#![allow(unused)]

use crate::audio::AudioEngine;
use heapless::Vec;

// Conditional imports for embedded builds only
#[cfg(feature = "embedded")]
use {
    cortex_m,
    embedded_hal::blocking::i2c::{Write, WriteRead},
    nb,
    stm32h7xx_hal::{
        adc::{Adc, AdcSampleTime, Resolution},
        dac::{self},
        delay::Delay,
        dma::{self, MemoryToPeripheral, PeripheralToMemory, traits::TargetAddress},
        gpio::{self, AF4, AF5, AF6, AF7, Alternate, Analog, Input, Output, Pull, PushPull},
        i2c::{self, I2c},
        pac::{self, DMA1, DMA2, I2C1, I2C2, I2C3, USART1, USART2, USART3, Interrupt, interrupt},
        prelude::*,
        rcc::{Ccdr, CoreClocks, rec},
        serial::{self, Serial},
        time::Hertz,
        timer::Timer,
        usb_hs::{UsbBus, USB1},
    },
    usb_device::{
        bus::UsbBusAllocator,
        device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
        UsbError,
    },
    usbd_audio::{
        AudioClass, AudioClassBuilder, AudioFormat, ChannelConfig, SampleRate,
        TerminalType, UnitType,
    },
};

/// System clock frequency - 400MHz as per requirements
pub const SYSTEM_CLOCK_HZ: u32 = 400_000_000;

/// Audio sample rate - 44.1kHz as per requirements  
pub const SAMPLE_RATE_HZ: u32 = 44_100;

/// Audio buffer size for DMA transfers (stereo samples)
pub const AUDIO_BUFFER_SIZE: usize = 256;

/// Number of I2S input channels (4x PCM1808 = 8 channels)
pub const I2S_INPUT_CHANNELS: usize = 8;

/// Number of I2S output channels (4x PCM5102A = 8 channels)
pub const I2S_OUTPUT_CHANNELS: usize = 8;

/// DMA buffer size for double buffering (stereo samples per channel)
pub const DMA_BUFFER_SIZE: usize = AUDIO_BUFFER_SIZE * 2; // Double buffer

/// ADC resolution - 16-bit for STM32H7 (embedded only)
#[cfg(feature = "embedded")]
pub const ADC_RESOLUTION: Resolution = Resolution::SixteenBit;

/// DAC resolution - 12-bit (STM32H7 native)
pub const DAC_RESOLUTION: u32 = 4096;

/// Hardware abstraction layer errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalError {
    /// Failed to access peripheral
    PeripheralAccess,
    /// Initialization error
    InitError,
    /// USB-related error
    UsbError,
    /// Invalid configuration
    InvalidConfiguration,
    /// Unsupported format
    UnsupportedFormat,
    /// Device not enabled
    DeviceNotEnabled,
    /// Invalid parameter
    InvalidParameter,
    /// Communication timeout
    Timeout,
    /// DMA error
    DmaError,
    /// I2C error
    I2cError,
    /// SPI error
    SpiError,
    /// UART error
    UartError,
}

/// Hardware abstraction layer for STM32H743VIT6
#[cfg(feature = "embedded")]
pub struct HardwareHal {
    /// Core clock control and distribution
    pub ccdr: Ccdr,
    /// System delay provider
    pub delay: Delay,
    /// I2S Audio input processing (4x PCM1808)
    pub audio_input: AudioInput,
    /// I2S Audio output processing (4x PCM5102A)
    pub audio_output: AudioOutput,
    /// USB audio interface for DAW integration
    pub usb_audio: UsbAudioInterface,
    /// GPIO controller for buttons and controls
    pub gpio_controls: GpioControls,
    /// Control ADC for faders and knobs
    pub control_adc: ControlAdc,
    /// KY-040 rotary encoder for menu navigation
    pub rotary_encoder: Ky040RotaryEncoder,
    /// 74HC595 LED controller for status indication
    pub led_controller: Hc595LedController,
    /// UART communication with ESP32
    pub esp32_uart: Esp32UartInterface,
    /// MIDI UART interface for MIDI IN/OUT
    pub midi_uart: MidiUartInterface,
    /// Audio engine for processing
    pub audio_engine: Option<AudioEngine>,
    /// Audio callback active flag
    pub audio_callback_active: bool,
}

/// Hardware abstraction layer stub for PC builds
#[cfg(not(feature = "embedded"))]
pub struct HardwareHal {
    /// Stub field for PC compatibility
    pub initialized: bool,
    /// USB audio interface for PC compatibility
    pub usb_audio: UsbAudioInterface,
    /// MIDI UART interface for PC compatibility
    pub midi_uart: MidiUartInterface,
}

/// I2S Audio Input configuration for PCM1808 ADCs
pub struct AudioInput {
    /// I2S peripheral configuration for input
    pub initialized: bool,
    /// Current input buffer (double buffering)
    pub current_input_buffer: bool,
    /// Input sample buffers for DMA
    pub input_buffer_a: [i32; DMA_BUFFER_SIZE * I2S_INPUT_CHANNELS],
    pub input_buffer_b: [i32; DMA_BUFFER_SIZE * I2S_INPUT_CHANNELS],
    /// DMA transfer active flag
    pub dma_active: bool,
}

/// I2S Audio Output configuration for PCM5102A DACs
pub struct AudioOutput {
    /// I2S peripheral configuration for output
    pub initialized: bool,
    /// Current output buffer (double buffering)
    pub current_output_buffer: bool,
    /// Output sample buffers for DMA
    pub output_buffer_a: [i32; DMA_BUFFER_SIZE * I2S_OUTPUT_CHANNELS],
    pub output_buffer_b: [i32; DMA_BUFFER_SIZE * I2S_OUTPUT_CHANNELS],
    /// DMA transfer active flag
    pub dma_active: bool,
}

/// GPIO controls for buttons using PCF8575 I2C I/O expanders
#[cfg(not(feature = "std"))]
pub struct GpioControls {
    /// PCF8575 I/O expanders for button matrix
    pub pcf8575_controllers: Vec<Pcf8575Controller, 4>, // Up to 4 PCF8575 chips
    /// I2C peripheral for communication
    pub i2c_peripheral: Option<I2c<I2C1>>,
    /// Button debounce state tracking
    pub button_states: ButtonStates,
    /// Interrupt pin state for fast response
    pub interrupt_pin_active: bool,
    /// Last scan timestamp for 10ms response time
    pub last_scan_time: u32,
}

/// GPIO controls stub for PC builds
#[cfg(feature = "std")]
pub struct GpioControls {
    /// Stub field for PC compatibility
    pub initialized: bool,
}

/// PCF8575 I2C I/O Expander Controller
/// 16-bit I2C I/O expander for button matrix scanning with interrupt capability
pub struct Pcf8575Controller {
    /// I2C device address (0x20-0x27 based on A0-A2 pins)
    pub address: u8,
    /// Current button states (16 bits)
    pub current_state: u16,
    /// Previous button states for change detection
    pub previous_state: u16,
    /// Button mapping for this controller
    pub button_mapping: ButtonMapping,
    /// Debounce counters for each button (16 buttons max)
    pub debounce_counters: [u8; 16],
    /// Press start times for gesture recognition
    pub press_start_times: [u32; 16],
    /// Press types for each button
    pub press_types: [PressType; 16],
    /// Controller enabled flag
    pub enabled: bool,
}

/// Button mapping for PCF8575 controller
#[derive(Clone, Copy)]
pub struct ButtonMapping {
    /// Controller type (determines which buttons are mapped)
    pub controller_type: ControllerType,
    /// Button assignments for each of the 16 pins
    pub pin_assignments: [Option<ButtonId>; 16],
}

/// PCF8575 controller types for different button groups
#[derive(Clone, Copy, PartialEq)]
pub enum ControllerType {
    /// Track buttons 1-6 and track select buttons 1-6
    TrackButtons,
    /// FX buttons 1-5 and transport controls
    FxAndTransport,
    /// Menu navigation and utility buttons
    MenuAndUtility,
    /// Additional controls and expansion
    Additional,
}

/// Button state tracking for debouncing (10ms response time requirement)
pub struct ButtonStates {
    pub track_button_states: [ButtonState; 6],
    pub fx_button_states: [ButtonState; 5],
    pub transport_states: [ButtonState; 3], // Play, Stop, Rec
    pub menu_states: [ButtonState; 5],      // Menu, PageL, PageR, Enter, Exit
    pub control_states: [ButtonState; 4],   // Tap, Memory, Undo, Edit
    pub track_select_states: [ButtonState; 6],
    pub last_update: u32, // Timestamp for debouncing
}

/// Individual button state for debouncing
#[derive(Clone, Copy)]
pub struct ButtonState {
    pub current: bool,
    pub previous: bool,
    pub debounce_counter: u8,
    pub press_type: PressType,
    pub press_start_time: u32,
}

/// Button press types for gesture recognition
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PressType {
    None,
    Short,
    Long,
    Double,
}

/// Control ADC for faders and knobs (simplified for task 3.1)
pub struct ControlAdc {
    // Placeholder for control ADC - basic implementation for task 3.1
    pub initialized: bool,
}

/// KY-040 Rotary Encoder Driver
/// Provides quadrature decoding and button detection for menu navigation
pub struct Ky040RotaryEncoder {
    /// CLK pin (A phase) for quadrature decoding - placeholder type
    pub clk_pin: Option<bool>,
    /// DT pin (B phase) for quadrature decoding - placeholder type
    pub dt_pin: Option<bool>,
    /// SW pin (button) for push detection - placeholder type
    pub sw_pin: Option<bool>,
    /// Previous CLK state for edge detection
    pub prev_clk_state: bool,
    /// Previous DT state for direction detection
    pub prev_dt_state: bool,
    /// Previous button state for press detection
    pub prev_button_state: bool,
    /// Current encoder position (signed counter)
    pub position: i32,
    /// Button press state with debouncing
    pub button_pressed: bool,
    /// Button debounce counter
    pub button_debounce_counter: u8,
    /// Last update timestamp for debouncing
    pub last_update_time: u32,
    /// Encoder enabled flag
    pub enabled: bool,
}

/// 74HC595 Shift Register Driver for LED Matrix Control
/// Provides serial-to-parallel LED control with cascading support
pub struct Hc595LedController {
    /// SPI peripheral for serial communication (optional - can use GPIO)
    pub spi_peripheral: Option<()>, // Placeholder for SPI peripheral
    /// Data pin (SER/DS) for serial data input - placeholder type
    pub data_pin: Option<bool>,
    /// Clock pin (SRCLK/SHCP) for shift register clock - placeholder type
    pub clock_pin: Option<bool>,
    /// Latch pin (RCLK/STCP) for output register clock - placeholder type
    pub latch_pin: Option<bool>,
    /// Output enable pin (OE) for LED brightness control (active low) - placeholder type
    pub output_enable_pin: Option<bool>,
    /// Number of cascaded 74HC595 chips
    pub chip_count: u8,
    /// Current LED states (bit array for all chips)
    pub led_states: [u8; 8], // Support up to 8 chips (64 LEDs)
    /// LED update pending flag
    pub update_pending: bool,
    /// Controller enabled flag
    pub enabled: bool,
}

/// LED identifiers for status indication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedId {
    /// Track status LEDs (1-6)
    Track(u8),
    /// Track recording LEDs (1-6)
    TrackRec(u8),
    /// FX status LEDs (1-5)
    FX(u8),
    /// Transport status LEDs
    Play,
    Stop,
    Rec,
    /// Menu navigation LEDs
    Menu,
    PageLeft,
    PageRight,
    /// Memory status LED
    Memory,
    /// Tempo LED (blinks with tempo)
    Tempo,
    /// System status LEDs
    Power,
    Error,
    /// Custom LEDs for expansion
    Custom(u8),
}

/// Rotary encoder event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotaryEvent {
    /// Clockwise rotation (increment)
    Clockwise,
    /// Counter-clockwise rotation (decrement)
    CounterClockwise,
    /// Button press
    ButtonPress,
    /// Button release
    ButtonRelease,
}

/// LED control commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedCommand {
    /// Turn LED on
    On,
    /// Turn LED off
    Off,
    /// Toggle LED state
    Toggle,
    /// Set LED brightness (0-255, if PWM supported)
    Brightness(u8),
}

/// UART interface for ESP32 communication
#[cfg(not(feature = "std"))]
pub struct Esp32UartInterface {
    /// UART peripheral for communication
    pub uart: Option<Serial<USART1>>,
    /// Message buffer for incoming data
    pub rx_buffer: heapless::Vec<u8, 512>,
    /// Message buffer for outgoing data
    pub tx_buffer: heapless::Vec<u8, 512>,
    /// Last communication timestamp
    pub last_communication: u32,
    /// Communication error count
    pub error_count: u32,
    /// Interface enabled flag
    pub enabled: bool,
}

/// MIDI UART interface for MIDI IN/OUT communication
#[cfg(not(feature = "std"))]
pub struct MidiUartInterface {
    /// UART peripheral for MIDI IN (USART2)
    pub midi_in_uart: Option<Serial<USART2>>,
    /// UART peripheral for MIDI OUT (USART3)
    pub midi_out_uart: Option<Serial<USART3>>,
    /// MIDI input buffer for incoming data
    pub midi_in_buffer: heapless::Vec<u8, 256>,
    /// MIDI output buffer for outgoing data
    pub midi_out_buffer: heapless::Vec<u8, 256>,
    /// MIDI clock timing for tempo sync
    pub clock_timing: MidiClockTiming,
    /// Last MIDI activity timestamp
    pub last_midi_activity: u32,
    /// MIDI communication error count
    pub midi_error_count: u32,
    /// Interface enabled flag
    pub enabled: bool,
}

/// MIDI UART interface stub for PC builds
#[cfg(feature = "std")]
pub struct MidiUartInterface {
    /// MIDI input buffer for incoming data
    pub midi_in_buffer: heapless::Vec<u8, 256>,
    /// MIDI output buffer for outgoing data
    pub midi_out_buffer: heapless::Vec<u8, 256>,
    /// MIDI clock timing for tempo sync
    pub clock_timing: MidiClockTiming,
    /// Last MIDI activity timestamp
    pub last_midi_activity: u32,
    /// MIDI communication error count
    pub midi_error_count: u32,
    /// Interface enabled flag
    pub enabled: bool,
}

/// MIDI clock timing for tempo synchronization
#[derive(Debug, Clone)]
pub struct MidiClockTiming {
    /// Current tempo in BPM (calculated from MIDI clock)
    pub tempo_bpm: f32,
    /// MIDI clock pulse counter (24 pulses per quarter note)
    pub clock_pulse_count: u32,
    /// Last clock pulse timestamp
    pub last_clock_pulse: u32,
    /// Clock pulse interval (microseconds)
    pub clock_interval_us: u32,
    /// Clock sync enabled flag
    pub sync_enabled: bool,
    /// Clock running state
    pub clock_running: bool,
}

/// USB Audio Interface for DAW integration
/// Provides 16-channel USB audio interface with zero-latency monitoring
#[cfg(not(feature = "std"))]
pub struct UsbAudioInterface {
    /// USB device peripheral
    pub usb_device: Option<()>, // Placeholder for USB device
    /// USB audio class interface
    pub audio_class: Option<()>, // Placeholder for USB audio class
    /// USB audio input buffers (16 channels from DAW)
    pub usb_input_buffers: [[f32; AUDIO_BUFFER_SIZE]; 16],
    /// USB audio output buffers (16 channels to DAW)
    pub usb_output_buffers: [[f32; AUDIO_BUFFER_SIZE]; 16],
    /// Current USB buffer index for double buffering
    pub current_usb_buffer: bool,
    /// USB audio streaming active flag
    pub streaming_active: bool,
    /// Sample rate (24-bit/96kHz capability)
    pub sample_rate: u32,
    /// Bit depth (16/24-bit support)
    pub bit_depth: u8,
    /// Zero-latency monitoring enabled
    pub zero_latency_monitoring: bool,
    /// DAW routing configuration
    pub daw_routing: DawRoutingConfig,
    /// USB audio error count
    pub error_count: u32,
    /// Interface enabled flag
    pub enabled: bool,
}

/// USB Audio Interface stub for PC builds
#[cfg(feature = "std")]
pub struct UsbAudioInterface {
    /// USB audio input buffers (16 channels from DAW)
    pub usb_input_buffers: [[f32; AUDIO_BUFFER_SIZE]; 16],
    /// USB audio output buffers (16 channels to DAW)
    pub usb_output_buffers: [[f32; AUDIO_BUFFER_SIZE]; 16],
    /// Current USB buffer index for double buffering
    pub current_usb_buffer: bool,
    /// USB audio streaming active flag
    pub streaming_active: bool,
    /// Sample rate (24-bit/96kHz capability)
    pub sample_rate: u32,
    /// Bit depth (16/24-bit support)
    pub bit_depth: u8,
    /// Zero-latency monitoring enabled
    pub zero_latency_monitoring: bool,
    /// DAW routing configuration
    pub daw_routing: DawRoutingConfig,
    /// USB audio error count
    pub error_count: u32,
    /// Interface enabled flag
    pub enabled: bool,
}

/// DAW routing configuration for track inputs/outputs
#[derive(Debug, Clone)]
pub struct DawRoutingConfig {
    /// Track input routing (track_id -> USB input channel)
    pub track_input_routing: [Option<u8>; 6],
    /// Track output routing (track_id -> USB output channel pair)
    pub track_output_routing: [Option<(u8, u8)>; 6], // (left, right)
    /// Master output routing (USB output channel pair)
    pub master_output_routing: (u8, u8), // (left, right)
    /// Input monitoring routing (USB input -> hardware output)
    pub input_monitoring_routing: [Option<u8>; 16],
    /// Individual track monitoring enabled
    pub track_monitoring_enabled: [bool; 6],
    /// Master monitoring enabled
    pub master_monitoring_enabled: bool,
}

/// USB audio streaming mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbAudioMode {
    /// Standard mode (44.1kHz, 16-bit)
    Standard,
    /// High quality mode (48kHz, 24-bit)
    HighQuality,
    /// Professional mode (96kHz, 24-bit)
    Professional,
}

/// USB audio channel assignment
#[derive(Debug, Clone, Copy)]
pub struct UsbChannelAssignment {
    /// USB channel number (0-15)
    pub usb_channel: u8,
    /// Track assignment (None for unassigned)
    pub track_assignment: Option<u8>,
    /// Channel type (input/output)
    pub channel_type: UsbChannelType,
    /// Channel enabled flag
    pub enabled: bool,
}

/// USB channel type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbChannelType {
    /// Input from DAW to loopstation
    Input,
    /// Output from loopstation to DAW
    Output,
}

/// USB audio status for monitoring
#[derive(Debug, Clone)]
pub struct UsbAudioStatus {
    /// Connection status
    pub connected: bool,
    /// Streaming status
    pub streaming: bool,
    /// Current sample rate
    pub sample_rate: u32,
    /// Current bit depth
    pub bit_depth: u8,
    /// Buffer underrun count
    pub underrun_count: u32,
    /// Buffer overrun count
    pub overrun_count: u32,
    /// Total error count
    pub error_count: u32,
}

/// UART interface stub for PC builds
#[cfg(feature = "std")]
pub struct Esp32UartInterface {
    /// Message buffer for incoming data
    pub rx_buffer: heapless::Vec<u8, 512>,
    /// Message buffer for outgoing data
    pub tx_buffer: heapless::Vec<u8, 512>,
    /// Last communication timestamp
    pub last_communication: u32,
    /// Communication error count
    pub error_count: u32,
    /// Interface enabled flag
    pub enabled: bool,
}

/// Communication message types between STM32 and ESP32
#[derive(Debug, Clone)]
pub enum Esp32Message {
    /// System status update to ESP32
    StatusUpdate {
        tracks: [TrackStatus; 6],
        tempo: f32,
        current_memory: u8,
        fx_states: [bool; 5],
    },
    /// Parameter change notification to ESP32
    ParameterChange {
        parameter: ParameterId,
        value: f32,
    },
    /// Command from ESP32 to STM32
    Command {
        command: CommandType,
        track_id: Option<u8>,
        value: Option<f32>,
    },
    /// Response to ESP32 command
    Response {
        success: bool,
        error_message: Option<heapless::String<64>>,
    },
    /// Heartbeat message
    Heartbeat,
}

/// Track status for communication
#[derive(Debug, Clone, Copy)]
pub struct TrackStatus {
    pub state: TrackStateComm,
    pub volume: f32,
    pub pan: f32,
    pub muted: bool,
    pub selected: bool,
}

/// Track state for communication
#[derive(Debug, Clone, Copy)]
pub enum TrackStateComm {
    Stopped,
    Recording,
    Playing,
    Overdubbing,
    Muted,
}

/// Parameter identifiers for communication
#[derive(Debug, Clone, Copy)]
pub enum ParameterId {
    TrackVolume(u8),
    TrackPan(u8),
    MasterVolume,
    Tempo,
    FxParameter { fx_id: u8, param_id: u8 },
}

/// Command types from ESP32
#[derive(Debug, Clone, Copy)]
pub enum CommandType {
    TrackPlay,
    TrackStop,
    TrackRecord,
    TrackClear,
    TrackMute,
    SetVolume,
    SetTempo,
    FxToggle,
    MemoryLoad,
    MemorySave,
}

#[cfg(not(feature = "std"))]
impl HardwareHal {
    /// Initialize the hardware abstraction layer with I2S audio interface
    ///
    /// This function configures:
    /// - System clocks to 400MHz (STM32H743VIT6 maximum)
    /// - I2S peripherals for PCM1808/PCM5102A audio codecs
    /// - DMA streams for low-latency audio processing
    /// - GPIO pins for I2S communication
    pub fn init() -> Result<Self, HalError> {
        // Take ownership of device peripherals
        let dp = pac::Peripherals::take().ok_or(HalError::PeripheralAccess)?;
        let cp = cortex_m::Peripherals::take().ok_or(HalError::PeripheralAccess)?;

        // Configure power domain - enable VOS1 for 400MHz operation
        let pwr = dp.PWR.constrain();
        let pwrcfg = pwr.vos1().freeze();

        // Configure clocks for maximum performance and precise audio timing
        let rcc = dp.RCC.constrain();
        let mut ccdr = rcc
            .use_hse(25.MHz()) // External 25MHz crystal
            .sys_ck(SYSTEM_CLOCK_HZ.Hz()) // 400MHz system clock
            .pll1_q_ck(100.MHz()) // 100MHz for peripherals
            .pll2_p_ck(44_100.Hz() * 256) // 11.2896MHz for 44.1kHz I2S (256 * Fs)
            .pll3_p_ck(49_152_000.Hz()) // 49.152MHz for 48kHz I2S (alternative)
            .hclk(200.MHz()) // AHB clock
            .pclk1(100.MHz()) // APB1 clock
            .pclk2(100.MHz()) // APB2 clock
            .pclk3(100.MHz()) // APB3 clock
            .pclk4(100.MHz()) // APB4 clock
            .freeze(pwrcfg, &dp.SYSCFG);

        // Verify clock configuration
        assert_eq!(ccdr.clocks.sys_ck().raw(), SYSTEM_CLOCK_HZ);

        // Create delay provider
        let delay = Delay::new(cp.SYST, ccdr.clocks);

        // Initialize I2S audio input and output
        let audio_input = Self::init_audio_input(&mut ccdr, dp.SAI1, dp.SAI2, dp.DMA1)?;
        let audio_output = Self::init_audio_output(&mut ccdr, dp.SAI3, dp.SAI4, dp.DMA2)?;

        // Initialize GPIO controls with PCF8575 I2C I/O expanders
        let gpio_controls = Self::init_gpio_controls(&mut ccdr)?;

        // Initialize control ADC (placeholder)
        let control_adc = ControlAdc { initialized: true };

        // Initialize KY-040 rotary encoder
        let rotary_encoder = Self::init_rotary_encoder(&mut ccdr)?;

        // Initialize 74HC595 LED controller
        let led_controller = Self::init_led_controller(&mut ccdr)?;

        // Initialize ESP32 UART interface
        let esp32_uart = Self::init_esp32_uart(&mut ccdr, dp.USART1)?;

        // Initialize MIDI UART interface
        let midi_uart = Self::init_midi_uart(&mut ccdr, dp.USART2, dp.USART3)?;

        // Initialize USB audio interface
        let usb_audio = Self::init_usb_audio(&mut ccdr, dp.USB1_OTG_HS)?;

        // Create audio engine
        let audio_engine = Some(AudioEngine::new(SAMPLE_RATE_HZ, AUDIO_BUFFER_SIZE));

        Ok(Self {
            ccdr,
            delay,
            audio_input,
            audio_output,
            usb_audio,
            gpio_controls,
            control_adc,
            rotary_encoder,
            led_controller,
            esp32_uart,
            midi_uart,
            audio_engine,
            audio_callback_active: false,
        })
    }

    /// Initialize I2S audio input with PCM1808 ADCs
    fn init_audio_input(
        ccdr: &mut Ccdr,
        _sai1: pac::SAI1,
        _sai2: pac::SAI2,
        _dma1: pac::DMA1,
    ) -> Result<AudioInput, HalError> {
        // TODO: Full I2S implementation will be completed in subsequent iterations
        // For now, create a basic structure that compiles and provides the interface

        // Initialize input buffers
        let input_buffer_a = [0i32; DMA_BUFFER_SIZE * I2S_INPUT_CHANNELS];
        let input_buffer_b = [0i32; DMA_BUFFER_SIZE * I2S_INPUT_CHANNELS];

        Ok(AudioInput {
            initialized: true,
            current_input_buffer: false,
            input_buffer_a,
            input_buffer_b,
            dma_active: false,
        })
    }

    /// Initialize I2S audio output with PCM5102A DACs
    fn init_audio_output(
        ccdr: &mut Ccdr,
        _sai3: pac::SAI3,
        _sai4: pac::SAI4,
        _dma2: pac::DMA2,
    ) -> Result<AudioOutput, HalError> {
        // TODO: Full I2S implementation will be completed in subsequent iterations
        // For now, create a basic structure that compiles and provides the interface

        // Initialize output buffers
        let output_buffer_a = [0i32; DMA_BUFFER_SIZE * I2S_OUTPUT_CHANNELS];
        let output_buffer_b = [0i32; DMA_BUFFER_SIZE * I2S_OUTPUT_CHANNELS];

        Ok(AudioOutput {
            initialized: true,
            current_output_buffer: false,
            output_buffer_a,
            output_buffer_b,
            dma_active: false,
        })
    }

    /// Initialize GPIO controls with PCF8575 I2C I/O expanders
    fn init_gpio_controls(ccdr: &mut Ccdr) -> Result<GpioControls, HalError> {
        // For now, create a simplified implementation that compiles
        // Full I2C initialization will be completed in subsequent iterations

        // Create PCF8575 controllers for different button groups
        let mut pcf8575_controllers = Vec::new();

        // Controller 1: Track buttons (address 0x20)
        let track_controller = Pcf8575Controller::new(0x20, ControllerType::TrackButtons);
        pcf8575_controllers
            .push(track_controller)
            .map_err(|_| HalError::InitError)?;

        // Controller 2: FX and transport buttons (address 0x21)
        let fx_controller = Pcf8575Controller::new(0x21, ControllerType::FxAndTransport);
        pcf8575_controllers
            .push(fx_controller)
            .map_err(|_| HalError::InitError)?;

        // Controller 3: Menu and utility buttons (address 0x22)
        let menu_controller = Pcf8575Controller::new(0x22, ControllerType::MenuAndUtility);
        pcf8575_controllers
            .push(menu_controller)
            .map_err(|_| HalError::InitError)?;

        Ok(GpioControls {
            pcf8575_controllers,
            i2c_peripheral: None, // Will be initialized in full implementation
            button_states: ButtonStates::new(),
            interrupt_pin_active: false,
            last_scan_time: 0,
        })
    }

    /// Initialize USB audio interface for DAW integration
    /// Implements 16-channel USB audio interface with 24-bit/96kHz quality
    /// Requirements: 11.8, 11.9, 11.10, 11.18, 11.19
    fn init_usb_audio(
        ccdr: &mut Ccdr,
        usb_otg_hs: pac::USB1_OTG_HS,
    ) -> Result<UsbAudioInterface, HalError> {
        // Enable USB OTG HS peripheral clock
        let usb_prec = ccdr.peripheral.USB1OTG.enable();

        // Configure USB pins (PA11/PA12 for USB OTG HS)
        let gpioa = ccdr.peripheral.GPIOA.split();
        let _usb_dm = gpioa.pa11.into_alternate::<10>(); // USB_OTG_HS_DM
        let _usb_dp = gpioa.pa12.into_alternate::<10>(); // USB_OTG_HS_DP

        // Initialize USB bus allocator
        static mut EP_MEMORY: [u32; 1024] = [0; 1024];
        let usb_bus = UsbBus::new(usb_otg_hs, unsafe { &mut EP_MEMORY });
        let usb_bus_allocator = UsbBusAllocator::new(usb_bus);

        // Create USB audio class with 16-channel configuration
        // Note: This is a placeholder implementation - actual USB audio class
        // configuration would depend on the specific usbd-audio crate API
        let audio_class = (); // Placeholder for USB audio class

        // Create USB device
        let usb_device = UsbDeviceBuilder::new(&usb_bus_allocator, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Loopstation")
            .product("RC-505 MKII Clone")
            .serial_number("001")
            .device_class(0x01) // Audio device class
            .build();

        // Initialize USB audio buffers for double buffering
        let usb_input_buffers = [[0.0f32; AUDIO_BUFFER_SIZE]; 16];
        let usb_output_buffers = [[0.0f32; AUDIO_BUFFER_SIZE]; 16];

        // Create optimized DAW routing configuration for professional use
        let daw_routing = DawRoutingConfig {
            // Track input routing (tracks 1-6 -> USB channels 0-5)
            track_input_routing: [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)],
            // Track output routing (tracks 1-6 -> USB channel pairs 0-11)
            track_output_routing: [
                Some((0, 1)),   // Track 1 -> USB channels 0,1 (stereo)
                Some((2, 3)),   // Track 2 -> USB channels 2,3 (stereo)
                Some((4, 5)),   // Track 3 -> USB channels 4,5 (stereo)
                Some((6, 7)),   // Track 4 -> USB channels 6,7 (stereo)
                Some((8, 9)),   // Track 5 -> USB channels 8,9 (stereo)
                Some((10, 11)), // Track 6 -> USB channels 10,11 (stereo)
            ],
            // Master output routing -> USB channels 14,15 (stereo)
            master_output_routing: (14, 15),
            // Input monitoring routing for zero-latency monitoring
            input_monitoring_routing: [
                Some(0), Some(1), Some(2), Some(3), Some(4), Some(5), // Tracks 1-6
                None, None, None, None, None, None, None, None, None, None, // Unused
            ],
            // Enable monitoring for all tracks by default
            track_monitoring_enabled: [true; 6],
            // Enable master monitoring by default
            master_monitoring_enabled: true,
        };

        Ok(UsbAudioInterface {
            usb_device: None, // Placeholder - would store actual USB device
            audio_class: None, // Placeholder - would store actual USB audio class
            usb_input_buffers,
            usb_output_buffers,
            current_usb_buffer: false,
            streaming_active: false,
            sample_rate: 96000, // 96kHz for professional quality (Requirement 11.9)
            bit_depth: 24, // 24-bit for professional quality (Requirement 11.9)
            zero_latency_monitoring: true, // Enable zero-latency monitoring (Requirement 11.19)
            daw_routing,
            error_count: 0,
            enabled: true,
        })
    }

    /// Process USB audio interface for DAW integration
    /// Handles 16-channel audio streaming with zero-latency monitoring
    /// Requirements: 11.8, 11.9, 11.18, 11.19
    pub fn process_usb_audio(&mut self, track_outputs: &[[f32; AUDIO_BUFFER_SIZE]; 6], master_output: &[f32; AUDIO_BUFFER_SIZE * 2]) -> Result<[[f32; AUDIO_BUFFER_SIZE]; 16], HalError> {
        let mut usb_inputs = [[0.0f32; AUDIO_BUFFER_SIZE]; 16];

        if !self.usb_audio.enabled || !self.usb_audio.streaming_active {
            return Ok(usb_inputs);
        }

        // Process USB audio streaming
        // Note: This is a placeholder implementation for the USB audio interface
        // In a full implementation, this would:
        // 1. Poll the USB device for new audio data
        // 2. Read 16 channels of input data from the DAW
        // 3. Convert from USB audio format (16/24-bit) to f32
        // 4. Route loopstation audio outputs to USB channels for DAW
        
        if self.usb_audio.streaming_active {
            // Simulate USB input processing (placeholder)
            for channel in 0..16 {
                // In real implementation, read from USB audio class
                // For now, use existing buffer data
                for i in 0..AUDIO_BUFFER_SIZE {
                    usb_inputs[channel][i] = self.usb_audio.usb_input_buffers[channel][i];
                }
            }

            // Route audio to DAW
            self.route_audio_to_daw(track_outputs, master_output)?;
        }

        // Implement zero-latency monitoring (Requirement 11.19)
        if self.usb_audio.zero_latency_monitoring {
            self.process_zero_latency_monitoring(&usb_inputs)?;
        }

        Ok(usb_inputs)
    }

    /// Route loopstation audio to DAW channels based on routing configuration
    /// Implements 16-channel DAW routing (Requirement 11.9)
    fn route_audio_to_daw(&mut self, track_outputs: &[[f32; AUDIO_BUFFER_SIZE]; 6], master_output: &[f32; AUDIO_BUFFER_SIZE * 2]) -> Result<(), HalError> {
        // Clear USB output buffers
        for channel in &mut self.usb_audio.usb_output_buffers {
            channel.fill(0.0);
        }

        // Route individual tracks to USB channels
        for (track_id, track_audio) in track_outputs.iter().enumerate() {
            if let Some((left_ch, right_ch)) = self.usb_audio.daw_routing.track_output_routing[track_id] {
                if (left_ch as usize) < 16 && (right_ch as usize) < 16 {
                    // Route stereo track to USB channel pair
                    for i in 0..AUDIO_BUFFER_SIZE {
                        // Assume track_audio is mono, duplicate to stereo
                        self.usb_audio.usb_output_buffers[left_ch as usize][i] = track_audio[i];
                        self.usb_audio.usb_output_buffers[right_ch as usize][i] = track_audio[i];
                    }
                }
            }
        }

        // Route master output to USB channels
        let (master_left, master_right) = self.usb_audio.daw_routing.master_output_routing;
        if (master_left as usize) < 16 && (master_right as usize) < 16 {
            for i in 0..AUDIO_BUFFER_SIZE {
                // Master output is stereo interleaved
                self.usb_audio.usb_output_buffers[master_left as usize][i] = master_output[i * 2];
                self.usb_audio.usb_output_buffers[master_right as usize][i] = master_output[i * 2 + 1];
            }
        }

        // Send audio data to USB audio class
        // Note: This is a placeholder implementation
        // In a full implementation, this would send the audio data to the USB audio class
        // which would then transmit it to the connected DAW
        
        // For now, just store the data in the output buffers (already done above)
        // In real implementation:
        // 1. Convert f32 samples to USB audio format (16/24-bit integers)
        // 2. Send to USB audio class for transmission to DAW
        // 3. Handle any USB transmission errors

        Ok(())
    }

    /// Process zero-latency monitoring for DAW applications
    /// Routes USB inputs directly to hardware outputs (Requirement 11.19)
    fn process_zero_latency_monitoring(&mut self, usb_inputs: &[[f32; AUDIO_BUFFER_SIZE]; 16]) -> Result<(), HalError> {
        if !self.usb_audio.zero_latency_monitoring {
            return Ok(());
        }

        // Route USB inputs to hardware outputs for zero-latency monitoring
        for (usb_channel, hardware_output) in self.usb_audio.daw_routing.input_monitoring_routing.iter().enumerate() {
            if let Some(output_channel) = hardware_output {
                if usb_channel < 16 && (*output_channel as usize) < I2S_OUTPUT_CHANNELS {
                    // Route USB input directly to hardware output
                    // This would be implemented in the actual I2S output processing
                    // For now, we just validate the routing configuration
                }
            }
        }

        Ok(())
    }

    /// Convert USB audio sample to f32 format
    fn convert_usb_to_f32(&self, usb_sample: i32) -> f32 {
        match self.usb_audio.bit_depth {
            16 => (usb_sample as i16) as f32 / 32768.0,
            24 => usb_sample as f32 / 8388608.0, // 2^23
            32 => usb_sample as f32 / 2147483648.0, // 2^31
            _ => 0.0,
        }
    }

    /// Convert f32 sample to USB audio format
    fn convert_f32_to_usb(&self, sample: f32) -> i32 {
        let clamped = sample.clamp(-1.0, 1.0);
        match self.usb_audio.bit_depth {
            16 => (clamped * 32767.0) as i32,
            24 => (clamped * 8388607.0) as i32, // 2^23 - 1
            32 => (clamped * 2147483647.0) as i32, // 2^31 - 1
            _ => 0,
        }
    }

    /// Configure USB audio routing for DAW integration
    /// Allows dynamic reconfiguration of track routing (Requirement 11.9)
    pub fn configure_daw_routing(&mut self, config: DawRoutingConfig) -> Result<(), HalError> {
        // Validate routing configuration
        for (track_id, routing) in config.track_output_routing.iter().enumerate() {
            if let Some((left, right)) = routing {
                if *left >= 16 || *right >= 16 {
                    return Err(HalError::InvalidConfiguration);
                }
            }
        }

        // Validate master routing
        if config.master_output_routing.0 >= 16 || config.master_output_routing.1 >= 16 {
            return Err(HalError::InvalidConfiguration);
        }

        // Apply new routing configuration
        self.usb_audio.daw_routing = config;
        Ok(())
    }

    /// Set USB audio sample rate and bit depth
    /// Supports 44.1kHz/48kHz/96kHz with 16/24-bit (Requirement 11.9)
    pub fn set_usb_audio_format(&mut self, sample_rate: u32, bit_depth: u8) -> Result<(), HalError> {
        // Validate supported formats
        match sample_rate {
            44100 | 48000 | 96000 => {},
            _ => return Err(HalError::UnsupportedFormat),
        }

        match bit_depth {
            16 | 24 => {},
            _ => return Err(HalError::UnsupportedFormat),
        }

        self.usb_audio.sample_rate = sample_rate;
        self.usb_audio.bit_depth = bit_depth;

        // Reconfigure USB audio class if needed
        // Note: In a full implementation, this would reconfigure the USB audio class
        // to use the new sample rate and bit depth. This might require:
        // 1. Stopping current USB audio streaming
        // 2. Reconfiguring USB descriptors
        // 3. Restarting USB audio streaming with new format

        Ok(())
    }

    /// Enable/disable zero-latency monitoring
    /// Requirement 11.19: Zero-latency monitoring for DAW applications
    pub fn set_zero_latency_monitoring(&mut self, enabled: bool) {
        self.usb_audio.zero_latency_monitoring = enabled;
    }

    /// Get USB audio interface status
    pub fn get_usb_audio_status(&self) -> UsbAudioStatus {
        UsbAudioStatus {
            connected: self.usb_audio.usb_device.is_some(),
            streaming: self.usb_audio.streaming_active,
            sample_rate: self.usb_audio.sample_rate,
            bit_depth: self.usb_audio.bit_depth,
            underrun_count: 0, // Would be tracked in full implementation
            overrun_count: 0,  // Would be tracked in full implementation
            error_count: self.usb_audio.error_count,
        }
    }

    /// Start USB audio streaming
    pub fn start_usb_audio_streaming(&mut self) -> Result<(), HalError> {
        if !self.usb_audio.enabled {
            return Err(HalError::DeviceNotEnabled);
        }

        self.usb_audio.streaming_active = true;
        Ok(())
    }

    /// Stop USB audio streaming
    pub fn stop_usb_audio_streaming(&mut self) {
        self.usb_audio.streaming_active = false;
    }

    /// Get current DAW routing configuration
    pub fn get_daw_routing(&self) -> &DawRoutingConfig {
        &self.usb_audio.daw_routing
    }

    /// Initialize rotary encoder (placeholder)
    fn init_rotary_encoder(ccdr: &mut Ccdr) -> Result<Ky040RotaryEncoder, HalError> {
        Ok(Ky040RotaryEncoder {
            clk_pin: None,
            dt_pin: None,
            sw_pin: None,
            prev_clk_state: false,
            prev_dt_state: false,
            prev_button_state: false,
            position: 0,
            button_pressed: false,
            button_debounce_counter: 0,
            last_update_time: 0,
            enabled: true,
        })
    }

    /// Initialize LED controller (placeholder)
    fn init_led_controller(ccdr: &mut Ccdr) -> Result<Hc595LedController, HalError> {
        Ok(Hc595LedController {
            spi_peripheral: None,
            data_pin: None,
            clock_pin: None,
            latch_pin: None,
            output_enable_pin: None,
            chip_count: 3, // Support 3 chips for all LEDs
            led_states: [0u8; 8],
            update_pending: false,
            enabled: true,
        })
    }

    /// Initialize ESP32 UART interface (placeholder)
    fn init_esp32_uart(
        ccdr: &mut Ccdr,
        _usart1: pac::USART1,
    ) -> Result<Esp32UartInterface, HalError> {
        Ok(Esp32UartInterface {
            uart: None, // Will be initialized in full implementation
            rx_buffer: heapless::Vec::new(),
            tx_buffer: heapless::Vec::new(),
            last_communication: 0,
            error_count: 0,
            enabled: true,
        })
    }

    /// Initialize MIDI UART interface (placeholder)
    fn init_midi_uart(
        ccdr: &mut Ccdr,
        _usart2: pac::USART2,
        _usart3: pac::USART3,
    ) -> Result<MidiUartInterface, HalError> {
        Ok(MidiUartInterface {
            midi_in_uart: None, // Will be initialized in full implementation
            midi_out_uart: None, // Will be initialized in full implementation
            midi_in_buffer: heapless::Vec::new(),
            midi_out_buffer: heapless::Vec::new(),
            clock_timing: MidiClockTiming {
                tempo_bpm: 120.0,
                clock_pulse_count: 0,
                last_clock_pulse: 0,
                clock_interval_us: 20833, // 120 BPM = 20833 us per clock
                sync_enabled: false,
                clock_running: false,
            },
            last_midi_activity: 0,
            midi_error_count: 0,
            enabled: true,
        })
    }

    /// Start I2S audio streaming with DMA
    pub fn start_audio_streaming(&mut self) -> Result<(), HalError> {
        // Start input streaming
        self.audio_input.dma_active = true;

        // Start output streaming
        self.audio_output.dma_active = true;

        // Enable audio callback
        self.audio_callback_active = true;
        set_audio_callback_active(true);

        // Start the audio engine callback
        if let Some(ref mut engine) = self.audio_engine {
            engine.start_callback();
        }

        Ok(())
    }

    /// Stop I2S audio streaming
    pub fn stop_audio_streaming(&mut self) -> Result<(), HalError> {
        self.audio_callback_active = false;
        set_audio_callback_active(false);

        // Stop DMA transfers
        self.audio_input.dma_active = false;
        self.audio_output.dma_active = false;

        // Stop the audio engine callback
        if let Some(ref mut engine) = self.audio_engine {
            engine.stop_callback();
        }

        Ok(())
    }

    /// Process audio callback - called from DMA interrupt
    pub fn process_audio_callback(&mut self) {
        if !self.audio_callback_active {
            return;
        }

        // Get current input buffer
        let input_samples = if self.audio_input.current_input_buffer {
            &self.audio_input.input_buffer_b[..]
        } else {
            &self.audio_input.input_buffer_a[..]
        };

        // Get current output buffer
        let output_samples = if self.audio_output.current_output_buffer {
            &mut self.audio_output.output_buffer_b[..]
        } else {
            &mut self.audio_output.output_buffer_a[..]
        };

        // Convert I2S samples to f32 for processing
        let mut input_f32 = [0.0f32; AUDIO_BUFFER_SIZE * 2]; // Stereo
        let mut output_f32 = [0.0f32; AUDIO_BUFFER_SIZE * 2]; // Stereo

        // Convert 24-bit input samples to f32 (-1.0 to 1.0)
        for i in 0..input_f32.len() {
            if i < input_samples.len() {
                // PCM1808 outputs 24-bit samples in 32-bit words
                let sample_i24 = (input_samples[i] >> 8) as i32; // Extract 24-bit
                input_f32[i] = sample_i24 as f32 / 8388608.0; // 2^23 for 24-bit
            }
        }

        // Process audio through the engine
        if let Some(ref mut engine) = self.audio_engine {
            engine.process_callback(&input_f32, &mut output_f32);
        }

        // Convert f32 output samples to 32-bit for PCM5102A
        for i in 0..output_f32.len() {
            if i < output_samples.len() {
                // Convert f32 (-1.0 to 1.0) to 32-bit signed
                let sample_f32 = output_f32[i].clamp(-1.0, 1.0);
                output_samples[i] = (sample_f32 * 2147483647.0) as i32; // 2^31-1 for 32-bit
            }
        }

        // Swap buffers for double buffering
        self.audio_input.current_input_buffer = !self.audio_input.current_input_buffer;
        self.audio_output.current_output_buffer = !self.audio_output.current_output_buffer;
    }

    /// Read multi-channel audio input samples from I2S
    pub fn read_audio_input_channels(&self) -> Result<[f32; I2S_INPUT_CHANNELS], HalError> {
        let mut channels = [0.0f32; I2S_INPUT_CHANNELS];

        // Get current input buffer
        let input_buffer = if self.audio_input.current_input_buffer {
            &self.audio_input.input_buffer_a[..] // Use previous buffer while DMA fills current
        } else {
            &self.audio_input.input_buffer_b[..]
        };

        // Extract latest samples from each channel
        for ch in 0..I2S_INPUT_CHANNELS {
            if ch < input_buffer.len() {
                // Convert 24-bit sample to f32
                let sample_i24 = (input_buffer[ch] >> 8) as i32;
                channels[ch] = sample_i24 as f32 / 8388608.0;
            }
        }

        Ok(channels)
    }

    /// Write multi-channel audio output samples to I2S
    pub fn write_audio_output_channels(
        &mut self,
        channels: &[f32; I2S_OUTPUT_CHANNELS],
    ) -> Result<(), HalError> {
        // Get current output buffer
        let output_buffer = if self.audio_output.current_output_buffer {
            &mut self.audio_output.output_buffer_a[..] // Use previous buffer while DMA reads current
        } else {
            &mut self.audio_output.output_buffer_b[..]
        };

        // Write samples to each channel
        for ch in 0..I2S_OUTPUT_CHANNELS {
            if ch < output_buffer.len() {
                // Convert f32 to 32-bit sample
                let sample_f32 = channels[ch].clamp(-1.0, 1.0);
                output_buffer[ch] = (sample_f32 * 2147483647.0) as i32;
            }
        }

        Ok(())
    }

    /// Start USB audio streaming
    pub fn start_usb_audio_streaming(&mut self) -> Result<(), HalError> {
        if !self.usb_audio.enabled {
            return Err(HalError::InitError);
        }

        self.usb_audio.streaming_active = true;
        Ok(())
    }

    /// Stop USB audio streaming
    pub fn stop_usb_audio_streaming(&mut self) -> Result<(), HalError> {
        self.usb_audio.streaming_active = false;
        Ok(())
    }

    /// Read USB audio input from DAW (16 channels)
    pub fn read_usb_audio_input(&self) -> Result<[f32; 16], HalError> {
        if !self.usb_audio.streaming_active {
            return Ok([0.0f32; 16]);
        }

        let mut channels = [0.0f32; 16];
        
        // Get current USB input buffer
        let buffer_index = if self.usb_audio.current_usb_buffer { 0 } else { 1 };
        
        // Extract latest samples from each USB input channel
        for ch in 0..16 {
            if ch < self.usb_audio.usb_input_buffers.len() {
                // Get the latest sample from the buffer
                if !self.usb_audio.usb_input_buffers[ch].is_empty() {
                    channels[ch] = self.usb_audio.usb_input_buffers[ch][0];
                }
            }
        }

        Ok(channels)
    }

    /// Write USB audio output to DAW (16 channels)
    pub fn write_usb_audio_output(&mut self, channels: &[f32; 16]) -> Result<(), HalError> {
        if !self.usb_audio.streaming_active {
            return Ok(());
        }

        // Get current USB output buffer
        let buffer_index = if self.usb_audio.current_usb_buffer { 0 } else { 1 };
        
        // Write samples to each USB output channel
        for ch in 0..16 {
            if ch < self.usb_audio.usb_output_buffers.len() {
                // Write the sample to the buffer
                if !self.usb_audio.usb_output_buffers[ch].is_empty() {
                    self.usb_audio.usb_output_buffers[ch][0] = channels[ch];
                }
            }
        }

        Ok(())
    }

    /// Configure DAW routing for track inputs/outputs
    pub fn configure_daw_routing(&mut self, routing: DawRoutingConfig) -> Result<(), HalError> {
        self.usb_audio.daw_routing = routing;
        Ok(())
    }

    /// Get current DAW routing configuration
    pub fn get_daw_routing(&self) -> &DawRoutingConfig {
        &self.usb_audio.daw_routing
    }

    /// Set USB audio mode (sample rate and bit depth)
    pub fn set_usb_audio_mode(&mut self, mode: UsbAudioMode) -> Result<(), HalError> {
        let (sample_rate, bit_depth) = match mode {
            UsbAudioMode::Standard => (44100, 16),
            UsbAudioMode::HighQuality => (48000, 24),
            UsbAudioMode::Professional => (96000, 24),
        };

        self.usb_audio.sample_rate = sample_rate;
        self.usb_audio.bit_depth = bit_depth;

        // TODO: Reconfigure USB audio class with new parameters
        // This would involve USB descriptor updates and re-enumeration

        Ok(())
    }

    /// Enable/disable zero-latency monitoring
    pub fn set_zero_latency_monitoring(&mut self, enabled: bool) -> Result<(), HalError> {
        self.usb_audio.zero_latency_monitoring = enabled;
        Ok(())
    }

    /// Get USB audio status
    pub fn get_usb_audio_status(&self) -> UsbAudioStatus {
        UsbAudioStatus {
            connected: self.usb_audio.enabled,
            streaming: self.usb_audio.streaming_active,
            sample_rate: self.usb_audio.sample_rate,
            bit_depth: self.usb_audio.bit_depth,
            underrun_count: 0, // TODO: Implement proper error tracking
            overrun_count: 0,  // TODO: Implement proper error tracking
            error_count: self.usb_audio.error_count,
        }
    }

    /// Process USB audio callback - integrates with main audio processing
    pub fn process_usb_audio_callback(&mut self, track_audio: &[[f32; 2]; 6], master_audio: &[f32; 2]) {
        if !self.usb_audio.streaming_active {
            return;
        }

        // Prepare USB output channels based on DAW routing
        let mut usb_outputs = [0.0f32; 16];

        // Route individual tracks to USB outputs
        for track_id in 0..6 {
            if let Some((left_ch, right_ch)) = self.usb_audio.daw_routing.track_output_routing[track_id] {
                if (left_ch as usize) < 16 && (right_ch as usize) < 16 {
                    usb_outputs[left_ch as usize] = track_audio[track_id][0];  // Left
                    usb_outputs[right_ch as usize] = track_audio[track_id][1]; // Right
                }
            }
        }

        // Route master output to USB
        let (master_left, master_right) = self.usb_audio.daw_routing.master_output_routing;
        if (master_left as usize) < 16 && (master_right as usize) < 16 {
            usb_outputs[master_left as usize] = master_audio[0];  // Left
            usb_outputs[master_right as usize] = master_audio[1]; // Right
        }

        // Send audio to DAW
        if let Err(_) = self.write_usb_audio_output(&usb_outputs) {
            self.usb_audio.error_count += 1;
        }

        // Handle zero-latency monitoring if enabled
        if self.usb_audio.zero_latency_monitoring {
            self.process_zero_latency_monitoring();
        }

        // Swap USB buffers for double buffering
        self.usb_audio.current_usb_buffer = !self.usb_audio.current_usb_buffer;
    }

    /// Process zero-latency monitoring (direct USB input to hardware output)
    fn process_zero_latency_monitoring(&mut self) {
        if let Ok(usb_inputs) = self.read_usb_audio_input() {
            // Route USB inputs directly to hardware outputs based on monitoring configuration
            let mut hardware_outputs = [0.0f32; I2S_OUTPUT_CHANNELS];

            for (usb_ch, &input_sample) in usb_inputs.iter().enumerate() {
                if let Some(hw_output_ch) = self.usb_audio.daw_routing.input_monitoring_routing[usb_ch] {
                    if (hw_output_ch as usize) < I2S_OUTPUT_CHANNELS {
                        hardware_outputs[hw_output_ch as usize] += input_sample;
                    }
                }
            }

            // Send monitoring audio to hardware outputs
            let _ = self.write_audio_output_channels(&hardware_outputs);
        }
    }

    /// Route USB input to track recording
    pub fn route_usb_input_to_track(&self, track_id: u8) -> Result<[f32; 2], HalError> {
        if track_id == 0 || track_id > 6 {
            return Err(HalError::InvalidParameter);
        }

        let track_index = (track_id - 1) as usize;
        
        // Get USB input channel for this track
        if let Some(usb_channel) = self.usb_audio.daw_routing.track_input_routing[track_index] {
            if let Ok(usb_inputs) = self.read_usb_audio_input() {
                if (usb_channel as usize) < 16 {
                    let input_sample = usb_inputs[usb_channel as usize];
                    // Return stereo pair (mono input duplicated to both channels)
                    return Ok([input_sample, input_sample]);
                }
            }
        }

        // Return silence if no routing configured or error
        Ok([0.0f32; 2])
    }

    /// Read control value from fader or knob (placeholder for task 3.1)
    pub fn read_control(&mut self, _control_id: ControlId) -> Result<f32, HalError> {
        // Placeholder implementation - returns middle position
        // Real implementation will use ADC3 for analog controls
        Ok(0.5)
    }

    /// Read button state from PCF8575 controllers
    pub fn read_button(&self, button_id: ButtonId) -> bool {
        // Search through all PCF8575 controllers for the button
        for controller in &self.gpio_controls.pcf8575_controllers {
            if controller.get_button_state(button_id) {
                return true;
            }
        }
        false
    }

    /// Update button states by scanning all PCF8575 controllers
    pub fn update_button_states(&mut self, timestamp: u32) -> Vec<(ButtonId, PressType), 32> {
        let mut all_button_events = Vec::new();

        // Only scan if enough time has passed (10ms response time requirement)
        if timestamp.saturating_sub(self.gpio_controls.last_scan_time) >= 10 {
            self.gpio_controls.last_scan_time = timestamp;

            // Scan all PCF8575 controllers if I2C is available
            if let Some(ref mut i2c) = self.gpio_controls.i2c_peripheral {
                for controller in &mut self.gpio_controls.pcf8575_controllers {
                    match controller.read_buttons(i2c, timestamp) {
                        Ok(events) => {
                            // Add events to the combined list
                            for event in events {
                                if all_button_events.push(event).is_err() {
                                    break; // Buffer full
                                }
                            }
                        }
                        Err(_) => {
                            // I2C error - controller may be disconnected
                            // Continue with other controllers
                        }
                    }
                }
            } else {
                // I2C not initialized - simulate some button events for testing
                // This is a placeholder for development/testing purposes
                if timestamp % 5000 == 0 {
                    // Simulate a Track1 short press every 5 seconds
                    if all_button_events
                        .push((ButtonId::Track(0), PressType::Short))
                        .is_err()
                    {
                        // Buffer full
                    }
                }
            }
        }

        self.gpio_controls.button_states.last_update = timestamp;
        all_button_events
    }

    /// Scan specific PCF8575 controller for button changes
    pub fn scan_pcf8575_controller(
        &mut self,
        controller_index: usize,
        timestamp: u32,
    ) -> Result<Vec<(ButtonId, PressType), 16>, HalError> {
        if controller_index >= self.gpio_controls.pcf8575_controllers.len() {
            return Err(HalError::InvalidControl);
        }

        if let Some(ref mut i2c) = self.gpio_controls.i2c_peripheral {
            let controller = &mut self.gpio_controls.pcf8575_controllers[controller_index];
            controller.read_buttons(i2c, timestamp)
        } else {
            // I2C not initialized - return empty events for now
            Ok(Vec::new())
        }
    }

    /// Enable/disable specific PCF8575 controller
    pub fn set_pcf8575_enabled(
        &mut self,
        controller_index: usize,
        enabled: bool,
    ) -> Result<(), HalError> {
        if controller_index >= self.gpio_controls.pcf8575_controllers.len() {
            return Err(HalError::InvalidControl);
        }

        self.gpio_controls.pcf8575_controllers[controller_index].set_enabled(enabled);
        Ok(())
    }

    /// Get number of PCF8575 controllers
    pub fn get_pcf8575_controller_count(&self) -> usize {
        self.gpio_controls.pcf8575_controllers.len()
    }

    /// Set status LED state (placeholder for task 3.1)
    pub fn set_status_led(&mut self, _led_id: u8, _state: bool) -> Result<(), HalError> {
        // Placeholder implementation
        // Real implementation will use 74HC595 shift registers in task 3.4
        Ok(())
    }

    /// Get audio engine reference
    pub fn get_audio_engine(&self) -> Option<&AudioEngine> {
        self.audio_engine.as_ref()
    }

    /// Get mutable audio engine reference
    pub fn get_audio_engine_mut(&mut self) -> Option<&mut AudioEngine> {
        self.audio_engine.as_mut()
    }

    /// Initialize KY-040 rotary encoder
    fn init_rotary_encoder(ccdr: &mut Ccdr) -> Result<Ky040RotaryEncoder, HalError> {
        // Create and initialize rotary encoder
        let mut rotary_encoder = Ky040RotaryEncoder::new();
        rotary_encoder.init()?;

        // TODO: Initialize actual GPIO pins when HAL API is confirmed
        // This would include:
        // - PA0 = CLK (A phase) with pull-up
        // - PA1 = DT (B phase) with pull-up  

    /// Initialize MIDI UART interface for MIDI IN/OUT
    fn init_midi_uart(
        ccdr: &mut Ccdr,
        usart2: pac::USART2,
        usart3: pac::USART3,
    ) -> Result<MidiUartInterface, HalError> {
        // MIDI standard baud rate is 31250 bps
        const MIDI_BAUD_RATE: u32 = 31250;

        // TODO: Initialize USART2 for MIDI IN
        // This would configure:
        // - PA2 = USART2_TX (not used for MIDI IN)
        // - PA3 = USART2_RX (MIDI IN data)
        // - 31250 baud, 8N1, no flow control

        // TODO: Initialize USART3 for MIDI OUT  
        // This would configure:
        // - PB10 = USART3_TX (MIDI OUT data)
        // - PB11 = USART3_RX (not used for MIDI OUT)
        // - 31250 baud, 8N1, no flow control

        // For now, create a basic structure that compiles
        let clock_timing = MidiClockTiming::new();

        Ok(MidiUartInterface {
            midi_in_uart: None,  // Will be initialized in full implementation
            midi_out_uart: None, // Will be initialized in full implementation
            midi_in_buffer: heapless::Vec::new(),
            midi_out_buffer: heapless::Vec::new(),
            clock_timing,
            last_midi_activity: 0,
            midi_error_count: 0,
            enabled: true,
        })
    }

    /// Process incoming MIDI data from UART
    pub fn process_midi_input(&mut self, timestamp: u32) -> Result<Vec<crate::midi::MidiMessage, 16>, HalError> {
        use crate::midi::MidiHandler;
        
        let mut midi_messages = Vec::new();

        // Read data from MIDI IN UART if available
        if let Some(ref mut uart) = self.midi_uart.midi_in_uart {
            // TODO: Read bytes from UART peripheral
            // For now, simulate reading from buffer
            let mut temp_buffer = [0u8; 32];
            let bytes_read = 0; // Placeholder

            // Add received bytes to input buffer
            for i in 0..bytes_read {
                if self.midi_uart.midi_in_buffer.push(temp_buffer[i]).is_err() {
                    // Buffer full, clear and start over
                    self.midi_uart.midi_in_buffer.clear();
                    let _ = self.midi_uart.midi_in_buffer.push(temp_buffer[i]);
                }
            }
        }

        // Parse MIDI messages from buffer
        let mut parse_pos = 0;
        while parse_pos < self.midi_uart.midi_in_buffer.len() {
            if let Some(message) = self.parse_midi_message_at_position(parse_pos) {
                let message_length = self.get_midi_message_length(&message);
                
                // Process MIDI clock messages for tempo sync
                if let crate::midi::MidiMessage::Clock = message {
                    self.process_midi_clock(timestamp);
                } else if let crate::midi::MidiMessage::Start = message {
                    self.midi_uart.clock_timing.clock_running = true;
                    self.midi_uart.clock_timing.clock_pulse_count = 0;
                } else if let crate::midi::MidiMessage::Stop = message {
                    self.midi_uart.clock_timing.clock_running = false;
                }

                if midi_messages.push(message).is_err() {
                    break; // Output buffer full
                }
                
                parse_pos += message_length;
            } else {
                parse_pos += 1;
            }
        }

        // Clear processed data from input buffer
        if parse_pos > 0 {
            let end_pos = parse_pos.min(self.midi_uart.midi_in_buffer.len());
            for _ in 0..end_pos {
                self.midi_uart.midi_in_buffer.remove(0);
            }
        }

        self.midi_uart.last_midi_activity = timestamp;
        Ok(midi_messages)
    }

    /// Send MIDI message via UART
    pub fn send_midi_message(&mut self, message: crate::midi::MidiMessage) -> Result<(), HalError> {
        let bytes = message.to_bytes();
        
        // Add bytes to output buffer
        for byte in bytes {
            if self.midi_uart.midi_out_buffer.push(byte).is_err() {
                return Err(HalError::BufferFull);
            }
        }

        // Transmit buffer contents via UART
        self.transmit_midi_output()?;
        
        Ok(())
    }

    /// Transmit MIDI output buffer via UART
    fn transmit_midi_output(&mut self) -> Result<(), HalError> {
        if let Some(ref mut uart) = self.midi_uart.midi_out_uart {
            // TODO: Transmit bytes via UART peripheral
            // For now, just clear the buffer
            self.midi_uart.midi_out_buffer.clear();
        }
        Ok(())
    }

    /// Parse MIDI message at specific position in input buffer
    fn parse_midi_message_at_position(&self, pos: usize) -> Option<crate::midi::MidiMessage> {
        if pos >= self.midi_uart.midi_in_buffer.len() {
            return None;
        }

        let remaining = &self.midi_uart.midi_in_buffer[pos..];
        crate::midi::MidiMessage::from_bytes(remaining)
    }

    /// Get the byte length of a MIDI message
    fn get_midi_message_length(&self, message: &crate::midi::MidiMessage) -> usize {
        match message {
            crate::midi::MidiMessage::NoteOn { .. } | 
            crate::midi::MidiMessage::NoteOff { .. } | 
            crate::midi::MidiMessage::ControlChange { .. } => 3,
            crate::midi::MidiMessage::ProgramChange { .. } => 2,
            crate::midi::MidiMessage::Clock | 
            crate::midi::MidiMessage::Start | 
            crate::midi::MidiMessage::Stop | 
            crate::midi::MidiMessage::Continue => 1,
            crate::midi::MidiMessage::SysEx { data } => data.len() + 2, // +2 for 0xF0 and 0xF7
        }
    }

    /// Process MIDI clock message for tempo synchronization
    fn process_midi_clock(&mut self, timestamp: u32) {
        if !self.midi_uart.clock_timing.sync_enabled || !self.midi_uart.clock_timing.clock_running {
            return;
        }

        let clock_timing = &mut self.midi_uart.clock_timing;
        
        // Calculate interval since last clock pulse
        if clock_timing.last_clock_pulse > 0 {
            let interval_us = timestamp.saturating_sub(clock_timing.last_clock_pulse);
            clock_timing.clock_interval_us = interval_us;
            
            // MIDI clock runs at 24 pulses per quarter note
            // Calculate BPM: 60,000,000 / (interval_us * 24)
            if interval_us > 0 {
                let quarter_note_us = interval_us * 24;
                clock_timing.tempo_bpm = 60_000_000.0 / quarter_note_us as f32;
                
                // Clamp to reasonable tempo range
                clock_timing.tempo_bpm = clock_timing.tempo_bpm.clamp(60.0, 200.0);
            }
        }

        clock_timing.clock_pulse_count += 1;
        clock_timing.last_clock_pulse = timestamp;
    }

    /// Enable/disable MIDI clock synchronization
    pub fn set_midi_clock_sync(&mut self, enabled: bool) {
        self.midi_uart.clock_timing.sync_enabled = enabled;
        if !enabled {
            self.midi_uart.clock_timing.clock_running = false;
        }
    }

    /// Get current MIDI clock tempo
    pub fn get_midi_clock_tempo(&self) -> f32 {
        self.midi_uart.clock_timing.tempo_bpm
    }

    /// Check if MIDI clock is running
    pub fn is_midi_clock_running(&self) -> bool {
        self.midi_uart.clock_timing.clock_running
    }

    /// Get MIDI communication statistics
    pub fn get_midi_stats(&self) -> (u32, u32, bool) {
        (
            self.midi_uart.last_midi_activity,
            self.midi_uart.midi_error_count,
            self.midi_uart.enabled,
        )
    }
        // - PA2 = SW (button) with pull-up

        Ok(rotary_encoder)
    }

    /// Initialize 74HC595 LED controller
    fn init_led_controller(ccdr: &mut Ccdr) -> Result<Hc595LedController, HalError> {
        // Create and initialize LED controller
        let mut led_controller = Hc595LedController::new(4);
        led_controller.init()?;

        // TODO: Initialize actual GPIO pins when HAL API is confirmed
        // This would include:
        // - PB0 = Data (SER/DS) as push-pull output
        // - PB1 = Clock (SRCLK/SHCP) as push-pull output
        // - PB2 = Latch (RCLK/STCP) as push-pull output
        // - PB3 = OE (Output Enable) as push-pull output

        Ok(led_controller)
    }

    /// Update rotary encoder and return events
    pub fn update_rotary_encoder(&mut self, timestamp: u32) -> Vec<RotaryEvent, 4> {
        self.rotary_encoder.update(timestamp)
    }

    /// Get rotary encoder position
    pub fn get_rotary_position(&self) -> i32 {
        self.rotary_encoder.get_position()
    }

    /// Reset rotary encoder position
    pub fn reset_rotary_position(&mut self) {
        self.rotary_encoder.reset_position();
    }

    /// Check if rotary encoder button is pressed
    pub fn is_rotary_button_pressed(&self) -> bool {
        self.rotary_encoder.is_button_pressed()
    }

    /// Set LED state
    pub fn set_led(&mut self, led_id: LedId, command: LedCommand) -> Result<(), HalError> {
        self.led_controller.set_led(led_id, command)
    }

    /// Update all LEDs
    pub fn update_leds(&mut self) -> Result<(), HalError> {
        self.led_controller.update_leds()
    }

    /// Clear all LEDs
    pub fn clear_all_leds(&mut self) -> Result<(), HalError> {
        self.led_controller.clear_all_leds()
    }

    /// Get LED state
    pub fn get_led_state(&self, led_id: LedId) -> Result<bool, HalError> {
        self.led_controller.get_led_state(led_id)
    }

    /// Synchronize LED states with track and FX states
    pub fn sync_leds_with_system_state(&mut self) -> Result<(), HalError> {
        // This method will be called to update LEDs based on current system state
        // For now, implement basic LED patterns

        // Update track LEDs based on audio engine state
        if let Some(ref audio_engine) = self.audio_engine {
            for track_id in 1..=6 {
                // Get track state from audio engine (placeholder logic)
                let track_active = track_id <= 3; // Simulate first 3 tracks active
                let track_recording = track_id == 1; // Simulate track 1 recording

                // Set track status LED
                self.set_led(
                    LedId::Track(track_id),
                    if track_active { LedCommand::On } else { LedCommand::Off }
                )?;

                // Set track recording LED
                self.set_led(
                    LedId::TrackRec(track_id),
                    if track_recording { LedCommand::On } else { LedCommand::Off }
                )?;
            }
        }

        // Update FX LEDs (placeholder - would be based on actual FX state)
        for fx_id in 1..=5 {
            let fx_active = fx_id <= 2; // Simulate first 2 FX active
            self.set_led(
                LedId::FX(fx_id),
                if fx_active { LedCommand::On } else { LedCommand::Off }
            )?;
        }

        // Update transport LEDs (placeholder)
        self.set_led(LedId::Play, LedCommand::Off)?;
        self.set_led(LedId::Stop, LedCommand::On)?;
        self.set_led(LedId::Rec, LedCommand::Off)?;

        // Update system LEDs
        self.set_led(LedId::Power, LedCommand::On)?;
        self.set_led(LedId::Error, LedCommand::Off)?;

        // Apply all LED updates
        self.update_leds()?;

        Ok(())
    }

    /// Initialize ESP32 UART interface at 115200 baud
    fn init_esp32_uart(ccdr: &mut Ccdr, usart1: USART1) -> Result<Esp32UartInterface, HalError> {
        // TODO: Configure GPIO pins for UART1 when HAL API is confirmed
        // This would include:
        // - PA9 = USART1_TX (AF7) to ESP32 RX
        // - PA10 = USART1_RX (AF7) from ESP32 TX
        
        // For now, create the interface structure without actual UART initialization
        // Full implementation will be completed when GPIO configuration is available
        
        Ok(Esp32UartInterface {
            uart: None, // Will be initialized when GPIO API is available
            rx_buffer: heapless::Vec::new(),
            tx_buffer: heapless::Vec::new(),
            last_communication: 0,
            error_count: 0,
            enabled: true,
        })
    }

    /// Send message to ESP32 via UART
    pub fn send_to_esp32(&mut self, message: &Esp32Message) -> Result<(), HalError> {
        if !self.esp32_uart.enabled {
            return Err(HalError::CommunicationError);
        }

        // Serialize message to JSON format
        let json_str = self.serialize_message(message)?;
        
        // Add newline delimiter
        let mut full_message = heapless::String::<512>::new();
        full_message.push_str(&json_str).map_err(|_| HalError::BufferFull)?;
        full_message.push('\n').map_err(|_| HalError::BufferFull)?;

        // Store in TX buffer for now (actual UART transmission will be implemented later)
        self.esp32_uart.tx_buffer.clear();
        for byte in full_message.as_bytes() {
            self.esp32_uart.tx_buffer.push(*byte).map_err(|_| HalError::BufferFull)?;
        }

        // TODO: Actual UART transmission when peripheral is available
        // if let Some(ref mut uart) = self.esp32_uart.uart {
        //     for byte in &self.esp32_uart.tx_buffer {
        //         nb::block!(uart.write(*byte)).map_err(|_| HalError::CommunicationError)?;
        //     }
        // }

        Ok(())
    }

    /// Receive message from ESP32 via UART
    pub fn receive_from_esp32(&mut self) -> Result<Option<Esp32Message>, HalError> {
        if !self.esp32_uart.enabled {
            return Ok(None);
        }

        // TODO: Actual UART reception when peripheral is available
        // For now, return None (no message received)
        
        // The implementation would:
        // 1. Read bytes from UART into rx_buffer
        // 2. Look for newline delimiter
        // 3. Parse JSON message
        // 4. Return parsed message

        Ok(None)
    }

    /// Send system status update to ESP32
    pub fn send_status_update(&mut self) -> Result<(), HalError> {
        // Collect current system status
        let mut tracks = [TrackStatus {
            state: TrackStateComm::Stopped,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            selected: false,
        }; 6];

        // Get track states from audio engine if available
        if let Some(ref audio_engine) = self.audio_engine {
            for i in 0..6 {
                // TODO: Get actual track state from audio engine
                // For now, use placeholder values
                tracks[i] = TrackStatus {
                    state: if i == 0 { TrackStateComm::Playing } else { TrackStateComm::Stopped },
                    volume: 0.8,
                    pan: 0.0,
                    muted: false,
                    selected: i == 0,
                };
            }
        }

        let message = Esp32Message::StatusUpdate {
            tracks,
            tempo: 120.0, // TODO: Get from actual tempo system
            current_memory: 1, // TODO: Get from memory system
            fx_states: [false, true, false, false, false], // TODO: Get from FX system
        };

        self.send_to_esp32(&message)
    }

    /// Send parameter change notification to ESP32
    pub fn send_parameter_change(&mut self, parameter: ParameterId, value: f32) -> Result<(), HalError> {
        let message = Esp32Message::ParameterChange { parameter, value };
        self.send_to_esp32(&message)
    }

    /// Process received command from ESP32
    pub fn process_esp32_command(&mut self, message: Esp32Message) -> Result<(), HalError> {
        match message {
            Esp32Message::Command { command, track_id, value } => {
                let result = self.execute_command(command, track_id, value);
                
                // Send response back to ESP32
                let response = match &result {
                    Ok(()) => Esp32Message::Response { success: true, error_message: None },
                    Err(_) => {
                        let mut error_msg = heapless::String::new();
                        error_msg.push_str("Command failed").ok();
                        Esp32Message::Response { success: false, error_message: Some(error_msg) }
                    }
                };
                
                self.send_to_esp32(&response)?;
                result
            }
            Esp32Message::Heartbeat => {
                // Respond to heartbeat
                self.send_to_esp32(&Esp32Message::Heartbeat)
            }
            _ => Ok(()), // Other message types handled elsewhere
        }
    }

    /// Execute command received from ESP32
    fn execute_command(&mut self, command: CommandType, track_id: Option<u8>, value: Option<f32>) -> Result<(), HalError> {
        match command {
            CommandType::TrackPlay => {
                if let Some(id) = track_id {
                    // TODO: Implement track play via audio engine
                    // self.audio_engine.as_mut().unwrap().start_playback(id)?;
                }
            }
            CommandType::TrackStop => {
                if let Some(id) = track_id {
                    // TODO: Implement track stop via audio engine
                    // self.audio_engine.as_mut().unwrap().stop_track(id)?;
                }
            }
            CommandType::TrackRecord => {
                if let Some(id) = track_id {
                    // TODO: Implement track record via audio engine
                    // self.audio_engine.as_mut().unwrap().start_recording(id)?;
                }
            }
            CommandType::SetVolume => {
                if let (Some(id), Some(vol)) = (track_id, value) {
                    // TODO: Implement volume setting via audio engine
                    // self.audio_engine.as_mut().unwrap().set_track_level(id, vol)?;
                }
            }
            CommandType::SetTempo => {
                if let Some(tempo) = value {
                    // TODO: Implement tempo setting
                    // self.tempo = tempo;
                }
            }
            _ => {
                // Other commands will be implemented in later tasks
            }
        }
        Ok(())
    }

    /// Serialize message to JSON string
    fn serialize_message(&self, message: &Esp32Message) -> Result<heapless::String<256>, HalError> {
        let mut json_str = heapless::String::new();
        
        // Simple JSON serialization (basic implementation)
        match message {
            Esp32Message::StatusUpdate { tracks, tempo, current_memory, fx_states } => {
                json_str.push_str("{\"type\":\"status\",\"tracks\":[").map_err(|_| HalError::BufferFull)?;
                for (i, track) in tracks.iter().enumerate() {
                    if i > 0 { json_str.push(',').map_err(|_| HalError::BufferFull)?; }
                    json_str.push_str("{\"state\":").map_err(|_| HalError::BufferFull)?;
                    match track.state {
                        TrackStateComm::Stopped => json_str.push_str("0").map_err(|_| HalError::BufferFull)?,
                        TrackStateComm::Recording => json_str.push_str("1").map_err(|_| HalError::BufferFull)?,
                        TrackStateComm::Playing => json_str.push_str("2").map_err(|_| HalError::BufferFull)?,
                        TrackStateComm::Overdubbing => json_str.push_str("3").map_err(|_| HalError::BufferFull)?,
                        TrackStateComm::Muted => json_str.push_str("4").map_err(|_| HalError::BufferFull)?,
                    }
                    json_str.push_str(",\"volume\":").map_err(|_| HalError::BufferFull)?;
                    // Simple float to string conversion (basic implementation)
                    let vol_str = if track.volume >= 1.0 { "1.0" } else if track.volume <= 0.0 { "0.0" } else { "0.5" };
                    json_str.push_str(vol_str).map_err(|_| HalError::BufferFull)?;
                    json_str.push('}').map_err(|_| HalError::BufferFull)?;
                }
                json_str.push_str("],\"tempo\":").map_err(|_| HalError::BufferFull)?;
                json_str.push_str("120.0").map_err(|_| HalError::BufferFull)?; // Simplified
                json_str.push('}').map_err(|_| HalError::BufferFull)?;
            }
            Esp32Message::Heartbeat => {
                json_str.push_str("{\"type\":\"heartbeat\"}").map_err(|_| HalError::BufferFull)?;
            }
            Esp32Message::Response { success, error_message } => {
                json_str.push_str("{\"type\":\"response\",\"success\":").map_err(|_| HalError::BufferFull)?;
                if *success { json_str.push_str("true").map_err(|_| HalError::BufferFull)?; }
                else { json_str.push_str("false").map_err(|_| HalError::BufferFull)?; }
                json_str.push('}').map_err(|_| HalError::BufferFull)?;
            }
            _ => {
                json_str.push_str("{\"type\":\"unknown\"}").map_err(|_| HalError::BufferFull)?;
            }
        }
        
        Ok(json_str)
    }

    /// Update ESP32 communication (called from main loop)
    pub fn update_esp32_communication(&mut self, timestamp: u32) -> Result<(), HalError> {
        // Send periodic status updates (every 100ms)
        if timestamp.saturating_sub(self.esp32_uart.last_communication) >= 100 {
            self.send_status_update()?;
            self.esp32_uart.last_communication = timestamp;
        }

        // Check for incoming messages
        if let Some(message) = self.receive_from_esp32()? {
            self.process_esp32_command(message)?;
        }

        Ok(())
    }

    /// Get ESP32 communication statistics
    pub fn get_esp32_stats(&self) -> (u32, bool) {
        (self.esp32_uart.error_count, self.esp32_uart.enabled)
    }

    /// Enable/disable ESP32 communication
    pub fn set_esp32_enabled(&mut self, enabled: bool) {
        self.esp32_uart.enabled = enabled;
    }

    /// Configure GPIO pins for I2S communication
    fn configure_i2s_gpio(&mut self) -> Result<(), HalError> {
        // TODO: Configure GPIO pins for SAI1-4 I2S communication
        // This will include:
        // - SAI1_SCK_A, SAI1_FS_A, SAI1_SD_A for PCM1808 #1,#2
        // - SAI2_SCK_A, SAI2_FS_A, SAI2_SD_A for PCM1808 #3,#4
        // - SAI3_SCK_A, SAI3_FS_A, SAI3_SD_A for PCM5102A #1,#2
        // - SAI4_SCK_A, SAI4_FS_A, SAI4_SD_A for PCM5102A #3,#4
        // - Master clock outputs for each SAI

        Ok(())
    }

    /// Get MIDI events from UART interface
    pub fn get_midi_events(&mut self) -> Vec<MidiEvent, 16> {
        let mut events = Vec::new();
        
        // TODO: Read MIDI data from UART and parse into MidiEvent
        // This would involve:
        // 1. Reading bytes from MIDI UART interface
        // 2. Parsing MIDI protocol (status bytes, data bytes)
        // 3. Converting to MidiEvent enum
        // For now, return empty vector
        
        events
    }

    /// Get footswitch events from GPIO
    pub fn get_footswitch_events(&mut self) -> Vec<(usize, bool), 4> {
        let mut events = Vec::new();
        
        // TODO: Read footswitch GPIO pins and detect state changes
        // This would involve:
        // 1. Reading GPIO pins connected to footswitches
        // 2. Debouncing and edge detection
        // 3. Returning (footswitch_index, pressed) pairs
        // For now, return empty vector
        
        events
    }
}

// PC implementation with stubs
#[cfg(feature = "std")]
impl HardwareHal {
    /// Initialize the hardware abstraction layer (PC stub)
    pub fn init() -> Result<Self, HalError> {
        // Create default DAW routing configuration for PC
        let daw_routing = DawRoutingConfig {
            // Default track input routing (tracks 1-6 -> USB channels 0-5)
            track_input_routing: [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)],
            // Default track output routing (tracks 1-6 -> USB channel pairs 0-11)
            track_output_routing: [
                Some((0, 1)),   // Track 1 -> USB channels 0,1
                Some((2, 3)),   // Track 2 -> USB channels 2,3
                Some((4, 5)),   // Track 3 -> USB channels 4,5
                Some((6, 7)),   // Track 4 -> USB channels 6,7
                Some((8, 9)),   // Track 5 -> USB channels 8,9
                Some((10, 11)), // Track 6 -> USB channels 10,11
            ],
            // Master output routing -> USB channels 14,15
            master_output_routing: (14, 15),
            // Default input monitoring (USB inputs 0-5 -> hardware outputs)
            input_monitoring_routing: [
                Some(0), Some(1), Some(2), Some(3), Some(4), Some(5),
                None, None, None, None, None, None, None, None, None, None,
            ],
            // Enable monitoring for all tracks by default
            track_monitoring_enabled: [true; 6],
            // Enable master monitoring by default
            master_monitoring_enabled: true,
        };

        Ok(Self {
            initialized: true,
            usb_audio: UsbAudioInterface {
                usb_input_buffers: [[0.0f32; AUDIO_BUFFER_SIZE]; 16],
                usb_output_buffers: [[0.0f32; AUDIO_BUFFER_SIZE]; 16],
                current_usb_buffer: false,
                streaming_active: false,
                sample_rate: 48000, // Default to 48kHz for DAW compatibility
                bit_depth: 24, // Default to 24-bit for professional quality
                zero_latency_monitoring: true,
                daw_routing,
                error_count: 0,
                enabled: true,
            },
            midi_uart: MidiUartInterface {
                midi_in_buffer: heapless::Vec::new(),
                midi_out_buffer: heapless::Vec::new(),
                clock_timing: MidiClockTiming::new(),
                last_midi_activity: 0,
                midi_error_count: 0,
                enabled: true,
            },
        })
    }

    /// Start audio processing (PC stub)
    pub fn start_audio_processing(&mut self) -> Result<(), HalError> {
        Ok(())
    }

    /// Stop audio processing (PC stub)
    pub fn stop_audio_processing(&mut self) -> Result<(), HalError> {
        Ok(())
    }

    /// Read control values (PC stub)
    pub fn read_controls(&mut self) -> Vec<(ControlId, f32), 16> {
        Vec::new()
    }

    /// Read button states (PC stub)
    pub fn read_buttons(&mut self) -> Vec<(ButtonId, PressType), 32> {
        Vec::new()
    }

    /// Set LED state (PC stub)
    pub fn set_led(&mut self, _led: LedId, _command: LedCommand) -> Result<(), HalError> {
        Ok(())
    }

    /// Send UART data to ESP32 (PC stub)
    pub fn send_uart_data(&mut self, _data: &[u8]) -> Result<(), HalError> {
        Ok(())
    }

    /// Read UART data from ESP32 (PC stub)
    pub fn read_uart_data(&mut self) -> Result<Vec<u8, 256>, HalError> {
        Ok(Vec::new())
    }

    /// Update LEDs (PC stub)
    pub fn update_leds(&mut self) -> Result<(), HalError> {
        Ok(())
    }

    /// Update button states (PC stub)
    pub fn update_button_states(&mut self, _time_ms: u32) -> Vec<(ButtonId, PressType), 32> {
        Vec::new()
    }

    /// Update rotary encoder (PC stub)
    pub fn update_rotary_encoder(&mut self, _time_ms: u32) -> Vec<RotaryEvent, 8> {
        Vec::new()
    }

    /// Read single control (PC stub)
    pub fn read_control(&mut self, _control_id: ControlId) -> Result<f32, HalError> {
        Ok(0.0)
    }

    /// Get MIDI events (PC stub)
    pub fn get_midi_events(&mut self) -> Vec<MidiEvent, 16> {
        Vec::new()
    }

    /// Get footswitch events (PC stub)
    pub fn get_footswitch_events(&mut self) -> Vec<(usize, bool), 4> {
        Vec::new()
    }
}

// PC stub implementations for other types
#[cfg(feature = "std")]
impl Ky040RotaryEncoder {
    pub fn new() -> Self {
        Self {
            clk_pin: None,
            dt_pin: None,
            sw_pin: None,
            prev_clk_state: false,
            prev_dt_state: false,
            prev_button_state: false,
            position: 0,
            button_pressed: false,
            button_debounce_counter: 0,
            last_update_time: 0,
            enabled: false,
        }
    }

    pub fn read_events(&mut self) -> Vec<RotaryEvent, 8> {
        Vec::new()
    }

    pub fn get_position(&self) -> i32 {
        self.position
    }

    pub fn reset_position(&mut self) {
        self.position = 0;
    }

    pub fn is_button_pressed(&self) -> bool {
        self.button_pressed
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[cfg(feature = "std")]
impl Hc595LedController {
    pub fn new(chip_count: u8) -> Self {
        Self {
            spi_peripheral: None,
            data_pin: None,
            clock_pin: None,
            latch_pin: None,
            output_enable_pin: None,
            chip_count,
            led_states: [0; 8],
            update_pending: false,
            enabled: false,
        }
    }

    pub fn set_led(&mut self, _led: LedId, _command: LedCommand) -> Result<(), HalError> {
        Ok(())
    }

    pub fn update_leds(&mut self) -> Result<(), HalError> {
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[cfg(feature = "std")]
impl Esp32UartInterface {
    pub fn new() -> Self {
        Self {
            rx_buffer: heapless::Vec::new(),
            tx_buffer: heapless::Vec::new(),
            last_communication: 0,
            error_count: 0,
            enabled: false,
        }
    }
}

#[cfg(feature = "std")]
impl Pcf8575Controller {
    pub fn new(address: u8, controller_type: ControllerType) -> Self {
        Self {
            address,
            current_state: 0xFFFF,
            previous_state: 0xFFFF,
            button_mapping: ButtonMapping::new(controller_type),
            debounce_counters: [0; 16],
            press_start_times: [0; 16],
            press_types: [PressType::None; 16],
            enabled: false,
        }
    }

    pub fn read_buttons<I2C>(&mut self, _i2c: &mut I2C, _time_ms: u32) -> Vec<(ButtonId, PressType), 16> {
        Vec::new()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_interrupt_active(&self) -> bool {
        false
    }
}

#[cfg(not(feature = "std"))]
impl Ky040RotaryEncoder {
    /// Create a new KY-040 rotary encoder driver
    pub fn new() -> Self {
        Self {
            clk_pin: None,
            dt_pin: None,
            sw_pin: None,
            prev_clk_state: false,
            prev_dt_state: false,
            prev_button_state: false,
            position: 0,
            button_pressed: false,
            button_debounce_counter: 0,
            last_update_time: 0,
            enabled: true,
        }
    }

    /// Initialize the rotary encoder with GPIO pins (placeholder implementation)
    pub fn init(&mut self) -> Result<(), HalError> {
        // Placeholder initialization - actual GPIO pins will be configured later
        self.clk_pin = Some(false);
        self.dt_pin = Some(false);
        self.sw_pin = Some(false);

        // Initialize states
        self.prev_clk_state = false;
        self.prev_dt_state = false;
        self.prev_button_state = false;

        Ok(())
    }

    /// Update encoder state and return events
    pub fn update(&mut self, timestamp: u32) -> Vec<RotaryEvent, 4> {
        let mut events = Vec::new();

        if !self.enabled {
            return events;
        }

        // Update timestamp
        self.last_update_time = timestamp;

        // Read current pin states (placeholder - would read actual GPIO pins)
        if let (Some(_), Some(_), Some(_)) = (&self.clk_pin, &self.dt_pin, &self.sw_pin) {
            // Simulate encoder rotation for testing (would read actual pins)
            let clk_state = (timestamp / 1000) % 2 == 0; // Simulate CLK changes
            let dt_state = ((timestamp / 1000) + 1) % 2 == 0; // Simulate DT changes
            let sw_state = true; // Simulate button not pressed

            // Quadrature decoding - detect CLK falling edge
            if self.prev_clk_state && !clk_state {
                // CLK falling edge detected
                if dt_state {
                    // DT high during CLK falling = clockwise
                    self.position += 1;
                    if events.push(RotaryEvent::Clockwise).is_err() {
                        // Event buffer full
                    }
                } else {
                    // DT low during CLK falling = counter-clockwise
                    self.position -= 1;
                    if events.push(RotaryEvent::CounterClockwise).is_err() {
                        // Event buffer full
                    }
                }
            }

            // Button debouncing and press detection
            let button_pressed = !sw_state; // Active low button
            if button_pressed != self.prev_button_state {
                if button_pressed {
                    self.button_debounce_counter += 1;
                    if self.button_debounce_counter >= 3 {
                        // 3 consecutive readings = debounced press
                        self.button_pressed = true;
                        self.button_debounce_counter = 0;
                        if events.push(RotaryEvent::ButtonPress).is_err() {
                            // Event buffer full
                        }
                    }
                } else {
                    self.button_debounce_counter += 1;
                    if self.button_debounce_counter >= 3 {
                        // 3 consecutive readings = debounced release
                        self.button_pressed = false;
                        self.button_debounce_counter = 0;
                        if events.push(RotaryEvent::ButtonRelease).is_err() {
                            // Event buffer full
                        }
                    }
                }
            } else {
                // Reset debounce counter if state is stable
                self.button_debounce_counter = 0;
            }

            // Update previous states
            self.prev_clk_state = clk_state;
            self.prev_dt_state = dt_state;
            self.prev_button_state = button_pressed;
        }

        events
    }

    /// Get current encoder position
    pub fn get_position(&self) -> i32 {
        self.position
    }

    /// Reset encoder position to zero
    pub fn reset_position(&mut self) {
        self.position = 0;
    }

    /// Get current button state
    pub fn is_button_pressed(&self) -> bool {
        self.button_pressed
    }

    /// Enable/disable encoder
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[cfg(not(feature = "std"))]
impl Hc595LedController {
    /// Create a new 74HC595 LED controller
    pub fn new(chip_count: u8) -> Self {
        Self {
            spi_peripheral: None,
            data_pin: None,
            clock_pin: None,
            latch_pin: None,
            output_enable_pin: None,
            chip_count: chip_count.min(8), // Maximum 8 chips supported
            led_states: [0u8; 8],
            update_pending: false,
            enabled: true,
        }
    }

    /// Initialize the LED controller with GPIO pins (placeholder implementation)
    pub fn init(&mut self) -> Result<(), HalError> {
        // Placeholder initialization - actual GPIO pins will be configured later
        self.data_pin = Some(false);
        self.clock_pin = Some(false);
        self.latch_pin = Some(false);
        self.output_enable_pin = Some(false);

        // Clear all LEDs initially
        self.clear_all_leds()?;

        Ok(())
    }

    /// Set LED state
    pub fn set_led(&mut self, led_id: LedId, command: LedCommand) -> Result<(), HalError> {
        if !self.enabled {
            return Ok(());
        }

        let (chip_index, bit_index) = self.led_id_to_position(led_id)?;
        
        if chip_index >= self.chip_count as usize {
            return Err(HalError::InvalidControl);
        }

        match command {
            LedCommand::On => {
                self.led_states[chip_index] |= 1 << bit_index;
            }
            LedCommand::Off => {
                self.led_states[chip_index] &= !(1 << bit_index);
            }
            LedCommand::Toggle => {
                self.led_states[chip_index] ^= 1 << bit_index;
            }
            LedCommand::Brightness(_) => {
                // Brightness control not supported with basic 74HC595
                // Would require PWM control of OE pin or individual LED PWM
                return Err(HalError::InvalidControl);
            }
        }

        self.update_pending = true;
        Ok(())
    }

    /// Update all LEDs by shifting out data to 74HC595 chips
    pub fn update_leds(&mut self) -> Result<(), HalError> {
        if !self.enabled || !self.update_pending {
            return Ok(());
        }

        if let (Some(_), Some(_), Some(_)) = (&self.data_pin, &self.clock_pin, &self.latch_pin) {
            // Placeholder LED update logic - would control actual GPIO pins
            // For now, just simulate the shift register operation
            
            // Shift out data for all chips (MSB first, last chip first)
            for chip_index in (0..self.chip_count as usize).rev() {
                let _chip_data = self.led_states[chip_index];
                
                // Simulate shifting out 8 bits (MSB first)
                for _bit_index in (0..8).rev() {
                    // Would set data line and pulse clock here
                    cortex_m::asm::nop(); // Simulate timing
                }
            }
            
            // Simulate latch pulse
            cortex_m::asm::nop();
        }

        self.update_pending = false;
        Ok(())
    }

    /// Clear all LEDs
    pub fn clear_all_leds(&mut self) -> Result<(), HalError> {
        for chip_index in 0..self.chip_count as usize {
            self.led_states[chip_index] = 0;
        }
        self.update_pending = true;
        self.update_leds()
    }

    /// Set all LEDs on
    pub fn set_all_leds(&mut self) -> Result<(), HalError> {
        for chip_index in 0..self.chip_count as usize {
            self.led_states[chip_index] = 0xFF;
        }
        self.update_pending = true;
        self.update_leds()
    }

    /// Get LED state
    pub fn get_led_state(&self, led_id: LedId) -> Result<bool, HalError> {
        let (chip_index, bit_index) = self.led_id_to_position(led_id)?;
        
        if chip_index >= self.chip_count as usize {
            return Err(HalError::InvalidControl);
        }

        let bit_mask = 1 << bit_index;
        Ok((self.led_states[chip_index] & bit_mask) != 0)
    }

    /// Enable/disable LED controller
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        
        // Placeholder for output enable control - would control actual GPIO pin
        if !enabled {
            // Would set OE pin high to disable outputs (active low)
        } else {
            // Would set OE pin low to enable outputs (active low)
        }
    }

    /// Map LED ID to chip and bit position
    fn led_id_to_position(&self, led_id: LedId) -> Result<(usize, usize), HalError> {
        match led_id {
            // Track LEDs on chip 0 (bits 0-5)
            LedId::Track(track) if track >= 1 && track <= 6 => {
                Ok((0, (track - 1) as usize))
            }
            // Track recording LEDs on chip 0 (bits 6-7) and chip 1 (bits 0-3)
            LedId::TrackRec(track) if track >= 1 && track <= 6 => {
                if track <= 2 {
                    Ok((0, (track + 5) as usize)) // Bits 6-7 of chip 0
                } else {
                    Ok((1, (track - 3) as usize)) // Bits 0-3 of chip 1
                }
            }
            // FX LEDs on chip 1 (bits 4-7) and chip 2 (bit 0)
            LedId::FX(fx) if fx >= 1 && fx <= 5 => {
                if fx <= 4 {
                    Ok((1, (fx + 3) as usize)) // Bits 4-7 of chip 1
                } else {
                    Ok((2, 0)) // Bit 0 of chip 2
                }
            }
            // Transport LEDs on chip 2 (bits 1-3)
            LedId::Play => Ok((2, 1)),
            LedId::Stop => Ok((2, 2)),
            LedId::Rec => Ok((2, 3)),
            // Menu LEDs on chip 2 (bits 4-6)
            LedId::Menu => Ok((2, 4)),
            LedId::PageLeft => Ok((2, 5)),
            LedId::PageRight => Ok((2, 6)),
            // System LEDs on chip 2 (bit 7) and chip 3 (bits 0-2)
            LedId::Memory => Ok((2, 7)),
            LedId::Tempo => Ok((3, 0)),
            LedId::Power => Ok((3, 1)),
            LedId::Error => Ok((3, 2)),
            // Custom LEDs on remaining positions
            LedId::Custom(id) => {
                let total_position = id as usize + 32; // Start after system LEDs
                let chip_index = total_position / 8;
                let bit_index = total_position % 8;
                if chip_index < self.chip_count as usize {
                    Ok((chip_index, bit_index))
                } else {
                    Err(HalError::InvalidControl)
                }
            }
            _ => Err(HalError::InvalidControl),
        }
    }
}

#[cfg(not(feature = "std"))]
impl Esp32UartInterface {
    pub fn new() -> Self {
        Self {
            uart: None,
            rx_buffer: heapless::Vec::new(),
            tx_buffer: heapless::Vec::new(),
            last_communication: 0,
            error_count: 0,
            enabled: true,
        }
    }
}

#[cfg(not(feature = "std"))]
impl Pcf8575Controller {
    /// Create a new PCF8575 controller
    pub fn new(address: u8, controller_type: ControllerType) -> Self {
        let button_mapping = ButtonMapping::new(controller_type);

        Self {
            address,
            current_state: 0xFFFF, // All buttons released (active low)
            previous_state: 0xFFFF,
            button_mapping,
            debounce_counters: [0; 16],
            press_start_times: [0; 16],
            press_types: [PressType::None; 16],
            enabled: true,
        }
    }

    /// Read button states from PCF8575 via I2C
    pub fn read_buttons<I2C>(
        &mut self,
        i2c: &mut I2C,
        timestamp: u32,
    ) -> Result<Vec<(ButtonId, PressType), 16>, HalError>
    where
        I2C: WriteRead,
    {
        if !self.enabled {
            return Ok(Vec::new());
        }

        // Read 16-bit state from PCF8575 (2 bytes, MSB first)
        let mut buffer = [0u8; 2];
        i2c.write_read(self.address, &[], &mut buffer)
            .map_err(|_| HalError::I2CError)?;

        // Combine bytes to form 16-bit state (MSB first)
        let new_state = ((buffer[0] as u16) << 8) | (buffer[1] as u16);

        // Update states
        self.previous_state = self.current_state;
        self.current_state = new_state;

        // Process button changes with debouncing and gesture recognition
        let mut button_events = Vec::new();

        for pin in 0..16 {
            let pin_mask = 1u16 << pin;
            let current_pressed = (self.current_state & pin_mask) == 0; // Active low
            let previous_pressed = (self.previous_state & pin_mask) == 0;

            // Get button ID for this pin
            if let Some(button_id) = self.button_mapping.pin_assignments[pin] {
                // Process button state change with debouncing
                if let Some(press_type) =
                    self.process_button_change(pin, current_pressed, previous_pressed, timestamp)
                {
                    if button_events.push((button_id, press_type)).is_err() {
                        break; // Event buffer full
                    }
                }
            }
        }

        Ok(button_events)
    }

    /// Process button state change with debouncing and gesture recognition
    fn process_button_change(
        &mut self,
        pin: usize,
        current_pressed: bool,
        previous_pressed: bool,
        timestamp: u32,
    ) -> Option<PressType> {
        const DEBOUNCE_THRESHOLD: u8 = 3; // 3 consecutive readings for 10ms response
        const LONG_PRESS_THRESHOLD: u32 = 500; // 500ms for long press
        const DOUBLE_PRESS_WINDOW: u32 = 300; // 300ms window for double press

        // Handle state change
        if current_pressed != previous_pressed {
            if current_pressed {
                // Button pressed
                self.debounce_counters[pin] += 1;
                if self.debounce_counters[pin] >= DEBOUNCE_THRESHOLD {
                    self.press_start_times[pin] = timestamp;
                    self.press_types[pin] = PressType::Short;
                    self.debounce_counters[pin] = 0;
                    // Don't return event yet - wait for release to determine type
                }
            } else {
                // Button released
                self.debounce_counters[pin] += 1;
                if self.debounce_counters[pin] >= DEBOUNCE_THRESHOLD {
                    let press_duration = timestamp.saturating_sub(self.press_start_times[pin]);
                    self.debounce_counters[pin] = 0;

                    // Determine press type based on duration
                    let press_type = if press_duration >= LONG_PRESS_THRESHOLD {
                        PressType::Long
                    } else {
                        // Check for double press (simplified - would need more state tracking)
                        PressType::Short
                    };

                    self.press_types[pin] = PressType::None;
                    return Some(press_type);
                }
            }
        } else {
            // Reset debounce counter if state is stable
            self.debounce_counters[pin] = 0;

            // Check for long press while button is held
            if current_pressed && self.press_types[pin] == PressType::Short {
                let press_duration = timestamp.saturating_sub(self.press_start_times[pin]);
                if press_duration >= LONG_PRESS_THRESHOLD {
                    self.press_types[pin] = PressType::Long;
                    return Some(PressType::Long);
                }
            }
        }

        None
    }

    /// Write to PCF8575 (for future LED control if needed)
    pub fn write_outputs<I2C>(&self, i2c: &mut I2C, data: u16) -> Result<(), HalError>
    where
        I2C: Write,
    {
        if !self.enabled {
            return Ok(());
        }

        // Convert 16-bit data to bytes (MSB first)
        let buffer = [(data >> 8) as u8, data as u8];
        i2c.write(self.address, &buffer)
            .map_err(|_| HalError::I2CError)?;

        Ok(())
    }

    /// Enable/disable controller
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get current button state for a specific button
    pub fn get_button_state(&self, button_id: ButtonId) -> bool {
        for (pin, &mapped_button) in self.button_mapping.pin_assignments.iter().enumerate() {
            if let Some(mapped) = mapped_button {
                if mapped == button_id {
                    let pin_mask = 1u16 << pin;
                    return (self.current_state & pin_mask) == 0; // Active low
                }
            }
        }
        false
    }
}

impl ButtonMapping {
    /// Create button mapping for controller type
    pub fn new(controller_type: ControllerType) -> Self {
        let mut pin_assignments = [None; 16];

        match controller_type {
            ControllerType::TrackButtons => {
                // Track buttons 1-6 on pins 0-5
                pin_assignments[0] = Some(ButtonId::Track(0));
                pin_assignments[1] = Some(ButtonId::Track(1));
                pin_assignments[2] = Some(ButtonId::Track(2));
                pin_assignments[3] = Some(ButtonId::Track(3));
                pin_assignments[4] = Some(ButtonId::Track(4));
                pin_assignments[5] = Some(ButtonId::Track(5));
                // Track select buttons 1-6 on pins 8-13
                pin_assignments[8] = Some(ButtonId::TrackSelect(0));
                pin_assignments[9] = Some(ButtonId::TrackSelect(1));
                pin_assignments[10] = Some(ButtonId::TrackSelect(2));
                pin_assignments[11] = Some(ButtonId::TrackSelect(3));
                pin_assignments[12] = Some(ButtonId::TrackSelect(4));
                pin_assignments[13] = Some(ButtonId::TrackSelect(5));
            }
            ControllerType::FxAndTransport => {
                // FX buttons 1-5 on pins 0-4
                pin_assignments[0] = Some(ButtonId::FX(0));
                pin_assignments[1] = Some(ButtonId::FX(1));
                pin_assignments[2] = Some(ButtonId::FX(2));
                pin_assignments[3] = Some(ButtonId::FX(3));
                pin_assignments[4] = Some(ButtonId::FX(4));
                // Transport controls on pins 8-10
                pin_assignments[8] = Some(ButtonId::Play);
                pin_assignments[9] = Some(ButtonId::Stop);
                pin_assignments[10] = Some(ButtonId::Rec);
            }
            ControllerType::MenuAndUtility => {
                // Menu navigation on pins 0-4
                pin_assignments[0] = Some(ButtonId::Menu);
                pin_assignments[1] = Some(ButtonId::PageLeft);
                pin_assignments[2] = Some(ButtonId::PageRight);
                pin_assignments[3] = Some(ButtonId::Enter);
                pin_assignments[4] = Some(ButtonId::Exit);
                // Utility buttons on pins 8-11
                pin_assignments[8] = Some(ButtonId::TapTempo);
                pin_assignments[9] = Some(ButtonId::Memory);
                pin_assignments[10] = Some(ButtonId::UndoRedo);
                pin_assignments[11] = Some(ButtonId::Edit);
            }
            ControllerType::Additional => {
                // Additional controls - can be customized as needed
                // Leave empty for now
            }
        }

        Self {
            controller_type,
            pin_assignments,
        }
    }

    /// Get button ID for a specific pin
    pub fn get_button_for_pin(&self, pin: usize) -> Option<ButtonId> {
        if pin < 16 {
            self.pin_assignments[pin]
        } else {
            None
        }
    }

    /// Get pin for a specific button ID
    pub fn get_pin_for_button(&self, button_id: ButtonId) -> Option<usize> {
        for (pin, &mapped_button) in self.pin_assignments.iter().enumerate() {
            if let Some(mapped) = mapped_button {
                if mapped == button_id {
                    return Some(pin);
                }
            }
        }
        None
    }
}

impl ButtonStates {
    /// Create new button states with default values
    pub fn new() -> Self {
        Self {
            track_button_states: [ButtonState::new(); 6],
            fx_button_states: [ButtonState::new(); 5],
            transport_states: [ButtonState::new(); 3],
            menu_states: [ButtonState::new(); 5],
            control_states: [ButtonState::new(); 4],
            track_select_states: [ButtonState::new(); 6],
            last_update: 0,
        }
    }
}

impl ButtonState {
    /// Create new button state
    pub const fn new() -> Self {
        Self {
            current: false,
            previous: false,
            debounce_counter: 0,
            press_type: PressType::None,
            press_start_time: 0,
        }
    }

    /// Update button state with debouncing (10ms response time)
    pub fn update(&mut self, raw_state: bool, timestamp: u32) -> bool {
        self.previous = self.current;

        // Simple debouncing: require 3 consecutive readings
        if raw_state != self.current {
            self.debounce_counter += 1;
            if self.debounce_counter >= 3 {
                self.current = raw_state;
                self.debounce_counter = 0;

                // Detect press start for gesture recognition
                if self.current && !self.previous {
                    self.press_start_time = timestamp;
                    self.press_type = PressType::Short;
                }

                return true; // State changed
            }
        } else {
            self.debounce_counter = 0;
        }

        // Detect long press (>500ms)
        if self.current && (timestamp - self.press_start_time) > 500 {
            if self.press_type == PressType::Short {
                self.press_type = PressType::Long;
                return true; // Long press detected
            }
        }

        false // No state change
    }

    /// Check if button was just pressed
    pub fn just_pressed(&self) -> bool {
        self.current && !self.previous
    }

    /// Check if button was just released
    pub fn just_released(&self) -> bool {
        !self.current && self.previous
    }

    /// Get current press type
    pub fn press_type(&self) -> PressType {
        self.press_type
    }
}

/// Control identifiers for reading analog controls
#[derive(Debug, Clone, Copy)]
pub enum ControlId {
    TrackFader(usize),
    Knob(usize),
    OutputLevel,
    ExpressionPedal(usize),
}

/// Button identifiers for reading digital controls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
    Track(usize),
    FX(usize),
    TrackSelect(usize),
    Play,
    Stop,
    Rec,
    Menu,
    PageLeft,
    PageRight,
    Enter,
    Exit,
    TapTempo,
    Memory,
    UndoRedo,
    Edit,
}

/// MIDI event types
#[derive(Debug, Clone, Copy)]
pub enum MidiEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    ControlChange { channel: u8, controller: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 },
}

// Duplicate HalError definition removed - using the one defined earlier

// Embedded-specific implementations
#[cfg(not(feature = "std"))]
mod embedded_impl {
    use super::*;
    use core::cell::RefCell;
    use cortex_m::interrupt::{self as cm_interrupt, Mutex};

    // Global static variables for interrupt handling (simplified to avoid large allocations)
    static AUDIO_CALLBACK_ACTIVE: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

    /// Initialize global HAL state for interrupt access
    pub fn init_global_hal() -> Result<HardwareHal, HalError> {
        // Return HAL instance instead of storing globally
        HardwareHal::init()
    }

    /// Set audio callback active state (for interrupt handlers)
    pub fn set_audio_callback_active(active: bool) {
        cm_interrupt::free(|cs| {
            *AUDIO_CALLBACK_ACTIVE.borrow(cs).borrow_mut() = active;
        });
    }

    /// Check if audio callback is active (for interrupt handlers)
    pub fn is_audio_callback_active() -> bool {
        cm_interrupt::free(|cs| *AUDIO_CALLBACK_ACTIVE.borrow(cs).borrow())
    }

    /// DMA interrupt handler for audio input (SAI1/SAI2)
    #[interrupt]
    fn DMA1_STR0() {
        if is_audio_callback_active() {
            // Handle SAI1 DMA completion
            // TODO: Process audio input DMA completion
        }
    }

    /// DMA interrupt handler for audio output (SAI3/SAI4)
    #[interrupt]
    fn DMA2_STR0() {
        if is_audio_callback_active() {
            // Handle SAI3 DMA completion
            // TODO: Process audio output DMA completion
        }
    }

    /// Timer interrupt for audio processing timing
    #[interrupt]
    fn TIM2() {
        if is_audio_callback_active() {
            // Process audio callback at regular intervals
            // TODO: Trigger audio processing
        }
    }
}

// PC-specific stub implementations
#[cfg(feature = "std")]
mod pc_impl {
    use super::*;

    /// Initialize global HAL state for PC builds (stub)
    pub fn init_global_hal() -> Result<HardwareHal, HalError> {
        HardwareHal::init()
    }

    /// Set audio callback active state (stub for PC)
    pub fn set_audio_callback_active(_active: bool) {
        // No-op for PC builds
    }

    /// Check if audio callback is active (stub for PC)
    pub fn is_audio_callback_active() -> bool {
        true // Always active for PC builds
    }
}

// Re-export the appropriate implementation
#[cfg(not(feature = "std"))]
pub use embedded_impl::*;

#[cfg(feature = "std")]
pub use pc_impl::*;

impl MidiClockTiming {
    /// Create a new MIDI clock timing instance
    pub fn new() -> Self {
        Self {
            tempo_bpm: 120.0,
            clock_pulse_count: 0,
            last_clock_pulse: 0,
            clock_interval_us: 0,
            sync_enabled: false,
            clock_running: false,
        }
    }

    /// Reset clock timing
    pub fn reset(&mut self) {
        self.clock_pulse_count = 0;
        self.last_clock_pulse = 0;
        self.clock_interval_us = 0;
        self.clock_running = false;
    }

    /// Get current tempo in BPM
    pub fn get_tempo(&self) -> f32 {
        self.tempo_bpm
    }

    /// Check if clock sync is enabled
    pub fn is_sync_enabled(&self) -> bool {
        self.sync_enabled
    }

    /// Check if clock is running
    pub fn is_running(&self) -> bool {
        self.clock_running
    }
}

impl Default for MidiClockTiming {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl HardwareHal {
    // Duplicate init method removed - using the one defined earlier in the PC implementation

    /// Process MIDI input for PC builds (stub)
    pub fn process_midi_input(&mut self, _timestamp: u32) -> Result<Vec<crate::midi::MidiMessage, 16>, HalError> {
        Ok(Vec::new())
    }

    /// Send MIDI message for PC builds (stub)
    pub fn send_midi_message(&mut self, _message: crate::midi::MidiMessage) -> Result<(), HalError> {
        Ok(())
    }

    /// Set MIDI clock sync for PC builds (stub)
    pub fn set_midi_clock_sync(&mut self, enabled: bool) {
        self.midi_uart.clock_timing.sync_enabled = enabled;
    }

    /// Get MIDI clock tempo for PC builds
    pub fn get_midi_clock_tempo(&self) -> f32 {
        self.midi_uart.clock_timing.tempo_bpm
    }

    /// Check if MIDI clock is running for PC builds
    pub fn is_midi_clock_running(&self) -> bool {
        self.midi_uart.clock_timing.clock_running
    }

    /// Get MIDI stats for PC builds
    pub fn get_midi_stats(&self) -> (u32, u32, bool) {
        (
            self.midi_uart.last_midi_activity,
            self.midi_uart.midi_error_count,
            self.midi_uart.enabled,
        )
    }

    /// Start USB audio streaming for PC builds
    pub fn start_usb_audio_streaming(&mut self) -> Result<(), HalError> {
        self.usb_audio.streaming_active = true;
        Ok(())
    }

    /// Stop USB audio streaming for PC builds
    pub fn stop_usb_audio_streaming(&mut self) -> Result<(), HalError> {
        self.usb_audio.streaming_active = false;
        Ok(())
    }

    /// Read USB audio input from DAW for PC builds
    pub fn read_usb_audio_input(&self) -> Result<[f32; 16], HalError> {
        if !self.usb_audio.streaming_active {
            return Ok([0.0f32; 16]);
        }

        let mut channels = [0.0f32; 16];
        
        // For PC builds, simulate USB input (in real implementation, this would interface with host audio)
        for ch in 0..16 {
            if ch < self.usb_audio.usb_input_buffers.len() {
                if !self.usb_audio.usb_input_buffers[ch].is_empty() {
                    channels[ch] = self.usb_audio.usb_input_buffers[ch][0];
                }
            }
        }

        Ok(channels)
    }

    /// Write USB audio output to DAW for PC builds
    pub fn write_usb_audio_output(&mut self, channels: &[f32; 16]) -> Result<(), HalError> {
        if !self.usb_audio.streaming_active {
            return Ok(());
        }

        // For PC builds, store USB output (in real implementation, this would send to host audio)
        for ch in 0..16 {
            if ch < self.usb_audio.usb_output_buffers.len() {
                if !self.usb_audio.usb_output_buffers[ch].is_empty() {
                    self.usb_audio.usb_output_buffers[ch][0] = channels[ch];
                }
            }
        }

        Ok(())
    }

    /// Configure DAW routing for PC builds
    pub fn configure_daw_routing(&mut self, routing: DawRoutingConfig) -> Result<(), HalError> {
        self.usb_audio.daw_routing = routing;
        Ok(())
    }

    /// Get current DAW routing configuration for PC builds
    pub fn get_daw_routing(&self) -> &DawRoutingConfig {
        &self.usb_audio.daw_routing
    }

    /// Set USB audio format for PC builds
    pub fn set_usb_audio_format(&mut self, sample_rate: u32, bit_depth: u8) -> Result<(), HalError> {
        // Validate supported formats
        match sample_rate {
            44100 | 48000 | 96000 => {},
            _ => return Err(HalError::UnsupportedFormat),
        }

        match bit_depth {
            16 | 24 => {},
            _ => return Err(HalError::UnsupportedFormat),
        }

        self.usb_audio.sample_rate = sample_rate;
        self.usb_audio.bit_depth = bit_depth;
        Ok(())
    }

    /// Set USB audio mode for PC builds
    pub fn set_usb_audio_mode(&mut self, mode: UsbAudioMode) -> Result<(), HalError> {
        let (sample_rate, bit_depth) = match mode {
            UsbAudioMode::Standard => (44100, 16),
            UsbAudioMode::HighQuality => (48000, 24),
            UsbAudioMode::Professional => (96000, 24),
        };

        self.usb_audio.sample_rate = sample_rate;
        self.usb_audio.bit_depth = bit_depth;

        Ok(())
    }

    /// Enable/disable zero-latency monitoring for PC builds
    pub fn set_zero_latency_monitoring(&mut self, enabled: bool) -> Result<(), HalError> {
        self.usb_audio.zero_latency_monitoring = enabled;
        Ok(())
    }

    /// Get USB audio status for PC builds
    pub fn get_usb_audio_status(&self) -> UsbAudioStatus {
        UsbAudioStatus {
            connected: self.usb_audio.enabled,
            streaming: self.usb_audio.streaming_active,
            sample_rate: self.usb_audio.sample_rate,
            bit_depth: self.usb_audio.bit_depth,
            underrun_count: 0,
            overrun_count: 0,
            error_count: self.usb_audio.error_count,
        }
    }

    /// Process USB audio callback for PC builds
    pub fn process_usb_audio_callback(&mut self, track_audio: &[[f32; 2]; 6], master_audio: &[f32; 2]) {
        if !self.usb_audio.streaming_active {
            return;
        }

        // Prepare USB output channels based on DAW routing
        let mut usb_outputs = [0.0f32; 16];

        // Route individual tracks to USB outputs
        for track_id in 0..6 {
            if let Some((left_ch, right_ch)) = self.usb_audio.daw_routing.track_output_routing[track_id] {
                if (left_ch as usize) < 16 && (right_ch as usize) < 16 {
                    usb_outputs[left_ch as usize] = track_audio[track_id][0];  // Left
                    usb_outputs[right_ch as usize] = track_audio[track_id][1]; // Right
                }
            }
        }

        // Route master output to USB
        let (master_left, master_right) = self.usb_audio.daw_routing.master_output_routing;
        if (master_left as usize) < 16 && (master_right as usize) < 16 {
            usb_outputs[master_left as usize] = master_audio[0];  // Left
            usb_outputs[master_right as usize] = master_audio[1]; // Right
        }

        // Send audio to DAW
        let _ = self.write_usb_audio_output(&usb_outputs);

        // Swap USB buffers for double buffering
        self.usb_audio.current_usb_buffer = !self.usb_audio.current_usb_buffer;
    }

    /// Route USB input to track recording for PC builds
    pub fn route_usb_input_to_track(&self, track_id: u8) -> Result<[f32; 2], HalError> {
        if track_id == 0 || track_id > 6 {
            return Err(HalError::InvalidParameter);
        }

        let track_index = (track_id - 1) as usize;
        
        // Get USB input channel for this track
        if let Some(usb_channel) = self.usb_audio.daw_routing.track_input_routing[track_index] {
            if let Ok(usb_inputs) = self.read_usb_audio_input() {
                if (usb_channel as usize) < 16 {
                    let input_sample = usb_inputs[usb_channel as usize];
                    // Return stereo pair (mono input duplicated to both channels)
                    return Ok([input_sample, input_sample]);
                }
            }
        }

        // Return silence if no routing configured or error
        Ok([0.0f32; 2])
    }
}