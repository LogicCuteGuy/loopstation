use serde::{Deserialize, Serialize};
use heapless::Vec;

// Re-export control assignments from storage module
pub use crate::storage::{
    ControlAssignments, FXButtonAssignment, MidiAssignment, MidiTarget,
    ExpressionAssignment, FootswitchAssignment
};

/// Button identifiers for the loopstation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonId {
    // Track buttons
    Track1,
    Track2,
    Track3,
    Track4,
    Track5,
    Track6,
    
    // Track select buttons
    TrackSelect1,
    TrackSelect2,
    TrackSelect3,
    TrackSelect4,
    TrackSelect5,
    TrackSelect6,
    
    // FX buttons
    FX1,
    FX2,
    FX3,
    FX4,
    FX5,
    
    // Transport controls
    Play,
    Stop,
    Rec,
    
    // Utility buttons
    UndoRedo,
    TapTempo,
    Memory,
    
    // Menu navigation
    Menu,
    PageLeft,
    PageRight,
    Enter,
    Exit,
    Edit,
    
    // Expression/Control
    CTL1,
    CTL2,
    CTL3,
    CTL4,
}

/// Button press types for gesture recognition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonPress {
    /// Short press (< 500ms)
    Short,
    /// Long press (> 500ms)
    Long,
    /// Double press (two short presses within 300ms)
    Double,
}

/// Button functions that can be assigned
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonFunction {
    // Track operations
    TrackPlayStop(u8),      // Track number 1-6
    TrackRecord(u8),        // Track number 1-6
    TrackClear(u8),         // Track number 1-6
    TrackSelect(u8),        // Track number 1-6
    TrackMute(u8),          // Track number 1-6
    
    // Effect operations
    EffectMomentary(EffectSlot),
    EffectToggle(EffectSlot),
    
    // Transport
    AllStart,
    AllStop,
    AllClear,
    
    // Utility
    Undo,
    Redo,
    TapTempo,
    TempoReset,
    
    // Menu navigation
    MenuOpen,
    MenuExit,
    PageLeft,
    PageRight,
    Enter,
    
    // Memory operations
    MemorySave,
    MemoryLoad,
    MemoryInc,
    MemoryDec,
    
    // System
    Panic,  // Stop all audio immediately
}

/// Effect slot identifier for FX button assignments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectSlot {
    /// Effect chain type
    pub chain_type: crate::effects::EffectChainType,
    /// Slot index (0-3)
    pub slot_index: u8,
    /// Target track (for Track FX only)
    pub track_id: Option<u8>,
}

/// Knob identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnobId {
    Knob1,
    Knob2,
    Knob3,
    Knob4,
    OutputLevel,
}

/// Fader identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaderId {
    Track1Level,
    Track2Level,
    Track3Level,
    Track4Level,
    Track5Level,
    Track6Level,
}

/// Expression input identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpressionInput {
    CTL1_EXP1,
    CTL2_EXP1,
    CTL3_EXP2,
    CTL4_EXP2,
}

/// Control input event
#[derive(Debug, Clone, PartialEq)]
pub enum ControlEvent {
    /// Button press event
    ButtonPress {
        button: ButtonId,
        press_type: ButtonPress,
    },
    /// Knob turn event
    KnobTurn {
        knob: KnobId,
        value: f32,  // 0.0 to 1.0
    },
    /// Fader movement event
    FaderMove {
        fader: FaderId,
        value: f32,  // 0.0 to 1.0
    },
    /// Expression pedal/control input
    ExpressionInput {
        input: ExpressionInput,
        value: f32,  // 0.0 to 1.0
    },
}

/// Control system state and configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlSystem {
    /// Control assignments
    pub assignments: ControlAssignments,
    /// Button debounce time in milliseconds
    pub debounce_time_ms: u32,
    /// Long press threshold in milliseconds
    pub long_press_threshold_ms: u32,
    /// Double press window in milliseconds
    pub double_press_window_ms: u32,
}

impl ControlSystem {
    /// Create a new control system with default settings
    pub fn new() -> Self {
        Self {
            assignments: ControlAssignments::new(),
            debounce_time_ms: 10,      // 10ms debounce for buttons
            long_press_threshold_ms: 500,  // 500ms for long press
            double_press_window_ms: 300,   // 300ms window for double press
        }
    }

    /// Process a control event and return the corresponding function
    pub fn process_control_event(&self, event: ControlEvent) -> Option<ButtonFunction> {
        match event {
            ControlEvent::ButtonPress { button, press_type } => {
                self.get_button_function(button, press_type)
            },
            ControlEvent::KnobTurn { knob: _, value: _ } => {
                // Knob events are context-dependent and handled elsewhere
                None
            },
            ControlEvent::FaderMove { fader, value: _ } => {
                // Convert fader movement to track level change
                match fader {
                    FaderId::Track1Level => Some(ButtonFunction::TrackPlayStop(1)), // Placeholder
                    FaderId::Track2Level => Some(ButtonFunction::TrackPlayStop(2)), // Placeholder
                    FaderId::Track3Level => Some(ButtonFunction::TrackPlayStop(3)), // Placeholder
                    FaderId::Track4Level => Some(ButtonFunction::TrackPlayStop(4)), // Placeholder
                    FaderId::Track5Level => Some(ButtonFunction::TrackPlayStop(5)), // Placeholder
                    FaderId::Track6Level => Some(ButtonFunction::TrackPlayStop(6)), // Placeholder
                }
            },
            ControlEvent::ExpressionInput { input: _, value: _ } => {
                // Expression inputs are handled via assignments
                None
            },
        }
    }

    /// Get button function based on button and press type
    fn get_button_function(&self, button: ButtonId, press_type: ButtonPress) -> Option<ButtonFunction> {
        match (button, press_type) {
            // Track buttons - context dependent behavior
            (ButtonId::Track1, ButtonPress::Short) => Some(ButtonFunction::TrackPlayStop(1)),
            (ButtonId::Track1, ButtonPress::Long) => Some(ButtonFunction::TrackRecord(1)),
            (ButtonId::Track1, ButtonPress::Double) => Some(ButtonFunction::TrackClear(1)),
            
            (ButtonId::Track2, ButtonPress::Short) => Some(ButtonFunction::TrackPlayStop(2)),
            (ButtonId::Track2, ButtonPress::Long) => Some(ButtonFunction::TrackRecord(2)),
            (ButtonId::Track2, ButtonPress::Double) => Some(ButtonFunction::TrackClear(2)),
            
            (ButtonId::Track3, ButtonPress::Short) => Some(ButtonFunction::TrackPlayStop(3)),
            (ButtonId::Track3, ButtonPress::Long) => Some(ButtonFunction::TrackRecord(3)),
            (ButtonId::Track3, ButtonPress::Double) => Some(ButtonFunction::TrackClear(3)),
            
            (ButtonId::Track4, ButtonPress::Short) => Some(ButtonFunction::TrackPlayStop(4)),
            (ButtonId::Track4, ButtonPress::Long) => Some(ButtonFunction::TrackRecord(4)),
            (ButtonId::Track4, ButtonPress::Double) => Some(ButtonFunction::TrackClear(4)),
            
            (ButtonId::Track5, ButtonPress::Short) => Some(ButtonFunction::TrackPlayStop(5)),
            (ButtonId::Track5, ButtonPress::Long) => Some(ButtonFunction::TrackRecord(5)),
            (ButtonId::Track5, ButtonPress::Double) => Some(ButtonFunction::TrackClear(5)),
            
            (ButtonId::Track6, ButtonPress::Short) => Some(ButtonFunction::TrackPlayStop(6)),
            (ButtonId::Track6, ButtonPress::Long) => Some(ButtonFunction::TrackRecord(6)),
            (ButtonId::Track6, ButtonPress::Double) => Some(ButtonFunction::TrackClear(6)),
            
            // Track select buttons
            (ButtonId::TrackSelect1, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(1)),
            (ButtonId::TrackSelect1, ButtonPress::Long) => Some(ButtonFunction::TrackMute(1)),
            (ButtonId::TrackSelect2, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(2)),
            (ButtonId::TrackSelect2, ButtonPress::Long) => Some(ButtonFunction::TrackMute(2)),
            (ButtonId::TrackSelect3, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(3)),
            (ButtonId::TrackSelect3, ButtonPress::Long) => Some(ButtonFunction::TrackMute(3)),
            (ButtonId::TrackSelect4, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(4)),
            (ButtonId::TrackSelect4, ButtonPress::Long) => Some(ButtonFunction::TrackMute(4)),
            (ButtonId::TrackSelect5, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(5)),
            (ButtonId::TrackSelect5, ButtonPress::Long) => Some(ButtonFunction::TrackMute(5)),
            (ButtonId::TrackSelect6, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(6)),
            (ButtonId::TrackSelect6, ButtonPress::Long) => Some(ButtonFunction::TrackMute(6)),
            
            // FX buttons - assignments from FX button assignments
            (ButtonId::FX1, press_type) => self.get_fx_button_function(0, press_type),
            (ButtonId::FX2, press_type) => self.get_fx_button_function(1, press_type),
            (ButtonId::FX3, press_type) => self.get_fx_button_function(2, press_type),
            (ButtonId::FX4, press_type) => self.get_fx_button_function(3, press_type),
            (ButtonId::FX5, press_type) => self.get_fx_button_function(4, press_type),
            
            // Transport controls
            (ButtonId::Play, ButtonPress::Short) => Some(ButtonFunction::AllStart),
            (ButtonId::Play, ButtonPress::Long) => Some(ButtonFunction::AllStop),
            (ButtonId::Stop, ButtonPress::Short) => Some(ButtonFunction::AllStop),
            (ButtonId::Stop, ButtonPress::Long) => Some(ButtonFunction::AllClear),
            
            // Utility buttons
            (ButtonId::UndoRedo, ButtonPress::Short) => Some(ButtonFunction::Undo),
            (ButtonId::UndoRedo, ButtonPress::Long) => Some(ButtonFunction::Redo),
            (ButtonId::TapTempo, ButtonPress::Short) => Some(ButtonFunction::TapTempo),
            (ButtonId::TapTempo, ButtonPress::Long) => Some(ButtonFunction::TempoReset),
            
            // Memory
            (ButtonId::Memory, ButtonPress::Short) => Some(ButtonFunction::MemoryLoad),
            (ButtonId::Memory, ButtonPress::Long) => Some(ButtonFunction::MemorySave),
            
            // Menu navigation
            (ButtonId::Menu, ButtonPress::Short) => Some(ButtonFunction::MenuOpen),
            (ButtonId::Menu, ButtonPress::Long) => Some(ButtonFunction::MenuExit),
            (ButtonId::PageLeft, _) => Some(ButtonFunction::PageLeft),
            (ButtonId::PageRight, _) => Some(ButtonFunction::PageRight),
            (ButtonId::Enter, _) => Some(ButtonFunction::Enter),
            (ButtonId::Exit, _) => Some(ButtonFunction::MenuExit),
            
            _ => None,
        }
    }

    /// Get FX button function from assignments
    fn get_fx_button_function(&self, fx_button_index: usize, press_type: ButtonPress) -> Option<ButtonFunction> {
        if let Some(assignment) = self.assignments.fx_button_assignments.get(fx_button_index)? {
            let effect_slot = EffectSlot {
                chain_type: assignment.chain_type,
                slot_index: assignment.slot_index,
                track_id: assignment.target_track,
            };

            match press_type {
                ButtonPress::Short if assignment.momentary => Some(ButtonFunction::EffectMomentary(effect_slot)),
                ButtonPress::Short => Some(ButtonFunction::EffectToggle(effect_slot)),
                ButtonPress::Long => Some(ButtonFunction::EffectToggle(effect_slot)),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Set FX button assignment
    pub fn set_fx_button_assignment(&mut self, button_index: usize, assignment: Option<FXButtonAssignment>) {
        if button_index < self.assignments.fx_button_assignments.len() {
            self.assignments.fx_button_assignments[button_index] = assignment;
        }
    }

    /// Get FX button assignment
    pub fn get_fx_button_assignment(&self, button_index: usize) -> Option<&FXButtonAssignment> {
        self.assignments.fx_button_assignments.get(button_index)?.as_ref()
    }
}

impl Default for ControlSystem {
    fn default() -> Self {
        Self::new()
    }
}
// Hardware control interface with debouncing and gesture recognition
pub struct HardwareControlInterface {
    /// Button states for debouncing
    button_states: [ButtonState; 32], // Support up to 32 buttons
    /// Control system configuration
    control_system: ControlSystem,
    /// Current timestamp for timing
    current_time_ms: u32,
    /// Analog control values with smoothing
    analog_controls: AnalogControlManager,
}

/// Button state for debouncing and gesture recognition
#[derive(Debug, Clone, Copy)]
struct ButtonState {
    /// Current physical state (pressed/released)
    physical_state: bool,
    /// Debounced logical state
    logical_state: bool,
    /// Last state change timestamp
    last_change_time: u32,
    /// Press start timestamp
    press_start_time: u32,
    /// Last press end timestamp (for double press detection)
    last_press_end_time: u32,
    /// Press count for double press detection
    press_count: u8,
    /// Whether this is a long press
    is_long_press: bool,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self {
            physical_state: false,
            logical_state: false,
            last_change_time: 0,
            press_start_time: 0,
            last_press_end_time: 0,
            press_count: 0,
            is_long_press: false,
        }
    }
}

/// Analog control manager for smooth value reading
#[derive(Debug)]
struct AnalogControlManager {
    /// Smoothed values for each control
    smoothed_values: [f32; 16], // Support up to 16 analog controls
    /// Previous raw values for change detection
    previous_values: [f32; 16],
    /// Smoothing factor (0.0 = no smoothing, 1.0 = maximum smoothing)
    smoothing_factor: f32,
    /// Minimum change threshold to register as movement
    change_threshold: f32,
}

impl Default for AnalogControlManager {
    fn default() -> Self {
        Self {
            smoothed_values: [0.0; 16],
            previous_values: [0.0; 16],
            smoothing_factor: 0.1, // Light smoothing
            change_threshold: 0.005, // 0.5% change threshold
        }
    }
}

impl HardwareControlInterface {
    /// Create a new hardware control interface
    pub fn new() -> Self {
        Self {
            button_states: [ButtonState::default(); 32],
            control_system: ControlSystem::new(),
            current_time_ms: 0,
            analog_controls: AnalogControlManager::default(),
        }
    }

    /// Update the current timestamp (called from main loop)
    pub fn update_time(&mut self, time_ms: u32) {
        self.current_time_ms = time_ms;
    }

    /// Process button input with debouncing and gesture recognition
    /// Returns Some(ControlEvent) if a button event is detected
    pub fn process_button(&mut self, button_id: ButtonId, physical_pressed: bool) -> Option<ControlEvent> {
        let button_index = self.button_id_to_index(button_id);
        if button_index >= self.button_states.len() {
            return None;
        }

        let button_state = &mut self.button_states[button_index];
        let debounce_time = self.control_system.debounce_time_ms;
        let long_press_threshold = self.control_system.long_press_threshold_ms;
        let double_press_window = self.control_system.double_press_window_ms;

        // Update physical state
        if button_state.physical_state != physical_pressed {
            button_state.physical_state = physical_pressed;
            button_state.last_change_time = self.current_time_ms;
        }

        // Debouncing: only update logical state after debounce time
        let time_since_change = self.current_time_ms.saturating_sub(button_state.last_change_time);
        if time_since_change >= debounce_time {
            let new_logical_state = button_state.physical_state;
            
            if button_state.logical_state != new_logical_state {
                button_state.logical_state = new_logical_state;
                
                if new_logical_state {
                    // Button pressed
                    button_state.press_start_time = self.current_time_ms;
                    button_state.is_long_press = false;
                    return None; // Wait for release to determine press type
                } else {
                    // Button released
                    let press_duration = self.current_time_ms.saturating_sub(button_state.press_start_time);
                    
                    // Determine press type
                    let press_type = if button_state.is_long_press {
                        ButtonPress::Long
                    } else if press_duration >= long_press_threshold {
                        button_state.is_long_press = true;
                        ButtonPress::Long
                    } else {
                        // Check for double press
                        let time_since_last_press = self.current_time_ms.saturating_sub(button_state.last_press_end_time);
                        if time_since_last_press <= double_press_window && button_state.press_count == 1 {
                            button_state.press_count = 0;
                            button_state.last_press_end_time = self.current_time_ms;
                            ButtonPress::Double
                        } else {
                            button_state.press_count = 1;
                            button_state.last_press_end_time = self.current_time_ms;
                            ButtonPress::Short
                        }
                    };

                    return Some(ControlEvent::ButtonPress {
                        button: button_id,
                        press_type,
                    });
                }
            }
        }

        // Check for long press while button is held
        if button_state.logical_state && !button_state.is_long_press {
            let press_duration = self.current_time_ms.saturating_sub(button_state.press_start_time);
            if press_duration >= long_press_threshold {
                button_state.is_long_press = true;
                return Some(ControlEvent::ButtonPress {
                    button: button_id,
                    press_type: ButtonPress::Long,
                });
            }
        }

        None
    }

    /// Process analog control input with smoothing
    /// Returns Some(ControlEvent) if a significant change is detected
    pub fn process_analog_control(&mut self, control_id: AnalogControlId, raw_value: f32) -> Option<ControlEvent> {
        let control_index = self.analog_control_id_to_index(control_id);
        if control_index >= self.analog_controls.smoothed_values.len() {
            return None;
        }

        let clamped_value = raw_value.clamp(0.0, 1.0);
        let previous_smoothed = self.analog_controls.smoothed_values[control_index];
        
        // Apply smoothing
        let smoothing = self.analog_controls.smoothing_factor;
        let new_smoothed = previous_smoothed * smoothing + clamped_value * (1.0 - smoothing);
        self.analog_controls.smoothed_values[control_index] = new_smoothed;

        // Check if change is significant enough to report
        let change = (new_smoothed - previous_smoothed).abs();
        if change >= self.analog_controls.change_threshold {
            self.analog_controls.previous_values[control_index] = new_smoothed;
            
            match control_id {
                AnalogControlId::Knob(knob_id) => Some(ControlEvent::KnobTurn {
                    knob: knob_id,
                    value: new_smoothed,
                }),
                AnalogControlId::Fader(fader_id) => Some(ControlEvent::FaderMove {
                    fader: fader_id,
                    value: new_smoothed,
                }),
                AnalogControlId::Expression(exp_id) => Some(ControlEvent::ExpressionInput {
                    input: exp_id,
                    value: new_smoothed,
                }),
            }
        } else {
            None
        }
    }

    /// Get the current smoothed value for an analog control
    pub fn get_analog_value(&self, control_id: AnalogControlId) -> f32 {
        let control_index = self.analog_control_id_to_index(control_id);
        if control_index < self.analog_controls.smoothed_values.len() {
            self.analog_controls.smoothed_values[control_index]
        } else {
            0.0
        }
    }

    /// Process a control event and return the corresponding function
    pub fn process_control_event(&self, event: ControlEvent) -> Option<ButtonFunction> {
        self.control_system.process_control_event(event)
    }

    /// Get control system reference
    pub fn control_system(&self) -> &ControlSystem {
        &self.control_system
    }

    /// Get mutable control system reference
    pub fn control_system_mut(&mut self) -> &mut ControlSystem {
        &mut self.control_system
    }

    /// Set smoothing parameters for analog controls
    pub fn set_analog_smoothing(&mut self, smoothing_factor: f32, change_threshold: f32) {
        self.analog_controls.smoothing_factor = smoothing_factor.clamp(0.0, 1.0);
        self.analog_controls.change_threshold = change_threshold.clamp(0.0, 1.0);
    }

    /// Update LED states based on control system state
    pub fn update_leds(&mut self, hal: &mut crate::hal::HardwareHal) -> Result<(), crate::hal::HalError> {
        // Update LEDs based on current control assignments and system state
        
        // Update FX button LEDs based on assignments
        for (fx_index, assignment) in self.control_system().assignments.fx_button_assignments.iter().enumerate() {
            let fx_id = (fx_index + 1) as u8;
            if let Some(assignment) = assignment {
                // LED on if FX is assigned
                hal.set_led(crate::hal::LedId::FX(fx_id), crate::hal::LedCommand::On)?;
            } else {
                // LED off if no assignment
                hal.set_led(crate::hal::LedId::FX(fx_id), crate::hal::LedCommand::Off)?;
            }
        }

        // Update system status LEDs
        hal.set_led(crate::hal::LedId::Power, crate::hal::LedCommand::On)?;
        hal.set_led(crate::hal::LedId::Error, crate::hal::LedCommand::Off)?;

        // Apply LED updates
        hal.update_leds()?;

        Ok(())
    }

    /// Convert ButtonId to array index
    fn button_id_to_index(&self, button_id: ButtonId) -> usize {
        match button_id {
            ButtonId::Track1 => 0,
            ButtonId::Track2 => 1,
            ButtonId::Track3 => 2,
            ButtonId::Track4 => 3,
            ButtonId::Track5 => 4,
            ButtonId::Track6 => 5,
            ButtonId::TrackSelect1 => 6,
            ButtonId::TrackSelect2 => 7,
            ButtonId::TrackSelect3 => 8,
            ButtonId::TrackSelect4 => 9,
            ButtonId::TrackSelect5 => 10,
            ButtonId::TrackSelect6 => 11,
            ButtonId::FX1 => 12,
            ButtonId::FX2 => 13,
            ButtonId::FX3 => 14,
            ButtonId::FX4 => 15,
            ButtonId::FX5 => 16,
            ButtonId::Play => 17,
            ButtonId::Stop => 18,
            ButtonId::Rec => 19,
            ButtonId::UndoRedo => 20,
            ButtonId::TapTempo => 21,
            ButtonId::Memory => 22,
            ButtonId::Menu => 23,
            ButtonId::PageLeft => 24,
            ButtonId::PageRight => 25,
            ButtonId::Enter => 26,
            ButtonId::Exit => 27,
            ButtonId::Edit => 28,
            ButtonId::CTL1 => 29,
            ButtonId::CTL2 => 30,
            ButtonId::CTL3 => 31,
            ButtonId::CTL4 => 31, // Reuse last slot if needed
        }
    }

    /// Convert analog control ID to array index
    fn analog_control_id_to_index(&self, control_id: AnalogControlId) -> usize {
        match control_id {
            AnalogControlId::Knob(KnobId::Knob1) => 0,
            AnalogControlId::Knob(KnobId::Knob2) => 1,
            AnalogControlId::Knob(KnobId::Knob3) => 2,
            AnalogControlId::Knob(KnobId::Knob4) => 3,
            AnalogControlId::Knob(KnobId::OutputLevel) => 4,
            AnalogControlId::Fader(FaderId::Track1Level) => 5,
            AnalogControlId::Fader(FaderId::Track2Level) => 6,
            AnalogControlId::Fader(FaderId::Track3Level) => 7,
            AnalogControlId::Fader(FaderId::Track4Level) => 8,
            AnalogControlId::Fader(FaderId::Track5Level) => 9,
            AnalogControlId::Fader(FaderId::Track6Level) => 10,
            AnalogControlId::Expression(ExpressionInput::CTL1_EXP1) => 11,
            AnalogControlId::Expression(ExpressionInput::CTL2_EXP1) => 12,
            AnalogControlId::Expression(ExpressionInput::CTL3_EXP2) => 13,
            AnalogControlId::Expression(ExpressionInput::CTL4_EXP2) => 14,
        }
    }
}

/// Unified analog control identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalogControlId {
    Knob(KnobId),
    Fader(FaderId),
    Expression(ExpressionInput),
}

/// Control interface integration with hardware HAL
pub struct ControlInterfaceHal {
    /// Hardware control interface
    control_interface: HardwareControlInterface,
    /// Last button read states to detect changes
    last_button_states: [bool; 32],
    /// Last analog values to detect changes
    last_analog_values: [f32; 16],
}

impl ControlInterfaceHal {
    /// Create a new control interface HAL
    pub fn new() -> Self {
        Self {
            control_interface: HardwareControlInterface::new(),
            last_button_states: [false; 32],
            last_analog_values: [0.0; 16],
        }
    }

    /// Update control interface with hardware readings using PCF8575 I2C expanders and rotary encoder
    /// This should be called regularly (e.g., every 10ms) from the main loop for 10ms response time
    pub fn update(&mut self, hal: &mut crate::hal::HardwareHal, time_ms: u32) -> Vec<ControlEvent, 16> {
        self.control_interface.update_time(time_ms);
        let mut events = Vec::new();

        // Get button events from PCF8575 controllers with debouncing and gesture recognition
        let button_events = hal.update_button_states(time_ms);
        
        // Convert HAL button events to control events
        for (hal_button_id, press_type) in button_events {
            // Map HAL ButtonId to controls ButtonId
            if let Some(control_button_id) = self.map_hal_button_to_control_button(hal_button_id) {
                // Convert PressType to ButtonPress
                let button_press = match press_type {
                    crate::hal::PressType::Short => ButtonPress::Short,
                    crate::hal::PressType::Long => ButtonPress::Long,
                    crate::hal::PressType::Double => ButtonPress::Double,
                    crate::hal::PressType::None => continue, // Skip None events
                };

                let control_event = ControlEvent::ButtonPress {
                    button: control_button_id,
                    press_type: button_press,
                };

                if events.push(control_event).is_err() {
                    break; // Event buffer full
                }
            }
        }

        // Get rotary encoder events and convert to control events
        let rotary_events = hal.update_rotary_encoder(time_ms);
        for rotary_event in rotary_events {
            let control_event = match rotary_event {
                crate::hal::RotaryEvent::Clockwise => {
                    // Treat clockwise rotation as PageRight button press
                    ControlEvent::ButtonPress {
                        button: ButtonId::PageRight,
                        press_type: ButtonPress::Short,
                    }
                }
                crate::hal::RotaryEvent::CounterClockwise => {
                    // Treat counter-clockwise rotation as PageLeft button press
                    ControlEvent::ButtonPress {
                        button: ButtonId::PageLeft,
                        press_type: ButtonPress::Short,
                    }
                }
                crate::hal::RotaryEvent::ButtonPress => {
                    // Treat encoder button press as Enter button press
                    ControlEvent::ButtonPress {
                        button: ButtonId::Enter,
                        press_type: ButtonPress::Short,
                    }
                }
                crate::hal::RotaryEvent::ButtonRelease => {
                    // Ignore button release for now
                    continue;
                }
            };

            if events.push(control_event).is_err() {
                break; // Event buffer full
            }
        }

        events
    }

    /// Map HAL ButtonId to controls ButtonId
    fn map_hal_button_to_control_button(&self, hal_button_id: crate::hal::ButtonId) -> Option<ButtonId> {
        match hal_button_id {
            crate::hal::ButtonId::Track(0) => Some(ButtonId::Track1),
            crate::hal::ButtonId::Track(1) => Some(ButtonId::Track2),
            crate::hal::ButtonId::Track(2) => Some(ButtonId::Track3),
            crate::hal::ButtonId::Track(3) => Some(ButtonId::Track4),
            crate::hal::ButtonId::Track(4) => Some(ButtonId::Track5),
            crate::hal::ButtonId::Track(5) => Some(ButtonId::Track6),
            crate::hal::ButtonId::TrackSelect(0) => Some(ButtonId::TrackSelect1),
            crate::hal::ButtonId::TrackSelect(1) => Some(ButtonId::TrackSelect2),
            crate::hal::ButtonId::TrackSelect(2) => Some(ButtonId::TrackSelect3),
            crate::hal::ButtonId::TrackSelect(3) => Some(ButtonId::TrackSelect4),
            crate::hal::ButtonId::TrackSelect(4) => Some(ButtonId::TrackSelect5),
            crate::hal::ButtonId::TrackSelect(5) => Some(ButtonId::TrackSelect6),
            crate::hal::ButtonId::FX(0) => Some(ButtonId::FX1),
            crate::hal::ButtonId::FX(1) => Some(ButtonId::FX2),
            crate::hal::ButtonId::FX(2) => Some(ButtonId::FX3),
            crate::hal::ButtonId::FX(3) => Some(ButtonId::FX4),
            crate::hal::ButtonId::FX(4) => Some(ButtonId::FX5),
            crate::hal::ButtonId::Play => Some(ButtonId::Play),
            crate::hal::ButtonId::Stop => Some(ButtonId::Stop),
            crate::hal::ButtonId::Rec => Some(ButtonId::Rec),
            crate::hal::ButtonId::Menu => Some(ButtonId::Menu),
            crate::hal::ButtonId::PageLeft => Some(ButtonId::PageLeft),
            crate::hal::ButtonId::PageRight => Some(ButtonId::PageRight),
            crate::hal::ButtonId::Enter => Some(ButtonId::Enter),
            crate::hal::ButtonId::Exit => Some(ButtonId::Exit),
            crate::hal::ButtonId::TapTempo => Some(ButtonId::TapTempo),
            crate::hal::ButtonId::Memory => Some(ButtonId::Memory),
            crate::hal::ButtonId::UndoRedo => Some(ButtonId::UndoRedo),
            crate::hal::ButtonId::Edit => Some(ButtonId::Edit),
            _ => None,
        }
    }

    /// Process analog controls (should be called less frequently, e.g., every 10ms)
    pub fn update_analog_controls(&mut self, hal: &mut crate::hal::HardwareHal) -> Vec<ControlEvent, 16> {
        let mut events = Vec::new();

        // Read all analog controls
        let analog_controls = [
            (AnalogControlId::Knob(KnobId::Knob1), crate::hal::ControlId::Knob(0)),
            (AnalogControlId::Knob(KnobId::Knob2), crate::hal::ControlId::Knob(1)),
            (AnalogControlId::Knob(KnobId::Knob3), crate::hal::ControlId::Knob(2)),
            (AnalogControlId::Knob(KnobId::Knob4), crate::hal::ControlId::Knob(3)),
            (AnalogControlId::Knob(KnobId::OutputLevel), crate::hal::ControlId::OutputLevel),
            (AnalogControlId::Fader(FaderId::Track1Level), crate::hal::ControlId::TrackFader(0)),
            (AnalogControlId::Fader(FaderId::Track2Level), crate::hal::ControlId::TrackFader(1)),
            (AnalogControlId::Fader(FaderId::Track3Level), crate::hal::ControlId::TrackFader(2)),
            (AnalogControlId::Fader(FaderId::Track4Level), crate::hal::ControlId::TrackFader(3)),
            (AnalogControlId::Fader(FaderId::Track5Level), crate::hal::ControlId::TrackFader(4)),
            (AnalogControlId::Fader(FaderId::Track6Level), crate::hal::ControlId::TrackFader(5)),
            (AnalogControlId::Expression(ExpressionInput::CTL1_EXP1), crate::hal::ControlId::ExpressionPedal(0)),
            (AnalogControlId::Expression(ExpressionInput::CTL3_EXP2), crate::hal::ControlId::ExpressionPedal(1)),
        ];

        for (control_id, hal_control_id) in analog_controls.iter() {
            if let Ok(value) = hal.read_control(*hal_control_id) {
                if let Some(event) = self.control_interface.process_analog_control(*control_id, value) {
                    if events.push(event).is_err() {
                        break; // Event buffer full
                    }
                }
            }
        }

        events
    }

    /// Get control interface reference
    pub fn control_interface(&self) -> &HardwareControlInterface {
        &self.control_interface
    }

    /// Get mutable control interface reference
    pub fn control_interface_mut(&mut self) -> &mut HardwareControlInterface {
        &mut self.control_interface
    }
}

impl Default for HardwareControlInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ControlInterfaceHal {
    fn default() -> Self {
        Self::new()
    }
}