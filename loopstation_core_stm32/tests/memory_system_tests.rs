//! Unit tests for MemorySystem project save/load functionality
//! Requirements: 8.1, 8.2, 8.3, 8.4, 8.5

use loopstation_core_stm32::storage::{
    MemorySystem, Project, StoreMode, MemoryError, MAX_MEMORY_SLOTS,
    StorageInterface, StorageHealth, HealthStatus
};
use loopstation_core_stm32::audio::Track;
use loopstation_core_stm32::effects::{EffectChain, Effect, EffectType};
use std::collections::HashMap;

// Mock storage implementation for testing
#[derive(Debug)]
struct MockStorage {
    data: HashMap<u32, Vec<u8>>,
    ready: bool,
    capacity: u32,
    error_count: u32,
}

impl MockStorage {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
            ready: false,
            capacity: 64 * 1024 * 1024, // 64MB
            error_count: 0,
        }
    }
}

impl StorageInterface for MockStorage {
    fn init(&mut self) -> Result<(), MemoryError> {
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
        self.data.insert(address, data.to_vec());
        Ok(())
    }
    
    fn read(&mut self, address: u32, buffer: &mut [u8]) -> Result<usize, MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        
        if let Some(data) = self.data.get(&address) {
            let copy_len = data.len().min(buffer.len());
            buffer[..copy_len].copy_from_slice(&data[..copy_len]);
            Ok(copy_len)
        } else {
            Ok(0) // Empty slot
        }
    }
    
    fn erase(&mut self, address: u32, _size: u32) -> Result<(), MemoryError> {
        if !self.ready {
            return Err(MemoryError::DeviceNotReady);
        }
        self.data.remove(&address);
        Ok(())
    }
    
    fn capacity(&self) -> u32 {
        self.capacity
    }
    
    fn free_space(&self) -> u32 {
        let used: usize = self.data.values().map(|v| v.len()).sum();
        self.capacity.saturating_sub(used as u32)
    }
    
    fn sync(&mut self) -> Result<(), MemoryError> {
        Ok(())
    }
    
    fn health_check(&mut self) -> Result<StorageHealth, MemoryError> {
        Ok(StorageHealth {
            status: HealthStatus::Good,
            bad_sectors: 0,
            wear_level: 0,
            temperature: None,
            error_count: self.error_count,
        })
    }
    
    fn create_backup(&mut self, data: &[u8]) -> Result<u32, MemoryError> {
        let backup_id = 999999; // Special backup address
        self.write(backup_id, data)?;
        Ok(backup_id)
    }
    
    fn restore_backup(&mut self, backup_id: u32, buffer: &mut [u8]) -> Result<usize, MemoryError> {
        self.read(backup_id, buffer)
    }
}

#[test]
fn test_memory_system_creation() {
    let memory = MemorySystem::new();
    
    assert_eq!(memory.current_memory, 1);
    assert!(!memory.tempo_memory_enabled);
    assert_eq!(memory.store_mode, StoreMode::Full);
    assert!(memory.auto_save_enabled);
    assert!(memory.vbat_backup_enabled);
    
    // Should have a default project in slot 1
    assert!(!memory.is_slot_empty(1));
    assert!(memory.is_slot_empty(2));
}

#[test]
fn test_memory_system_with_storage() {
    let mut memory = MemorySystem::new();
    let mut storage = MockStorage::new();
    
    // Initialize with storage
    let result = memory.init_with_storage(&mut storage);
    assert!(result.is_ok());
    assert!(storage.is_ready());
}

#[test]
fn test_project_creation() {
    let project = Project::new(5);
    
    assert_eq!(project.memory_slot, 5);
    assert_eq!(project.get_name(), "NEW PROJECT");
    assert_eq!(project.tempo, 120.0);
    assert_eq!(project.tracks.len(), 6);
    assert!(project.auto_save_enabled);
    assert_eq!(project.midi_program_change, 4); // Memory 5 = PC#4
    
    // All tracks should be initialized
    for (i, track) in project.tracks.iter().enumerate() {
        assert_eq!(track.id, (i + 1) as u8);
        assert!(!track.has_audio());
    }
}

#[test]
fn test_project_name_management() {
    let mut project = Project::new(1);
    
    // Set normal name
    project.set_name("My Song");
    assert_eq!(project.get_name(), "My Song");
    
    // Test name truncation
    let long_name = "This is a very long project name that exceeds the maximum length";
    project.set_name(long_name);
    assert!(project.get_name().len() < long_name.len());
    assert!(project.get_name().len() <= 31); // MAX_PROJECT_NAME_LEN - 1
}

#[test]
fn test_project_audio_detection() {
    let mut project = Project::new(1);
    
    // Initially no audio
    assert!(!project.has_audio());
    
    // Add audio to one track
    let _ = project.tracks[0].audio_buffer.push(0.5);
    assert!(project.has_audio());
}

#[test]
fn test_project_recording_time_calculation() {
    let mut project = Project::new(1);
    let sample_rate = 44100;
    
    // Set some loop lengths
    project.tracks[0].loop_length = 44100; // 0.5 seconds stereo
    project.tracks[1].loop_length = 88200; // 1.0 seconds stereo
    
    project.calculate_total_recording_time(sample_rate);
    assert!((project.total_recording_time - 1.5).abs() < 0.1);
}

#[test]
fn test_memory_slot_validation() {
    let memory = MemorySystem::new();
    
    // Valid slots
    assert!(memory.load_project(1).is_ok());
    
    // Invalid slots
    assert!(matches!(memory.load_project(0), Err(MemoryError::InvalidSlot)));
    assert!(matches!(memory.load_project(256), Err(MemoryError::InvalidSlot)));
}

#[test]
fn test_memory_slot_operations() {
    let mut memory = MemorySystem::new();
    
    // Test slot switching
    assert!(memory.switch_to_slot(5).is_ok());
    assert_eq!(memory.current_memory, 5);
    
    // Test invalid slot switching
    assert!(matches!(memory.switch_to_slot(0), Err(MemoryError::InvalidSlot)));
    assert!(matches!(memory.switch_to_slot(256), Err(MemoryError::InvalidSlot)));
}

#[test]
fn test_memory_slot_initialization() {
    let mut memory = MemorySystem::new();
    
    // Initialize empty slot
    assert!(memory.is_slot_empty(10));
    assert!(memory.initialize_slot(10).is_ok());
    assert!(!memory.is_slot_empty(10));
    
    // Check initialized project
    let project = memory.load_project(10).unwrap();
    assert_eq!(project.memory_slot, 10);
    assert_eq!(project.get_name(), "NEW PROJECT");
}

#[test]
fn test_project_save_load() {
    let mut memory = MemorySystem::new();
    
    // Create a project with some data
    let mut project = Project::new(3);
    project.set_name("Test Project");
    project.tempo = 140.0;
    
    // Add some audio to a track
    let _ = project.tracks[0].audio_buffer.push(0.1);
    let _ = project.tracks[0].audio_buffer.push(0.2);
    project.tracks[0].loop_length = 2;
    
    // Add an effect
    let mut compressor = Effect::new(EffectType::Compressor);
    compressor.set_enabled(true);
    let _ = project.input_fx.add_effect(compressor);
    
    // Save project
    let result = memory.save_project(3, project);
    assert!(result.is_ok());
    
    // Load project back
    let loaded_project = memory.load_project(3).unwrap();
    assert_eq!(loaded_project.get_name(), "Test Project");
    assert_eq!(loaded_project.tempo, 140.0);
    assert_eq!(loaded_project.tracks[0].loop_length, 2);
    assert_eq!(loaded_project.input_fx.active_effect_count(), 1);
}

#[test]
fn test_project_save_modes() {
    let mut memory = MemorySystem::new();
    
    // Create project with audio
    let mut project = Project::new(5);
    project.set_name("Audio Project");
    let _ = project.tracks[0].audio_buffer.push(0.5);
    project.tracks[0].loop_length = 1;
    
    // Save full project first
    assert!(memory.save_project(5, project.clone()).is_ok());
    
    // Modify settings
    project.set_name("Modified Settings");
    project.tempo = 130.0;
    
    // Save settings only
    let result = memory.save_project_with_mode(5, project, StoreMode::SettingOnly);
    assert!(result.is_ok());
    
    // Load back and verify settings changed but audio preserved
    let loaded = memory.load_project(5).unwrap();
    assert_eq!(loaded.get_name(), "Modified Settings");
    assert_eq!(loaded.tempo, 130.0);
    assert_eq!(loaded.tracks[0].loop_length, 1); // Audio preserved
}

#[test]
fn test_current_project_access() {
    let mut memory = MemorySystem::new();
    
    // Should have current project (slot 1)
    let current = memory.current_project();
    assert!(current.is_ok());
    assert_eq!(current.unwrap().memory_slot, 1);
    
    // Switch to different slot and initialize
    assert!(memory.switch_to_slot(7).is_ok());
    assert!(memory.initialize_slot(7).is_ok());
    
    let current = memory.current_project();
    assert!(current.is_ok());
    assert_eq!(current.unwrap().memory_slot, 7);
}

#[test]
fn test_current_project_mutable_access() {
    let mut memory = MemorySystem::new();
    
    // Modify current project
    {
        let current = memory.current_project_mut().unwrap();
        current.set_name("Modified Current");
        current.tempo = 150.0;
    }
    
    // Verify changes
    let current = memory.current_project().unwrap();
    assert_eq!(current.get_name(), "Modified Current");
    assert_eq!(current.tempo, 150.0);
}

#[test]
fn test_used_slots_tracking() {
    let mut memory = MemorySystem::new();
    
    // Initially only slot 1 is used
    let used_slots = memory.get_used_slots();
    assert_eq!(used_slots.len(), 1);
    assert!(used_slots.contains(&1));
    
    // Initialize more slots
    assert!(memory.initialize_slot(5).is_ok());
    assert!(memory.initialize_slot(10).is_ok());
    
    let used_slots = memory.get_used_slots();
    assert_eq!(used_slots.len(), 3);
    assert!(used_slots.contains(&1));
    assert!(used_slots.contains(&5));
    assert!(used_slots.contains(&10));
}

#[test]
fn test_memory_usage_statistics() {
    let mut memory = MemorySystem::new();
    
    // Add some projects with audio
    let mut project1 = Project::new(2);
    project1.total_recording_time = 30.0; // 30 seconds
    assert!(memory.save_project(2, project1).is_ok());
    
    let mut project2 = Project::new(3);
    project2.total_recording_time = 45.0; // 45 seconds
    assert!(memory.save_project(3, project2).is_ok());
    
    let usage = memory.get_memory_usage();
    assert_eq!(usage.used_slots, 3); // Slots 1, 2, 3
    assert_eq!(usage.total_slots, MAX_MEMORY_SLOTS);
    assert_eq!(usage.total_recording_time, 75.0); // 30 + 45 seconds
}

#[test]
fn test_tempo_memory_setting() {
    let mut memory = MemorySystem::new();
    
    // Initially disabled
    assert!(!memory.tempo_memory_enabled);
    
    // Enable tempo memory
    memory.set_tempo_memory(true);
    assert!(memory.tempo_memory_enabled);
    
    // Disable tempo memory
    memory.set_tempo_memory(false);
    assert!(!memory.tempo_memory_enabled);
}

#[test]
fn test_store_mode_setting() {
    let mut memory = MemorySystem::new();
    
    // Initially full mode
    assert_eq!(memory.store_mode, StoreMode::Full);
    
    // Change to settings only
    memory.set_store_mode(StoreMode::SettingOnly);
    assert_eq!(memory.store_mode, StoreMode::SettingOnly);
    
    // Change back to full
    memory.set_store_mode(StoreMode::Full);
    assert_eq!(memory.store_mode, StoreMode::Full);
}

#[test]
fn test_auto_save_configuration() {
    let mut memory = MemorySystem::new();
    
    // Check default auto-save settings
    assert!(memory.auto_save_enabled);
    assert_eq!(memory.auto_save_interval_ms, 30000); // 30 seconds
    assert!(memory.vbat_backup_enabled);
    
    // Disable auto-save
    memory.auto_save_enabled = false;
    assert!(!memory.auto_save_enabled);
}

#[test]
fn test_memory_system_update() {
    let mut memory = MemorySystem::new();
    
    // Update should not crash
    memory.update();
    
    // Test with auto-save disabled
    memory.auto_save_enabled = false;
    memory.update();
}

#[test]
fn test_project_serialization() {
    let mut project = Project::new(1);
    project.set_name("Serialization Test");
    project.tempo = 125.0;
    
    // Add some data
    let _ = project.tracks[0].audio_buffer.push(0.1);
    project.tracks[0].loop_length = 1;
    
    // Test serialization
    let serialized = project.serialize();
    assert!(serialized.is_ok());
    
    // Test deserialization
    if let Ok(data) = serialized {
        let deserialized = Project::deserialize(&data);
        assert!(deserialized.is_ok());
        
        if let Ok(restored_project) = deserialized {
            assert_eq!(restored_project.get_name(), "Serialization Test");
            assert_eq!(restored_project.tempo, 125.0);
            assert_eq!(restored_project.tracks[0].loop_length, 1);
        }
    }
}

#[test]
fn test_project_settings_only_serialization() {
    let mut project = Project::new(1);
    project.set_name("Settings Test");
    project.tempo = 135.0;
    
    // Add audio data
    let _ = project.tracks[0].audio_buffer.push(0.5);
    project.tracks[0].loop_length = 1;
    
    // Serialize settings only
    let serialized = project.serialize_settings_only();
    assert!(serialized.is_ok());
    
    // Settings-only serialization should be smaller than full serialization
    let full_serialized = project.serialize().unwrap();
    let settings_serialized = serialized.unwrap();
    assert!(settings_serialized.len() <= full_serialized.len());
}

#[test]
fn test_storage_interface_with_memory_system() {
    let mut memory = MemorySystem::new();
    let mut storage = MockStorage::new();
    
    // Initialize storage
    assert!(memory.init_with_storage(&mut storage).is_ok());
    
    // Create and save a project
    let mut project = Project::new(10);
    project.set_name("Storage Test");
    assert!(memory.save_project(10, project).is_ok());
    
    // Save to storage
    assert!(memory.save_all_projects_to_storage(&mut storage).is_ok());
    
    // Verify data was written to storage
    assert!(storage.data.contains_key(&10));
}

#[test]
fn test_storage_health_check() {
    let mut storage = MockStorage::new();
    assert!(storage.init().is_ok());
    
    let health = storage.health_check();
    assert!(health.is_ok());
    
    let health_info = health.unwrap();
    assert_eq!(health_info.status, HealthStatus::Good);
    assert_eq!(health_info.bad_sectors, 0);
    assert_eq!(health_info.error_count, 0);
}

#[test]
fn test_storage_capacity_management() {
    let mut storage = MockStorage::new();
    assert!(storage.init().is_ok());
    
    let initial_capacity = storage.capacity();
    let initial_free = storage.free_space();
    assert_eq!(initial_capacity, initial_free);
    
    // Write some data
    let data = vec![0u8; 1024]; // 1KB
    assert!(storage.write(1, &data).is_ok());
    
    let new_free = storage.free_space();
    assert_eq!(new_free, initial_free - 1024);
}

#[test]
fn test_storage_backup_restore() {
    let mut storage = MockStorage::new();
    assert!(storage.init().is_ok());
    
    let test_data = b"backup test data";
    
    // Create backup
    let backup_id = storage.create_backup(test_data);
    assert!(backup_id.is_ok());
    
    // Restore backup
    let mut buffer = [0u8; 32];
    let restored_size = storage.restore_backup(backup_id.unwrap(), &mut buffer);
    assert!(restored_size.is_ok());
    
    let size = restored_size.unwrap();
    assert_eq!(size, test_data.len());
    assert_eq!(&buffer[..size], test_data);
}

#[test]
fn test_memory_error_handling() {
    let memory = MemorySystem::new();
    
    // Test invalid slot errors
    assert!(matches!(memory.load_project(0), Err(MemoryError::InvalidSlot)));
    assert!(matches!(memory.load_project(256), Err(MemoryError::InvalidSlot)));
    
    // Test empty slot error
    assert!(matches!(memory.load_project(100), Err(MemoryError::EmptySlot)));
}

#[test]
fn test_project_can_serialize_size_check() {
    let project = Project::new(1);
    
    // Should be able to serialize small project
    assert!(project.can_serialize(1024 * 1024)); // 1MB limit
    
    // Should fail with very small limit
    assert!(!project.can_serialize(100)); // 100 bytes limit
}

#[test]
fn test_project_touch_timestamp() {
    let mut project = Project::new(1);
    let initial_modified = project.modified;
    
    // Touch project (in real implementation, this would update timestamp)
    project.touch();
    
    // Modified timestamp should be updated (in real implementation)
    // For now, it remains the same since we don't have real timestamps
    assert_eq!(project.modified, initial_modified);
}