use heapless::Vec;
use serde::{Deserialize, Serialize};
use crate::audio::Track;
use crate::effects::EffectChain;

/// Maximum number of memory slots for project storage
pub const MAX_MEMORY_SLOTS: usize = 255;

/// Maximum project name length
pub const MAX_PROJECT_NAME_LEN: usize = 32;

/// Store mode for memory system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreMode {
    /// Save loops + settings (full project)
    Full,
    /// Save only settings (no audio loops)
    SettingOnly,
}

impl Default for StoreMode {
    fn default() -> Self {
        StoreMode::Full
    }
}

/// Rhythm pattern for drum machine functionality
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RhythmPattern {
    /// Pattern name
    pub name: heapless::String<16>,
    /// Beats per measure
    pub beats_per_measure: u8,
    /// Pattern data (simplified for now)
    pub pattern_data: Vec<u8, 64>,
    /// Pattern enabled
    pub enabled: bool,
}

impl Default for RhythmPattern {
    fn default() -> Self {
        let mut name = heapless::String::new();
        let _ = name.push_str("DEFAULT");
        Self {
            name,
            beats_per_measure: 4,
            pattern_data: Vec::new(),
            enabled: false,
        }
    }
}

/// Control assignments for buttons, MIDI, and expression pedals
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlAssignments {
    /// FX button assignments (FX1-5 to effect slots)
    pub fx_button_assignments: [Option<FXButtonAssignment>; 5],
    /// MIDI CC assignments
    pub midi_assignments: Vec<MidiAssignment, 32>,
    /// Expression pedal assignments
    pub expression_assignments: [Option<ExpressionAssignment>; 4], // CTL1,2/EXP1 + CTL3,4/EXP2
    /// Footswitch assignments
    pub footswitch_assignments: [Option<FootswitchAssignment>; 2],
}

impl ControlAssignments {
    /// Create new default control assignments
    pub fn new() -> Self {
        Self {
            fx_button_assignments: [const { None }; 5],
            midi_assignments: Vec::new(),
            expression_assignments: [const { None }; 4],
            footswitch_assignments: [const { None }; 2],
        }
    }
}

impl Default for ControlAssignments {
    fn default() -> Self {
        Self::new()
    }
}

/// FX button assignment configuration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FXButtonAssignment {
    /// Target effect chain type
    pub chain_type: crate::effects::EffectChainType,
    /// Effect slot index (0-3)
    pub slot_index: u8,
    /// Target track (for Track FX only)
    pub target_track: Option<u8>,
    /// Momentary or toggle mode
    pub momentary: bool,
}

/// MIDI CC assignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MidiAssignment {
    /// MIDI CC number
    pub cc_number: u8,
    /// Target parameter
    pub target: MidiTarget,
}

/// MIDI assignment targets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MidiTarget {
    /// Track volume
    TrackVolume(u8),
    /// Track pan
    TrackPan(u8),
    /// Effect parameter
    EffectParameter {
        chain_type: crate::effects::EffectChainType,
        slot_index: u8,
        parameter_index: u8,
        track_id: Option<u8>, // For Track FX
    },
    /// Master volume
    MasterVolume,
    /// Tempo
    Tempo,
}

/// Expression pedal assignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionAssignment {
    /// Target parameter (same as MIDI target)
    pub target: MidiTarget,
    /// Minimum value
    pub min_value: f32,
    /// Maximum value
    pub max_value: f32,
}

/// Footswitch assignment
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FootswitchAssignment {
    /// Record/Play function
    RecPlay(u8), // Track number
    /// Memory increment
    MemoryInc,
    /// Memory decrement
    MemoryDec,
    /// Undo/Redo
    UndoRedo,
    /// Tap tempo
    TapTempo,
    /// All start
    AllStart,
    /// All stop
    AllStop,
}

/// Complete project containing all loopstation state
#[derive(Debug, Clone)]
pub struct Project {
    /// Memory slot number (1-255)
    pub memory_slot: u8,
    /// Project name (editable via RC-505mk2 Manager)
    pub name: heapless::String<MAX_PROJECT_NAME_LEN>,
    /// All 6 tracks with individual Track FX chains
    pub tracks: [Track; 6],
    /// Tempo in BPM
    pub tempo: f32,
    /// Rhythm pattern configuration
    pub rhythm_pattern: RhythmPattern,
    /// Input FX chain (4 slots, pre-recording)
    pub input_fx: EffectChain,
    /// Master FX chain (4 slots, final output)
    pub master_fx: EffectChain,
    /// Control assignments and MIDI mappings
    pub assignments: ControlAssignments,
    /// Creation timestamp (simplified as u32 for embedded)
    pub created: u32,
    /// Last modified timestamp
    pub modified: u32,
    /// Auto-save enabled (VBAT backup)
    pub auto_save_enabled: bool,
    /// Total recording time across all tracks (seconds)
    pub total_recording_time: f32,
    /// MIDI Program Change number for external control
    pub midi_program_change: u8,
}

impl Project {
    /// Create a new empty project
    pub fn new(memory_slot: u8) -> Self {
        Self {
            memory_slot,
            name: {
                let mut name = heapless::String::new();
                let _ = name.push_str("NEW PROJECT");
                name
            },
            tracks: [
                Track::new(1), Track::new(2), Track::new(3),
                Track::new(4), Track::new(5), Track::new(6)
            ],
            tempo: 120.0,
            rhythm_pattern: RhythmPattern::default(),
            input_fx: EffectChain::new_input_fx(),
            master_fx: EffectChain::new_master_fx(),
            assignments: ControlAssignments::new(),
            created: 0, // TODO: Add proper timestamp
            modified: 0,
            auto_save_enabled: true,
            total_recording_time: 0.0,
            midi_program_change: memory_slot.saturating_sub(1), // Memory 1 = PC#0
        }
    }

    /// Update the modified timestamp
    pub fn touch(&mut self) {
        self.modified = 0; // TODO: Add proper timestamp
    }

    /// Calculate total recording time across all tracks
    pub fn calculate_total_recording_time(&mut self, sample_rate: u32) {
        self.total_recording_time = self.tracks
            .iter()
            .map(|track| track.duration_seconds(sample_rate))
            .sum();
    }

    /// Check if project has any recorded audio
    pub fn has_audio(&self) -> bool {
        self.tracks.iter().any(|track| track.has_audio())
    }

    /// Set project name (truncated to max length)
    pub fn set_name(&mut self, name: &str) {
        self.name.clear();
        let truncated = &name[..name.len().min(MAX_PROJECT_NAME_LEN - 1)];
        let _ = self.name.push_str(truncated);
    }

    /// Get project name as string slice
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/// Memory system managing 255 project slots
#[derive(Debug, Clone)]
pub struct MemorySystem {
    /// Memory slots (1-255, index 0 unused)
    pub memory_slots: [Option<Project>; MAX_MEMORY_SLOTS],
    /// Currently selected memory slot (1-255)
    pub current_memory: u8,
    /// Tempo memory enabled (prevent tempo changes on load)
    pub tempo_memory_enabled: bool,
    /// Store mode (full or settings only)
    pub store_mode: StoreMode,
}

impl MemorySystem {
    /// Create a new memory system
    pub fn new() -> Self {
        // Initialize with empty slots
        const INIT: Option<Project> = None;
        let mut memory_slots = [INIT; MAX_MEMORY_SLOTS];
        
        // Create a default project in slot 1
        memory_slots[0] = Some(Project::new(1));

        Self {
            memory_slots,
            current_memory: 1,
            tempo_memory_enabled: false,
            store_mode: StoreMode::default(),
        }
    }

    /// Save current project to specified memory slot
    pub fn save_project(&mut self, slot: u8, project: Project) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        let index = (slot - 1) as usize;
        self.memory_slots[index] = Some(project);
        Ok(())
    }

    /// Load project from specified memory slot
    pub fn load_project(&self, slot: u8) -> Result<&Project, MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        let index = (slot - 1) as usize;
        self.memory_slots[index].as_ref().ok_or(MemoryError::EmptySlot)
    }

    /// Load project mutably from specified memory slot
    pub fn load_project_mut(&mut self, slot: u8) -> Result<&mut Project, MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        let index = (slot - 1) as usize;
        self.memory_slots[index].as_mut().ok_or(MemoryError::EmptySlot)
    }

    /// Get current project
    pub fn current_project(&self) -> Result<&Project, MemoryError> {
        self.load_project(self.current_memory)
    }

    /// Get current project mutably
    pub fn current_project_mut(&mut self) -> Result<&mut Project, MemoryError> {
        self.load_project_mut(self.current_memory)
    }

    /// Switch to specified memory slot
    pub fn switch_to_slot(&mut self, slot: u8) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        self.current_memory = slot;
        Ok(())
    }

    /// Initialize (clear) specified memory slot
    pub fn initialize_slot(&mut self, slot: u8) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        let index = (slot - 1) as usize;
        self.memory_slots[index] = Some(Project::new(slot));
        Ok(())
    }

    /// Check if memory slot is empty
    pub fn is_slot_empty(&self, slot: u8) -> bool {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return true;
        }

        let index = (slot - 1) as usize;
        self.memory_slots[index].is_none()
    }

    /// Get list of used memory slots
    pub fn get_used_slots(&self) -> Vec<u8, MAX_MEMORY_SLOTS> {
        let mut used_slots = Vec::new();
        for (i, slot) in self.memory_slots.iter().enumerate() {
            if slot.is_some() {
                let _ = used_slots.push((i + 1) as u8);
            }
        }
        used_slots
    }

    /// Get memory usage statistics
    pub fn get_memory_usage(&self) -> MemoryUsage {
        let used_slots = self.get_used_slots().len();
        let total_recording_time: f32 = self.memory_slots
            .iter()
            .filter_map(|slot| slot.as_ref())
            .map(|project| project.total_recording_time)
            .sum();

        MemoryUsage {
            used_slots,
            total_slots: MAX_MEMORY_SLOTS,
            total_recording_time,
        }
    }

    /// Set tempo memory mode
    pub fn set_tempo_memory(&mut self, enabled: bool) {
        self.tempo_memory_enabled = enabled;
    }

    /// Set store mode
    pub fn set_store_mode(&mut self, mode: StoreMode) {
        self.store_mode = mode;
    }
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryUsage {
    /// Number of used memory slots
    pub used_slots: usize,
    /// Total available memory slots
    pub total_slots: usize,
    /// Total recording time across all projects
    pub total_recording_time: f32,
}

/// Memory system errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    /// Invalid memory slot number
    InvalidSlot,
    /// Memory slot is empty
    EmptySlot,
    /// Memory is full
    MemoryFull,
    /// Serialization error
    SerializationError,
    /// Storage hardware error
    StorageError,
}

/// Serialization support for projects (basic implementation)
impl Project {
    /// Serialize project to bytes (placeholder implementation)
    pub fn serialize(&self) -> Result<Vec<u8, 4096>, MemoryError> {
        // This is a placeholder - in a real implementation, you would use
        // a proper serialization format like postcard or similar
        // For now, just return an empty vector
        Ok(Vec::new())
    }

    /// Deserialize project from bytes (placeholder implementation)
    pub fn deserialize(_data: &[u8]) -> Result<Self, MemoryError> {
        // This is a placeholder - in a real implementation, you would use
        // a proper deserialization format
        Err(MemoryError::SerializationError)
    }
}