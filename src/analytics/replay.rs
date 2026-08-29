// src/analytics/replay.rs
//! Event replay and recording system for simulation playback.
//!
//! This module provides:
//! - Recording of simulation states at configurable intervals
//! - Playback of recorded states
//! - State snapshots for debugging and analysis
//! - Export/import of recorded sessions

use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// A snapshot of the simulation at a specific tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Tick number when this snapshot was taken
    pub tick: u64,
    /// Timestamp when snapshot was taken (milliseconds since epoch)
    pub timestamp: u64,
    /// Serialized population state
    pub population_state: Vec<AgentSnapshot>,
    /// World state summary
    pub world_state: WorldSnapshot,
    /// Custom metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Snapshot of a single agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub id: Uuid,
    pub position: (i32, i32, i32),
    pub health: f32,
    pub energy: f32,
    pub age: u32,
    pub is_alive: bool,
    /// Drive values (name -> value)
    pub drives: std::collections::HashMap<String, f32>,
    /// Inventory item counts
    pub inventory_items: u32,
    /// Number of relationships
    pub relationship_count: usize,
    /// Current action/behavior (if any)
    pub current_action: Option<String>,
}

/// Snapshot of world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub width: u32,
    pub height: u32,
    /// Number of buildings
    pub building_count: usize,
    /// Resource counts by type
    pub resources: std::collections::HashMap<String, u32>,
    /// Weather/season info
    pub environment: EnvironmentSnapshot,
}

/// Snapshot of environment conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub season: String,
    pub weather: String,
    pub temperature: f32,
    pub time_of_day: String,
}

impl Default for EnvironmentSnapshot {
    fn default() -> Self {
        Self {
            season: "Unknown".to_string(),
            weather: "Clear".to_string(),
            temperature: 20.0,
            time_of_day: "Day".to_string(),
        }
    }
}

impl Default for WorldSnapshot {
    fn default() -> Self {
        Self {
            width: 100,
            height: 100,
            building_count: 0,
            resources: std::collections::HashMap::new(),
            environment: EnvironmentSnapshot::default(),
        }
    }
}

/// Recording configuration
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// Record a snapshot every N ticks
    pub snapshot_interval: u32,
    /// Maximum number of snapshots to keep (0 = unlimited)
    pub max_snapshots: usize,
    /// Whether to compress snapshots
    pub compress: bool,
    /// Whether to include full agent details
    pub detailed_agents: bool,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            snapshot_interval: 10,
            max_snapshots: 1000,
            compress: false,
            detailed_agents: true,
        }
    }
}

impl RecordingConfig {
    /// Create config for minimal recording (less memory)
    pub fn minimal() -> Self {
        Self {
            snapshot_interval: 100,
            max_snapshots: 100,
            compress: true,
            detailed_agents: false,
        }
    }

    /// Create config for detailed recording (more memory)
    pub fn detailed() -> Self {
        Self {
            snapshot_interval: 1,
            max_snapshots: 10000,
            compress: false,
            detailed_agents: true,
        }
    }
}

/// Session recorder for capturing simulation state
pub struct SessionRecorder {
    config: RecordingConfig,
    snapshots: VecDeque<StateSnapshot>,
    recording: bool,
    last_snapshot_tick: u64,
    session_id: Uuid,
    session_name: String,
    start_tick: u64,
}

impl SessionRecorder {
    /// Create a new session recorder
    pub fn new(config: RecordingConfig) -> Self {
        Self {
            config,
            snapshots: VecDeque::new(),
            recording: false,
            last_snapshot_tick: 0,
            session_id: Uuid::new_v4(),
            session_name: "Unnamed Session".to_string(),
            start_tick: 0,
        }
    }

    /// Start recording from a specific tick
    pub fn start_recording(&mut self, tick: u64, name: Option<String>) {
        self.recording = true;
        self.start_tick = tick;
        self.last_snapshot_tick = tick;
        self.session_id = Uuid::new_v4();
        self.session_name = name.unwrap_or_else(|| format!("Session_{}", tick));
        self.snapshots.clear();
    }

    /// Stop recording
    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Check if we should take a snapshot at this tick
    pub fn should_snapshot(&self, tick: u64) -> bool {
        self.recording && (tick - self.last_snapshot_tick) >= self.config.snapshot_interval as u64
    }

    /// Record a snapshot
    pub fn record_snapshot(&mut self, snapshot: StateSnapshot) {
        if !self.recording {
            return;
        }

        self.last_snapshot_tick = snapshot.tick;

        // Enforce max snapshots limit
        if self.config.max_snapshots > 0 && self.snapshots.len() >= self.config.max_snapshots {
            self.snapshots.pop_front();
        }

        self.snapshots.push_back(snapshot);
    }




    /// Get number of recorded snapshots
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Get session info
    pub fn session_info(&self) -> SessionInfo {
        SessionInfo {
            id: self.session_id,
            name: self.session_name.clone(),
            start_tick: self.start_tick,
            end_tick: self.snapshots.back().map(|s| s.tick).unwrap_or(self.start_tick),
            snapshot_count: self.snapshots.len(),
            recording: self.recording,
        }
    }


    /// Load session from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        // Try JSON first, then MessagePack
        let session: RecordedSession = match serde_json::from_reader(reader) {
            Ok(s) => s,
            Err(_) => {
                // Try MessagePack
                let file = File::open(path)?;
                let reader = BufReader::new(file);
                rmp_serde::decode::from_read(reader)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            }
        };

        let last_snapshot_tick = session.snapshots.last().map(|s| s.tick).unwrap_or(0);

        Ok(Self {
            config: session.config,
            snapshots: session.snapshots.into(),
            recording: false,
            last_snapshot_tick,
            session_id: session.id,
            session_name: session.name,
            start_tick: session.start_tick,
        })
    }

    /// Clear all snapshots
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.last_snapshot_tick = 0;
    }
}

/// Serializable recorded session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedSession {
    pub id: Uuid,
    pub name: String,
    pub start_tick: u64,
    pub config: RecordingConfig,
    pub snapshots: Vec<StateSnapshot>,
}

impl RecordedSession {
    /// Get the tick of the last snapshot (for internal use)
    #[allow(dead_code)]
    fn snapshots_end_tick(&self) -> u64 {
        self.snapshots.last().map(|s| s.tick).unwrap_or(0)
    }
}

/// Session metadata
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: Uuid,
    pub name: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub snapshot_count: usize,
    pub recording: bool,
}

/// Playback controller for replaying recorded sessions
pub struct SessionPlayer {
    session: Option<RecordedSession>,
    current_index: usize,
    playback_speed: f32,
    playing: bool,
    loop_playback: bool,
}

impl SessionPlayer {
    /// Create a new session player
    pub fn new() -> Self {
        Self {
            session: None,
            current_index: 0,
            playback_speed: 1.0,
            playing: false,
            loop_playback: false,
        }
    }

    /// Load a session for playback
    pub fn load_session(&mut self, session: RecordedSession) {
        self.session = Some(session);
        self.current_index = 0;
        self.playing = false;
    }

    /// Load from file
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let recorder = SessionRecorder::load_from_file(path)?;
        self.session = Some(RecordedSession {
            id: recorder.session_id,
            name: recorder.session_name,
            start_tick: recorder.start_tick,
            config: recorder.config,
            snapshots: recorder.snapshots.into_iter().collect(),
        });
        self.current_index = 0;
        self.playing = false;
        Ok(())
    }

    /// Start playback
    pub fn play(&mut self) {
        if self.session.is_some() {
            self.playing = true;
        }
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Stop playback and reset to beginning
    pub fn stop(&mut self) {
        self.playing = false;
        self.current_index = 0;
    }

    /// Set playback speed (1.0 = normal, 2.0 = 2x, 0.5 = half speed)
    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed.clamp(0.1, 10.0);
    }


    /// Advance to next frame (returns snapshot if available)
    pub fn next_frame(&mut self) -> Option<&StateSnapshot> {
        let session = self.session.as_ref()?;

        if !self.playing {
            return session.snapshots.get(self.current_index);
        }

        if self.current_index < session.snapshots.len() - 1 {
            self.current_index += 1;
        } else if self.loop_playback {
            self.current_index = 0;
        } else {
            self.playing = false;
        }

        session.snapshots.get(self.current_index)
    }

    /// Go to previous frame
    pub fn prev_frame(&mut self) -> Option<&StateSnapshot> {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
        self.current_snapshot()
    }

    /// Jump to specific frame index
    pub fn goto_frame(&mut self, index: usize) -> Option<&StateSnapshot> {
        if let Some(session) = &self.session {
            self.current_index = index.min(session.snapshots.len().saturating_sub(1));
            return session.snapshots.get(self.current_index);
        }
        None
    }


    /// Get current snapshot without advancing
    pub fn current_snapshot(&self) -> Option<&StateSnapshot> {
        self.session
            .as_ref()
            .and_then(|s| s.snapshots.get(self.current_index))
    }


    /// Get total number of frames
    pub fn total_frames(&self) -> usize {
        self.session
            .as_ref()
            .map(|s| s.snapshots.len())
            .unwrap_or(0)
    }

    /// Get playback progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let total = self.total_frames();
        if total == 0 {
            return 0.0;
        }
        self.current_index as f32 / (total - 1).max(1) as f32
    }

    /// Is currently playing?
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Is session loaded?
    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    /// Get session info
    pub fn session_info(&self) -> Option<SessionInfo> {
        self.session.as_ref().map(|s| SessionInfo {
            id: s.id,
            name: s.name.clone(),
            start_tick: s.start_tick,
            end_tick: s.snapshots.last().map(|snap| snap.tick).unwrap_or(s.start_tick),
            snapshot_count: s.snapshots.len(),
            recording: false,
        })
    }
}

impl Default for SessionPlayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create agent snapshot from agent data
impl AgentSnapshot {
    pub fn new(
        id: Uuid,
        position: (i32, i32, i32),
        health: f32,
        energy: f32,
        age: u32,
        is_alive: bool,
    ) -> Self {
        Self {
            id,
            position,
            health,
            energy,
            age,
            is_alive,
            drives: std::collections::HashMap::new(),
            inventory_items: 0,
            relationship_count: 0,
            current_action: None,
        }
    }

    pub fn with_drive(mut self, name: &str, value: f32) -> Self {
        self.drives.insert(name.to_string(), value);
        self
    }

    pub fn with_inventory(mut self, items: u32) -> Self {
        self.inventory_items = items;
        self
    }


}

/// Helper to create state snapshot
impl StateSnapshot {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            population_state: Vec::new(),
            world_state: WorldSnapshot::default(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_agents(mut self, agents: Vec<AgentSnapshot>) -> Self {
        self.population_state = agents;
        self
    }


    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Get population count
    pub fn population_count(&self) -> usize {
        self.population_state.len()
    }

    /// Get alive agent count
    pub fn alive_count(&self) -> usize {
        self.population_state.iter().filter(|a| a.is_alive).count()
    }

    /// Get average health of alive agents
    pub fn average_health(&self) -> f32 {
        let alive: Vec<_> = self.population_state.iter().filter(|a| a.is_alive).collect();
        if alive.is_empty() {
            return 0.0;
        }
        alive.iter().map(|a| a.health).sum::<f32>() / alive.len() as f32
    }
}

// Implement Serialize/Deserialize for RecordingConfig
impl Serialize for RecordingConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RecordingConfig", 4)?;
        state.serialize_field("snapshot_interval", &self.snapshot_interval)?;
        state.serialize_field("max_snapshots", &self.max_snapshots)?;
        state.serialize_field("compress", &self.compress)?;
        state.serialize_field("detailed_agents", &self.detailed_agents)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RecordingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RecordingConfigHelper {
            snapshot_interval: u32,
            max_snapshots: usize,
            compress: bool,
            detailed_agents: bool,
        }

        let helper = RecordingConfigHelper::deserialize(deserializer)?;
        Ok(RecordingConfig {
            snapshot_interval: helper.snapshot_interval,
            max_snapshots: helper.max_snapshots,
            compress: helper.compress,
            detailed_agents: helper.detailed_agents,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_snapshot_creation() {
        let snapshot = StateSnapshot::new(100)
            .with_metadata("test", "value");

        assert_eq!(snapshot.tick, 100);
        assert_eq!(snapshot.metadata.get("test"), Some(&"value".to_string()));
    }

    #[test]
    fn test_agent_snapshot() {
        let agent = AgentSnapshot::new(
            Uuid::new_v4(),
            (10, 20, 0),
            80.0,
            90.0,
            500,
            true,
        )
        .with_drive("Hunger", 0.3)
        .with_inventory(5);

        assert_eq!(agent.health, 80.0);
        assert_eq!(agent.drives.get("Hunger"), Some(&0.3));
        assert_eq!(agent.inventory_items, 5);
    }

    #[test]
    fn test_session_recorder() {
        let mut recorder = SessionRecorder::new(RecordingConfig::default());

        recorder.start_recording(0, Some("Test Session".to_string()));
        assert!(recorder.is_recording());

        // Record some snapshots
        for tick in (0..100).step_by(10) {
            if recorder.should_snapshot(tick) {
                let snapshot = StateSnapshot::new(tick);
                recorder.record_snapshot(snapshot);
            }
        }

        assert!(recorder.snapshot_count() > 0);

        recorder.stop_recording();
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_session_player() {
        let mut recorder = SessionRecorder::new(RecordingConfig {
            snapshot_interval: 1,
            ..Default::default()
        });

        recorder.start_recording(0, None);
        for tick in 0..10 {
            recorder.record_snapshot(StateSnapshot::new(tick));
        }
        recorder.stop_recording();

        let session = RecordedSession {
            id: recorder.session_id,
            name: recorder.session_name.clone(),
            start_tick: recorder.start_tick,
            config: recorder.config.clone(),
            snapshots: recorder.snapshots.into_iter().collect(),
        };

        let mut player = SessionPlayer::new();
        player.load_session(session);

        assert!(player.has_session());
        assert_eq!(player.total_frames(), 10);

        // Test navigation
        player.play();
        assert!(player.is_playing());

        let first = player.current_snapshot().unwrap();
        assert_eq!(first.tick, 0);

        player.next_frame();
        let second = player.current_snapshot().unwrap();
        assert_eq!(second.tick, 1);

        player.goto_frame(5);
        let sixth = player.current_snapshot().unwrap();
        assert_eq!(sixth.tick, 5);

        player.prev_frame();
        let fifth = player.current_snapshot().unwrap();
        assert_eq!(fifth.tick, 4);
    }

    #[test]
    fn test_snapshot_stats() {
        let agents = vec![
            AgentSnapshot::new(Uuid::new_v4(), (0, 0, 0), 100.0, 100.0, 100, true),
            AgentSnapshot::new(Uuid::new_v4(), (1, 1, 0), 50.0, 80.0, 200, true),
            AgentSnapshot::new(Uuid::new_v4(), (2, 2, 0), 0.0, 0.0, 300, false),
        ];

        let snapshot = StateSnapshot::new(100).with_agents(agents);

        assert_eq!(snapshot.population_count(), 3);
        assert_eq!(snapshot.alive_count(), 2);
        assert_eq!(snapshot.average_health(), 75.0); // (100 + 50) / 2
    }
}
