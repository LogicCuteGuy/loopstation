use anyhow::Result;
use esp_idf_svc::hal::{
    gpio::{Gpio21, Gpio22, Output, PinDriver},
    i2c::{I2cConfig, I2cDriver},
    peripheral::Peripheral,
    prelude::*,
};
use embedded_graphics::{
    mono_font::{ascii::{FONT_6X10, FONT_5X8, FONT_4X6}, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Text, Alignment, Baseline},
    primitives::{Line, Rectangle, Circle, PrimitiveStyle, PrimitiveStyleBuilder},
    geometry::{Point, Size},
    draw_target::DrawTarget,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use std::sync::Mutex;

use crate::communication::{SystemState, TrackStateEnum};
use crate::menu::{MenuSystem, MenuType, MenuPage, MenuItem, MenuItemType, MenuValue, MenuAction};

pub struct DisplayManager {
    display: Mutex<Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x64, BinaryColorMode>>,
    current_screen: Mutex<DisplayScreen>,
    brightness: Mutex<u8>,
    menu_system: Mutex<MenuSystem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayScreen {
    Main,
    TrackDetail(u8),
    Menu(crate::menu::MenuType),
    Settings,
    NetworkStatus,
}

impl DisplayManager {
    pub fn new(
        i2c: impl Peripheral<P = esp_idf_svc::hal::i2c::I2C0> + 'static,
        pins: esp_idf_svc::hal::gpio::Pins,
    ) -> Result<Self> {
        // Configure I2C for display (SDA: GPIO21, SCL: GPIO22)
        let sda = pins.gpio21;
        let scl = pins.gpio22;
        
        let config = I2cConfig::new().baudrate(400.kHz().into());
        let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

        // Initialize SSD1306 display
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        
        display.init().map_err(|e| anyhow::anyhow!("Display init failed: {:?}", e))?;
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;
        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;

        Ok(Self {
            display: Mutex::new(display),
            current_screen: Mutex::new(DisplayScreen::Main),
            brightness: Mutex::new(255),
            menu_system: Mutex::new(MenuSystem::new()),
        })
    }

    pub fn update_display(&self, state: &SystemState) -> Result<()> {
        let current_screen = self.current_screen.lock().unwrap().clone();
        
        match current_screen {
            DisplayScreen::Main => self.draw_main_screen(state),
            DisplayScreen::TrackDetail(track_id) => self.draw_track_detail_screen(state, track_id),
            DisplayScreen::Menu(menu_type) => self.draw_menu_screen(state, menu_type),
            DisplayScreen::Settings => self.draw_settings_screen(state),
            DisplayScreen::NetworkStatus => self.draw_network_status_screen(state),
        }
    }

    fn draw_main_screen(&self, state: &SystemState) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;

        // Header with title and tempo
        self.draw_header(&mut *display, "RC-505 MKII", Some(state.tempo))?;
        
        // Track status grid (2x3 layout)
        self.draw_track_grid(&mut *display, &state.tracks)?;
        
        // Bottom status bar
        self.draw_status_bar(&mut *display, state)?;

        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;
        Ok(())
    }

    fn draw_track_detail_screen(&self, state: &SystemState, track_id: u8) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;

        if let Some(track) = state.tracks.get(track_id as usize) {
            // Header
            let title = format!("TRACK {}", track_id + 1);
            self.draw_header(&mut *display, &title, None)?;

            // Track details
            let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
            let small_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);

            // State and volume
            let state_text = match track.state {
                TrackStateEnum::Recording => "RECORDING",
                TrackStateEnum::Playing => "PLAYING",
                TrackStateEnum::Overdubbing => "OVERDUBBING",
                TrackStateEnum::Stopped => "STOPPED",
                TrackStateEnum::Muted => "MUTED",
            };
            
            Text::new(state_text, Point::new(5, 25), text_style)
                .draw(&mut *display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

            // Volume bar
            self.draw_volume_bar(&mut *display, Point::new(5, 35), track.volume)?;
            
            // Pan indicator
            self.draw_pan_indicator(&mut *display, Point::new(5, 45), track.pan)?;

            // Track controls info
            Text::new("REC | PLAY | STOP", Point::new(5, 58), small_style)
                .draw(&mut *display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
        }

        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;
        Ok(())
    }

    fn draw_menu_screen(&self, state: &SystemState, menu_type: crate::menu::MenuType) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;

        let menu_system = self.menu_system.lock().unwrap();
        
        if let Some(page) = menu_system.get_current_page() {
            // Draw header with menu title
            self.draw_header(&mut *display, &page.title, None)?;
            
            if menu_system.is_edit_mode() {
                // In EDIT mode, show large parameter display
                self.draw_edit_parameter_display(&mut *display, &menu_system)?;
            } else {
                // In navigation mode, show menu items
                self.draw_menu_items(&mut *display, page, &menu_system)?;
            }
            
            // Draw navigation hints
            self.draw_menu_navigation_hints(&mut *display, &menu_system)?;
        }

        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;
        Ok(())
    }

    fn draw_settings_screen(&self, state: &SystemState) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;

        self.draw_header(&mut *display, "SETTINGS", None)?;

        let text_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        
        let settings = [
            format!("Memory: {}", state.current_memory),
            format!("Tempo: {:.1} BPM", state.tempo),
            format!("WiFi: {}", if state.network_connected { "ON" } else { "OFF" }),
            "MIDI: Channel 1",
            "Display: 100%",
        ];

        for (i, setting) in settings.iter().enumerate() {
            Text::new(setting, Point::new(5, 25 + (i as i32 * 9)), text_style)
                .draw(&mut *display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
        }

        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;
        Ok(())
    }

    fn draw_network_status_screen(&self, state: &SystemState) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;

        self.draw_header(&mut *display, "NETWORK", None)?;

        let text_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        
        let status_text = if state.network_connected {
            "Status: CONNECTED"
        } else {
            "Status: DISCONNECTED"
        };

        Text::new(status_text, Point::new(5, 25), text_style)
            .draw(&mut *display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        Text::new("OSC Port: 8000", Point::new(5, 35), text_style)
            .draw(&mut *display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        Text::new("Service: RC-505-Clone", Point::new(5, 45), text_style)
            .draw(&mut *display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;
        Ok(())
    }

    pub fn show_startup_screen(&self) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;
        
        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        
        Text::new("Loopstation ESP32", Point::new(10, 20), text_style)
            .draw(&mut *display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            
        Text::new("Initializing...", Point::new(20, 40), text_style)
            .draw(&mut *display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;

        Ok(())
    }
}
   
 // Graphics primitives and UI helper methods
    fn draw_header(&self, display: &mut impl DrawTarget<Color = BinaryColor>, title: &str, tempo: Option<f32>) -> Result<()> {
        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let line_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

        // Title
        Text::new(title, Point::new(2, 10), text_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        // Tempo on the right if provided
        if let Some(bpm) = tempo {
            let tempo_text = format!("{:.0}", bpm);
            Text::new(&tempo_text, Point::new(100, 10), text_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
        }

        // Header separator line
        Line::new(Point::new(0, 12), Point::new(127, 12))
            .into_styled(line_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Line draw failed: {:?}", e))?;

        Ok(())
    }

    fn draw_track_grid(&self, display: &mut impl DrawTarget<Color = BinaryColor>, tracks: &[crate::communication::TrackState]) -> Result<()> {
        let text_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        let small_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        
        // Draw 2x3 grid of tracks
        for (i, track) in tracks.iter().take(6).enumerate() {
            let col = i % 2;
            let row = i / 2;
            let x = 5 + (col * 60) as i32;
            let y = 20 + (row * 15) as i32;

            // Track number and state
            let track_text = format!("T{}", i + 1);
            Text::new(&track_text, Point::new(x, y), text_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

            // State indicator
            let state_char = match track.state {
                TrackStateEnum::Recording => "R",
                TrackStateEnum::Playing => "P",
                TrackStateEnum::Overdubbing => "O",
                TrackStateEnum::Stopped => "-",
                TrackStateEnum::Muted => "M",
            };
            
            Text::new(state_char, Point::new(x + 15, y), text_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

            // Volume level (simple bar)
            let vol_width = (track.volume * 20.0) as i32;
            if vol_width > 0 {
                Rectangle::new(Point::new(x + 25, y - 3), Size::new(vol_width as u32, 3))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Rectangle draw failed: {:?}", e))?;
            }

            // Mute indicator
            if track.muted {
                Text::new("MUTE", Point::new(x, y + 8), small_style)
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            }
        }

        Ok(())
    }

    fn draw_status_bar(&self, display: &mut impl DrawTarget<Color = BinaryColor>, state: &SystemState) -> Result<()> {
        let text_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        let line_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

        // Status bar separator line
        Line::new(Point::new(0, 55), Point::new(127, 55))
            .into_styled(line_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Line draw failed: {:?}", e))?;

        // Memory slot
        let memory_text = format!("MEM:{}", state.current_memory);
        Text::new(&memory_text, Point::new(2, 62), text_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        // Network status
        let network_text = if state.network_connected { "NET:ON" } else { "NET:OFF" };
        Text::new(network_text, Point::new(40, 62), text_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        // FX status (show active effects count)
        let fx_count = state.fx_states.iter().filter(|&&active| active).count();
        let fx_text = format!("FX:{}", fx_count);
        Text::new(&fx_text, Point::new(80, 62), text_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        Ok(())
    }

    fn draw_volume_bar(&self, display: &mut impl DrawTarget<Color = BinaryColor>, pos: Point, volume: f32) -> Result<()> {
        let text_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        let bar_width = 60;
        let bar_height = 6;

        // Volume label
        Text::new("VOL:", pos, text_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        // Volume bar background
        Rectangle::new(Point::new(pos.x + 25, pos.y - 3), Size::new(bar_width, bar_height))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Rectangle draw failed: {:?}", e))?;

        // Volume bar fill
        let fill_width = (volume * (bar_width - 2) as f32) as u32;
        if fill_width > 0 {
            Rectangle::new(Point::new(pos.x + 26, pos.y - 2), Size::new(fill_width, bar_height - 2))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Rectangle draw failed: {:?}", e))?;
        }

        // Volume percentage
        let vol_text = format!("{:.0}%", volume * 100.0);
        Text::new(&vol_text, Point::new(pos.x + 90, pos.y), text_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        Ok(())
    }

    fn draw_pan_indicator(&self, display: &mut impl DrawTarget<Color = BinaryColor>, pos: Point, pan: f32) -> Result<()> {
        let text_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        let pan_width = 60;

        // Pan label
        Text::new("PAN:", pos, text_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;

        // Pan line
        Line::new(Point::new(pos.x + 25, pos.y - 1), Point::new(pos.x + 25 + pan_width, pos.y - 1))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Line draw failed: {:?}", e))?;

        // Pan center mark
        let center_x = pos.x + 25 + (pan_width / 2);
        Line::new(Point::new(center_x, pos.y - 3), Point::new(center_x, pos.y + 1))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Line draw failed: {:?}", e))?;

        // Pan position indicator
        let pan_pos = center_x + (pan * (pan_width / 2) as f32) as i32;
        Circle::new(Point::new(pan_pos - 1, pos.y - 2), 3)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Circle draw failed: {:?}", e))?;

        Ok(())
    }

    // Screen navigation methods
    pub fn set_screen(&self, screen: DisplayScreen) {
        *self.current_screen.lock().unwrap() = screen;
    }

    pub fn get_current_screen(&self) -> DisplayScreen {
        self.current_screen.lock().unwrap().clone()
    }

    pub fn set_brightness(&self, brightness: u8) -> Result<()> {
        *self.brightness.lock().unwrap() = brightness;
        // Note: SSD1306 brightness control would be implemented here
        // This is a placeholder for the actual brightness control
        Ok(())
    }

    pub fn get_brightness(&self) -> u8 {
        *self.brightness.lock().unwrap()
    }

    // Display buffer management
    pub fn clear_display(&self) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;
        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;
        Ok(())
    }

    pub fn force_refresh(&self) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.flush().map_err(|e| anyhow::anyhow!("Display flush failed: {:?}", e))?;
        Ok(())
    }

    // Animation and transition effects
    pub fn fade_transition(&self, from_screen: DisplayScreen, to_screen: DisplayScreen) -> Result<()> {
        // Simple fade effect by clearing and redrawing
        // In a more advanced implementation, this could do actual fade animation
        self.clear_display()?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.set_screen(to_screen);
        Ok(())
    }

    // Menu system integration methods
    pub fn handle_value_knob_turn(&self, direction: i32) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.value_knob_turn(direction)
    }

    pub fn handle_value_knob_press(&self) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.value_knob_press()
    }

    pub fn handle_page_left(&self) -> Result<MenuAction> {
        self.handle_page_advanced_edit(-1)
    }

    pub fn handle_page_right(&self) -> Result<MenuAction> {
        self.handle_page_advanced_edit(1)
    }

    pub fn handle_menu_button(&self) -> Result<()> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.navigate_to_main()?;
        self.set_screen(DisplayScreen::Menu(crate::menu::MenuType::Main));
        Ok(())
    }

    pub fn handle_exit_button(&self) -> Result<()> {
        let mut menu_system = self.menu_system.lock().unwrap();
        if menu_system.is_edit_mode() {
            menu_system.exit_edit_mode();
        } else {
            menu_system.navigate_back()?;
            if menu_system.get_menu_stack().is_empty() {
                self.set_screen(DisplayScreen::Main);
            }
        }
        Ok(())
    }

    pub fn handle_enter_button(&self) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.value_knob_press()
    }

    pub fn handle_edit_button(&self) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.handle_edit_button_press()
    }

    pub fn enter_direct_override_mode(&self) -> Result<()> {
        // Enter direct parameter override mode where knobs directly control parameters
        // This would be implemented based on the current screen context
        Ok(())
    }

    pub fn handle_knob_override(&self, knob_id: u8, value: f32) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.handle_direct_parameter_override(knob_id, value)
    }

    pub fn get_knob_led_feedback(&self) -> [u8; 4] {
        let menu_system = self.menu_system.lock().unwrap();
        menu_system.get_knob_led_values()
    }

    // Context-sensitive EDIT mode based on current screen
    pub fn handle_fx_edit_combination(&self, fx_button: u8) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        
        // Navigate to appropriate FX menu and enter edit mode
        match fx_button {
            1..=4 => {
                // FX1-4 buttons - enter FX edit mode for that slot
                menu_system.navigate_to(crate::menu::MenuType::TrackFx)?;
                menu_system.handle_context_edit("fx")
            }
            _ => Ok(MenuAction::SelectItem(0)),
        }
    }

    pub fn handle_track_edit_combination(&self, track_id: u8) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        
        // Navigate to track menu and enter edit mode for that track
        menu_system.navigate_to(crate::menu::MenuType::Track)?;
        menu_system.handle_context_edit("track")
    }

    // Real-time parameter editing with visual feedback
    pub fn update_edit_display(&self, parameter_name: &str, value: f32) -> Result<()> {
        // This would update the display to show real-time parameter changes
        // with value bars and parameter names during EDIT mode
        Ok(())
    }

    // Advanced parameter access in EDIT mode
    pub fn handle_page_advanced_edit(&self, direction: i32) -> Result<MenuAction> {
        let mut menu_system = self.menu_system.lock().unwrap();
        if menu_system.is_edit_mode() {
            menu_system.access_advanced_edit_parameters(direction)
        } else {
            // Normal page navigation
            if direction > 0 {
                menu_system.page_right()
            } else {
                menu_system.page_left()
            }
        }
    }

    pub fn navigate_to_menu(&self, menu_type: crate::menu::MenuType) -> Result<()> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.navigate_to(menu_type.clone())?;
        self.set_screen(DisplayScreen::Menu(menu_type));
        Ok(())
    }

    pub fn update_menu_from_system_state(&self, state: &SystemState) -> Result<()> {
        let mut menu_system = self.menu_system.lock().unwrap();
        menu_system.update_from_system_state(state)
    }

    pub fn is_in_menu(&self) -> bool {
        matches!(self.get_current_screen(), DisplayScreen::Menu(_))
    }

    pub fn is_in_edit_mode(&self) -> bool {
        let menu_system = self.menu_system.lock().unwrap();
        menu_system.is_edit_mode()
    }

    // Menu rendering helper methods
    fn draw_menu_items(&self, display: &mut impl DrawTarget<Color = BinaryColor>, page: &MenuPage, menu_system: &MenuSystem) -> Result<()> {
        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let small_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        let selected_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off); // Inverted for selection
        
        let start_y = 20;
        let line_height = 9;
        
        // Calculate visible items based on scroll offset
        let visible_items = page.items.iter()
            .skip(page.scroll_offset)
            .take(page.max_visible_items)
            .enumerate();
        
        for (display_index, (item_index, item)) in visible_items {
            let actual_index = page.scroll_offset + display_index;
            let y_pos = start_y + (display_index as i32 * line_height);
            let is_selected = actual_index == page.selected_index;
            
            // Draw selection background
            if is_selected {
                Rectangle::new(Point::new(0, y_pos - 7), Size::new(128, 8))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Rectangle draw failed: {:?}", e))?;
            }
            
            // Draw item name
            let style = if is_selected { selected_style } else { text_style };
            Text::new(&item.name, Point::new(2, y_pos), style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            
            // Draw item value/status
            self.draw_menu_item_value(display, item, Point::new(80, y_pos), is_selected, menu_system)?;
            
            // Draw edit indicator if in edit mode for this item
            if menu_system.is_edit_mode() {
                if let Some(edit_param) = menu_system.get_edit_parameter() {
                    if edit_param == &item.id {
                        Text::new("*", Point::new(120, y_pos), style)
                            .draw(display)
                            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                    }
                }
            }
        }
        
        // Draw scroll indicators
        if page.scroll_offset > 0 {
            Text::new("^", Point::new(120, 15), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
        }
        
        if page.scroll_offset + page.max_visible_items < page.items.len() {
            Text::new("v", Point::new(120, 55), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
        }
        
        Ok(())
    }

    fn draw_menu_item_value(&self, display: &mut impl DrawTarget<Color = BinaryColor>, item: &MenuItem, pos: Point, is_selected: bool, menu_system: &MenuSystem) -> Result<()> {
        let text_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        let selected_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::Off);
        let style = if is_selected { selected_style } else { text_style };
        
        match &item.value {
            Some(MenuValue::Float(value, _, _)) => {
                let value_text = format!("{:.1}", value);
                Text::new(&value_text, pos, style)
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            }
            Some(MenuValue::Int(value, _, _)) => {
                let value_text = format!("{}", value);
                Text::new(&value_text, pos, style)
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            }
            Some(MenuValue::Bool(value)) => {
                let value_text = if *value { "ON" } else { "OFF" };
                Text::new(value_text, pos, style)
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            }
            Some(MenuValue::String(value)) => {
                // Truncate long strings
                let display_text = if value.len() > 8 {
                    &value[..8]
                } else {
                    value
                };
                Text::new(display_text, pos, style)
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            }
            Some(MenuValue::Selection(options, selected)) => {
                if let Some(selected_option) = options.get(*selected) {
                    // Truncate long option names
                    let display_text = if selected_option.len() > 8 {
                        &selected_option[..8]
                    } else {
                        selected_option
                    };
                    Text::new(display_text, pos, style)
                        .draw(display)
                        .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                }
            }
            None => {
                // For functions and navigation items, show arrow or indicator
                match item.item_type {
                    MenuItemType::Navigation => {
                        Text::new(">", pos, style)
                            .draw(display)
                            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                    }
                    MenuItemType::Function => {
                        Text::new("EXEC", pos, style)
                            .draw(display)
                            .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }

    fn draw_menu_navigation_hints(&self, display: &mut impl DrawTarget<Color = BinaryColor>, menu_system: &MenuSystem) -> Result<()> {
        let small_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        let line_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
        
        // Bottom separator line
        Line::new(Point::new(0, 55), Point::new(127, 55))
            .into_styled(line_style)
            .draw(display)
            .map_err(|e| anyhow::anyhow!("Line draw failed: {:?}", e))?;
        
        if menu_system.is_edit_mode() {
            // Edit mode hints with knob assignments
            Text::new("EDIT MODE", Point::new(2, 62), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            
            // Show knob LED feedback indicators
            self.draw_knob_led_indicators(display, menu_system)?;
            
            Text::new("PAGE:ADV", Point::new(50, 62), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            
            Text::new("EXIT", Point::new(100, 62), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
        } else {
            // Navigation mode hints
            Text::new("VALUE:SEL", Point::new(2, 62), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            
            Text::new("PAGE:NAV", Point::new(50, 62), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
            
            Text::new("ENTER", Point::new(100, 62), small_style)
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
        }
        
        Ok(())
    }

    fn draw_knob_led_indicators(&self, display: &mut impl DrawTarget<Color = BinaryColor>, menu_system: &MenuSystem) -> Result<()> {
        let led_values = menu_system.get_knob_led_values();
        
        // Draw small LED ring indicators for knobs 1-4
        for (i, &value) in led_values.iter().enumerate() {
            let x = 20 + (i as i32 * 20);
            let y = 50;
            
            // Draw knob ring (simplified as a small circle with fill level)
            Circle::new(Point::new(x - 3, y - 3), 6)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)
                .map_err(|e| anyhow::anyhow!("Circle draw failed: {:?}", e))?;
            
            // Fill based on LED value (0-127)
            let fill_level = (value as f32 / 127.0 * 4.0) as i32;
            if fill_level > 0 {
                Rectangle::new(Point::new(x - 2, y + 1 - fill_level), Size::new(4, fill_level as u32))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)
                    .map_err(|e| anyhow::anyhow!("Rectangle draw failed: {:?}", e))?;
            }
        }
        
        Ok(())
    }

    // Real-time parameter value display during EDIT mode
    fn draw_edit_parameter_display(&self, display: &mut impl DrawTarget<Color = BinaryColor>, menu_system: &MenuSystem) -> Result<()> {
        if !menu_system.is_edit_mode() {
            return Ok(());
        }

        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let large_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        
        if let Some(edit_param) = menu_system.get_edit_parameter() {
            if let Some(page) = menu_system.get_current_page() {
                if let Some(item) = page.items.iter().find(|i| i.id == *edit_param) {
                    // Draw large parameter name
                    Text::new(&item.name, Point::new(10, 30), large_style)
                        .draw(display)
                        .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                    
                    // Draw large parameter value with bar
                    match &item.value {
                        Some(MenuValue::Float(value, min, max)) => {
                            let value_text = format!("{:.1}", value);
                            Text::new(&value_text, Point::new(10, 45), large_style)
                                .draw(display)
                                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                            
                            // Draw value bar
                            let bar_width = 80;
                            let fill_width = ((value - min) / (max - min) * bar_width as f32) as u32;
                            
                            Rectangle::new(Point::new(10, 48), Size::new(bar_width, 4))
                                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                                .draw(display)
                                .map_err(|e| anyhow::anyhow!("Rectangle draw failed: {:?}", e))?;
                            
                            if fill_width > 0 {
                                Rectangle::new(Point::new(11, 49), Size::new(fill_width, 2))
                                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                                    .draw(display)
                                    .map_err(|e| anyhow::anyhow!("Rectangle draw failed: {:?}", e))?;
                            }
                        }
                        Some(MenuValue::Selection(options, selected)) => {
                            if let Some(selected_option) = options.get(*selected) {
                                Text::new(selected_option, Point::new(10, 45), large_style)
                                    .draw(display)
                                    .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                            }
                        }
                        Some(MenuValue::Bool(value)) => {
                            let value_text = if *value { "ON" } else { "OFF" };
                            Text::new(value_text, Point::new(10, 45), large_style)
                                .draw(display)
                                .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Ok(())
    }
}