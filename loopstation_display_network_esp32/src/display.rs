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

pub struct DisplayManager {
    display: Mutex<Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x64, BinaryColorMode>>,
    current_screen: Mutex<DisplayScreen>,
    brightness: Mutex<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayScreen {
    Main,
    TrackDetail(u8),
    Menu(MenuType),
    Settings,
    NetworkStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuType {
    Main,
    Track,
    Effects,
    System,
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

    fn draw_menu_screen(&self, state: &SystemState, menu_type: MenuType) -> Result<()> {
        let mut display = self.display.lock().unwrap();
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Display clear failed: {:?}", e))?;

        let title = match menu_type {
            MenuType::Main => "MAIN MENU",
            MenuType::Track => "TRACK MENU",
            MenuType::Effects => "EFFECTS MENU",
            MenuType::System => "SYSTEM MENU",
        };

        self.draw_header(&mut *display, title, None)?;

        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        
        match menu_type {
            MenuType::Main => {
                let menu_items = ["1. Track Control", "2. Effects", "3. System", "4. Network"];
                for (i, item) in menu_items.iter().enumerate() {
                    Text::new(item, Point::new(5, 25 + (i as i32 * 10)), text_style)
                        .draw(&mut *display)
                        .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                }
            }
            MenuType::Track => {
                for i in 0..6 {
                    let item = format!("Track {} Settings", i + 1);
                    Text::new(&item, Point::new(5, 25 + (i as i32 * 8)), text_style)
                        .draw(&mut *display)
                        .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                }
            }
            MenuType::Effects => {
                let fx_items = ["Input FX", "Track FX", "Master FX", "FX Assign"];
                for (i, item) in fx_items.iter().enumerate() {
                    Text::new(item, Point::new(5, 25 + (i as i32 * 10)), text_style)
                        .draw(&mut *display)
                        .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                }
            }
            MenuType::System => {
                let sys_items = ["Tempo", "MIDI", "Storage", "Reset"];
                for (i, item) in sys_items.iter().enumerate() {
                    Text::new(item, Point::new(5, 25 + (i as i32 * 10)), text_style)
                        .draw(&mut *display)
                        .map_err(|e| anyhow::anyhow!("Text draw failed: {:?}", e))?;
                }
            }
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
}