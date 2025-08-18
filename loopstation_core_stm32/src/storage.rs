use heapless::Vec;
use serde::{Deserialize, Serialize};
use crate::audio::Track;
use crate::effects::EffectChain;

/// Storage abstraction trait for different storage backends
pub trait StorageInterface {
    /// Initialize the storage system
    fn init(&mut self) -> Result<(), MemoryError>;
    
    /// Check if storage is ready for operations
    fn is_ready(&self) -> bool;
    
    /// Write data to storage at specified address/path
    fn write(&mut self, address: u32, data: &[u8]) -> Result<(), MemoryError>;
    
    /// Read data from storage at specified address/path
    fn read(&mut self, address: u32, buffer: &mut [u8]) -> Result<usize, MemoryError>;
    
    /// Erase storage sector/file
    fn erase(&mut self, address: u32, size: u32) -> Result<(), MemoryError>;
    
    /// Get total storage capacity in bytes
    fn capacity(&self) -> u32;
    
    /// Get available free space in bytes
    fn free_space(&self) -> u32;
    
    /// Sync/flush pending writes to storage
    fn sync(&mut self) -> Result<(), MemoryError>;
    
    /// Check storage health and integrity
    fn health_check(&mut self) -> Result<StorageHealth, MemoryError>;
    
    /// Create backup of critical data
    fn create_backup(&mut self, data: &[u8]) -> Result<u32, MemoryError>; // Returns backup ID
    
    /// Restore from backup
    fn restore_backup(&mut self, backup_id: u32, buffer: &mut [u8]) -> Result<usize, MemoryError>;
}

/// Storage health information
#[derive(Debug, Clone, PartialEq)]
pub struct StorageHealth {
    /// Overall health status
    pub status: HealthStatus,
    /// Number of bad sectors/blocks
    pub bad_sectors: u32,
    /// Wear level (0-100%)
    pub wear_level: u8,
    /// Temperature (if available)
    pub temperature: Option<f32>,
    /// Error count since last check
    pub error_count: u32,
}

/// Storage health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Storage is healthy
    Good,
    /// Storage has minor issues but is functional
    Warning,
    /// Storage has serious issues
    Critical,
    /// Storage has failed
    Failed,
}

/// Flash memory storage implementation for STM32
#[cfg(feature = "embedded")]
pub struct FlashStorage {
    /// Base address for project storage
    base_address: u32,
    /// Total storage size
    total_size: u32,
    /// Sector size for erase operations
    sector_size: u32,
    /// Current write position
    write_position: u32,
    /// Storage ready flag
    ready: bool,
    /// Error counter
    error_count: u32,
}

#[cfg(feature = "embedded")]
impl FlashStorage {
    /// Create new flash storage interface
    pub fn new(base_address: u32, total_size: u32, sector_size: u32) -> Self {
        Self {
            base_address,
            total_size,
            sector_size,
            write_position: base_address,
            ready: false,
            error_count: 0,
        }
    }
    
    /// Calculate address for memory slot
    fn slot_address(&self, slot: u8) -> u32 {
        self.base_address + (slot as u32 * 256 * 1024) // 256KB per slot
    }
}

#[cfg(feature = "embedded")]
impl StorageInterface for FlashStorage {
    fn init(&mut self) -> Result<(), MemoryError> {
        // Initialize flash memory controller
        // This would interface with STM32H7 flash controller
        self.ready = true;
        Ok(())
    }
    
    fn is_ready(&self) -> bool {
        self.ready
    }
    
    fn write(&mut self, address: u32, data: &[u8]) -> Result<(), MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        
        // Validate address range
        if address < self.base_address || address + data.len() as u32 > self.base_address + self.total_size {
            return Err(MemoryError::InvalidSlot);
        }
        
        // In real implementation, this would write to flash memory
        // For now, we simulate success
        Ok(())
    }
    
    fn read(&mut self, address: u32, buffer: &mut [u8]) -> Result<usize, MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        
        // Validate address range
        if address < self.base_address {
            return Err(MemoryError::InvalidSlot);
        }
        
        // In real implementation, this would read from flash memory
        // For now, we simulate empty data
        for byte in buffer.iter_mut() {
            *byte = 0xFF; // Flash default value
        }
        
        Ok(buffer.len())
    }
    
    fn erase(&mut self, address: u32, size: u32) -> Result<(), MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        
        // Validate address alignment to sector boundaries
        if address % self.sector_size != 0 {
            return Err(MemoryError::StorageError);
        }
        
        // In real implementation, this would erase flash sectors
        Ok(())
    }
    
    fn capacity(&self) -> u32 {
        self.total_size
    }
    
    fn free_space(&self) -> u32 {
        // Calculate free space based on used slots
        // This is a simplified calculation
        self.total_size.saturating_sub(self.write_position - self.base_address)
    }
    
    fn sync(&mut self) -> Result<(), MemoryError> {
        // Flash memory writes are typically synchronous
        Ok(())
    }
    
    fn health_check(&mut self) -> Result<StorageHealth, MemoryError> {
        // Check flash memory health
        let status = if self.error_count > 100 {
            HealthStatus::Critical
        } else if self.error_count > 10 {
            HealthStatus::Warning
        } else {
            HealthStatus::Good
        };
        
        Ok(StorageHealth {
            status,
            bad_sectors: 0, // Would be detected by flash controller
            wear_level: 0,  // Would be calculated based on erase cycles
            temperature: None,
            error_count: self.error_count,
        })
    }
    
    fn create_backup(&mut self, data: &[u8]) -> Result<u32, MemoryError> {
        // Write to backup sector
        let backup_address = self.base_address + self.total_size - self.sector_size;
        self.write(backup_address, data)?;
        Ok(backup_address)
    }
    
    fn restore_backup(&mut self, backup_id: u32, buffer: &mut [u8]) -> Result<usize, MemoryError> {
        self.read(backup_id, buffer)
    }
}

/// File system storage implementation for PC
#[cfg(not(feature = "embedded"))]
pub struct FileSystemStorage {
    /// Base directory for project storage
    base_path: heapless::String<256>,
    /// Storage ready flag
    ready: bool,
    /// Error counter
    error_count: u32,
}

#[cfg(not(feature = "embedded"))]
impl FileSystemStorage {
    /// Create new file system storage interface
    pub fn new(base_path: &str) -> Self {
        let mut path = heapless::String::new();
        let _ = path.push_str(base_path);
        
        Self {
            base_path: path,
            ready: false,
            error_count: 0,
        }
    }
    
    /// Get file path for memory slot
    fn slot_path(&self, slot: u8) -> heapless::String<512> {
        let mut path = heapless::String::new();
        let _ = path.push_str(&self.base_path);
        let _ = path.push_str("/project_");
        let _ = path.push_str(itoa::Buffer::new().format(slot));
        let _ = path.push_str(".lsp"); // Loopstation Project
        path
    }
}

#[cfg(not(feature = "embedded"))]
impl StorageInterface for FileSystemStorage {
    fn init(&mut self) -> Result<(), MemoryError> {
        // Create base directory if it doesn't exist
        // In real implementation, this would use std::fs
        self.ready = true;
        Ok(())
    }
    
    fn is_ready(&self) -> bool {
        self.ready
    }
    
    fn write(&mut self, slot: u32, data: &[u8]) -> Result<(), MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        
        // In real implementation, this would write to file
        // std::fs::write(self.slot_path(slot as u8), data)
        Ok(())
    }
    
    fn read(&mut self, slot: u32, buffer: &mut [u8]) -> Result<usize, MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        
        // In real implementation, this would read from file
        // let data = std::fs::read(self.slot_path(slot as u8))?;
        // buffer[..data.len().min(buffer.len())].copy_from_slice(&data);
        Ok(0)
    }
    
    fn erase(&mut self, slot: u32, _size: u32) -> Result<(), MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        
        // In real implementation, this would delete the file
        // std::fs::remove_file(self.slot_path(slot as u8))?;
        Ok(())
    }
    
    fn capacity(&self) -> u32 {
        // Return available disk space
        // In real implementation, this would check filesystem capacity
        1024 * 1024 * 1024 // 1GB default
    }
    
    fn free_space(&self) -> u32 {
        // Return available disk space
        // In real implementation, this would check filesystem free space
        512 * 1024 * 1024 // 512MB default
    }
    
    fn sync(&mut self) -> Result<(), MemoryError> {
        // File system sync
        // In real implementation: std::fs::sync_all()?;
        Ok(())
    }
    
    fn health_check(&mut self) -> Result<StorageHealth, MemoryError> {
        // File system health check
        Ok(StorageHealth {
            status: HealthStatus::Good,
            bad_sectors: 0,
            wear_level: 0,
            temperature: None,
            error_count: self.error_count,
        })
    }
    
    fn create_backup(&mut self, data: &[u8]) -> Result<u32, MemoryError> {
        // Create backup file
        // In real implementation, this would write to backup directory
        Ok(0) // Return backup file ID
    }
    
    fn restore_backup(&mut self, backup_id: u32, buffer: &mut [u8]) -> Result<usize, MemoryError> {
        // Restore from backup file
        Ok(0)
    }
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Memory system managing 255 project slots with persistent storage
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
    /// Auto-save enabled flag
    pub auto_save_enabled: bool,
    /// Auto-save interval in milliseconds
    pub auto_save_interval_ms: u32,
    /// Last auto-save timestamp
    pub last_auto_save_time: u32,
    /// VBAT backup protection enabled
    pub vbat_backup_enabled: bool,
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
            auto_save_enabled: true,
            auto_save_interval_ms: 30000, // 30 seconds
            last_auto_save_time: 0,
            vbat_backup_enabled: true,
        }
    }

    /// Initialize memory system with storage interface
    pub fn init_with_storage<S: StorageInterface>(&mut self, storage: &mut S) -> Result<(), MemoryError> {
        // Initialize storage
        storage.init()?;
        
        // Load existing projects from storage
        self.load_all_projects_from_storage(storage)?;
        
        Ok(())
    }

    /// Load all projects from persistent storage
    fn load_all_projects_from_storage<S: StorageInterface>(&mut self, storage: &mut S) -> Result<(), MemoryError> {
        let mut buffer = [0u8; 65536]; // 64KB buffer for reading projects
        
        for slot in 1..=255u8 {
            match storage.read(slot as u32, &mut buffer) {
                Ok(size) if size > 0 => {
                    // Try to deserialize project
                    if let Ok(project) = Project::deserialize(&buffer[..size]) {
                        let index = (slot - 1) as usize;
                        self.memory_slots[index] = Some(project);
                    }
                }
                _ => {
                    // Slot is empty or corrupted, skip
                }
            }
        }
        
        Ok(())
    }

    /// Save all projects to persistent storage
    pub fn save_all_projects_to_storage<S: StorageInterface>(&self, storage: &mut S) -> Result<(), MemoryError> {
        for (index, slot) in self.memory_slots.iter().enumerate() {
            if let Some(project) = slot {
                let slot_number = (index + 1) as u8;
                self.save_project_to_storage(storage, slot_number, project)?;
            }
        }
        
        storage.sync()?;
        Ok(())
    }

    /// Save single project to persistent storage
    fn save_project_to_storage<S: StorageInterface>(&self, storage: &mut S, slot: u8, project: &Project) -> Result<(), MemoryError> {
        let serialized = match self.store_mode {
            StoreMode::Full => project.serialize()?,
            StoreMode::SettingOnly => project.serialize_settings_only()?,
        };
        
        storage.write(slot as u32, &serialized)?;
        Ok(())
    }

    /// Save current project to specified memory slot with serialization
    pub fn save_project(&mut self, slot: u8, mut project: Project) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        // Update project metadata
        project.memory_slot = slot;
        project.touch();
        
        // Check if we can serialize the project
        const MAX_PROJECT_SIZE: usize = 64 * 1024 * 1024; // 64MB max per project
        if !project.can_serialize(MAX_PROJECT_SIZE) {
            return Err(MemoryError::InsufficientSpace);
        }

        // Attempt serialization to validate data integrity
        let _serialized = project.serialize().map_err(|_| MemoryError::SerializationError)?;

        // Save to memory slot
        let index = (slot - 1) as usize;
        self.memory_slots[index] = Some(project);
        
        Ok(())
    }

    /// Save project with specific store mode (full or settings only)
    pub fn save_project_with_mode(&mut self, slot: u8, mut project: Project, store_mode: StoreMode) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        // Update project metadata
        project.memory_slot = slot;
        project.touch();

        match store_mode {
            StoreMode::Full => {
                // Save complete project including audio
                self.save_project(slot, project)
            }
            StoreMode::SettingOnly => {
                // Save only settings, preserve existing audio if any
                if let Some(existing_project) = &mut self.memory_slots[(slot - 1) as usize] {
                    // Preserve audio data from existing project
                    let audio_data: Vec<_, 6> = existing_project.tracks
                        .iter()
                        .map(|track| (track.audio_buffer.clone(), track.loop_length, track.undo_buffer.clone()))
                        .collect();

                    // Update settings
                    existing_project.name = project.name;
                    existing_project.tempo = project.tempo;
                    existing_project.rhythm_pattern = project.rhythm_pattern;
                    existing_project.input_fx = project.input_fx;
                    existing_project.master_fx = project.master_fx;
                    existing_project.assignments = project.assignments;
                    existing_project.modified = project.modified;

                    // Update track settings but preserve audio
                    for (i, track) in existing_project.tracks.iter_mut().enumerate() {
                        let new_track = &project.tracks[i];
                        track.level = new_track.level;
                        track.pan = new_track.pan;
                        track.quantize_enabled = new_track.quantize_enabled;
                        track.input_source = new_track.input_source;
                        track.playback_mode = new_track.playback_mode;
                        track.fade_settings = new_track.fade_settings;
                        track.track_fx = new_track.track_fx.clone();
                        // Audio data is preserved from audio_data
                    }
                } else {
                    // No existing project, save as full project
                    self.save_project(slot, project)?;
                }
                Ok(())
            }
        }
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

    /// Update memory system state (called from main loop)
    pub fn update(&mut self) {
        // Handle auto-save functionality, cleanup, etc.
        // This is where background memory management would occur
        self.handle_auto_save();
        self.cleanup_old_snapshots();
    }

    /// Handle auto-save functionality with VBAT backup protection
    fn handle_auto_save(&mut self) {
        if !self.auto_save_enabled {
            return;
        }

        // In a real implementation, this would get the current system time
        let current_time = 0u32; // Placeholder - would be from system timer
        
        if current_time.saturating_sub(self.last_auto_save_time) >= self.auto_save_interval_ms {
            // Trigger auto-save of current project
            if let Ok(current_project) = self.current_project() {
                if current_project.auto_save_enabled {
                    // Create backup in VBAT memory if enabled
                    if self.vbat_backup_enabled {
                        let _ = self.create_vbat_backup(current_project);
                    }
                }
            }
            self.last_auto_save_time = current_time;
        }
    }

    /// Create VBAT backup of critical project data
    fn create_vbat_backup(&self, project: &Project) -> Result<(), MemoryError> {
        // Create settings-only backup for VBAT memory (limited space)
        let backup_data = project.serialize_settings_only()?;
        
        // In real implementation, this would write to VBAT-backed SRAM
        // For now, this is a placeholder
        Ok(())
    }

    /// Restore from VBAT backup after power loss
    pub fn restore_from_vbat_backup(&mut self) -> Result<(), MemoryError> {
        // In real implementation, this would read from VBAT-backed SRAM
        // and restore the last auto-saved project state
        Ok(())
    }



    /// Export project to WAV format for external use
    pub fn export_project_to_wav<S: StorageInterface>(&self, storage: &mut S, slot: u8, export_path: &str) -> Result<(), MemoryError> {
        let project = self.load_project(slot)?;
        
        // Export each track as individual WAV files
        for (track_index, track) in project.tracks.iter().enumerate() {
            if track.has_audio() {
                let track_filename = "track.wav"; // Simplified for embedded
                self.export_track_to_wav(track, &track_filename, 44100)?;
            }
        }
        
        // Export mixed output as master WAV
        let master_filename = "master_mix.wav"; // Simplified for embedded
        self.export_mixed_audio_to_wav(project, &master_filename, 44100)?;
        
        Ok(())
    }

    /// Export individual track to WAV file
    fn export_track_to_wav(&self, track: &Track, filename: &str, sample_rate: u32) -> Result<(), MemoryError> {
        // Create WAV header
        let wav_header = WavHeader::new(
            track.audio_buffer.len() as u32,
            sample_rate,
            2, // Stereo
            32, // 32-bit float
        );
        
        // In real implementation, this would write to file system or storage
        // For now, we simulate the export process
        Ok(())
    }

    /// Export mixed audio from all tracks to WAV file
    fn export_mixed_audio_to_wav(&self, project: &Project, filename: &str, sample_rate: u32) -> Result<(), MemoryError> {
        // Mix all active tracks
        let mixed_audio = self.mix_project_tracks(project)?;
        
        // Create WAV header for mixed audio
        let wav_header = WavHeader::new(
            mixed_audio.len() as u32,
            sample_rate,
            2, // Stereo
            32, // 32-bit float
        );
        
        // In real implementation, this would write to file system or storage
        Ok(())
    }

    /// Mix all project tracks into a single audio buffer
    fn mix_project_tracks(&self, project: &Project) -> Result<Vec<f32, 1048576>, MemoryError> {
        let mut mixed_buffer = Vec::new();
        
        // Find the longest track to determine mix length
        let max_length = project.tracks
            .iter()
            .map(|track| track.audio_buffer.len())
            .max()
            .unwrap_or(0);
        
        // Initialize mixed buffer with zeros
        for _ in 0..max_length {
            if mixed_buffer.push(0.0).is_err() {
                return Err(MemoryError::InsufficientSpace);
            }
        }
        
        // Mix all tracks
        for track in &project.tracks {
            if track.has_audio() {
                for (i, &sample) in track.audio_buffer.iter().enumerate() {
                    if i < mixed_buffer.len() {
                        mixed_buffer[i] += sample * track.level;
                    }
                }
            }
        }
        
        Ok(mixed_buffer)
    }

    /// Import WAV file into project track
    pub fn import_wav_to_track<S: StorageInterface>(&mut self, storage: &mut S, slot: u8, track_id: u8, wav_path: &str) -> Result<(), MemoryError> {
        if track_id == 0 || track_id > 6 {
            return Err(MemoryError::InvalidSlot);
        }
        
        // Load WAV file data (simplified - in real implementation would parse WAV format)
        let wav_data = self.load_wav_file(storage, wav_path)?;
        
        // Get project and update track
        let project = self.load_project_mut(slot)?;
        let track = &mut project.tracks[(track_id - 1) as usize];
        
        // Clear existing audio and load new data
        track.audio_buffer.clear();
        track.loop_length = wav_data.len() as u32;
        
        // Copy WAV data to track buffer
        for sample in wav_data.iter() {
            if track.audio_buffer.push(*sample).is_err() {
                return Err(MemoryError::InsufficientSpace);
            }
        }
        
        // Update project metadata
        project.touch();
        project.calculate_total_recording_time(44100);
        
        Ok(())
    }

    /// Load WAV file from storage
    fn load_wav_file<S: StorageInterface>(&self, storage: &mut S, wav_path: &str) -> Result<Vec<f32, 1048576>, MemoryError> {
        // In real implementation, this would:
        // 1. Read WAV file header
        // 2. Validate format (44.1/48kHz, 16-24bit, stereo)
        // 3. Convert to 32-bit float
        // 4. Return audio samples
        
        // For now, return empty buffer
        Ok(Vec::new())
    }

    /// Get storage space usage and management information
    pub fn get_storage_info<S: StorageInterface>(&self, storage: &mut S) -> Result<StorageInfo, MemoryError> {
        let health = storage.health_check()?;
        let total_capacity = storage.capacity();
        let free_space = storage.free_space();
        let used_space = total_capacity - free_space;
        
        // Calculate project sizes
        let mut project_sizes = Vec::new();
        for (index, slot) in self.memory_slots.iter().enumerate() {
            if let Some(project) = slot {
                let size = project.estimated_size() as u32;
                let slot_info = ProjectSlotInfo {
                    slot_number: (index + 1) as u8,
                    size_bytes: size,
                    has_audio: project.has_audio(),
                    last_modified: project.modified,
                };
                if project_sizes.push(slot_info).is_err() {
                    break; // Vec is full
                }
            }
        }
        
        Ok(StorageInfo {
            total_capacity,
            used_space,
            free_space,
            health,
            project_slots: project_sizes,
            fragmentation_level: self.calculate_fragmentation_level(used_space, total_capacity),
        })
    }

    /// Calculate storage fragmentation level (0-100%)
    fn calculate_fragmentation_level(&self, used_space: u32, total_capacity: u32) -> u8 {
        // Simplified fragmentation calculation
        // In real implementation, this would analyze actual storage layout
        let usage_percent = (used_space * 100) / total_capacity.max(1);
        
        // Higher usage typically means more fragmentation
        if usage_percent > 90 {
            80
        } else if usage_percent > 70 {
            50
        } else if usage_percent > 50 {
            30
        } else {
            10
        }
    }



    /// Cleanup old backup files
    fn cleanup_old_backups<S: StorageInterface>(&self, storage: &mut S) -> Result<u32, MemoryError> {
        // Remove backups older than a certain threshold
        // Return amount of space freed
        Ok(0)
    }

    /// Create full system backup to external storage
    pub fn create_system_backup<S: StorageInterface>(&self, storage: &mut S, backup_path: &str) -> Result<BackupInfo, MemoryError> {
        let backup_start_time = 0u32; // TODO: Get actual timestamp
        
        // Create backup directory structure
        self.create_backup_directory_structure(backup_path)?;
        
        // Backup all projects
        let mut backed_up_projects = Vec::new();
        for (index, slot) in self.memory_slots.iter().enumerate() {
            if let Some(project) = slot {
                let slot_number = (index + 1) as u8;
                self.backup_project_to_path(project, backup_path, slot_number)?;
                if backed_up_projects.push(slot_number).is_err() {
                    break;
                }
            }
        }
        
        // Create backup manifest
        let backup_info = BackupInfo {
            backup_time: backup_start_time,
            total_projects: backed_up_projects.len() as u8,
            backup_size_bytes: self.calculate_backup_size(),
            projects: backed_up_projects,
        };
        
        self.save_backup_manifest(&backup_info, backup_path)?;
        
        Ok(backup_info)
    }



    /// Helper functions for backup/restore operations
    fn create_backup_directory_structure(&self, backup_path: &str) -> Result<(), MemoryError> {
        // Create necessary directories for backup
        Ok(())
    }

    fn backup_project_to_path(&self, project: &Project, backup_path: &str, slot_number: u8) -> Result<(), MemoryError> {
        // Serialize and save project to backup location
        let serialized = project.serialize()?;
        // Write to backup file
        Ok(())
    }

    fn calculate_backup_size(&self) -> u32 {
        self.memory_slots
            .iter()
            .filter_map(|slot| slot.as_ref())
            .map(|project| project.estimated_size() as u32)
            .sum()
    }

    fn save_backup_manifest(&self, backup_info: &BackupInfo, backup_path: &str) -> Result<(), MemoryError> {
        // Save backup manifest as JSON or binary format
        Ok(())
    }

    fn load_backup_manifest(&self, backup_path: &str) -> Result<BackupInfo, MemoryError> {
        // Load backup manifest
        Ok(BackupInfo {
            backup_time: 0,
            total_projects: 0,
            backup_size_bytes: 0,
            projects: Vec::new(),
        })
    }

    fn restore_project_from_backup<S: StorageInterface>(&mut self, storage: &mut S, backup_path: &str, slot_number: u8) -> Result<(), MemoryError> {
        // Load project from backup and restore to memory slot
        Ok(())
    }
}

/// WAV file header structure
#[derive(Debug, Clone)]
struct WavHeader {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_size: u32,
}

impl WavHeader {
    fn new(data_size: u32, sample_rate: u32, channels: u16, bits_per_sample: u16) -> Self {
        Self {
            sample_rate,
            channels,
            bits_per_sample,
            data_size,
        }
    }
}

/// Storage information and statistics
#[derive(Debug, Clone)]
pub struct StorageInfo {
    /// Total storage capacity in bytes
    pub total_capacity: u32,
    /// Used space in bytes
    pub used_space: u32,
    /// Free space in bytes
    pub free_space: u32,
    /// Storage health information
    pub health: StorageHealth,
    /// Information about each project slot
    pub project_slots: Vec<ProjectSlotInfo, 255>,
    /// Storage fragmentation level (0-100%)
    pub fragmentation_level: u8,
}

/// Information about a project slot
#[derive(Debug, Clone)]
pub struct ProjectSlotInfo {
    /// Slot number (1-255)
    pub slot_number: u8,
    /// Size in bytes
    pub size_bytes: u32,
    /// Whether slot contains audio data
    pub has_audio: bool,
    /// Last modified timestamp
    pub last_modified: u32,
}

/// Storage cleanup options
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    /// Remove empty project slots
    pub remove_empty_slots: bool,
    /// Remove projects with settings only (no audio)
    pub remove_settings_only: bool,
    /// Perform storage defragmentation
    pub defragment_storage: bool,
    /// Remove old backup files
    pub remove_old_backups: bool,
}

impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            remove_empty_slots: true,
            remove_settings_only: false,
            defragment_storage: false,
            remove_old_backups: true,
        }
    }
}

/// Result of storage cleanup operation
#[derive(Debug, Clone)]
pub struct CleanupResult {
    /// Amount of space freed in bytes
    pub freed_space_bytes: u32,
    /// List of cleaned slot numbers
    pub cleaned_slots: Vec<u8, 255>,
    /// Whether defragmentation was performed
    pub defragmentation_performed: bool,
}

/// Backup information
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Backup creation time
    pub backup_time: u32,
    /// Total number of projects backed up
    pub total_projects: u8,
    /// Total backup size in bytes
    pub backup_size_bytes: u32,
    /// List of backed up project slots
    pub projects: Vec<u8, 255>,
}

/// Restore operation result
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// Successfully restored project slots
    pub restored_projects: Vec<u8, 255>,
    /// Failed project slots
    pub failed_projects: Vec<u8, 255>,
    /// Total number of restored projects
    pub total_restored: u8,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// Number of used memory slots
    pub used_slots: usize,
    /// Total available memory slots
    pub total_slots: usize,
    /// Total recording time across all projects (seconds)
    pub total_recording_time: f32,
}

/// Auto-save manager with VBAT backup protection
#[derive(Debug, Clone)]
pub struct AutoSaveManager {
    /// Auto-save enabled flag
    pub enabled: bool,
    /// Auto-save interval in milliseconds
    pub interval_ms: u32,
    /// Last auto-save timestamp
    pub last_save_time: u32,
    /// VBAT backup enabled
    pub vbat_backup_enabled: bool,
    /// Maximum number of auto-save snapshots to keep
    pub max_snapshots: u8,
    /// Current snapshot index (circular buffer)
    pub current_snapshot_index: u8,
}

impl AutoSaveManager {
    /// Create new auto-save manager
    pub fn new() -> Self {
        Self {
            enabled: true,
            interval_ms: 30000, // 30 seconds default
            vbat_backup_enabled: true,
            max_snapshots: 5,
            current_snapshot_index: 0,
            last_save_time: 0,
        }
    }

    /// Update auto-save manager (called from main loop)
    pub fn update<S: StorageInterface>(&mut self, storage: &mut S, memory_system: &MemorySystem, current_time: u32) -> Result<(), MemoryError> {
        if !self.enabled {
            return Ok(());
        }

        // Check if it's time for auto-save
        if current_time.saturating_sub(self.last_save_time) >= self.interval_ms {
            self.perform_auto_save(storage, memory_system, current_time)?;
            self.last_save_time = current_time;
        }

        Ok(())
    }

    /// Perform auto-save operation
    fn perform_auto_save<S: StorageInterface>(&mut self, storage: &mut S, memory_system: &MemorySystem, current_time: u32) -> Result<(), MemoryError> {
        // Get current project
        if let Ok(current_project) = memory_system.current_project() {
            if current_project.auto_save_enabled {
                // Create auto-save snapshot
                self.create_auto_save_snapshot(storage, current_project, current_time)?;
                
                // Create VBAT backup if enabled
                if self.vbat_backup_enabled {
                    self.create_vbat_backup(storage, current_project)?;
                }
            }
        }

        Ok(())
    }

    /// Create auto-save snapshot
    fn create_auto_save_snapshot<S: StorageInterface>(&mut self, storage: &mut S, project: &Project, timestamp: u32) -> Result<(), MemoryError> {
        // Serialize project (settings only for auto-save to save space)
        let snapshot_data = project.serialize_settings_only()?;
        
        // Calculate snapshot address (use circular buffer)
        let snapshot_address = self.get_snapshot_address(self.current_snapshot_index);
        
        // Write snapshot to storage
        storage.write(snapshot_address, &snapshot_data)?;
        
        // Update snapshot index
        self.current_snapshot_index = (self.current_snapshot_index + 1) % self.max_snapshots;
        
        Ok(())
    }

    /// Create VBAT backup for power loss protection
    fn create_vbat_backup<S: StorageInterface>(&self, storage: &mut S, project: &Project) -> Result<(), MemoryError> {
        // Create minimal backup for VBAT memory (limited space)
        let backup_data = project.serialize_settings_only()?;
        
        // Use special VBAT backup address
        let _vbat_address = 0xFFFF0000u32; // Special address for VBAT backup
        storage.create_backup(&backup_data)?;
        
        Ok(())
    }

    /// Get storage address for snapshot
    fn get_snapshot_address(&self, snapshot_index: u8) -> u32 {
        // Auto-save snapshots use special address range
        0xFFF00000 + (snapshot_index as u32 * 0x10000) // 64KB per snapshot
    }

    /// Restore from auto-save snapshot
    pub fn restore_from_snapshot<S: StorageInterface>(&self, storage: &mut S, snapshot_index: u8) -> Result<Project, MemoryError> {
        let snapshot_address = self.get_snapshot_address(snapshot_index);
        let mut buffer = [0u8; 8192]; // Buffer for settings-only data
        
        let size = storage.read(snapshot_address, &mut buffer)?;
        if size > 0 {
            Project::deserialize(&buffer[..size])
        } else {
            Err(MemoryError::EmptySlot)
        }
    }

    /// Restore from VBAT backup after power loss
    pub fn restore_from_vbat_backup<S: StorageInterface>(&self, storage: &mut S) -> Result<Project, MemoryError> {
        let vbat_address = 0xFFFF0000;
        let mut buffer = [0u8; 8192];
        
        let size = storage.restore_backup(vbat_address, &mut buffer)?;
        if size > 0 {
            Project::deserialize(&buffer[..size])
        } else {
            Err(MemoryError::EmptySlot)
        }
    }

    /// Set auto-save interval
    pub fn set_interval(&mut self, interval_ms: u32) {
        self.interval_ms = interval_ms;
    }

    /// Enable/disable auto-save
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Enable/disable VBAT backup
    pub fn set_vbat_backup_enabled(&mut self, enabled: bool) {
        self.vbat_backup_enabled = enabled;
    }
}

impl Default for AutoSaveManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced memory system with auto-save and storage management
impl MemorySystem {
    /// Initialize memory system with auto-save manager
    pub fn init_with_auto_save<S: StorageInterface>(&mut self, storage: &mut S, auto_save_manager: &mut AutoSaveManager) -> Result<(), MemoryError> {
        // Initialize storage
        storage.init()?;
        
        // Load existing projects
        self.load_all_projects_from_storage(storage)?;
        
        // Try to restore from VBAT backup if available
        if auto_save_manager.vbat_backup_enabled {
            if let Ok(backup_project) = auto_save_manager.restore_from_vbat_backup(storage) {
                // Restore the backed up project to current slot
                self.memory_slots[(self.current_memory - 1) as usize] = Some(backup_project);
            }
        }
        
        Ok(())
    }

    /// Update memory system with auto-save support
    pub fn update_with_auto_save<S: StorageInterface>(&mut self, storage: &mut S, auto_save_manager: &mut AutoSaveManager, current_time: u32) -> Result<(), MemoryError> {
        // Handle auto-save
        auto_save_manager.update(storage, self, current_time)?;
        
        // Handle regular memory management
        self.update();
        
        Ok(())
    }

    /// Force immediate auto-save
    pub fn force_auto_save<S: StorageInterface>(&self, storage: &mut S, auto_save_manager: &mut AutoSaveManager, current_time: u32) -> Result<(), MemoryError> {
        if let Ok(current_project) = self.current_project() {
            auto_save_manager.create_auto_save_snapshot(storage, current_project, current_time)?;
            
            if auto_save_manager.vbat_backup_enabled {
                auto_save_manager.create_vbat_backup(storage, current_project)?;
            }
        }
        
        Ok(())
    }
}

/// Enhanced memory system implementation
impl MemorySystem {
    /// Clean up old undo snapshots to manage memory usage
    fn cleanup_old_snapshots(&mut self) {
        const MAX_UNDO_LEVELS_CLEANUP: usize = 16; // Reduced from 32 to save memory
        
        for slot in &mut self.memory_slots {
            if let Some(project) = slot {
                for track in &mut project.tracks {
                    // Keep only the most recent undo snapshots
                    if track.undo_buffer.len() > MAX_UNDO_LEVELS_CLEANUP {
                        // Remove oldest snapshots
                        let excess = track.undo_buffer.len() - MAX_UNDO_LEVELS_CLEANUP;
                        for _ in 0..excess {
                            track.undo_buffer.remove(0);
                        }
                    }
                }
            }
        }
    }

    /// Validate project data integrity
    pub fn validate_project(&self, slot: u8) -> Result<bool, MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        let index = (slot - 1) as usize;
        if let Some(project) = &self.memory_slots[index] {
            // Attempt to serialize and deserialize to check integrity
            match project.serialize() {
                Ok(serialized) => {
                    match Project::deserialize(&serialized) {
                        Ok(_) => Ok(true),
                        Err(_) => Ok(false),
                    }
                }
                Err(_) => Ok(false),
            }
        } else {
            Err(MemoryError::EmptySlot)
        }
    }

    /// Recover corrupted project from backup if available
    pub fn recover_project(&mut self, slot: u8) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        // In a real implementation, this would attempt to recover from:
        // 1. VBAT backup memory
        // 2. Flash memory backup sectors
        // 3. SD card backup
        // For now, we'll initialize a new project
        
        self.initialize_slot(slot)?;
        Ok(())
    }

    /// Create backup of critical project data
    pub fn create_backup(&self, slot: u8) -> Result<Vec<u8, 65536>, MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        let index = (slot - 1) as usize;
        if let Some(project) = &self.memory_slots[index] {
            // Create settings-only backup for critical data
            project.serialize_settings_only()
        } else {
            Err(MemoryError::EmptySlot)
        }
    }

    /// Restore project from backup data
    pub fn restore_from_backup(&mut self, slot: u8, backup_data: &[u8]) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        // Deserialize backup data
        let mut project = Project::deserialize(backup_data)?;
        project.memory_slot = slot;

        // Save restored project
        let index = (slot - 1) as usize;
        self.memory_slots[index] = Some(project);
        
        Ok(())
    }

    /// Get storage usage statistics with detailed breakdown
    pub fn get_detailed_usage(&self) -> DetailedMemoryUsage {
        let mut total_audio_size = 0usize;
        let mut total_settings_size = 0usize;
        let mut used_slots = 0usize;
        let mut corrupted_slots = Vec::new();

        for (i, slot) in self.memory_slots.iter().enumerate() {
            if let Some(project) = slot {
                used_slots += 1;
                
                // Calculate audio data size
                let audio_size: usize = project.tracks
                    .iter()
                    .map(|track| track.audio_buffer.len() * 4) // 4 bytes per f32 sample
                    .sum();
                total_audio_size += audio_size;

                // Estimate settings size
                if let Ok(settings_data) = project.serialize_settings_only() {
                    total_settings_size += settings_data.len();
                }

                // Check for corruption
                if let Err(_) = project.serialize() {
                    let _ = corrupted_slots.push((i + 1) as u8);
                }
            }
        }

        DetailedMemoryUsage {
            used_slots,
            total_slots: MAX_MEMORY_SLOTS,
            total_audio_size,
            total_settings_size,
            corrupted_slots,
            total_recording_time: self.get_memory_usage().total_recording_time,
        }
    }

    /// Export project with WAV format support
    pub fn export_project(&self, slot: u8, include_audio: bool) -> Result<ProjectExport, MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        let index = (slot - 1) as usize;
        if let Some(project) = &self.memory_slots[index] {
            let mut export = ProjectExport {
                project_data: project.serialize_settings_only()?,
                audio_tracks: Vec::new(),
                metadata: ExportMetadata {
                    version: 1,
                    created: project.created,
                    modified: project.modified,
                    name: project.name.clone(),
                    sample_rate: 44100, // Standard sample rate
                    includes_audio: include_audio,
                },
            };

            if include_audio {
                // Export each track as WAV data
                for track in &project.tracks {
                    if track.has_audio() {
                        let wav_data = track.export_wav_data(44100)?;
                        let _ = export.audio_tracks.push(TrackExport {
                            track_id: track.id,
                            wav_data,
                        });
                    }
                }
            }

            Ok(export)
        } else {
            Err(MemoryError::EmptySlot)
        }
    }

    /// Import project from export data
    pub fn import_project(&mut self, slot: u8, export: ProjectExport) -> Result<(), MemoryError> {
        if slot == 0 || slot as usize > MAX_MEMORY_SLOTS {
            return Err(MemoryError::InvalidSlot);
        }

        // Deserialize project settings
        let mut project = Project::deserialize(&export.project_data)?;
        project.memory_slot = slot;

        // Import audio tracks if available
        if export.metadata.includes_audio {
            for track_export in &export.audio_tracks {
                if let Some(track) = project.tracks.get_mut((track_export.track_id - 1) as usize) {
                    track.import_wav_data(&track_export.wav_data)?;
                }
            }
        }

        // Save imported project
        let index = (slot - 1) as usize;
        self.memory_slots[index] = Some(project);

        Ok(())
    }

    /// Create storage space management and cleanup functionality
    pub fn cleanup_storage<S: StorageInterface>(&mut self, storage: &mut S) -> Result<CleanupReport, MemoryError> {
        let mut report = CleanupReport {
            slots_cleaned: 0,
            space_freed: 0,
            corrupted_slots_fixed: 0,
            errors: Vec::new(),
        };

        // Check storage health
        let health = storage.health_check()?;
        if health.status == HealthStatus::Critical || health.status == HealthStatus::Failed {
            let _ = report.errors.push(CleanupError::StorageFailure);
            return Ok(report);
        }

        // Clean up corrupted slots
        for slot in 1..=255u8 {
            if let Err(_) = self.validate_project(slot) {
                // Try to recover from backup
                if let Ok(_) = self.recover_project(slot) {
                    report.corrupted_slots_fixed += 1;
                } else {
                    // Clear corrupted slot
                    self.initialize_slot(slot)?;
                    report.slots_cleaned += 1;
                }
            }
        }

        // Optimize storage layout (defragmentation)
        self.defragment_storage(storage, &mut report)?;

        Ok(report)
    }

    /// Defragment storage to optimize space usage
    fn defragment_storage<S: StorageInterface>(&mut self, storage: &mut S, report: &mut CleanupReport) -> Result<(), MemoryError> {
        // In a real implementation, this would reorganize storage to eliminate fragmentation
        // For now, we'll just sync the storage
        storage.sync()?;
        Ok(())
    }
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::new()
    }
}



/// Project export data structure
#[derive(Debug, Clone)]
pub struct ProjectExport {
    /// Serialized project settings
    pub project_data: Vec<u8, 65536>,
    /// Audio track data
    pub audio_tracks: Vec<TrackExport, 6>,
    /// Export metadata
    pub metadata: ExportMetadata,
}

/// Individual track export data
#[derive(Debug, Clone)]
pub struct TrackExport {
    /// Track ID (1-6)
    pub track_id: u8,
    /// WAV format audio data
    pub wav_data: Vec<u8, 65536>, // 64KB max per track for embedded
}

/// Export metadata
#[derive(Debug, Clone)]
pub struct ExportMetadata {
    /// Export format version
    pub version: u8,
    /// Project creation timestamp
    pub created: u32,
    /// Project modification timestamp
    pub modified: u32,
    /// Project name
    pub name: heapless::String<32>,
    /// Sample rate
    pub sample_rate: u32,
    /// Whether audio data is included
    pub includes_audio: bool,
}

/// Storage cleanup report
#[derive(Debug, Clone)]
pub struct CleanupReport {
    /// Number of slots cleaned
    pub slots_cleaned: u8,
    /// Amount of space freed in bytes
    pub space_freed: u32,
    /// Number of corrupted slots fixed
    pub corrupted_slots_fixed: u8,
    /// Cleanup errors encountered
    pub errors: Vec<CleanupError, 16>,
}

/// Cleanup error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupError {
    /// Storage device failure
    StorageFailure,
    /// Insufficient space for cleanup
    InsufficientSpace,
    /// Corrupted data cannot be recovered
    UnrecoverableCorruption,
    /// Write protection prevents cleanup
    WriteProtected,
}

/// Maximum number of undo levels per track
const MAX_UNDO_LEVELS: usize = 10;

/// Detailed memory usage statistics with breakdown
#[derive(Debug, Clone, PartialEq)]
pub struct DetailedMemoryUsage {
    /// Number of used memory slots
    pub used_slots: usize,
    /// Total available memory slots
    pub total_slots: usize,
    /// Total size of audio data in bytes
    pub total_audio_size: usize,
    /// Total size of settings data in bytes
    pub total_settings_size: usize,
    /// List of corrupted memory slots
    pub corrupted_slots: Vec<u8, 32>,
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
    /// Corrupted data detected
    CorruptedData,
    /// Insufficient storage space
    InsufficientSpace,
    /// Write protection enabled
    WriteProtected,
    /// Storage device not ready
    DeviceNotReady,
}

impl From<&str> for MemoryError {
    fn from(_: &str) -> Self {
        MemoryError::StorageError
    }
}

/// Serialization support for projects using postcard
impl Project {
    /// Serialize project to bytes using postcard format
    /// Returns a heapless Vec with maximum size for embedded systems
    pub fn serialize(&self) -> Result<Vec<u8, 65536>, MemoryError> {
        // Use postcard to serialize the project
        let mut buffer = Vec::new();
        
        match postcard::to_vec::<Project, 65536>(self) {
            Ok(serialized) => {
                // Copy serialized data to our heapless Vec
                for byte in serialized.iter() {
                    if buffer.push(*byte).is_err() {
                        return Err(MemoryError::SerializationError);
                    }
                }
                Ok(buffer)
            }
            Err(_) => Err(MemoryError::SerializationError),
        }
    }

    /// Deserialize project from bytes using postcard format
    pub fn deserialize(data: &[u8]) -> Result<Self, MemoryError> {
        postcard::from_bytes(data).map_err(|_| MemoryError::SerializationError)
    }

    /// Serialize only settings (no audio data) for faster saves
    pub fn serialize_settings_only(&self) -> Result<Vec<u8, 65536>, MemoryError> {
        // Create a copy of the project with empty audio buffers
        let mut settings_only = self.clone();
        
        // Clear audio buffers from all tracks
        for track in &mut settings_only.tracks {
            track.audio_buffer.clear();
            track.loop_length = 0;
            track.play_position = 0;
            track.record_position = 0;
            track.undo_buffer.clear();
        }
        
        // Serialize the settings-only version
        let mut buffer = Vec::new();
        
        match postcard::to_vec::<Project, 65536>(&settings_only) {
            Ok(serialized) => {
                for byte in serialized.iter() {
                    if buffer.push(*byte).is_err() {
                        return Err(MemoryError::SerializationError);
                    }
                }
                Ok(buffer)
            }
            Err(_) => Err(MemoryError::SerializationError),
        }
    }

    /// Get estimated serialized size in bytes
    pub fn estimated_size(&self) -> usize {
        // Base project data (settings, metadata)
        let base_size = 1024; // Estimated size for all non-audio data
        
        // Audio data size (each sample is 4 bytes for f32)
        let audio_size: usize = self.tracks
            .iter()
            .map(|track| track.audio_buffer.len() * 4)
            .sum();
        
        base_size + audio_size
    }

    /// Check if project can be serialized within size limits
    pub fn can_serialize(&self, max_size: usize) -> bool {
        self.estimated_size() <= max_size
    }
}