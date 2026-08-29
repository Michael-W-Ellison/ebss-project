// src/analytics/events.rs
//! Event callback system for simulation observation.
//!
//! This module provides a flexible event system that allows external code
//! to register callbacks for various simulation events. This enables:
//! - Custom logging and monitoring
//! - Integration with external systems
//! - Real-time notifications
//! - Event-driven architectures

use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Types of events that can be observed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    // Population events
    AgentBorn,
    AgentDied,
    AgentAbandoned,

    // Agent state events
    AgentHealthChanged,
    AgentDriveCritical,
    AgentDriveSatisfied,
    AgentEmotionChanged,

    // Social events
    RelationshipFormed,
    RelationshipBroken,
    SocialInteraction,

    // Learning events
    KnowledgeLearned,
    SkillImproved,
    BehaviorAdopted,

    // World events
    ResourceDepleted,
    ResourceDiscovered,
    BuildingConstructed,
    BuildingDestroyed,

    // Simulation events
    TickCompleted,
    EmergenceDetected,
    MilestoneReached,

    // Custom event type
    Custom(String),
}

/// Data associated with an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Unique event ID
    pub id: Uuid,
    /// Event type
    pub event_type: EventType,
    /// Tick when event occurred
    pub tick: u64,
    /// Primary agent involved (if any)
    pub agent_id: Option<Uuid>,
    /// Secondary agent involved (if any)
    pub secondary_agent_id: Option<Uuid>,
    /// Position where event occurred (if applicable)
    pub position: Option<(i32, i32, i32)>,
    /// Event-specific data as key-value pairs
    pub data: HashMap<String, EventValue>,
    /// Human-readable description
    pub description: String,
    /// Severity/importance (0.0 to 1.0)
    pub severity: f32,
}

/// Value types for event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Uuid(Uuid),
    List(Vec<EventValue>),
}

impl EventData {
    /// Create a new event
    pub fn new(event_type: EventType, tick: u64, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            tick,
            agent_id: None,
            secondary_agent_id: None,
            position: None,
            data: HashMap::new(),
            description,
            severity: 0.5,
        }
    }

    /// Builder: set primary agent
    pub fn with_agent(mut self, agent_id: Uuid) -> Self {
        self.agent_id = Some(agent_id);
        self
    }



    /// Builder: set severity
    pub fn with_severity(mut self, severity: f32) -> Self {
        self.severity = severity.clamp(0.0, 1.0);
        self
    }

    /// Builder: add string data
    pub fn with_string(mut self, key: &str, value: String) -> Self {
        self.data.insert(key.to_string(), EventValue::String(value));
        self
    }

    /// Builder: add integer data
    pub fn with_int(mut self, key: &str, value: i64) -> Self {
        self.data.insert(key.to_string(), EventValue::Integer(value));
        self
    }

    /// Builder: add float data
    pub fn with_float(mut self, key: &str, value: f64) -> Self {
        self.data.insert(key.to_string(), EventValue::Float(value));
        self
    }


    /// Get string value from data
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.data.get(key) {
            Some(EventValue::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }


}

/// Callback function type
pub type EventCallback = Box<dyn Fn(&EventData) + Send + Sync>;

/// Subscription handle for unsubscribing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Filter for event subscriptions
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Only receive specific event types (empty = all)
    pub event_types: Vec<EventType>,
    /// Only receive events for specific agents (empty = all)
    pub agent_ids: Vec<Uuid>,
    /// Minimum severity to receive (0.0 = all)
    pub min_severity: f32,
}

impl EventFilter {
    /// Create filter for all events
    pub fn all() -> Self {
        Self::default()
    }

    /// Create filter for specific event types
    pub fn for_types(types: Vec<EventType>) -> Self {
        Self {
            event_types: types,
            ..Default::default()
        }
    }


    /// Create filter for high-severity events
    pub fn high_severity() -> Self {
        Self {
            min_severity: 0.7,
            ..Default::default()
        }
    }

    /// Check if event passes this filter
    pub fn matches(&self, event: &EventData) -> bool {
        // Check event type
        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type) {
            return false;
        }

        // Check agent IDs
        if !self.agent_ids.is_empty() {
            let matches_agent = event.agent_id.map(|id| self.agent_ids.contains(&id)).unwrap_or(false)
                || event.secondary_agent_id.map(|id| self.agent_ids.contains(&id)).unwrap_or(false);
            if !matches_agent {
                return false;
            }
        }

        // Check severity
        if event.severity < self.min_severity {
            return false;
        }

        true
    }
}

/// Internal subscription data
struct Subscription {
    id: SubscriptionId,
    filter: EventFilter,
    callback: EventCallback,
}

/// Event bus for publishing and subscribing to events
pub struct EventBus {
    subscriptions: RwLock<Vec<Subscription>>,
    /// Event history (optional, for replay)
    history: RwLock<Vec<EventData>>,
    /// Maximum history size (0 = disabled)
    max_history: usize,
    /// Whether to collect history
    collect_history: bool,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(Vec::new()),
            history: RwLock::new(Vec::new()),
            max_history: 0,
            collect_history: false,
        }
    }

    /// Create event bus with history collection enabled
    pub fn with_history(max_size: usize) -> Self {
        Self {
            subscriptions: RwLock::new(Vec::new()),
            history: RwLock::new(Vec::with_capacity(max_size.min(10000))),
            max_history: max_size,
            collect_history: true,
        }
    }

    /// Subscribe to events with a filter
    pub fn subscribe<F>(&self, filter: EventFilter, callback: F) -> SubscriptionId
    where
        F: Fn(&EventData) + Send + Sync + 'static,
    {
        let id = SubscriptionId::new();
        let subscription = Subscription {
            id,
            filter,
            callback: Box::new(callback),
        };

        self.subscriptions.write().unwrap().push(subscription);
        id
    }

    /// Subscribe to all events
    pub fn subscribe_all<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&EventData) + Send + Sync + 'static,
    {
        self.subscribe(EventFilter::all(), callback)
    }

    /// Subscribe to specific event types
    pub fn subscribe_types<F>(&self, types: Vec<EventType>, callback: F) -> SubscriptionId
    where
        F: Fn(&EventData) + Send + Sync + 'static,
    {
        self.subscribe(EventFilter::for_types(types), callback)
    }

    /// Unsubscribe from events
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        let mut subs = self.subscriptions.write().unwrap();
        let len_before = subs.len();
        subs.retain(|s| s.id != id);
        subs.len() < len_before
    }

    /// Publish an event to all matching subscribers
    pub fn publish(&self, event: EventData) {
        // Store in history if enabled
        if self.collect_history {
            let mut history = self.history.write().unwrap();
            if history.len() >= self.max_history && self.max_history > 0 {
                history.remove(0);
            }
            history.push(event.clone());
        }

        // Notify subscribers
        let subs = self.subscriptions.read().unwrap();
        for sub in subs.iter() {
            if sub.filter.matches(&event) {
                (sub.callback)(&event);
            }
        }
    }


    /// Get events of specific type from history
    pub fn get_events_by_type(&self, event_type: &EventType) -> Vec<EventData> {
        self.history
            .read()
            .unwrap()
            .iter()
            .filter(|e| &e.event_type == event_type)
            .cloned()
            .collect()
    }


    /// Get events in tick range from history
    pub fn get_events_in_range(&self, start_tick: u64, end_tick: u64) -> Vec<EventData> {
        self.history
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.tick >= start_tick && e.tick <= end_tick)
            .cloned()
            .collect()
    }



    /// Get history size
    pub fn history_size(&self) -> usize {
        self.history.read().unwrap().len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event emitter trait for types that can emit events
pub trait EventEmitter {
    /// Get the event bus to use
    fn event_bus(&self) -> Option<&EventBus> {
        None
    }

    /// Emit an event (to local bus only, no global)
    fn emit(&self, event: EventData) {
        if let Some(bus) = self.event_bus() {
            bus.publish(event);
        }
    }

    /// Emit agent birth event
    fn emit_agent_born(&self, tick: u64, agent_id: Uuid, parent_ids: Option<(Uuid, Uuid)>) {
        let mut event = EventData::new(
            EventType::AgentBorn,
            tick,
            format!("Agent {} was born", agent_id),
        )
        .with_agent(agent_id)
        .with_severity(0.6);

        if let Some((p1, p2)) = parent_ids {
            event = event
                .with_string("parent1", p1.to_string())
                .with_string("parent2", p2.to_string());
        }

        self.emit(event);
    }

    /// Emit agent death event
    fn emit_agent_died(&self, tick: u64, agent_id: Uuid, cause: &str) {
        self.emit(
            EventData::new(
                EventType::AgentDied,
                tick,
                format!("Agent {} died: {}", agent_id, cause),
            )
            .with_agent(agent_id)
            .with_string("cause", cause.to_string())
            .with_severity(0.8),
        );
    }

    /// Emit drive critical event
    fn emit_drive_critical(&self, tick: u64, agent_id: Uuid, drive: &str, value: f32) {
        self.emit(
            EventData::new(
                EventType::AgentDriveCritical,
                tick,
                format!("Agent {} has critical {} drive: {:.2}", agent_id, drive, value),
            )
            .with_agent(agent_id)
            .with_string("drive", drive.to_string())
            .with_float("value", value as f64)
            .with_severity(0.7),
        );
    }

    /// Emit tick completed event
    fn emit_tick_completed(&self, tick: u64, population: usize) {
        self.emit(
            EventData::new(
                EventType::TickCompleted,
                tick,
                format!("Tick {} completed, population: {}", tick, population),
            )
            .with_int("population", population as i64)
            .with_severity(0.1),
        );
    }

    /// Emit emergence detected event
    fn emit_emergence(&self, tick: u64, pattern: &str, severity: f32) {
        self.emit(
            EventData::new(
                EventType::EmergenceDetected,
                tick,
                format!("Emergence detected: {}", pattern),
            )
            .with_string("pattern", pattern.to_string())
            .with_severity(severity),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_event_creation() {
        let event = EventData::new(EventType::AgentBorn, 100, "Test event".to_string())
            .with_agent(Uuid::new_v4())
            .with_severity(0.8)
            .with_string("test", "value".to_string());

        assert_eq!(event.tick, 100);
        assert_eq!(event.severity, 0.8);
        assert_eq!(event.get_string("test"), Some("value"));
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter::for_types(vec![EventType::AgentBorn, EventType::AgentDied]);

        let birth_event = EventData::new(EventType::AgentBorn, 1, "Birth".to_string());
        let death_event = EventData::new(EventType::AgentDied, 2, "Death".to_string());
        let tick_event = EventData::new(EventType::TickCompleted, 3, "Tick".to_string());

        assert!(filter.matches(&birth_event));
        assert!(filter.matches(&death_event));
        assert!(!filter.matches(&tick_event));
    }

    #[test]
    fn test_severity_filter() {
        let filter = EventFilter::high_severity();

        let low_event = EventData::new(EventType::TickCompleted, 1, "Low".to_string())
            .with_severity(0.3);
        let high_event = EventData::new(EventType::AgentDied, 2, "High".to_string())
            .with_severity(0.9);

        assert!(!filter.matches(&low_event));
        assert!(filter.matches(&high_event));
    }

    #[test]
    fn test_event_bus_subscribe_publish() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let _sub = bus.subscribe_all(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(EventData::new(EventType::TickCompleted, 1, "Test".to_string()));
        bus.publish(EventData::new(EventType::TickCompleted, 2, "Test".to_string()));

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_event_bus_unsubscribe() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let sub_id = bus.subscribe_all(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(EventData::new(EventType::TickCompleted, 1, "Test".to_string()));
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        bus.unsubscribe(sub_id);
        bus.publish(EventData::new(EventType::TickCompleted, 2, "Test".to_string()));
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Still 1, callback not called
    }

    #[test]
    fn test_event_history() {
        let bus = EventBus::with_history(100);

        bus.publish(EventData::new(EventType::AgentBorn, 1, "Birth 1".to_string()));
        bus.publish(EventData::new(EventType::AgentDied, 2, "Death 1".to_string()));
        bus.publish(EventData::new(EventType::AgentBorn, 3, "Birth 2".to_string()));

        assert_eq!(bus.history_size(), 3);

        let births = bus.get_events_by_type(&EventType::AgentBorn);
        assert_eq!(births.len(), 2);

        let range = bus.get_events_in_range(1, 2);
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_filtered_subscription() {
        let bus = EventBus::new();
        let birth_counter = Arc::new(AtomicUsize::new(0));
        let death_counter = Arc::new(AtomicUsize::new(0));

        let birth_clone = birth_counter.clone();
        let death_clone = death_counter.clone();

        bus.subscribe_types(vec![EventType::AgentBorn], move |_| {
            birth_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.subscribe_types(vec![EventType::AgentDied], move |_| {
            death_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(EventData::new(EventType::AgentBorn, 1, "Birth".to_string()));
        bus.publish(EventData::new(EventType::AgentBorn, 2, "Birth".to_string()));
        bus.publish(EventData::new(EventType::AgentDied, 3, "Death".to_string()));
        bus.publish(EventData::new(EventType::TickCompleted, 4, "Tick".to_string()));

        assert_eq!(birth_counter.load(Ordering::SeqCst), 2);
        assert_eq!(death_counter.load(Ordering::SeqCst), 1);
    }
}
