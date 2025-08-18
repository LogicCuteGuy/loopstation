//! System Settings and Configuration
//! 
//! This module implements the comprehensive system settings organized in 6 pages:
//! GENERAL, CLOCK, MIDI, CONTROL, UTILITY, BACKUP as defined in Requirement 12.

use heapless::String;
use serde::{Deserialize, Serialize};

/// Maximum length for firmware version string
pub const MAX_FIRMWARE_VERSION_LEN: usize = 16;

/// System settings organized in 6 configuration pages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemSettings {
    /// GENERAL settings page
    pub general: GeneralSettings,
    /// CLOCK settings page  
    pub clock: ClockSettings,
    /// MIDI settings page
    pub midi: MidiSettings,
    /// CONTROL settings page
    pub control: ControlSettings,
    /// UTILITY settings page
    pub utility: UtilitySettings,
    /// BACKUP settings page
    pub backup: BackupSettings,
}

impl SystemSettings {
    /// Create new system settings with default values
    pub fn new() -> Self {
        Self {
            general: GeneralSettings::default(),
            clock: ClockSettings::default(),
            midi: MidiSettings::default(),
            control: ControlSettings::default(),
            utility: UtilitySettings::default(),
            backup: BackupSettings::default(),
        }
    }

    /// Reset all settings to factory defaults
    pub fn factory_reset(&mut self) {
        *self = Self::new();
    }

    /// Validate all settings and fix any invalid values
    pub fn validate_and_fix(&mut self) {
        self.general.validate_and_fix();
        self.clock.validate_and_fix();
        self.midi.validate_and_fix();
        self.control.validate_and_fix();
        self.utility.validate_and_fix();
        self.backup.validate_and_fix();
    }
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// GENERAL settings page - basic device behavior configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralSettings {
    /// Tempo Memory: preserve individual bank tempos when loading slots
    pub tempo_memory: bool,
    /// Quantize Mode: timing quantization behavior
    pub quantize_mode: QuantizeMode,
    /// Undo Mode: scope of undo operations
    pub undo_mode: UndoMode,
    /// Startup Screen: what to display on startup
    pub startup_screen: StartupScreen,
    /// Auto Off: automatic power off timer
    pub auto_off: AutoOffTime,
    /// Phones Mode: headphone output configuration
    pub phones_mode: PhonesMode,
    /// Store Mode: what to save in memory slots
    pub store_mode: StoreMode,
}

impl GeneralSettings {
    /// Validate and fix any invalid values
    pub fn validate_and_fix(&mut self) {
        // All enum values are validated by serde, no additional validation needed
    }
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            tempo_memory: false,  // OFF - allow tempo changes when loading slots
            quantize_mode: QuantizeMode::Full,
            undo_mode: UndoMode::RecordOnly,
            startup_screen: StartupScreen::Memory,
            auto_off: AutoOffTime::Off,
            phones_mode: PhonesMode::Stereo,
            store_mode: StoreMode::LoopAndSetting,
        }
    }
}

/// Quantize Mode options for timing quantization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantizeMode {
    /// Full quantization (recording and playback)
    Full,
    /// Record quantization only
    Record,
    /// No quantization
    Off,
}

/// Undo Mode options for undo operation scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UndoMode {
    /// Undo recording operations only
    RecordOnly,
    /// Undo recording and playback operations
    RecordAndPlay,
    /// Undo all operations
    All,
}

/// Startup Screen options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartupScreen {
    /// No special startup screen
    Off,
    /// Show memory slot number
    Memory,
    /// Show project name
    Name,
}

/// Auto Off timer options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoOffTime {
    /// 5 minutes
    FiveMinutes,
    /// 30 minutes
    ThirtyMinutes,
    /// Never auto-off
    Off,
}

/// Headphone output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhonesMode {
    /// Stereo output
    Stereo,
    /// Mono output
    Mono,
}

/// Store Mode options for memory slots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreMode {
    /// Save loops and settings (full project)
    LoopAndSetting,
    /// Save settings only (no audio loops)
    SettingOnly,
}

/// CLOCK settings page - timing and synchronization control
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClockSettings {
    /// Clock Source: where to get timing reference
    pub clock_source: ClockSource,
    /// Sync Out: where to send clock sync
    pub sync_out: SyncOut,
    /// Rec Quantize: recording quantization resolution
    pub rec_quantize: RecQuantize,
}

impl ClockSettings {
    /// Validate and fix any invalid values
    pub fn validate_and_fix(&mut self) {
        // All enum values are validated by serde, no additional validation needed
    }
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            clock_source: ClockSource::Internal,
            sync_out: SyncOut::Off,
            rec_quantize: RecQuantize::Quarter, // 1/4 note
        }
    }
}

/// Clock source options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockSource {
    /// Internal clock generator
    Internal,
    /// USB MIDI clock
    Usb,
    /// MIDI DIN clock
    Midi,
}

/// Sync output options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncOut {
    /// No sync output
    Off,
    /// Send sync via USB MIDI
    Usb,
    /// Send sync via MIDI DIN
    Midi,
}

/// Recording quantization resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecQuantize {
    /// Whole note (1/1)
    Whole,
    /// Half note (1/2)
    Half,
    /// Quarter note (1/4)
    Quarter,
    /// Eighth note (1/8)
    Eighth,
    /// Sixteenth note (1/16)
    Sixteenth,
    /// Thirty-second note (1/32)
    ThirtySecond,
}

/// MIDI settings page - MIDI integration configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MidiSettings {
    /// MIDI Channel: 1-16 or OMNI
    pub midi_channel: MidiChannel,
    /// Local Control: enable/disable internal triggers for DAW controller use
    pub local_control: bool,
    /// PC Out: send MIDI Program Change messages on bank load
    pub pc_out: bool,
    /// CC Tx/Rx: send/receive MIDI Control Change messages
    pub cc_tx_rx: bool,
}

impl MidiSettings {
    /// Validate and fix any invalid values
    pub fn validate_and_fix(&mut self) {
        // Validate MIDI channel
        if let MidiChannel::Channel(ch) = self.midi_channel {
            if ch < 1 || ch > 16 {
                self.midi_channel = MidiChannel::Omni;
            }
        }
    }
}

impl Default for MidiSettings {
    fn default() -> Self {
        Self {
            midi_channel: MidiChannel::Omni,
            local_control: true,  // ON - enable internal triggers
            pc_out: true,         // ON - send Program Change messages
            cc_tx_rx: true,       // ON - send/receive Control Change messages
        }
    }
}

/// MIDI channel configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiChannel {
    /// Specific MIDI channel (1-16)
    Channel(u8),
    /// Respond to all channels
    Omni,
}

/// CONTROL settings page - hardware customization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlSettings {
    /// CTL Func Assign: control function assignment mode
    pub ctl_func_assign: CtlFuncAssign,
    /// Foot SW Assign: footswitch assignment options
    pub foot_sw_assign: [Option<FootSwitchFunction>; 2], // Two footswitch inputs
    /// EXP Pedal Mode: expression pedal behavior
    pub exp_pedal_mode: ExpPedalMode,
}

impl ControlSettings {
    /// Validate and fix any invalid values
    pub fn validate_and_fix(&mut self) {
        // All enum values are validated by serde, no additional validation needed
    }
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            ctl_func_assign: CtlFuncAssign::Panel,
            foot_sw_assign: [Some(FootSwitchFunction::RecPlay), Some(FootSwitchFunction::UndoRedo)],
            exp_pedal_mode: ExpPedalMode::Continuous,
        }
    }
}

/// Control function assignment mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtlFuncAssign {
    /// Use panel button assignments
    Panel,
    /// Assign to undo function
    Undo,
    /// Disable control functions
    Off,
}

/// Footswitch function assignments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FootSwitchFunction {
    /// Record/Play function
    RecPlay,
    /// Memory increment
    MemoryInc,
    /// Memory decrement
    MemoryDec,
    /// Undo/Redo function
    UndoRedo,
    /// Tap tempo
    TapTempo,
    /// All start
    AllStart,
    /// All stop
    AllStop,
    /// Track 1 control
    Track1,
    /// Track 2 control
    Track2,
    /// Track 3 control
    Track3,
    /// Track 4 control
    Track4,
    /// Track 5 control
    Track5,
    /// Track 6 control
    Track6,
}

/// Expression pedal mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpPedalMode {
    /// Continuous control
    Continuous,
    /// Toggle mode
    Toggle,
}

/// UTILITY settings page - system maintenance functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtilitySettings {
    /// Firmware version (read-only)
    pub firmware_version: String<MAX_FIRMWARE_VERSION_LEN>,
    /// Initialize options for selective data clearing
    pub initialize_mode: InitializeMode,
}

impl UtilitySettings {
    /// Perform factory reset - restore all default settings
    pub fn factory_reset(&mut self) {
        // Keep firmware version, reset other settings
        let firmware_version = self.firmware_version.clone();
        *self = Self::default();
        self.firmware_version = firmware_version;
    }

    /// Initialize system based on selected mode
    pub fn initialize(&self, mode: InitializeMode) -> InitializeAction {
        match mode {
            InitializeMode::All => InitializeAction::ClearAll,
            InitializeMode::Settings => InitializeAction::ClearSettings,
            InitializeMode::Loops => InitializeAction::ClearLoops,
            InitializeMode::Memory => InitializeAction::ClearMemory,
        }
    }

    /// Format memory storage
    pub fn format_memory(&self) -> bool {
        // This would trigger memory formatting
        // Return true if formatting should proceed
        true
    }

    /// Validate and fix any invalid values
    pub fn validate_and_fix(&mut self) {
        // Ensure firmware version is not empty
        if self.firmware_version.is_empty() {
            let _ = self.firmware_version.push_str("1.0.0");
        }
    }
}

impl Default for UtilitySettings {
    fn default() -> Self {
        let mut firmware_version = String::new();
        let _ = firmware_version.push_str("1.0.0");
        
        Self {
            firmware_version,
            initialize_mode: InitializeMode::All,
        }
    }
}

/// Initialize mode options for selective data clearing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitializeMode {
    /// Clear all data (loops, settings, memory)
    All,
    /// Clear settings only
    Settings,
    /// Clear loops only
    Loops,
    /// Clear memory slots only
    Memory,
}

/// Initialize action to be performed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitializeAction {
    /// Clear all data
    ClearAll,
    /// Clear settings only
    ClearSettings,
    /// Clear loops only
    ClearLoops,
    /// Clear memory slots only
    ClearMemory,
}

/// BACKUP settings page - data management and backup/restore
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupSettings {
    /// Auto backup enabled
    pub auto_backup_enabled: bool,
    /// Backup interval in minutes
    pub backup_interval_minutes: u16,
    /// Last backup timestamp (simplified as u32 for embedded)
    pub last_backup_time: u32,
    /// USB backup format version
    pub backup_format_version: u8,
}

impl BackupSettings {
    /// Check if backup is due based on interval
    pub fn is_backup_due(&self, current_time: u32) -> bool {
        if !self.auto_backup_enabled {
            return false;
        }
        
        let interval_ms = self.backup_interval_minutes as u32 * 60 * 1000;
        current_time.saturating_sub(self.last_backup_time) >= interval_ms
    }

    /// Update last backup time
    pub fn update_backup_time(&mut self, current_time: u32) {
        self.last_backup_time = current_time;
    }

    /// Save all memory slots and settings to USB drive
    pub fn save_to_usb(&self) -> BackupResult {
        // This would trigger USB backup operation
        BackupResult::Success
    }

    /// Load all memory slots and settings from USB drive
    pub fn load_from_usb(&self) -> BackupResult {
        // This would trigger USB restore operation
        BackupResult::Success
    }

    /// Export projects via RC-505mk2 Manager software
    pub fn export_via_manager(&self) -> BackupResult {
        // This would trigger manager export
        BackupResult::Success
    }

    /// Import projects via RC-505mk2 Manager software
    pub fn import_via_manager(&self) -> BackupResult {
        // This would trigger manager import
        BackupResult::Success
    }

    /// Validate and fix any invalid values
    pub fn validate_and_fix(&mut self) {
        // Ensure backup interval is reasonable (1 minute to 24 hours)
        if self.backup_interval_minutes < 1 {
            self.backup_interval_minutes = 30; // Default 30 minutes
        } else if self.backup_interval_minutes > 1440 {
            self.backup_interval_minutes = 1440; // Max 24 hours
        }
        
        // Ensure backup format version is valid
        if self.backup_format_version == 0 {
            self.backup_format_version = 1;
        }
    }
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            auto_backup_enabled: true,
            backup_interval_minutes: 30, // 30 minutes default
            last_backup_time: 0,
            backup_format_version: 1,
        }
    }
}

/// Backup operation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupResult {
    /// Backup/restore successful
    Success,
    /// USB drive not found
    UsbNotFound,
    /// Insufficient space
    InsufficientSpace,
    /// File format error
    FormatError,
    /// I/O error
    IoError,
    /// Operation cancelled by user
    Cancelled,
}

/// System settings error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsError {
    /// Invalid setting value
    InvalidValue,
    /// Setting not found
    NotFound,
    /// Serialization error
    SerializationError,
    /// Storage error
    StorageError,
    /// Validation error
    ValidationError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_settings_default() {
        let settings = SystemSettings::new();
        
        // Test GENERAL defaults
        assert_eq!(settings.general.tempo_memory, false);
        assert_eq!(settings.general.quantize_mode, QuantizeMode::Full);
        assert_eq!(settings.general.undo_mode, UndoMode::RecordOnly);
        
        // Test CLOCK defaults
        assert_eq!(settings.clock.clock_source, ClockSource::Internal);
        assert_eq!(settings.clock.sync_out, SyncOut::Off);
        
        // Test MIDI defaults
        assert_eq!(settings.midi.midi_channel, MidiChannel::Omni);
        assert_eq!(settings.midi.local_control, true);
        assert_eq!(settings.midi.pc_out, true);
        assert_eq!(settings.midi.cc_tx_rx, true);
    }

    #[test]
    fn test_factory_reset() {
        let mut settings = SystemSettings::new();
        
        // Modify some settings
        settings.general.tempo_memory = true;
        settings.midi.local_control = false;
        
        // Perform factory reset
        settings.factory_reset();
        
        // Verify settings are back to defaults
        assert_eq!(settings.general.tempo_memory, false);
        assert_eq!(settings.midi.local_control, true);
    }

    #[test]
    fn test_midi_channel_validation() {
        let mut settings = MidiSettings::default();
        
        // Test invalid channel
        settings.midi_channel = MidiChannel::Channel(17); // Invalid
        settings.validate_and_fix();
        assert_eq!(settings.midi_channel, MidiChannel::Omni);
        
        // Test valid channel
        settings.midi_channel = MidiChannel::Channel(5); // Valid
        settings.validate_and_fix();
        assert_eq!(settings.midi_channel, MidiChannel::Channel(5));
    }

    #[test]
    fn test_backup_due_check() {
        let mut backup = BackupSettings::default();
        backup.auto_backup_enabled = true;
        backup.backup_interval_minutes = 30;
        backup.last_backup_time = 0;
        
        // Should be due after 30 minutes (1800000 ms)
        assert!(backup.is_backup_due(1800000));
        
        // Should not be due before 30 minutes
        assert!(!backup.is_backup_due(1000000));
        
        // Should not be due if auto backup disabled
        backup.auto_backup_enabled = false;
        assert!(!backup.is_backup_due(1800000));
    }

    #[test]
    fn test_initialize_actions() {
        let utility = UtilitySettings::default();
        
        assert_eq!(utility.initialize(InitializeMode::All), InitializeAction::ClearAll);
        assert_eq!(utility.initialize(InitializeMode::Settings), InitializeAction::ClearSettings);
        assert_eq!(utility.initialize(InitializeMode::Loops), InitializeAction::ClearLoops);
        assert_eq!(utility.initialize(InitializeMode::Memory), InitializeAction::ClearMemory);
    }
}