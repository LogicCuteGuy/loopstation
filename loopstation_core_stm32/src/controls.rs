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
    
    // Rhythm operations
    RhythmStart,
    RhythmStop,
    RhythmToggle,
    RhythmPatternNext,
    RhythmPatternPrev,
    RhythmPatternSelect(u8), // Pattern number 0-15
    
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
    /// MIDI CC input
    MidiCC {
        channel: u8,
        cc_number: u8,
        value: u8,   // 0-127
    },
    /// Footswitch input
    FootswitchPress {
        footswitch_index: usize,
        pressed: bool,
    },
}

/// MIDI control action result from CC processing
#[derive(Debug, Clone, PartialEq)]
pub struct MidiControlAction {
    /// Target parameter to control
    pub target: MidiTarget,
    /// Normalized value (0.0-1.0)
    pub value: f32,
}

/// Control processing result
#[derive(Debug, Clone, PartialEq)]
pub enum ControlResult {
    /// Button function to execute
    ButtonFunction(ButtonFunction),
    /// MIDI control action to apply
    MidiAction(MidiControlAction),
}

/// Control context modes for different operational states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlContext {
    /// Performance mode - buttons control tracks and effects
    Performance,
    /// Menu mode - buttons navigate menus
    Menu,
    /// Edit mode - buttons and knobs edit parameters
    Edit,
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
    /// Current control context mode
    pub context: ControlContext,
    /// MIDI channel for CC processing (1-16, 0 = OMNI)
    pub midi_channel: u8,
    /// MIDI CC processing enabled
    pub midi_cc_enabled: bool,
}

impl ControlSystem {
    /// Create a new control system with default settings
    pub fn new() -> Self {
        Self {
            assignments: ControlAssignments::new(),
            debounce_time_ms: 10,      // 10ms debounce for buttons
            long_press_threshold_ms: 500,  // 500ms for long press
            double_press_window_ms: 300,   // 300ms window for double press
            context: ControlContext::Performance,
            midi_channel: 0,           // OMNI mode
            midi_cc_enabled: true,
        }
    }

    /// Set control context mode
    pub fn set_context(&mut self, context: ControlContext) {
        self.context = context;
    }

    /// Get current control context
    pub fn get_context(&self) -> ControlContext {
        self.context
    }

    /// Set MIDI channel (1-16, 0 = OMNI)
    pub fn set_midi_channel(&mut self, channel: u8) {
        self.midi_channel = channel.min(16);
    }

    /// Enable/disable MIDI CC processing
    pub fn set_midi_cc_enabled(&mut self, enabled: bool) {
        self.midi_cc_enabled = enabled;
    }

    /// Process a control event and return the corresponding function
    /// Context-aware processing based on current control mode
    pub fn process_control_event(&self, event: ControlEvent) -> Option<ControlResult> {
        match event {
            ControlEvent::ButtonPress { button, press_type } => {
                // Context-aware button processing
                match self.context {
                    ControlContext::Performance => {
                        self.get_button_function(button, press_type).map(ControlResult::ButtonFunction)
                    },
                    ControlContext::Menu => {
                        self.get_menu_button_function(button, press_type).map(ControlResult::ButtonFunction)
                    },
                    ControlContext::Edit => {
                        self.get_edit_button_function(button, press_type).map(ControlResult::ButtonFunction)
                    },
                }
            },
            ControlEvent::KnobTurn { knob: _, value: _ } => {
                // Knob events are context-dependent and handled elsewhere
                None
            },
            ControlEvent::FaderMove { fader: _, value: _ } => {
                // Fader events are handled directly by the loopstation core
                None
            },
            ControlEvent::ExpressionInput { input: _, value: _ } => {
                // Expression inputs are handled via assignments
                None
            },
            ControlEvent::MidiCC { channel, cc_number, value } => {
                self.process_midi_cc(channel, cc_number, value).map(ControlResult::MidiAction)
            },
            ControlEvent::FootswitchPress { footswitch_index, pressed } => {
                self.process_footswitch(footswitch_index, pressed).map(ControlResult::ButtonFunction)
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

    /// Get button function for menu context
    fn get_menu_button_function(&self, button: ButtonId, press_type: ButtonPress) -> Option<ButtonFunction> {
        match (button, press_type) {
            // Menu navigation buttons work the same in menu context
            (ButtonId::Menu, ButtonPress::Short) => Some(ButtonFunction::MenuOpen),
            (ButtonId::Menu, ButtonPress::Long) => Some(ButtonFunction::MenuExit),
            (ButtonId::PageLeft, _) => Some(ButtonFunction::PageLeft),
            (ButtonId::PageRight, _) => Some(ButtonFunction::PageRight),
            (ButtonId::Enter, _) => Some(ButtonFunction::Enter),
            (ButtonId::Exit, _) => Some(ButtonFunction::MenuExit),
            
            // Other buttons may have different meanings in menu context
            // For now, disable track and FX buttons in menu mode
            _ => None,
        }
    }

    /// Get button function for edit context
    fn get_edit_button_function(&self, button: ButtonId, press_type: ButtonPress) -> Option<ButtonFunction> {
        match (button, press_type) {
            // Edit mode allows direct parameter access
            (ButtonId::Edit, _) => Some(ButtonFunction::MenuExit), // Exit edit mode
            (ButtonId::Enter, _) => Some(ButtonFunction::Enter),
            (ButtonId::Exit, _) => Some(ButtonFunction::MenuExit),
            (ButtonId::PageLeft, _) => Some(ButtonFunction::PageLeft),
            (ButtonId::PageRight, _) => Some(ButtonFunction::PageRight),
            
            // Track select buttons work for track selection in edit mode
            (ButtonId::TrackSelect1, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(1)),
            (ButtonId::TrackSelect2, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(2)),
            (ButtonId::TrackSelect3, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(3)),
            (ButtonId::TrackSelect4, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(4)),
            (ButtonId::TrackSelect5, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(5)),
            (ButtonId::TrackSelect6, ButtonPress::Short) => Some(ButtonFunction::TrackSelect(6)),
            
            // FX buttons can be used for direct effect selection in edit mode
            (ButtonId::FX1, press_type) => self.get_fx_button_function(0, press_type),
            (ButtonId::FX2, press_type) => self.get_fx_button_function(1, press_type),
            (ButtonId::FX3, press_type) => self.get_fx_button_function(2, press_type),
            (ButtonId::FX4, press_type) => self.get_fx_button_function(3, press_type),
            (ButtonId::FX5, press_type) => self.get_fx_button_function(4, press_type),
            
            _ => None,
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

    /// Process MIDI CC message and return corresponding control action
    pub fn process_midi_cc(&self, channel: u8, cc_number: u8, value: u8) -> Option<MidiControlAction> {
        // Check if MIDI CC processing is enabled
        if !self.midi_cc_enabled {
            return None;
        }

        // Check MIDI channel (0 = OMNI mode accepts all channels)
        if self.midi_channel != 0 && self.midi_channel != channel {
            return None;
        }

        // Find matching MIDI assignment
        for assignment in &self.assignments.midi_assignments {
            if assignment.cc_number == cc_number {
                let normalized_value = value as f32 / 127.0; // Convert to 0.0-1.0 range
                return Some(MidiControlAction {
                    target: assignment.target.clone(),
                    value: normalized_value,
                });
            }
        }

        None
    }

    /// Add MIDI CC assignment
    pub fn add_midi_assignment(&mut self, cc_number: u8, target: MidiTarget) -> Result<(), &'static str> {
        // Check if CC number is already assigned
        if self.assignments.midi_assignments.iter().any(|a| a.cc_number == cc_number) {
            return Err("CC number already assigned");
        }

        let assignment = MidiAssignment { cc_number, target };
        self.assignments.midi_assignments.push(assignment)
            .map_err(|_| "MIDI assignment buffer full")?;

        Ok(())
    }

    /// Remove MIDI CC assignment
    pub fn remove_midi_assignment(&mut self, cc_number: u8) -> bool {
        if let Some(pos) = self.assignments.midi_assignments.iter().position(|a| a.cc_number == cc_number) {
            self.assignments.midi_assignments.remove(pos);
            true
        } else {
            false
        }
    }

    /// Set expression pedal assignment
    pub fn set_expression_assignment(&mut self, input_index: usize, assignment: Option<ExpressionAssignment>) {
        if input_index < self.assignments.expression_assignments.len() {
            self.assignments.expression_assignments[input_index] = assignment;
        }
    }

    /// Get expression pedal assignment
    pub fn get_expression_assignment(&self, input_index: usize) -> Option<&ExpressionAssignment> {
        self.assignments.expression_assignments.get(input_index)?.as_ref()
    }

    /// Set footswitch assignment
    pub fn set_footswitch_assignment(&mut self, footswitch_index: usize, assignment: Option<FootswitchAssignment>) {
        if footswitch_index < self.assignments.footswitch_assignments.len() {
            self.assignments.footswitch_assignments[footswitch_index] = assignment;
        }
    }

    /// Get footswitch assignment
    pub fn get_footswitch_assignment(&self, footswitch_index: usize) -> Option<&FootswitchAssignment> {
        self.assignments.footswitch_assignments.get(footswitch_index)?.as_ref()
    }

    /// Process footswitch input and return corresponding function
    pub fn process_footswitch(&self, footswitch_index: usize, pressed: bool) -> Option<ButtonFunction> {
        if !pressed {
            return None; // Only process on press
        }

        if let Some(assignment) = self.get_footswitch_assignment(footswitch_index) {
            match assignment {
                FootswitchAssignment::RecPlay(track_id) => Some(ButtonFunction::TrackRecord(*track_id)),
                FootswitchAssignment::MemoryInc => Some(ButtonFunction::MemoryInc),
                FootswitchAssignment::MemoryDec => Some(ButtonFunction::MemoryDec),
                FootswitchAssignment::UndoRedo => Some(ButtonFunction::Undo),
                FootswitchAssignment::TapTempo => Some(ButtonFunction::TapTempo),
                FootswitchAssignment::AllStart => Some(ButtonFunction::AllStart),
                FootswitchAssignment::AllStop => Some(ButtonFunction::AllStop),
            }
        } else {
            None
        }
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

    /// Process a control event and return the corresponding result
    pub fn process_control_event(&self, event: ControlEvent) -> Option<ControlResult> {
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
    pub fn update_leds(&mut self, _hal: &mut crate::HardwareHal) -> Result<(), crate::HalError> {
        // Update LEDs based on current control assignments and system state
        
        // LED update functionality - placeholder implementation
        // In a full implementation, this would update LEDs based on control assignments
        // and system state using the HAL LED interface

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
    pub fn update(&mut self, hal: &mut crate::HardwareHal, time_ms: u32) -> Vec<ControlEvent, 16> {
        self.control_interface.update_time(time_ms);
        let mut events = Vec::new();

        // Get button events from PCF8575 controllers with debouncing and gesture recognition
        let button_events = hal.update_buttons(); // Simplified for compilation
        
        // Convert HAL button events to control events
        for (hal_button_id, _pressed) in button_events {
            // Map HAL ButtonId to controls ButtonId
            if let Some(control_button_id) = self.map_hal_button_to_control_button(hal_button_id) {
                // Convert to ButtonPress - using placeholder for now
                let button_press = ButtonPress::Short; // Simplified for compilation

                let control_event = ControlEvent::ButtonPress {
                    button: control_button_id,
                    press_type: button_press,
                };

                if events.push(control_event).is_err() {
                    break; // Event buffer full
                }
            }
        }

        // Get MIDI events from HAL
        let midi_events = hal.get_midi_events();
        for midi_event in midi_events {
            if let Some(control_event) = self.convert_midi_to_control_event(midi_event) {
                if events.push(control_event).is_err() {
                    break; // Event buffer full
                }
            }
        }

        // Get footswitch events from HAL
        let footswitch_events = hal.get_footswitch_events();
        for (footswitch_index, pressed) in footswitch_events {
            let control_event = ControlEvent::FootswitchPress {
                footswitch_index,
                pressed,
            };

            if events.push(control_event).is_err() {
                break; // Event buffer full
            }
        }

        // Get rotary encoder events and convert to control events - placeholder for now
        // let rotary_events = hal.update_rotary_encoder(time_ms);
        // Simplified for compilation

        events
    }

    /// Convert MIDI event to control event
    fn convert_midi_to_control_event(&self, midi_event: crate::MidiEvent) -> Option<ControlEvent> {
        match midi_event {
            crate::MidiEvent::ControlChange { channel, controller, value } => {
                Some(ControlEvent::MidiCC {
                    channel,
                    cc_number: controller,
                    value,
                })
            },
            _ => None, // Only handle CC messages for now
        }
    }

    /// Map HAL ButtonId to controls ButtonId
    fn map_hal_button_to_control_button(&self, hal_button_id: crate::ButtonId) -> Option<ButtonId> {
        match hal_button_id {
            crate::ButtonId::Track(0) => Some(ButtonId::Track1),
            crate::ButtonId::Track(1) => Some(ButtonId::Track2),
            crate::ButtonId::Track(2) => Some(ButtonId::Track3),
            crate::ButtonId::Track(3) => Some(ButtonId::Track4),
            crate::ButtonId::Track(4) => Some(ButtonId::Track5),
            crate::ButtonId::Track(5) => Some(ButtonId::Track6),
            crate::ButtonId::TrackSelect(0) => Some(ButtonId::TrackSelect1),
            crate::ButtonId::TrackSelect(1) => Some(ButtonId::TrackSelect2),
            crate::ButtonId::TrackSelect(2) => Some(ButtonId::TrackSelect3),
            crate::ButtonId::TrackSelect(3) => Some(ButtonId::TrackSelect4),
            crate::ButtonId::TrackSelect(4) => Some(ButtonId::TrackSelect5),
            crate::ButtonId::TrackSelect(5) => Some(ButtonId::TrackSelect6),
            crate::ButtonId::FX(0) => Some(ButtonId::FX1),
            crate::ButtonId::FX(1) => Some(ButtonId::FX2),
            crate::ButtonId::FX(2) => Some(ButtonId::FX3),
            crate::ButtonId::FX(3) => Some(ButtonId::FX4),
            crate::ButtonId::FX(4) => Some(ButtonId::FX5),
            crate::ButtonId::Play => Some(ButtonId::Play),
            crate::ButtonId::Stop => Some(ButtonId::Stop),
            crate::ButtonId::Rec => Some(ButtonId::Rec),
            crate::ButtonId::Menu => Some(ButtonId::Menu),
            crate::ButtonId::PageLeft => Some(ButtonId::PageLeft),
            crate::ButtonId::PageRight => Some(ButtonId::PageRight),
            crate::ButtonId::Enter => Some(ButtonId::Enter),
            crate::ButtonId::Exit => Some(ButtonId::Exit),
            crate::ButtonId::TapTempo => Some(ButtonId::TapTempo),
            crate::ButtonId::Memory => Some(ButtonId::Memory),
            crate::ButtonId::UndoRedo => Some(ButtonId::UndoRedo),
            crate::ButtonId::Edit => Some(ButtonId::Edit),
            _ => None,
        }
    }

    /// Process analog controls (should be called less frequently, e.g., every 10ms)
    pub fn update_analog_controls(&mut self, hal: &mut crate::HardwareHal) -> Vec<ControlEvent, 16> {
        let mut events = Vec::new();

        // Read all analog controls
        let analog_controls = [
            (AnalogControlId::Knob(KnobId::Knob1), crate::ControlId::Knob(0)),
            (AnalogControlId::Knob(KnobId::Knob2), crate::ControlId::Knob(1)),
            (AnalogControlId::Knob(KnobId::Knob3), crate::ControlId::Knob(2)),
            (AnalogControlId::Knob(KnobId::Knob4), crate::ControlId::Knob(3)),
            (AnalogControlId::Knob(KnobId::OutputLevel), crate::ControlId::OutputLevel),
            (AnalogControlId::Fader(FaderId::Track1Level), crate::ControlId::TrackFader(0)),
            (AnalogControlId::Fader(FaderId::Track2Level), crate::ControlId::TrackFader(1)),
            (AnalogControlId::Fader(FaderId::Track3Level), crate::ControlId::TrackFader(2)),
            (AnalogControlId::Fader(FaderId::Track4Level), crate::ControlId::TrackFader(3)),
            (AnalogControlId::Fader(FaderId::Track5Level), crate::ControlId::TrackFader(4)),
            (AnalogControlId::Fader(FaderId::Track6Level), crate::ControlId::TrackFader(5)),
            (AnalogControlId::Expression(ExpressionInput::CTL1_EXP1), crate::ControlId::ExpressionPedal(0)),
            (AnalogControlId::Expression(ExpressionInput::CTL3_EXP2), crate::ControlId::ExpressionPedal(1)),
        ];

        for (control_id, _hal_control_id) in analog_controls.iter() {
            // Placeholder - would read actual control values from HAL
            let value = 0.5f32; // Default middle position
            if let Some(event) = self.control_interface.process_analog_control(*control_id, value) {
                if events.push(event).is_err() {
                    break; // Event buffer full
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

    /// Update LED states based on control system state
    pub fn update_leds(&mut self, _hal: &mut crate::HardwareHal) -> Result<(), crate::HalError> {
        // Update LEDs based on current control assignments and system state
        
        // LED update functionality - placeholder implementation
        // In a full implementation, this would update LEDs based on control assignments
        // and system state using the HAL LED interface

        Ok(())
    }

    /// Process a control event and return the corresponding result
    pub fn process_control_event(&self, event: ControlEvent) -> Option<ControlResult> {
        self.control_interface.process_control_event(event)
    }

    /// Get control system reference
    pub fn control_system(&self) -> &ControlSystem {
        self.control_interface.control_system()
    }

    /// Get mutable control system reference
    pub fn control_system_mut(&mut self) -> &mut ControlSystem {
        self.control_interface.control_system_mut()
    }

    /// Set footswitch assignment for a specific footswitch
    pub fn set_footswitch_assignment(&mut self, footswitch_index: usize, assignment: crate::settings::FootSwitchFunction) {
        if footswitch_index < 2 {
            // Convert settings FootSwitchFunction to controls FootswitchAssignment
            let footswitch_assignment = match assignment {
                crate::settings::FootSwitchFunction::RecPlay => Some(crate::storage::FootswitchAssignment::RecPlay(1)), // Default to track 1
                crate::settings::FootSwitchFunction::MemoryInc => Some(crate::storage::FootswitchAssignment::MemoryInc),
                crate::settings::FootSwitchFunction::MemoryDec => Some(crate::storage::FootswitchAssignment::MemoryDec),
                crate::settings::FootSwitchFunction::UndoRedo => Some(crate::storage::FootswitchAssignment::UndoRedo),
                crate::settings::FootSwitchFunction::TapTempo => Some(crate::storage::FootswitchAssignment::TapTempo),
                crate::settings::FootSwitchFunction::AllStart => Some(crate::storage::FootswitchAssignment::AllStart),
                crate::settings::FootSwitchFunction::AllStop => Some(crate::storage::FootswitchAssignment::AllStop),
                crate::settings::FootSwitchFunction::Track1 => Some(crate::storage::FootswitchAssignment::RecPlay(1)),
                crate::settings::FootSwitchFunction::Track2 => Some(crate::storage::FootswitchAssignment::RecPlay(2)),
                crate::settings::FootSwitchFunction::Track3 => Some(crate::storage::FootswitchAssignment::RecPlay(3)),
                crate::settings::FootSwitchFunction::Track4 => Some(crate::storage::FootswitchAssignment::RecPlay(4)),
                crate::settings::FootSwitchFunction::Track5 => Some(crate::storage::FootswitchAssignment::RecPlay(5)),
                crate::settings::FootSwitchFunction::Track6 => Some(crate::storage::FootswitchAssignment::RecPlay(6)),
            };
            
            self.control_interface.control_system_mut().assignments.footswitch_assignments[footswitch_index] = footswitch_assignment;
        }
    }

    /// Set CTL function assignment mode
    pub fn set_ctl_func_assign_mode(&mut self, _mode: crate::settings::CtlFuncAssign) {
        // This would configure how CTL functions are assigned
        // Implementation depends on the specific control system design
    }

    /// Set expression pedal mode
    pub fn set_exp_pedal_mode(&mut self, _mode: crate::settings::ExpPedalMode) {
        // This would configure expression pedal behavior (continuous vs toggle)
        // Implementation depends on the specific control system design
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