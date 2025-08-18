use anyhow::Result;
use esp_idf_svc::hal::{
    gpio::{Input, PinDriver, Pull},
    peripheral::Peripheral,
};
use log::*;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
    thread,
};

use crate::{
    display::DisplayManager,
    communication::CommunicationManager,
    menu::MenuAction,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonEvent {
    Press,
    Release,
    ShortPress,
    LongPress,
    DoublePress,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    MenuButton(ButtonEvent),
    PageLeft(ButtonEvent),
    PageRight(ButtonEvent),
    EnterButton(ButtonEvent),
    ExitButton(ButtonEvent),
    ValueKnobTurn(i32), // Direction: positive = clockwise, negative = counter-clockwise
    ValueKnobPress(ButtonEvent),
    EditButton(ButtonEvent),
}

pub struct InputManager {
    // Button pin drivers
    menu_button: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    page_left_button: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    page_right_button: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    enter_button: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    exit_button: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    edit_button: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    
    // Rotary encoder pins
    encoder_clk: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    encoder_dt: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    encoder_sw: Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
    
    // Button state tracking
    button_states: Arc<Mutex<ButtonStates>>,
    
    // Encoder state
    encoder_state: Arc<Mutex<EncoderState>>,
}

#[derive(Debug, Clone)]
struct ButtonStates {
    menu_pressed: bool,
    menu_press_time: Option<Instant>,
    page_left_pressed: bool,
    page_left_press_time: Option<Instant>,
    page_right_pressed: bool,
    page_right_press_time: Option<Instant>,
    enter_pressed: bool,
    enter_press_time: Option<Instant>,
    exit_pressed: bool,
    exit_press_time: Option<Instant>,
    edit_pressed: bool,
    edit_press_time: Option<Instant>,
    encoder_sw_pressed: bool,
    encoder_sw_press_time: Option<Instant>,
    last_double_press: Option<Instant>,
}

#[derive(Debug, Clone)]
struct EncoderState {
    last_clk: bool,
    last_dt: bool,
    position: i32,
}

impl ButtonStates {
    fn new() -> Self {
        Self {
            menu_pressed: false,
            menu_press_time: None,
            page_left_pressed: false,
            page_left_press_time: None,
            page_right_pressed: false,
            page_right_press_time: None,
            enter_pressed: false,
            enter_press_time: None,
            exit_pressed: false,
            exit_press_time: None,
            edit_pressed: false,
            edit_press_time: None,
            encoder_sw_pressed: false,
            encoder_sw_press_time: None,
            last_double_press: None,
        }
    }
}

impl EncoderState {
    fn new() -> Self {
        Self {
            last_clk: false,
            last_dt: false,
            position: 0,
        }
    }
}

impl InputManager {
    pub fn new(
        pins: esp_idf_svc::hal::gpio::Pins,
    ) -> Result<Self> {
        // Initialize button pins with pull-up resistors
        let menu_button = PinDriver::input(pins.gpio32.downgrade())?;
        let page_left_button = PinDriver::input(pins.gpio33.downgrade())?;
        let page_right_button = PinDriver::input(pins.gpio25.downgrade())?;
        let enter_button = PinDriver::input(pins.gpio26.downgrade())?;
        let exit_button = PinDriver::input(pins.gpio27.downgrade())?;
        let edit_button = PinDriver::input(pins.gpio14.downgrade())?;
        
        // Initialize rotary encoder pins
        let encoder_clk = PinDriver::input(pins.gpio12.downgrade())?;
        let encoder_dt = PinDriver::input(pins.gpio13.downgrade())?;
        let encoder_sw = PinDriver::input(pins.gpio15.downgrade())?;

        Ok(Self {
            menu_button: Arc::new(Mutex::new(menu_button)),
            page_left_button: Arc::new(Mutex::new(page_left_button)),
            page_right_button: Arc::new(Mutex::new(page_right_button)),
            enter_button: Arc::new(Mutex::new(enter_button)),
            exit_button: Arc::new(Mutex::new(exit_button)),
            edit_button: Arc::new(Mutex::new(edit_button)),
            encoder_clk: Arc::new(Mutex::new(encoder_clk)),
            encoder_dt: Arc::new(Mutex::new(encoder_dt)),
            encoder_sw: Arc::new(Mutex::new(encoder_sw)),
            button_states: Arc::new(Mutex::new(ButtonStates::new())),
            encoder_state: Arc::new(Mutex::new(EncoderState::new())),
        })
    }

    pub fn start_input_polling(
        &self,
        display_manager: Arc<DisplayManager>,
        comm_manager: Arc<CommunicationManager>,
    ) {
        let menu_button = self.menu_button.clone();
        let page_left_button = self.page_left_button.clone();
        let page_right_button = self.page_right_button.clone();
        let enter_button = self.enter_button.clone();
        let exit_button = self.exit_button.clone();
        let edit_button = self.edit_button.clone();
        let encoder_clk = self.encoder_clk.clone();
        let encoder_dt = self.encoder_dt.clone();
        let encoder_sw = self.encoder_sw.clone();
        let button_states = self.button_states.clone();
        let encoder_state = self.encoder_state.clone();

        thread::spawn(move || {
            info!("Starting input polling thread");
            
            loop {
                let now = Instant::now();
                
                // Poll buttons and generate events
                let events = Self::poll_inputs(
                    &menu_button,
                    &page_left_button,
                    &page_right_button,
                    &enter_button,
                    &exit_button,
                    &edit_button,
                    &encoder_clk,
                    &encoder_dt,
                    &encoder_sw,
                    &button_states,
                    &encoder_state,
                    now,
                );
                
                // Process events
                for event in events {
                    if let Err(e) = Self::handle_input_event(
                        &event,
                        &display_manager,
                        &comm_manager,
                    ) {
                        error!("Error handling input event {:?}: {:?}", event, e);
                    }
                }
                
                // Poll at 100Hz for responsive input
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    fn poll_inputs(
        menu_button: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        page_left_button: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        page_right_button: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        enter_button: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        exit_button: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        edit_button: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        encoder_clk: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        encoder_dt: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        encoder_sw: &Arc<Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::AnyInputPin, Input>>>,
        button_states: &Arc<Mutex<ButtonStates>>,
        encoder_state: &Arc<Mutex<EncoderState>>,
        now: Instant,
    ) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Read current pin states (assuming active low with pull-up)
        let menu_current = !menu_button.lock().unwrap().is_high();
        let page_left_current = !page_left_button.lock().unwrap().is_high();
        let page_right_current = !page_right_button.lock().unwrap().is_high();
        let enter_current = !enter_button.lock().unwrap().is_high();
        let exit_current = !exit_button.lock().unwrap().is_high();
        let edit_current = !edit_button.lock().unwrap().is_high();
        let encoder_sw_current = !encoder_sw.lock().unwrap().is_high();
        
        // Read encoder pins
        let clk_current = encoder_clk.lock().unwrap().is_high();
        let dt_current = encoder_dt.lock().unwrap().is_high();
        
        let mut states = button_states.lock().unwrap();
        
        // Process button events with debouncing and gesture detection
        Self::process_button_event(
            &mut events,
            &mut states.menu_pressed,
            &mut states.menu_press_time,
            &mut states.last_double_press,
            menu_current,
            now,
            |event| InputEvent::MenuButton(event),
        );
        
        Self::process_button_event(
            &mut events,
            &mut states.page_left_pressed,
            &mut states.page_left_press_time,
            &mut states.last_double_press,
            page_left_current,
            now,
            |event| InputEvent::PageLeft(event),
        );
        
        Self::process_button_event(
            &mut events,
            &mut states.page_right_pressed,
            &mut states.page_right_press_time,
            &mut states.last_double_press,
            page_right_current,
            now,
            |event| InputEvent::PageRight(event),
        );
        
        Self::process_button_event(
            &mut events,
            &mut states.enter_pressed,
            &mut states.enter_press_time,
            &mut states.last_double_press,
            enter_current,
            now,
            |event| InputEvent::EnterButton(event),
        );
        
        Self::process_button_event(
            &mut events,
            &mut states.exit_pressed,
            &mut states.exit_press_time,
            &mut states.last_double_press,
            exit_current,
            now,
            |event| InputEvent::ExitButton(event),
        );
        
        Self::process_button_event(
            &mut events,
            &mut states.edit_pressed,
            &mut states.edit_press_time,
            &mut states.last_double_press,
            edit_current,
            now,
            |event| InputEvent::EditButton(event),
        );
        
        Self::process_button_event(
            &mut events,
            &mut states.encoder_sw_pressed,
            &mut states.encoder_sw_press_time,
            &mut states.last_double_press,
            encoder_sw_current,
            now,
            |event| InputEvent::ValueKnobPress(event),
        );
        
        // Process rotary encoder
        let mut encoder = encoder_state.lock().unwrap();
        if clk_current != encoder.last_clk {
            if !clk_current { // Falling edge of CLK
                if dt_current != clk_current {
                    // Clockwise rotation
                    encoder.position += 1;
                    events.push(InputEvent::ValueKnobTurn(1));
                } else {
                    // Counter-clockwise rotation
                    encoder.position -= 1;
                    events.push(InputEvent::ValueKnobTurn(-1));
                }
            }
        }
        encoder.last_clk = clk_current;
        encoder.last_dt = dt_current;
        
        events
    }

    fn process_button_event<F>(
        events: &mut Vec<InputEvent>,
        pressed_state: &mut bool,
        press_time: &mut Option<Instant>,
        last_double_press: &mut Option<Instant>,
        current_state: bool,
        now: Instant,
        event_constructor: F,
    ) where
        F: Fn(ButtonEvent) -> InputEvent,
    {
        const DEBOUNCE_TIME: Duration = Duration::from_millis(20);
        const LONG_PRESS_TIME: Duration = Duration::from_millis(500);
        const DOUBLE_PRESS_TIME: Duration = Duration::from_millis(300);
        
        if current_state && !*pressed_state {
            // Button press detected
            *pressed_state = true;
            *press_time = Some(now);
            events.push(event_constructor(ButtonEvent::Press));
        } else if !current_state && *pressed_state {
            // Button release detected
            if let Some(press_start) = *press_time {
                let press_duration = now.duration_since(press_start);
                
                if press_duration >= DEBOUNCE_TIME {
                    *pressed_state = false;
                    *press_time = None;
                    events.push(event_constructor(ButtonEvent::Release));
                    
                    // Determine press type
                    if press_duration >= LONG_PRESS_TIME {
                        events.push(event_constructor(ButtonEvent::LongPress));
                    } else {
                        // Check for double press
                        let is_double_press = if let Some(last_press) = *last_double_press {
                            now.duration_since(last_press) <= DOUBLE_PRESS_TIME
                        } else {
                            false
                        };
                        
                        if is_double_press {
                            events.push(event_constructor(ButtonEvent::DoublePress));
                            *last_double_press = None; // Reset to prevent triple press
                        } else {
                            events.push(event_constructor(ButtonEvent::ShortPress));
                            *last_double_press = Some(now);
                        }
                    }
                }
            }
        }
    }

    fn handle_input_event(
        event: &InputEvent,
        display_manager: &Arc<DisplayManager>,
        comm_manager: &Arc<CommunicationManager>,
    ) -> Result<()> {
        match event {
            InputEvent::MenuButton(ButtonEvent::ShortPress) => {
                display_manager.handle_menu_button()?;
            }
            InputEvent::MenuButton(ButtonEvent::LongPress) => {
                // Long press MENU goes to top level
                display_manager.handle_menu_button()?;
            }
            InputEvent::PageLeft(ButtonEvent::ShortPress) => {
                let action = display_manager.handle_page_left()?;
                Self::execute_menu_action(&action, display_manager, comm_manager)?;
            }
            InputEvent::PageRight(ButtonEvent::ShortPress) => {
                let action = display_manager.handle_page_right()?;
                Self::execute_menu_action(&action, display_manager, comm_manager)?;
            }
            InputEvent::EnterButton(ButtonEvent::ShortPress) => {
                let action = display_manager.handle_enter_button()?;
                Self::execute_menu_action(&action, display_manager, comm_manager)?;
            }
            InputEvent::ExitButton(ButtonEvent::ShortPress) => {
                display_manager.handle_exit_button()?;
            }
            InputEvent::EditButton(ButtonEvent::ShortPress) => {
                let action = display_manager.handle_edit_button()?;
                Self::execute_menu_action(&action, display_manager, comm_manager)?;
            }
            InputEvent::EditButton(ButtonEvent::LongPress) => {
                // Long press EDIT for direct parameter override mode
                display_manager.enter_direct_override_mode()?;
            }
            InputEvent::ValueKnobTurn(direction) => {
                let action = display_manager.handle_value_knob_turn(*direction)?;
                Self::execute_menu_action(&action, display_manager, comm_manager)?;
            }
            InputEvent::ValueKnobPress(ButtonEvent::ShortPress) => {
                let action = display_manager.handle_value_knob_press()?;
                Self::execute_menu_action(&action, display_manager, comm_manager)?;
            }
            _ => {
                // Other button events (press, release, double press, long press) can be handled as needed
            }
        }
        
        Ok(())
    }

    fn execute_menu_action(
        action: &MenuAction,
        display_manager: &Arc<DisplayManager>,
        comm_manager: &Arc<CommunicationManager>,
    ) -> Result<()> {
        match action {
            MenuAction::Navigate(menu_type) => {
                display_manager.navigate_to_menu(menu_type.clone())?;
            }
            MenuAction::ExecuteFunction(function) => {
                info!("Executing function: {}", function);
                Self::execute_system_function(function, comm_manager)?;
            }
            MenuAction::EditParameter(param_id, value) => {
                info!("Parameter {} changed to {}", param_id, value);
                Self::send_parameter_change(param_id, *value, comm_manager)?;
                
                // Update display with real-time feedback
                display_manager.update_edit_display(param_id, *value)?;
            }
            MenuAction::Back => {
                display_manager.handle_exit_button()?;
            }
            MenuAction::Exit => {
                display_manager.handle_exit_button()?;
            }
            MenuAction::SelectItem(_) => {
                // Selection changes are handled internally by the menu system
            }
        }
        
        Ok(())
    }

    fn execute_system_function(function: &str, comm_manager: &Arc<CommunicationManager>) -> Result<()> {
        match function {
            "memory_load" => {
                // Memory load will be handled by menu system with current slot
                info!("Memory load function executed");
            }
            "memory_save" => {
                // Memory save will be handled by menu system with current slot
                info!("Memory save function executed");
            }
            "rhythm_start_stop" => {
                // Toggle rhythm pattern
                comm_manager.send_fx_command(0, "toggle")?;
            }
            _ => {
                info!("Unknown function: {}", function);
            }
        }
        Ok(())
    }

    fn send_parameter_change(param_id: &str, value: f32, comm_manager: &Arc<CommunicationManager>) -> Result<()> {
        match param_id {
            "track_volume" => {
                // Send track volume change - would need track ID context
                comm_manager.send_track_volume(1, value / 100.0)?; // Assuming track 1 for now
            }
            "track_pan" => {
                // Send track pan change - implementation depends on STM32 protocol
                info!("Track pan changed to {}", value);
            }
            "fx_depth" | "fx_rate" | "fx_feedback" | "fx_wet_dry" => {
                // Send FX parameter changes
                info!("FX parameter {} changed to {}", param_id, value);
            }
            "memory_slot" => {
                // Memory slot selection
                info!("Memory slot changed to {}", value);
            }
            _ => {
                info!("Parameter change: {} = {}", param_id, value);
            }
        }
        Ok(())
    }
}