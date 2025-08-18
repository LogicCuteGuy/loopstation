use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::communication::{SystemState, CommunicationManager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MenuType {
    Main,
    CtlFunc,
    Assign,
    Track,
    InputFx,
    TrackFx,
    MasterFx,
    Rhythm,
    Memory,
    LFO,
    StepSeq,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Navigate(MenuType),
    SelectItem(usize),
    EditParameter(String, f32),
    ExecuteFunction(String),
    Back,
    Exit,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub item_type: MenuItemType,
    pub value: Option<MenuValue>,
    pub submenu: Option<MenuType>,
    pub action: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MenuItemType {
    Navigation,
    Parameter,
    Function,
    Toggle,
    Selection,
    Value,
}

#[derive(Debug, Clone)]
pub enum MenuValue {
    Float(f32, f32, f32), // value, min, max
    Int(i32, i32, i32),   // value, min, max
    Bool(bool),
    String(String),
    Selection(Vec<String>, usize), // options, selected_index
}

#[derive(Debug, Clone)]
pub struct MenuPage {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub max_visible_items: usize,
}

pub struct MenuSystem {
    current_menu: MenuType,
    menu_stack: Vec<MenuType>,
    pages: HashMap<MenuType, MenuPage>,
    edit_mode: bool,
    edit_parameter: Option<String>,
    page_index: usize,
    total_pages: usize,
}

impl MenuSystem {
    pub fn new() -> Self {
        let mut menu_system = Self {
            current_menu: MenuType::Main,
            menu_stack: Vec::new(),
            pages: HashMap::new(),
            edit_mode: false,
            edit_parameter: None,
            page_index: 0,
            total_pages: 1,
        };

        menu_system.initialize_menus();
        menu_system
    }

    fn initialize_menus(&mut self) {
        // Main Menu
        let main_menu = MenuPage {
            title: "MAIN MENU".to_string(),
            items: vec![
                MenuItem {
                    id: "ctl_func".to_string(),
                    name: "CTL FUNC".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::CtlFunc),
                    action: None,
                },
                MenuItem {
                    id: "assign".to_string(),
                    name: "Assign".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::Assign),
                    action: None,
                },
                MenuItem {
                    id: "track".to_string(),
                    name: "Track".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::Track),
                    action: None,
                },
                MenuItem {
                    id: "input_fx".to_string(),
                    name: "Input FX".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::InputFx),
                    action: None,
                },
                MenuItem {
                    id: "track_fx".to_string(),
                    name: "Track FX".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::TrackFx),
                    action: None,
                },
                MenuItem {
                    id: "master_fx".to_string(),
                    name: "Master FX".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::MasterFx),
                    action: None,
                },
                MenuItem {
                    id: "rhythm".to_string(),
                    name: "Rhythm".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::Rhythm),
                    action: None,
                },
                MenuItem {
                    id: "memory".to_string(),
                    name: "Memory".to_string(),
                    item_type: MenuItemType::Navigation,
                    value: None,
                    submenu: Some(MenuType::Memory),
                    action: None,
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 6,
        };

        // CTL FUNC Menu - Button and pedal assignment
        let ctl_func_menu = MenuPage {
            title: "CTL FUNC".to_string(),
            items: vec![
                MenuItem {
                    id: "fx1_assign".to_string(),
                    name: "FX1 Button".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "DELAY".to_string(), "REVERB".to_string(), "SLICER".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("assign_fx1".to_string()),
                },
                MenuItem {
                    id: "fx2_assign".to_string(),
                    name: "FX2 Button".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "CHORUS".to_string(), "FLANGER".to_string(), "PITCH".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("assign_fx2".to_string()),
                },
                MenuItem {
                    id: "fx3_assign".to_string(),
                    name: "FX3 Button".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "FILTER".to_string(), "COMPRESSOR".to_string(), "EQ".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("assign_fx3".to_string()),
                },
                MenuItem {
                    id: "fx4_assign".to_string(),
                    name: "FX4 Button".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "BEAT_REPEAT".to_string(), "REVERSE".to_string(), "GATE".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("assign_fx4".to_string()),
                },
                MenuItem {
                    id: "ctl1_assign".to_string(),
                    name: "CTL1/EXP1".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "VOLUME".to_string(), "FX_DEPTH".to_string(), "TEMPO".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("assign_ctl1".to_string()),
                },
                MenuItem {
                    id: "ctl2_assign".to_string(),
                    name: "CTL3/EXP2".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "PAN".to_string(), "FILTER_CUTOFF".to_string(), "PITCH_BEND".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("assign_ctl2".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 5,
        };

        // Assign Menu - MIDI and CC mapping
        let assign_menu = MenuPage {
            title: "ASSIGN".to_string(),
            items: vec![
                MenuItem {
                    id: "midi_channel".to_string(),
                    name: "MIDI Channel".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        (1..=16).map(|i| i.to_string()).chain(std::iter::once("OMNI".to_string())).collect(),
                        0,
                    )),
                    submenu: None,
                    action: Some("set_midi_channel".to_string()),
                },
                MenuItem {
                    id: "cc_track1".to_string(),
                    name: "Track 1 CC".to_string(),
                    item_type: MenuItemType::Value,
                    value: Some(MenuValue::Int(1, 0, 127)),
                    submenu: None,
                    action: Some("assign_cc_track1".to_string()),
                },
                MenuItem {
                    id: "cc_track2".to_string(),
                    name: "Track 2 CC".to_string(),
                    item_type: MenuItemType::Value,
                    value: Some(MenuValue::Int(2, 0, 127)),
                    submenu: None,
                    action: Some("assign_cc_track2".to_string()),
                },
                MenuItem {
                    id: "cc_track3".to_string(),
                    name: "Track 3 CC".to_string(),
                    item_type: MenuItemType::Value,
                    value: Some(MenuValue::Int(3, 0, 127)),
                    submenu: None,
                    action: Some("assign_cc_track3".to_string()),
                },
                MenuItem {
                    id: "cc_track4".to_string(),
                    name: "Track 4 CC".to_string(),
                    item_type: MenuItemType::Value,
                    value: Some(MenuValue::Int(4, 0, 127)),
                    submenu: None,
                    action: Some("assign_cc_track4".to_string()),
                },
                MenuItem {
                    id: "cc_track5".to_string(),
                    name: "Track 5 CC".to_string(),
                    item_type: MenuItemType::Value,
                    value: Some(MenuValue::Int(5, 0, 127)),
                    submenu: None,
                    action: Some("assign_cc_track5".to_string()),
                },
                MenuItem {
                    id: "cc_track6".to_string(),
                    name: "Track 6 CC".to_string(),
                    item_type: MenuItemType::Value,
                    value: Some(MenuValue::Int(6, 0, 127)),
                    submenu: None,
                    action: Some("assign_cc_track6".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 5,
        };

        // Track Menu - Per-track settings
        let track_menu = MenuPage {
            title: "TRACK".to_string(),
            items: vec![
                MenuItem {
                    id: "track_select".to_string(),
                    name: "Select Track".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        (1..=6).map(|i| format!("Track {}", i)).collect(),
                        0,
                    )),
                    submenu: None,
                    action: Some("select_track".to_string()),
                },
                MenuItem {
                    id: "input_source".to_string(),
                    name: "Input Source".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["MIC 1".to_string(), "MIC 2".to_string(), "INST 1".to_string(), "INST 2".to_string(), "USB".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_input_source".to_string()),
                },
                MenuItem {
                    id: "play_mode".to_string(),
                    name: "Play Mode".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["NORMAL".to_string(), "REVERSE".to_string(), "1SHOT".to_string(), "GATE".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_play_mode".to_string()),
                },
                MenuItem {
                    id: "quantize".to_string(),
                    name: "Quantize".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "1/4".to_string(), "1/8".to_string(), "1/16".to_string(), "1/32".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_quantize".to_string()),
                },
                MenuItem {
                    id: "start_point".to_string(),
                    name: "Start Point".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(0.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_start_point".to_string()),
                },
                MenuItem {
                    id: "end_point".to_string(),
                    name: "End Point".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(100.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_end_point".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 5,
        };

        // Input FX Menu
        let input_fx_menu = MenuPage {
            title: "INPUT FX".to_string(),
            items: vec![
                MenuItem {
                    id: "input_fx_slot1".to_string(),
                    name: "Slot 1".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "COMPRESSOR".to_string(), "NOISE_GATE".to_string(), "EQ".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_input_fx1".to_string()),
                },
                MenuItem {
                    id: "input_fx_slot2".to_string(),
                    name: "Slot 2".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "CHORUS".to_string(), "FLANGER".to_string(), "PHASER".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_input_fx2".to_string()),
                },
                MenuItem {
                    id: "input_fx_slot3".to_string(),
                    name: "Slot 3".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "DELAY".to_string(), "REVERB".to_string(), "ECHO".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_input_fx3".to_string()),
                },
                MenuItem {
                    id: "input_fx_slot4".to_string(),
                    name: "Slot 4".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "PITCH_SHIFT".to_string(), "HARMONIZER".to_string(), "VOCODER".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_input_fx4".to_string()),
                },
                MenuItem {
                    id: "input_fx_bank".to_string(),
                    name: "FX Bank".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["Bank 1".to_string(), "Bank 2".to_string(), "Bank 3".to_string(), "Bank 4".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("select_input_fx_bank".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 5,
        };

        // Track FX Menu
        let track_fx_menu = MenuPage {
            title: "TRACK FX".to_string(),
            items: vec![
                MenuItem {
                    id: "track_fx_select".to_string(),
                    name: "Select Track".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        (1..=6).map(|i| format!("Track {}", i)).collect(),
                        0,
                    )),
                    submenu: None,
                    action: Some("select_track_fx".to_string()),
                },
                MenuItem {
                    id: "track_fx_slot1".to_string(),
                    name: "Slot 1".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "SLICER".to_string(), "BEAT_REPEAT".to_string(), "REVERSE".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_track_fx1".to_string()),
                },
                MenuItem {
                    id: "track_fx_slot2".to_string(),
                    name: "Slot 2".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "FILTER".to_string(), "ISOLATOR".to_string(), "AUTO_WAH".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_track_fx2".to_string()),
                },
                MenuItem {
                    id: "track_fx_slot3".to_string(),
                    name: "Slot 3".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "DELAY".to_string(), "REVERB".to_string(), "ECHO".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_track_fx3".to_string()),
                },
                MenuItem {
                    id: "track_fx_slot4".to_string(),
                    name: "Slot 4".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "PITCH_SHIFT".to_string(), "HARMONIZER".to_string(), "GATE".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_track_fx4".to_string()),
                },
                MenuItem {
                    id: "track_fx_bank".to_string(),
                    name: "FX Bank".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["Bank 1".to_string(), "Bank 2".to_string(), "Bank 3".to_string(), "Bank 4".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("select_track_fx_bank".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 5,
        };

        // Master FX Menu
        let master_fx_menu = MenuPage {
            title: "MASTER FX".to_string(),
            items: vec![
                MenuItem {
                    id: "master_fx_slot1".to_string(),
                    name: "Slot 1".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "MASTERING_EQ".to_string(), "MULTIBAND_COMP".to_string(), "LIMITER".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_master_fx1".to_string()),
                },
                MenuItem {
                    id: "master_fx_slot2".to_string(),
                    name: "Slot 2".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "DJ_FILTER".to_string(), "ISOLATOR".to_string(), "FILTER".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_master_fx2".to_string()),
                },
                MenuItem {
                    id: "master_fx_slot3".to_string(),
                    name: "Slot 3".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "REVERB".to_string(), "DELAY".to_string(), "SPACE_REVERB".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_master_fx3".to_string()),
                },
                MenuItem {
                    id: "master_fx_slot4".to_string(),
                    name: "Slot 4".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "SIDECHAIN".to_string(), "MIXER".to_string(), "UTILITY".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_master_fx4".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 4,
        };

        // Rhythm Menu
        let rhythm_menu = MenuPage {
            title: "RHYTHM".to_string(),
            items: vec![
                MenuItem {
                    id: "rhythm_pattern".to_string(),
                    name: "Pattern".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "KICK".to_string(), "SNARE".to_string(), "HI-HAT".to_string(), "FULL_KIT".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_rhythm_pattern".to_string()),
                },
                MenuItem {
                    id: "rhythm_volume".to_string(),
                    name: "Volume".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(50.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_rhythm_volume".to_string()),
                },
                MenuItem {
                    id: "rhythm_tempo_sync".to_string(),
                    name: "Tempo Sync".to_string(),
                    item_type: MenuItemType::Toggle,
                    value: Some(MenuValue::Bool(true)),
                    submenu: None,
                    action: Some("toggle_rhythm_sync".to_string()),
                },
                MenuItem {
                    id: "rhythm_start_stop".to_string(),
                    name: "Start/Stop".to_string(),
                    item_type: MenuItemType::Function,
                    value: None,
                    submenu: None,
                    action: Some("rhythm_start_stop".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 4,
        };

        // Memory Menu
        let memory_menu = MenuPage {
            title: "MEMORY".to_string(),
            items: vec![
                MenuItem {
                    id: "memory_slot".to_string(),
                    name: "Memory Slot".to_string(),
                    item_type: MenuItemType::Value,
                    value: Some(MenuValue::Int(1, 1, 255)),
                    submenu: None,
                    action: Some("select_memory_slot".to_string()),
                },
                MenuItem {
                    id: "memory_load".to_string(),
                    name: "Load".to_string(),
                    item_type: MenuItemType::Function,
                    value: None,
                    submenu: None,
                    action: Some("memory_load".to_string()),
                },
                MenuItem {
                    id: "memory_save".to_string(),
                    name: "Save".to_string(),
                    item_type: MenuItemType::Function,
                    value: None,
                    submenu: None,
                    action: Some("memory_save".to_string()),
                },
                MenuItem {
                    id: "memory_initialize".to_string(),
                    name: "Initialize".to_string(),
                    item_type: MenuItemType::Function,
                    value: None,
                    submenu: None,
                    action: Some("memory_initialize".to_string()),
                },
                MenuItem {
                    id: "memory_name".to_string(),
                    name: "Name".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::String("Memory 001".to_string())),
                    submenu: None,
                    action: Some("set_memory_name".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 5,
        };

        // Store all menu pages
        self.pages.insert(MenuType::Main, main_menu);
        self.pages.insert(MenuType::CtlFunc, ctl_func_menu);
        self.pages.insert(MenuType::Assign, assign_menu);
        self.pages.insert(MenuType::Track, track_menu);
        self.pages.insert(MenuType::InputFx, input_fx_menu);
        self.pages.insert(MenuType::TrackFx, track_fx_menu);
        self.pages.insert(MenuType::MasterFx, master_fx_menu);
        self.pages.insert(MenuType::Rhythm, rhythm_menu);
        self.pages.insert(MenuType::Memory, memory_menu);
    }

    // Navigation methods
    pub fn navigate_to(&mut self, menu_type: MenuType) -> Result<()> {
        if menu_type != self.current_menu {
            self.menu_stack.push(self.current_menu.clone());
            self.current_menu = menu_type;
            self.exit_edit_mode();
        }
        Ok(())
    }

    pub fn navigate_back(&mut self) -> Result<()> {
        if let Some(previous_menu) = self.menu_stack.pop() {
            self.current_menu = previous_menu;
            self.exit_edit_mode();
        }
        Ok(())
    }

    pub fn navigate_to_main(&mut self) -> Result<()> {
        self.menu_stack.clear();
        self.current_menu = MenuType::Main;
        self.exit_edit_mode();
        Ok(())
    }

    // VALUE knob navigation
    pub fn value_knob_turn(&mut self, direction: i32) -> Result<MenuAction> {
        if self.edit_mode {
            // In edit mode, adjust parameter value
            if let Some(page) = self.pages.get_mut(&self.current_menu) {
                if let Some(item) = page.items.get_mut(page.selected_index) {
                    return self.adjust_parameter_value(item, direction);
                }
            }
        } else {
            // In navigation mode, move selection
            if let Some(page) = self.pages.get_mut(&self.current_menu) {
                let old_index = page.selected_index;
                
                if direction > 0 {
                    page.selected_index = (page.selected_index + 1).min(page.items.len() - 1);
                } else if direction < 0 {
                    page.selected_index = page.selected_index.saturating_sub(1);
                }

                // Update scroll offset if needed
                if page.selected_index >= page.scroll_offset + page.max_visible_items {
                    page.scroll_offset = page.selected_index - page.max_visible_items + 1;
                } else if page.selected_index < page.scroll_offset {
                    page.scroll_offset = page.selected_index;
                }

                if page.selected_index != old_index {
                    return Ok(MenuAction::SelectItem(page.selected_index));
                }
            }
        }
        
        Ok(MenuAction::SelectItem(0)) // No change
    }

    pub fn value_knob_press(&mut self) -> Result<MenuAction> {
        if let Some(page) = self.pages.get(&self.current_menu) {
            if let Some(item) = page.items.get(page.selected_index) {
                match item.item_type {
                    MenuItemType::Navigation => {
                        if let Some(submenu) = &item.submenu {
                            return Ok(MenuAction::Navigate(submenu.clone()));
                        }
                    }
                    MenuItemType::Parameter | MenuItemType::Value | MenuItemType::Selection => {
                        // Enter edit mode for this parameter
                        self.edit_mode = true;
                        self.edit_parameter = Some(item.id.clone());
                        return Ok(MenuAction::EditParameter(item.id.clone(), 0.0));
                    }
                    MenuItemType::Function => {
                        if let Some(action) = &item.action {
                            return Ok(MenuAction::ExecuteFunction(action.clone()));
                        }
                    }
                    MenuItemType::Toggle => {
                        if let Some(MenuValue::Bool(current)) = &item.value {
                            // Toggle the boolean value
                            return self.toggle_boolean_value(&item.id, !current);
                        }
                    }
                }
            }
        }
        
        Ok(MenuAction::SelectItem(0))
    }

    // PAGE button navigation
    pub fn page_left(&mut self) -> Result<MenuAction> {
        if self.edit_mode {
            // In edit mode, PAGE buttons access advanced parameters
            return self.access_advanced_parameters(-1);
        } else {
            // In navigation mode, PAGE buttons navigate between menu tabs
            self.page_index = self.page_index.saturating_sub(1);
            return Ok(MenuAction::SelectItem(self.page_index));
        }
    }

    pub fn page_right(&mut self) -> Result<MenuAction> {
        if self.edit_mode {
            // In edit mode, PAGE buttons access advanced parameters
            return self.access_advanced_parameters(1);
        } else {
            // In navigation mode, PAGE buttons navigate between menu tabs
            self.page_index = (self.page_index + 1).min(self.total_pages - 1);
            return Ok(MenuAction::SelectItem(self.page_index));
        }
    }

    // EDIT mode management
    pub fn enter_edit_mode(&mut self, parameter_id: &str) -> Result<()> {
        self.edit_mode = true;
        self.edit_parameter = Some(parameter_id.to_string());
        Ok(())
    }

    pub fn exit_edit_mode(&mut self) {
        self.edit_mode = false;
        self.edit_parameter = None;
    }

    pub fn is_edit_mode(&self) -> bool {
        self.edit_mode
    }

    pub fn get_edit_parameter(&self) -> Option<&String> {
        self.edit_parameter.as_ref()
    }

    // EDIT button functionality for real-time parameter control
    pub fn handle_edit_button_press(&mut self) -> Result<MenuAction> {
        if self.edit_mode {
            // Exit edit mode if already in edit mode
            self.exit_edit_mode();
            Ok(MenuAction::Back)
        } else {
            // Enter edit mode for the currently selected item
            if let Some(page) = self.pages.get(&self.current_menu) {
                if let Some(item) = page.items.get(page.selected_index) {
                    match item.item_type {
                        MenuItemType::Parameter | MenuItemType::Value | MenuItemType::Selection => {
                            self.edit_mode = true;
                            self.edit_parameter = Some(item.id.clone());
                            Ok(MenuAction::EditParameter(item.id.clone(), 0.0))
                        }
                        _ => {
                            // Can't edit this type of item
                            Ok(MenuAction::SelectItem(0))
                        }
                    }
                } else {
                    Ok(MenuAction::SelectItem(0))
                }
            } else {
                Ok(MenuAction::SelectItem(0))
            }
        }
    }

    // Context-sensitive EDIT mode for different menu types
    pub fn handle_context_edit(&mut self, context: &str) -> Result<MenuAction> {
        match context {
            "track" => self.enter_track_edit_mode(),
            "fx" => self.enter_fx_edit_mode(),
            "memory" => self.enter_memory_edit_mode(),
            _ => Ok(MenuAction::SelectItem(0)),
        }
    }

    fn enter_track_edit_mode(&mut self) -> Result<MenuAction> {
        // Create a temporary track parameter edit page
        let track_edit_page = MenuPage {
            title: "TRACK EDIT".to_string(),
            items: vec![
                MenuItem {
                    id: "track_volume".to_string(),
                    name: "Volume".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(75.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_track_volume".to_string()),
                },
                MenuItem {
                    id: "track_pan".to_string(),
                    name: "Pan".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(0.0, -100.0, 100.0)),
                    submenu: None,
                    action: Some("set_track_pan".to_string()),
                },
                MenuItem {
                    id: "track_play_mode".to_string(),
                    name: "Play Mode".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["NORMAL".to_string(), "REVERSE".to_string(), "1SHOT".to_string(), "GATE".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_track_play_mode".to_string()),
                },
                MenuItem {
                    id: "track_quantize".to_string(),
                    name: "Quantize".to_string(),
                    item_type: MenuItemType::Selection,
                    value: Some(MenuValue::Selection(
                        vec!["OFF".to_string(), "1/4".to_string(), "1/8".to_string(), "1/16".to_string(), "1/32".to_string()],
                        0,
                    )),
                    submenu: None,
                    action: Some("set_track_quantize".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 4,
        };

        // Store the edit page temporarily
        self.pages.insert(MenuType::System, track_edit_page); // Using System as temporary storage
        self.navigate_to(MenuType::System)?;
        self.edit_mode = true;
        self.edit_parameter = Some("track_volume".to_string());
        
        Ok(MenuAction::EditParameter("track_volume".to_string(), 75.0))
    }

    fn enter_fx_edit_mode(&mut self) -> Result<MenuAction> {
        // Create a temporary FX parameter edit page
        let fx_edit_page = MenuPage {
            title: "FX EDIT".to_string(),
            items: vec![
                MenuItem {
                    id: "fx_depth".to_string(),
                    name: "Depth".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(50.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_fx_depth".to_string()),
                },
                MenuItem {
                    id: "fx_rate".to_string(),
                    name: "Rate".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(25.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_fx_rate".to_string()),
                },
                MenuItem {
                    id: "fx_feedback".to_string(),
                    name: "Feedback".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(30.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_fx_feedback".to_string()),
                },
                MenuItem {
                    id: "fx_wet_dry".to_string(),
                    name: "Wet/Dry".to_string(),
                    item_type: MenuItemType::Parameter,
                    value: Some(MenuValue::Float(50.0, 0.0, 100.0)),
                    submenu: None,
                    action: Some("set_fx_wet_dry".to_string()),
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: 4,
        };

        // Store the edit page temporarily
        self.pages.insert(MenuType::LFO, fx_edit_page); // Using LFO as temporary storage
        self.navigate_to(MenuType::LFO)?;
        self.edit_mode = true;
        self.edit_parameter = Some("fx_depth".to_string());
        
        Ok(MenuAction::EditParameter("fx_depth".to_string(), 50.0))
    }

    fn enter_memory_edit_mode(&mut self) -> Result<MenuAction> {
        // Enter edit mode for memory slot selection
        self.navigate_to(MenuType::Memory)?;
        self.edit_mode = true;
        self.edit_parameter = Some("memory_slot".to_string());
        
        Ok(MenuAction::EditParameter("memory_slot".to_string(), 1.0))
    }

    // Advanced parameter access with PAGE buttons in EDIT mode
    pub fn access_advanced_edit_parameters(&mut self, direction: i32) -> Result<MenuAction> {
        if !self.edit_mode {
            return Ok(MenuAction::SelectItem(0));
        }

        // Create advanced parameter pages based on current context
        match self.current_menu {
            MenuType::Track => self.show_advanced_track_parameters(direction),
            MenuType::InputFx | MenuType::TrackFx | MenuType::MasterFx => self.show_advanced_fx_parameters(direction),
            MenuType::Memory => self.show_advanced_memory_parameters(direction),
            _ => Ok(MenuAction::SelectItem(0)),
        }
    }

    fn show_advanced_track_parameters(&mut self, direction: i32) -> Result<MenuAction> {
        // Advanced track parameters revealed with PAGE buttons in EDIT mode
        let advanced_items = vec![
            MenuItem {
                id: "start_point".to_string(),
                name: "Start Point".to_string(),
                item_type: MenuItemType::Parameter,
                value: Some(MenuValue::Float(0.0, 0.0, 100.0)),
                submenu: None,
                action: Some("set_start_point".to_string()),
            },
            MenuItem {
                id: "end_point".to_string(),
                name: "End Point".to_string(),
                item_type: MenuItemType::Parameter,
                value: Some(MenuValue::Float(100.0, 0.0, 100.0)),
                submenu: None,
                action: Some("set_end_point".to_string()),
            },
            MenuItem {
                id: "reverse".to_string(),
                name: "Reverse".to_string(),
                item_type: MenuItemType::Toggle,
                value: Some(MenuValue::Bool(false)),
                submenu: None,
                action: Some("toggle_reverse".to_string()),
            },
            MenuItem {
                id: "pitch".to_string(),
                name: "Pitch".to_string(),
                item_type: MenuItemType::Parameter,
                value: Some(MenuValue::Float(0.0, -12.0, 12.0)),
                submenu: None,
                action: Some("set_pitch".to_string()),
            },
        ];

        // Add advanced parameters to current page
        if let Some(page) = self.pages.get_mut(&self.current_menu) {
            // Navigate through advanced parameters
            let current_advanced = page.selected_index.saturating_sub(4); // Assuming first 4 are basic
            let new_advanced = if direction > 0 {
                (current_advanced + 1).min(advanced_items.len() - 1)
            } else {
                current_advanced.saturating_sub(1)
            };
            
            page.selected_index = 4 + new_advanced; // Offset by basic parameters
            
            if let Some(item) = advanced_items.get(new_advanced) {
                self.edit_parameter = Some(item.id.clone());
                return Ok(MenuAction::EditParameter(item.id.clone(), 0.0));
            }
        }

        Ok(MenuAction::SelectItem(0))
    }

    fn show_advanced_fx_parameters(&mut self, direction: i32) -> Result<MenuAction> {
        // Advanced FX parameters revealed with PAGE buttons in EDIT mode
        let advanced_items = vec![
            MenuItem {
                id: "fx_sync".to_string(),
                name: "Tempo Sync".to_string(),
                item_type: MenuItemType::Toggle,
                value: Some(MenuValue::Bool(false)),
                submenu: None,
                action: Some("toggle_fx_sync".to_string()),
            },
            MenuItem {
                id: "fx_modulation".to_string(),
                name: "Modulation".to_string(),
                item_type: MenuItemType::Parameter,
                value: Some(MenuValue::Float(0.0, 0.0, 100.0)),
                submenu: None,
                action: Some("set_fx_modulation".to_string()),
            },
            MenuItem {
                id: "fx_filter".to_string(),
                name: "Filter".to_string(),
                item_type: MenuItemType::Parameter,
                value: Some(MenuValue::Float(50.0, 0.0, 100.0)),
                submenu: None,
                action: Some("set_fx_filter".to_string()),
            },
        ];

        // Similar navigation logic for FX parameters
        if let Some(page) = self.pages.get_mut(&self.current_menu) {
            let current_advanced = page.selected_index.saturating_sub(4);
            let new_advanced = if direction > 0 {
                (current_advanced + 1).min(advanced_items.len() - 1)
            } else {
                current_advanced.saturating_sub(1)
            };
            
            page.selected_index = 4 + new_advanced;
            
            if let Some(item) = advanced_items.get(new_advanced) {
                self.edit_parameter = Some(item.id.clone());
                return Ok(MenuAction::EditParameter(item.id.clone(), 0.0));
            }
        }

        Ok(MenuAction::SelectItem(0))
    }

    fn show_advanced_memory_parameters(&mut self, _direction: i32) -> Result<MenuAction> {
        // Advanced memory parameters (bank navigation, naming, etc.)
        Ok(MenuAction::EditParameter("memory_bank".to_string(), 1.0))
    }

    // Real-time parameter override with HOLD EDIT + knob
    pub fn handle_direct_parameter_override(&mut self, knob_id: u8, value: f32) -> Result<MenuAction> {
        let parameter_id = match knob_id {
            1 => "knob1_override",
            2 => "knob2_override", 
            3 => "knob3_override",
            4 => "knob4_override",
            _ => return Ok(MenuAction::SelectItem(0)),
        };

        // Map knobs to context-sensitive parameters based on current menu
        let mapped_parameter = match self.current_menu {
            MenuType::Track => {
                match knob_id {
                    1 => "track_volume",
                    2 => "track_pan",
                    3 => "track_start_point",
                    4 => "track_end_point",
                    _ => parameter_id,
                }
            }
            MenuType::InputFx | MenuType::TrackFx | MenuType::MasterFx => {
                match knob_id {
                    1 => "fx_depth",
                    2 => "fx_rate",
                    3 => "fx_feedback",
                    4 => "fx_wet_dry",
                    _ => parameter_id,
                }
            }
            MenuType::Memory => {
                match knob_id {
                    1 => "memory_slot",
                    2 => "memory_bank",
                    3 => "memory_tempo",
                    4 => "memory_volume",
                    _ => parameter_id,
                }
            }
            _ => parameter_id,
        };

        Ok(MenuAction::EditParameter(mapped_parameter.to_string(), value))
    }

    // Knob LED feedback for real-time control
    pub fn get_knob_led_values(&self) -> [u8; 4] {
        let mut led_values = [0u8; 4];
        
        if self.edit_mode {
            if let Some(page) = self.pages.get(&self.current_menu) {
                if let Some(item) = page.items.get(page.selected_index) {
                    // Map current parameter value to LED ring position
                    match &item.value {
                        Some(MenuValue::Float(value, min, max)) => {
                            let normalized = ((value - min) / (max - min) * 127.0) as u8;
                            led_values[0] = normalized; // Knob 1 shows current parameter
                        }
                        Some(MenuValue::Int(value, min, max)) => {
                            let normalized = ((*value - min) as f32 / (*max - min) as f32 * 127.0) as u8;
                            led_values[0] = normalized;
                        }
                        Some(MenuValue::Selection(options, selected)) => {
                            let normalized = (*selected as f32 / (options.len() - 1) as f32 * 127.0) as u8;
                            led_values[0] = normalized;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        led_values
    }

    // Parameter adjustment
    fn adjust_parameter_value(&mut self, item: &mut MenuItem, direction: i32) -> Result<MenuAction> {
        match &mut item.value {
            Some(MenuValue::Float(value, min, max)) => {
                let step = (max - min) / 100.0; // 1% steps
                let new_value = (*value + (direction as f32 * step)).clamp(*min, *max);
                *value = new_value;
                Ok(MenuAction::EditParameter(item.id.clone(), new_value))
            }
            Some(MenuValue::Int(value, min, max)) => {
                let new_value = (*value + direction).clamp(*min, *max);
                *value = new_value;
                Ok(MenuAction::EditParameter(item.id.clone(), new_value as f32))
            }
            Some(MenuValue::Selection(options, selected)) => {
                let new_index = if direction > 0 {
                    (*selected + 1).min(options.len() - 1)
                } else {
                    selected.saturating_sub(1)
                };
                *selected = new_index;
                Ok(MenuAction::EditParameter(item.id.clone(), new_index as f32))
            }
            Some(MenuValue::Bool(value)) => {
                *value = !*value;
                Ok(MenuAction::EditParameter(item.id.clone(), if *value { 1.0 } else { 0.0 }))
            }
            _ => Ok(MenuAction::SelectItem(0)),
        }
    }

    fn toggle_boolean_value(&mut self, item_id: &str, new_value: bool) -> Result<MenuAction> {
        if let Some(page) = self.pages.get_mut(&self.current_menu) {
            if let Some(item) = page.items.iter_mut().find(|i| i.id == item_id) {
                if let Some(MenuValue::Bool(value)) = &mut item.value {
                    *value = new_value;
                    return Ok(MenuAction::EditParameter(item_id.to_string(), if new_value { 1.0 } else { 0.0 }));
                }
            }
        }
        Ok(MenuAction::SelectItem(0))
    }

    fn access_advanced_parameters(&mut self, direction: i32) -> Result<MenuAction> {
        // Implementation for accessing advanced parameters with PAGE buttons in EDIT mode
        // This would reveal secondary parameters like Start Point, End Point, Reverse, Pitch
        // For now, return a placeholder action
        Ok(MenuAction::EditParameter("advanced_param".to_string(), direction as f32))
    }

    // State getters
    pub fn get_current_menu(&self) -> &MenuType {
        &self.current_menu
    }

    pub fn get_current_page(&self) -> Option<&MenuPage> {
        self.pages.get(&self.current_menu)
    }

    pub fn get_selected_item(&self) -> Option<&MenuItem> {
        if let Some(page) = self.pages.get(&self.current_menu) {
            page.items.get(page.selected_index)
        } else {
            None
        }
    }

    pub fn get_menu_stack(&self) -> &Vec<MenuType> {
        &self.menu_stack
    }

    // Update menu state based on system state
    pub fn update_from_system_state(&mut self, state: &SystemState) -> Result<()> {
        // Update memory slot in Memory menu
        if let Some(memory_page) = self.pages.get_mut(&MenuType::Memory) {
            if let Some(memory_item) = memory_page.items.iter_mut().find(|i| i.id == "memory_slot") {
                if let Some(MenuValue::Int(value, _, _)) = &mut memory_item.value {
                    *value = state.current_memory as i32;
                }
            }
        }

        // Update track selection based on system state
        // This could be expanded to sync other parameters as well
        
        Ok(())
    }

    // Execute menu actions through communication manager
    pub fn execute_action(&self, action: &MenuAction, comm_manager: &CommunicationManager) -> Result<()> {
        match action {
            MenuAction::ExecuteFunction(function) => {
                self.execute_function(function, comm_manager)?;
            }
            MenuAction::EditParameter(param_id, value) => {
                self.execute_parameter_change(param_id, *value, comm_manager)?;
            }
            _ => {
                // Navigation actions don't need communication with STM32
            }
        }
        Ok(())
    }

    fn execute_function(&self, function: &str, comm_manager: &CommunicationManager) -> Result<()> {
        match function {
            "memory_load" => {
                if let Some(memory_page) = self.pages.get(&MenuType::Memory) {
                    if let Some(memory_item) = memory_page.items.iter().find(|i| i.id == "memory_slot") {
                        if let Some(MenuValue::Int(slot, _, _)) = &memory_item.value {
                            comm_manager.send_memory_command(*slot as u8, "load")?;
                        }
                    }
                }
            }
            "memory_save" => {
                if let Some(memory_page) = self.pages.get(&MenuType::Memory) {
                    if let Some(memory_item) = memory_page.items.iter().find(|i| i.id == "memory_slot") {
                        if let Some(MenuValue::Int(slot, _, _)) = &memory_item.value {
                            comm_manager.send_memory_command(*slot as u8, "save")?;
                        }
                    }
                }
            }
            "rhythm_start_stop" => {
                // Send rhythm start/stop command
                comm_manager.send_fx_command(0, "toggle")?; // Placeholder for rhythm control
            }
            _ => {
                // Other functions can be implemented as needed
            }
        }
        Ok(())
    }

    fn execute_parameter_change(&self, param_id: &str, value: f32, comm_manager: &CommunicationManager) -> Result<()> {
        match param_id {
            id if id.starts_with("cc_track") => {
                // Handle MIDI CC assignments
                if let Some(track_num) = id.chars().last().and_then(|c| c.to_digit(10)) {
                    // This would update MIDI CC mapping - implementation depends on STM32 protocol
                }
            }
            "rhythm_volume" => {
                // Send rhythm volume change
                comm_manager.send_fx_command(0, "toggle")?; // Placeholder
            }
            _ => {
                // Other parameter changes can be implemented as needed
            }
        }
        Ok(())
    }
}