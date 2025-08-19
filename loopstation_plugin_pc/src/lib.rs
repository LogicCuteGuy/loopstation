use loopstation_core_stm32::midi::{MidiMessage, cc_mappings, note_mappings};
use loopstation_core_stm32::{LoopstationCore, MidiHandler};
use nih_plug::prelude::*;
use std::sync::Arc;

pub const NUM_CHANNELS: u32 = 2;
pub const NUM_TRACKS: usize = 6;
pub const NUM_FX_SLOTS: usize = 4;

/// Project state for DAW serialization
#[derive(Debug, Clone, PartialEq)]
struct ProjectState {
    /// Project name
    pub name: String,
    /// Track audio data (simplified - in real implementation would be more complex)
    pub track_data: [Vec<f32>; NUM_TRACKS],
    /// Effect settings
    pub input_fx_settings: Vec<(String, f32)>,
    pub master_fx_settings: Vec<(String, f32)>,
    /// Tempo
    pub tempo: f32,
    /// Version for compatibility
    pub version: u32,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            name: "Untitled Project".to_string(),
            track_data: Default::default(),
            input_fx_settings: Vec::new(),
            master_fx_settings: Vec::new(),
            tempo: 120.0,
            version: 1,
        }
    }
}

struct LoopstationPlugin {
    params: Arc<LoopstationPluginParams>,
    loopstation_core: LoopstationCore,
    /// MIDI handler for processing MIDI input
    midi_handler: MidiHandler,
    /// Project state for DAW serialization
    project_state: ProjectState,
}

#[derive(Params)]
struct LoopstationPluginParams {
    // Track 1 parameters
    #[id = "track_1_volume"]
    pub track_1_volume: FloatParam,
    #[id = "track_1_pan"]
    pub track_1_pan: FloatParam,
    #[id = "track_1_record"]
    pub track_1_record: BoolParam,
    #[id = "track_1_play"]
    pub track_1_play: BoolParam,
    #[id = "track_1_mute"]
    pub track_1_mute: BoolParam,

    // Track 2 parameters
    #[id = "track_2_volume"]
    pub track_2_volume: FloatParam,
    #[id = "track_2_pan"]
    pub track_2_pan: FloatParam,
    #[id = "track_2_record"]
    pub track_2_record: BoolParam,
    #[id = "track_2_play"]
    pub track_2_play: BoolParam,
    #[id = "track_2_mute"]
    pub track_2_mute: BoolParam,

    // Track 3 parameters
    #[id = "track_3_volume"]
    pub track_3_volume: FloatParam,
    #[id = "track_3_pan"]
    pub track_3_pan: FloatParam,
    #[id = "track_3_record"]
    pub track_3_record: BoolParam,
    #[id = "track_3_play"]
    pub track_3_play: BoolParam,
    #[id = "track_3_mute"]
    pub track_3_mute: BoolParam,

    // Track 4 parameters
    #[id = "track_4_volume"]
    pub track_4_volume: FloatParam,
    #[id = "track_4_pan"]
    pub track_4_pan: FloatParam,
    #[id = "track_4_record"]
    pub track_4_record: BoolParam,
    #[id = "track_4_play"]
    pub track_4_play: BoolParam,
    #[id = "track_4_mute"]
    pub track_4_mute: BoolParam,

    // Track 5 parameters
    #[id = "track_5_volume"]
    pub track_5_volume: FloatParam,
    #[id = "track_5_pan"]
    pub track_5_pan: FloatParam,
    #[id = "track_5_record"]
    pub track_5_record: BoolParam,
    #[id = "track_5_play"]
    pub track_5_play: BoolParam,
    #[id = "track_5_mute"]
    pub track_5_mute: BoolParam,

    // Track 6 parameters
    #[id = "track_6_volume"]
    pub track_6_volume: FloatParam,
    #[id = "track_6_pan"]
    pub track_6_pan: FloatParam,
    #[id = "track_6_record"]
    pub track_6_record: BoolParam,
    #[id = "track_6_play"]
    pub track_6_play: BoolParam,
    #[id = "track_6_mute"]
    pub track_6_mute: BoolParam,

    // Global parameters
    #[id = "master_level"]
    pub master_level: FloatParam,

    #[id = "tempo"]
    pub tempo: FloatParam,

    // Transport controls
    #[id = "all_start"]
    pub all_start: BoolParam,
    #[id = "all_stop"]
    pub all_stop: BoolParam,
    #[id = "tap_tempo"]
    pub tap_tempo: BoolParam,

    // Expression pedal inputs
    #[id = "expression_1"]
    pub expression_1: FloatParam,
    #[id = "expression_2"]
    pub expression_2: FloatParam,
    #[id = "expression_3"]
    pub expression_3: FloatParam,
    #[id = "expression_4"]
    pub expression_4: FloatParam,
}

// Removed nested parameter structs for now - will be implemented in task 6.2

impl Default for LoopstationPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(LoopstationPluginParams::default()),
            loopstation_core: LoopstationCore::new(),
            midi_handler: MidiHandler::new(),
            project_state: ProjectState::default(),
        }
    }
}

impl Default for LoopstationPluginParams {
    fn default() -> Self {
        Self {
            // Track 1 parameters
            track_1_volume: Self::create_track_volume_param(1),
            track_1_pan: Self::create_track_pan_param(1),
            track_1_record: BoolParam::new("Track 1 Record", false),
            track_1_play: BoolParam::new("Track 1 Play", false),
            track_1_mute: BoolParam::new("Track 1 Mute", false),

            // Track 2 parameters
            track_2_volume: Self::create_track_volume_param(2),
            track_2_pan: Self::create_track_pan_param(2),
            track_2_record: BoolParam::new("Track 2 Record", false),
            track_2_play: BoolParam::new("Track 2 Play", false),
            track_2_mute: BoolParam::new("Track 2 Mute", false),

            // Track 3 parameters
            track_3_volume: Self::create_track_volume_param(3),
            track_3_pan: Self::create_track_pan_param(3),
            track_3_record: BoolParam::new("Track 3 Record", false),
            track_3_play: BoolParam::new("Track 3 Play", false),
            track_3_mute: BoolParam::new("Track 3 Mute", false),

            // Track 4 parameters
            track_4_volume: Self::create_track_volume_param(4),
            track_4_pan: Self::create_track_pan_param(4),
            track_4_record: BoolParam::new("Track 4 Record", false),
            track_4_play: BoolParam::new("Track 4 Play", false),
            track_4_mute: BoolParam::new("Track 4 Mute", false),

            // Track 5 parameters
            track_5_volume: Self::create_track_volume_param(5),
            track_5_pan: Self::create_track_pan_param(5),
            track_5_record: BoolParam::new("Track 5 Record", false),
            track_5_play: BoolParam::new("Track 5 Play", false),
            track_5_mute: BoolParam::new("Track 5 Mute", false),

            // Track 6 parameters
            track_6_volume: Self::create_track_volume_param(6),
            track_6_pan: Self::create_track_pan_param(6),
            track_6_record: BoolParam::new("Track 6 Record", false),
            track_6_play: BoolParam::new("Track 6 Play", false),
            track_6_mute: BoolParam::new("Track 6 Mute", false),

            // Global parameters
            master_level: FloatParam::new(
                "Master Level",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-60.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-60.0, 12.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            tempo: FloatParam::new(
                "Tempo",
                120.0,
                FloatRange::Linear {
                    min: 60.0,
                    max: 200.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(100.0))
            .with_unit(" BPM")
            .with_step_size(0.1),

            // Transport controls
            all_start: BoolParam::new("All Start", false),
            all_stop: BoolParam::new("All Stop", false),
            tap_tempo: BoolParam::new("Tap Tempo", false),

            // Expression pedal inputs
            expression_1: FloatParam::new(
                "Expression 1",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0)),
            expression_2: FloatParam::new(
                "Expression 2",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0)),
            expression_3: FloatParam::new(
                "Expression 3",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0)),
            expression_4: FloatParam::new(
                "Expression 4",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0)),
        }
    }
}

impl LoopstationPluginParams {
    fn create_track_volume_param(track_id: u8) -> FloatParam {
        FloatParam::new(
            &format!("Track {} Volume", track_id),
            util::db_to_gain(0.0),
            FloatRange::Skewed {
                min: util::db_to_gain(-60.0),
                max: util::db_to_gain(12.0),
                factor: FloatRange::gain_skew_factor(-60.0, 12.0),
            },
        )
        .with_smoother(SmoothingStyle::Logarithmic(50.0))
        .with_unit(" dB")
        .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
        .with_string_to_value(formatters::s2v_f32_gain_to_db())
    }

    fn create_track_pan_param(track_id: u8) -> FloatParam {
        FloatParam::new(
            &format!("Track {} Pan", track_id),
            0.0,
            FloatRange::Linear {
                min: -1.0,
                max: 1.0,
            },
        )
        .with_smoother(SmoothingStyle::Linear(50.0))
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
    }
}

// Removed old helper implementations - simplified for task 6.1

impl LoopstationPlugin {
    /// Update loopstation core with current parameter values
    fn update_loopstation_parameters(&mut self) {
        // Extract parameter values first to avoid borrowing conflicts
        let track_params = [
            (
                self.params.track_1_volume.smoothed.next(),
                self.params.track_1_pan.smoothed.next(),
                self.params.track_1_record.value(),
                self.params.track_1_play.value(),
                self.params.track_1_mute.value(),
            ),
            (
                self.params.track_2_volume.smoothed.next(),
                self.params.track_2_pan.smoothed.next(),
                self.params.track_2_record.value(),
                self.params.track_2_play.value(),
                self.params.track_2_mute.value(),
            ),
            (
                self.params.track_3_volume.smoothed.next(),
                self.params.track_3_pan.smoothed.next(),
                self.params.track_3_record.value(),
                self.params.track_3_play.value(),
                self.params.track_3_mute.value(),
            ),
            (
                self.params.track_4_volume.smoothed.next(),
                self.params.track_4_pan.smoothed.next(),
                self.params.track_4_record.value(),
                self.params.track_4_play.value(),
                self.params.track_4_mute.value(),
            ),
            (
                self.params.track_5_volume.smoothed.next(),
                self.params.track_5_pan.smoothed.next(),
                self.params.track_5_record.value(),
                self.params.track_5_play.value(),
                self.params.track_5_mute.value(),
            ),
            (
                self.params.track_6_volume.smoothed.next(),
                self.params.track_6_pan.smoothed.next(),
                self.params.track_6_record.value(),
                self.params.track_6_play.value(),
                self.params.track_6_mute.value(),
            ),
        ];

        // Update track parameters
        for (track_id, (volume, pan, record, play, mute)) in track_params.iter().enumerate() {
            self.update_track_parameters(
                (track_id + 1) as u8,
                *volume,
                *pan,
                *record,
                *play,
                *mute,
            );
        }

        // Update master level
        let master_level = self.params.master_level.smoothed.next();
        self.loopstation_core.set_master_level(master_level);

        // Update tempo
        let tempo = self.params.tempo.smoothed.next();
        self.project_state.tempo = tempo; // Store in project state
        self.loopstation_core.set_tempo(tempo); // Update core using the appropriate method
    }

    /// Update parameters for a specific track
    fn update_track_parameters(
        &mut self,
        track_id: u8,
        volume: f32,
        pan: f32,
        record: bool,
        play: bool,
        mute: bool,
    ) {
        // Update volume and pan
        if let Err(_) = self.loopstation_core.set_track_level(track_id, volume) {
            // Handle error silently for now
        }

        if let Some(track) = self
            .loopstation_core
            .audio_engine_mut()
            .get_track_mut(track_id)
        {
            track.set_pan(pan, 0); // Using 0 as default timestamp
        }

        // Simple state logic - in a real implementation this would be more sophisticated
        if record && !play {
            let _ = self.loopstation_core.start_recording(track_id);
        } else if play && !record {
            // Start playback if track has content, otherwise start recording
            if let Some(track) = self.loopstation_core.audio_engine().get_track(track_id) {
                if track.has_audio() {
                    // Track has audio, start playback
                    if let Some(track_mut) = self
                        .loopstation_core
                        .audio_engine_mut()
                        .get_track_mut(track_id)
                    {
                        track_mut.state = loopstation_core_stm32::TrackState::Playing;
                    }
                } else {
                    // No audio, start recording
                    let _ = self.loopstation_core.start_recording(track_id);
                }
            }
        } else if !play && !record {
            let _ = self.loopstation_core.stop_track(track_id);
        }

        if mute {
            let _ = self.loopstation_core.toggle_mute(track_id);
        }
    }

    /// Process MIDI input and update parameters accordingly
    fn process_midi_input(&mut self, context: &mut impl ProcessContext<Self>) {
        // Process MIDI events from the context
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn {
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    let midi_data = vec![0x90 | channel, note, (velocity * 127.0) as u8];
                    let messages = self.midi_handler.process_input(&midi_data);
                    for message in messages {
                        self.handle_midi_message(message);
                    }
                }
                NoteEvent::NoteOff {
                    channel,
                    note,
                    velocity,
                    ..
                } => {
                    let midi_data = vec![0x80 | channel, note, (velocity * 127.0) as u8];
                    let messages = self.midi_handler.process_input(&midi_data);
                    for message in messages {
                        self.handle_midi_message(message);
                    }
                }
                NoteEvent::MidiCC {
                    channel, cc, value, ..
                } => {
                    let midi_data = vec![0xB0 | channel, cc, (value * 127.0) as u8];
                    let messages = self.midi_handler.process_input(&midi_data);
                    for message in messages {
                        self.handle_midi_message(message);
                    }
                }
                NoteEvent::MidiProgramChange {
                    channel, program, ..
                } => {
                    let midi_data = vec![0xC0 | channel, program];
                    let messages = self.midi_handler.process_input(&midi_data);
                    for message in messages {
                        self.handle_midi_message(message);
                    }
                }
                _ => {} // Skip other event types for now
            }
        }
    }

    /// Handle individual MIDI messages and map them to loopstation functions
    fn handle_midi_message(&mut self, message: MidiMessage) {
        match message {
            MidiMessage::ControlChange {
                controller, value, ..
            } => {
                self.handle_midi_cc(controller, value);
            }
            MidiMessage::NoteOn { note, velocity, .. } => {
                if velocity > 0 {
                    self.handle_midi_note(note, true);
                }
            }
            MidiMessage::NoteOff { note, .. } => {
                self.handle_midi_note(note, false);
            }
            MidiMessage::ProgramChange { program, .. } => {
                self.handle_midi_program_change(program);
            }
            MidiMessage::Clock => {
                self.midi_handler.process_clock(0); // Using 0 as default timestamp
            }
            _ => {} // Handle other message types as needed
        }
    }

    /// Handle MIDI Control Change messages
    fn handle_midi_cc(&mut self, controller: u8, value: u8) {
        let normalized_value = value as f32 / 127.0;

        match controller {
            // Track volumes - directly control loopstation core
            cc_mappings::TRACK_1_VOLUME => {
                let gain = util::db_to_gain((normalized_value * 72.0) - 60.0);
                let _ = self.loopstation_core.set_track_level(1, gain);
            }
            cc_mappings::TRACK_2_VOLUME => {
                let gain = util::db_to_gain((normalized_value * 72.0) - 60.0);
                let _ = self.loopstation_core.set_track_level(2, gain);
            }
            cc_mappings::TRACK_3_VOLUME => {
                let gain = util::db_to_gain((normalized_value * 72.0) - 60.0);
                let _ = self.loopstation_core.set_track_level(3, gain);
            }
            cc_mappings::TRACK_4_VOLUME => {
                let gain = util::db_to_gain((normalized_value * 72.0) - 60.0);
                let _ = self.loopstation_core.set_track_level(4, gain);
            }
            cc_mappings::TRACK_5_VOLUME => {
                let gain = util::db_to_gain((normalized_value * 72.0) - 60.0);
                let _ = self.loopstation_core.set_track_level(5, gain);
            }
            cc_mappings::TRACK_6_VOLUME => {
                let gain = util::db_to_gain((normalized_value * 72.0) - 60.0);
                let _ = self.loopstation_core.set_track_level(6, gain);
            }

            // Track pans - directly control loopstation core
            cc_mappings::TRACK_1_PAN => {
                if let Some(track) = self.loopstation_core.audio_engine_mut().get_track_mut(1) {
                    track.set_pan((normalized_value * 2.0) - 1.0, 0);
                }
            }
            cc_mappings::TRACK_2_PAN => {
                if let Some(track) = self.loopstation_core.audio_engine_mut().get_track_mut(2) {
                    track.set_pan((normalized_value * 2.0) - 1.0, 0);
                }
            }
            cc_mappings::TRACK_3_PAN => {
                if let Some(track) = self.loopstation_core.audio_engine_mut().get_track_mut(3) {
                    track.set_pan((normalized_value * 2.0) - 1.0, 0);
                }
            }
            cc_mappings::TRACK_4_PAN => {
                if let Some(track) = self.loopstation_core.audio_engine_mut().get_track_mut(4) {
                    track.set_pan((normalized_value * 2.0) - 1.0, 0);
                }
            }
            cc_mappings::TRACK_5_PAN => {
                if let Some(track) = self.loopstation_core.audio_engine_mut().get_track_mut(5) {
                    track.set_pan((normalized_value * 2.0) - 1.0, 0);
                }
            }
            cc_mappings::TRACK_6_PAN => {
                if let Some(track) = self.loopstation_core.audio_engine_mut().get_track_mut(6) {
                    track.set_pan((normalized_value * 2.0) - 1.0, 0);
                }
            }

            // Master volume
            cc_mappings::MASTER_VOLUME => {
                let gain = util::db_to_gain((normalized_value * 72.0) - 60.0);
                self.loopstation_core.set_master_level(gain);
            }

            // Tempo
            cc_mappings::TEMPO => {
                let tempo = 60.0 + (normalized_value * 140.0); // 60-200 BPM range
                self.project_state.tempo = tempo; // Store in project state
                self.loopstation_core.set_tempo(tempo); // Update core using the appropriate method
            }

            // Expression pedals - store in project state for now
            cc_mappings::EXPRESSION_1 => {
                // In a real implementation, this would control assigned parameters
                // For now, just store the value
            }
            cc_mappings::EXPRESSION_2 => {
                // In a real implementation, this would control assigned parameters
            }
            cc_mappings::EXPRESSION_3 => {
                // In a real implementation, this would control assigned parameters
            }
            cc_mappings::EXPRESSION_4 => {
                // In a real implementation, this would control assigned parameters
            }

            _ => {} // Ignore unmapped CCs
        }
    }

    /// Handle MIDI Note messages for track control
    fn handle_midi_note(&mut self, note: u8, note_on: bool) {
        if !note_on {
            return; // Only handle note on events
        }

        match note {
            // Track record/play - directly control loopstation core
            note_mappings::TRACK_1_REC_PLAY => {
                let _ = self.loopstation_core.start_recording(1);
            }
            note_mappings::TRACK_2_REC_PLAY => {
                let _ = self.loopstation_core.start_recording(2);
            }
            note_mappings::TRACK_3_REC_PLAY => {
                let _ = self.loopstation_core.start_recording(3);
            }
            note_mappings::TRACK_4_REC_PLAY => {
                let _ = self.loopstation_core.start_recording(4);
            }
            note_mappings::TRACK_5_REC_PLAY => {
                let _ = self.loopstation_core.start_recording(5);
            }
            note_mappings::TRACK_6_REC_PLAY => {
                let _ = self.loopstation_core.start_recording(6);
            }

            // Track stop
            note_mappings::TRACK_1_STOP => {
                let _ = self.loopstation_core.stop_track(1);
            }
            note_mappings::TRACK_2_STOP => {
                let _ = self.loopstation_core.stop_track(2);
            }
            note_mappings::TRACK_3_STOP => {
                let _ = self.loopstation_core.stop_track(3);
            }
            note_mappings::TRACK_4_STOP => {
                let _ = self.loopstation_core.stop_track(4);
            }
            note_mappings::TRACK_5_STOP => {
                let _ = self.loopstation_core.stop_track(5);
            }
            note_mappings::TRACK_6_STOP => {
                let _ = self.loopstation_core.stop_track(6);
            }

            // Transport control
            note_mappings::ALL_START => {
                // Start all tracks that have audio
                for track_id in 1..=6 {
                    if let Some(track) = self.loopstation_core.audio_engine().get_track(track_id) {
                        if track.has_audio() {
                            if let Some(track_mut) = self
                                .loopstation_core
                                .audio_engine_mut()
                                .get_track_mut(track_id)
                            {
                                track_mut.state = loopstation_core_stm32::TrackState::Playing;
                            }
                        }
                    }
                }
            }
            note_mappings::ALL_STOP => {
                // Stop all tracks
                for track_id in 1..=6 {
                    let _ = self.loopstation_core.stop_track(track_id);
                }
            }
            note_mappings::TAP_TEMPO => {
                // Simple tap tempo implementation - in real implementation would track timing
                // For now, just acknowledge the tap
            }

            _ => {} // Ignore unmapped notes
        }
    }

    /// Handle MIDI Program Change for memory slot switching
    fn handle_midi_program_change(&mut self, program: u8) {
        // Program Change maps to memory slots (PC#0 = Memory 1, etc.)
        let memory_slot = program + 1;

        // In a full implementation, this would load the memory slot
        // For now, we'll just update the project state
        self.project_state.name = format!("Memory Slot {}", memory_slot);

        // TODO: Load actual project data from memory slot
        // This would involve loading track audio data, effect settings, etc.
    }

    /// Handle transport control parameters
    fn handle_transport_controls(&mut self) {
        // Handle all start
        if self.params.all_start.value() {
            for track_id in 1..=6 {
                if let Some(track) = self.loopstation_core.audio_engine().get_track(track_id) {
                    if track.has_audio() {
                        // Start playback on tracks with audio
                        if let Some(track_mut) = self
                            .loopstation_core
                            .audio_engine_mut()
                            .get_track_mut(track_id)
                        {
                            track_mut.state = loopstation_core_stm32::TrackState::Playing;
                        }
                    }
                }
            }
            // Note: Can't reset trigger since parameters are read-only in process context
        }

        // Handle all stop
        if self.params.all_stop.value() {
            for track_id in 1..=6 {
                let _ = self.loopstation_core.stop_track(track_id);
            }
            // Note: Can't reset trigger since parameters are read-only in process context
        }

        // Handle tap tempo
        if self.params.tap_tempo.value() {
            // Simple tap tempo implementation - in real implementation would track timing
            // Note: Can't reset trigger since parameters are read-only in process context
        }
    }
}

impl Plugin for LoopstationPlugin {
    const NAME: &'static str = "LoopStation Plugin";
    const VENDOR: &'static str = "LogicCuteGuy";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "contact@logiccuteguy.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(NUM_CHANNELS),
        main_output_channels: NonZeroU32::new(NUM_CHANNELS),

        aux_input_ports: &[new_nonzero_u32(NUM_CHANNELS); 3],
        aux_output_ports: &[new_nonzero_u32(NUM_CHANNELS); 3],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames {
            layout: Some("Loopstation Audio Layout"),

            main_input: Some("Microphone"),
            // We won't output any sound here
            main_output: Some("Headphones"),
            aux_inputs: &["Microphone2", "Inst1", "Inst2"],
            aux_outputs: &["Main", "Aux1", "Aux2"],
        },
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Initialize loopstation core with proper sample rate and buffer size
        self.loopstation_core = LoopstationCore::new();
        self.loopstation_core.audio_engine.sample_rate = buffer_config.sample_rate as u32;
        self.loopstation_core.audio_engine.buffer_size = buffer_config.max_buffer_size as usize;

        // Start the audio callback
        self.loopstation_core.audio_engine.start_callback();

        true
    }

    fn reset(&mut self) {
        // Reset loopstation core state
        for track in &mut self.loopstation_core.audio_engine.tracks {
            track.stop();
            track.play_position = 0;
            track.record_position = 0;
        }

        // Reset statistics
        self.loopstation_core.audio_engine.stats =
            loopstation_core_stm32::audio::AudioStats::default();

        // Reset MIDI handler
        self.midi_handler = MidiHandler::new();

        // Reset project state
        self.project_state = ProjectState::default();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Process MIDI input first
        self.process_midi_input(context);

        // Handle transport controls
        self.handle_transport_controls();

        // Update loopstation core with current parameter values
        self.update_loopstation_parameters();

        // Process audio through loopstation core
        let num_samples = buffer.samples();
        let num_channels = buffer.channels();

        if num_channels >= 2 && num_samples > 0 {
            // Prepare input buffer (interleaved stereo)
            let mut input_buffer = vec![0.0f32; num_samples * 2];
            let mut output_buffer = vec![0.0f32; num_samples * 2];

            // Convert from nih-plug buffer format to interleaved stereo
            for (sample_idx, mut channel_samples) in buffer.iter_samples().enumerate() {
                let left = *channel_samples.get_mut(0).unwrap_or(&mut 0.0);
                let right = if channel_samples.len() > 1 {
                    *channel_samples.get_mut(1).unwrap_or(&mut 0.0)
                } else {
                    left // Mono to stereo
                };

                input_buffer[sample_idx * 2] = left;
                input_buffer[sample_idx * 2 + 1] = right;
            }

            // Process through loopstation core
            self.loopstation_core
                .process_audio(&input_buffer, &mut output_buffer);

            // Convert back to nih-plug buffer format
            for (sample_idx, mut channel_samples) in buffer.iter_samples().enumerate() {
                let left = output_buffer[sample_idx * 2];
                let right = output_buffer[sample_idx * 2 + 1];

                if let Some(left_channel) = channel_samples.get_mut(0) {
                    *left_channel = left;
                }
                if let Some(right_channel) = channel_samples.get_mut(1) {
                    *right_channel = right;
                }
            }
        } else {
            // Fallback: just apply master level
            for channel_samples in buffer.iter_samples() {
                let master_level = self.params.master_level.smoothed.next();
                for sample in channel_samples {
                    *sample *= master_level;
                }
            }
        }

        // Send MIDI output if needed
        self.send_midi_output(context);

        ProcessStatus::Normal
    }
}

impl LoopstationPlugin {
    /// Send MIDI output events
    fn send_midi_output(&mut self, context: &mut impl ProcessContext<Self>) {
        let output_data = self.midi_handler.get_output_data();

        if !output_data.is_empty() {
            // Convert our MIDI data back to nih-plug events
            let mut i = 0;
            while i < output_data.len() {
                if let Some(message) = MidiMessage::from_bytes(&output_data[i..]) {
                    match message {
                        MidiMessage::ControlChange {
                            channel,
                            controller,
                            value,
                        } => {
                            context.send_event(NoteEvent::MidiCC {
                                timing: 0,
                                channel: (channel - 1) as u8,
                                cc: controller,
                                value: value as f32 / 127.0,
                            });
                            i += 3;
                        }
                        MidiMessage::ProgramChange { channel, program } => {
                            context.send_event(NoteEvent::MidiProgramChange {
                                timing: 0,
                                channel: (channel - 1) as u8,
                                program,
                            });
                            i += 2;
                        }
                        _ => {
                            i += 1; // Skip unknown messages
                        }
                    }
                } else {
                    i += 1;
                }
            }

            // Clear the output buffer after sending
            self.midi_handler.clear_output_buffer();
        }
    }

    /// Send MIDI Program Change when memory slot changes
    pub fn send_memory_slot_change(&mut self, slot: u8) {
        if let Err(_) = self.midi_handler.send_program_change(slot) {
            // Handle error silently for now
        }
    }

    /// Send MIDI CC for parameter changes
    pub fn send_parameter_cc(&mut self, controller: u8, value: f32) {
        let midi_value = (value * 127.0).clamp(0.0, 127.0) as u8;
        if let Err(_) = self
            .midi_handler
            .send_control_change(controller, midi_value)
        {
            // Handle error silently for now
        }
    }

    /// Update project state with current loopstation state
    fn update_project_state(&mut self) {
        // Update tempo - get current parameter value instead of accessing core directly
        self.project_state.tempo = self.params.tempo.value();

        // In a real implementation, we would also update:
        // - Track audio data from loopstation core
        // - Effect settings from effect chains
        // - Other project parameters

        // For now, just sync basic parameters
        // (already done above)
    }

    /// Load project from memory slot (simplified implementation)
    pub fn load_memory_slot(&mut self, slot: u8) {
        // In a real implementation, this would load from persistent storage
        // For now, just create a placeholder project
        self.project_state.name = format!("Memory Slot {}", slot);
        self.project_state.version = 1;

        // Send MIDI Program Change if enabled
        self.send_memory_slot_change(slot);
    }

    /// Save current state to memory slot (simplified implementation)
    pub fn save_memory_slot(&mut self, slot: u8) {
        // Update project state with current settings
        self.update_project_state();

        // In a real implementation, this would save to persistent storage
        // For now, just update the project name
        self.project_state.name = format!("Memory Slot {} - Saved", slot);

        // Send MIDI Program Change if enabled
        self.send_memory_slot_change(slot);
    }

    /// Serialize project state for DAW (simplified implementation)
    pub fn serialize_project_state(&self) -> Vec<u8> {
        // In a real implementation, this would use a proper serialization format
        // For now, just create a simple JSON-like string representation
        let state_json = format!(
            r#"{{"name":"{}","version":{},"tempo":{}}}"#,
            self.project_state.name, self.project_state.version, self.project_state.tempo
        );
        state_json.into_bytes()
    }

    /// Deserialize project state from DAW (simplified implementation)
    pub fn deserialize_project_state(&mut self, data: &[u8]) {
        // In a real implementation, this would use proper deserialization
        // For now, just handle basic parsing
        if let Ok(state_str) = std::str::from_utf8(data) {
            // Very basic parsing - in real implementation would use serde_json
            if let Some(name_start) = state_str.find(r#""name":""#) {
                let name_start = name_start + 8;
                if let Some(name_end) = state_str[name_start..].find('"') {
                    self.project_state.name =
                        state_str[name_start..name_start + name_end].to_string();
                }
            }

            if let Some(tempo_start) = state_str.find(r#""tempo":"#) {
                let tempo_start = tempo_start + 8;
                if let Some(tempo_end) = state_str[tempo_start..].find('}') {
                    if let Ok(tempo) =
                        state_str[tempo_start..tempo_start + tempo_end].parse::<f32>()
                    {
                        self.project_state.tempo = tempo;
                        self.loopstation_core.set_tempo(tempo);
                    }
                }
            }
        }
    }
}

impl ClapPlugin for LoopstationPlugin {
    const CLAP_ID: &'static str = "logiccuteguy.loopstation-plugin";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("clone rc505");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::MultiEffects,
    ];
}

impl Vst3Plugin for LoopstationPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"LoopstationPlugi";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Stereo];
}

nih_export_clap!(LoopstationPlugin);
nih_export_vst3!(LoopstationPlugin);
