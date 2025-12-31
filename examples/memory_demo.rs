// examples/memory_demo.rs
//! Comprehensive demonstration of the enhanced memory system
//!
//! This example shows:
//! - Episodic memory (autobiographical events)
//! - Working memory (task management)
//! - Long-term memory (spatial, social, knowledge)
//! - Memory consolidation
//! - Context-based recall
//! - Memory-based decision making

use ebss::core::{
    MemoryManager, EpisodeType, TaskPriority,
    SpatialMemoryType,
};
use uuid::Uuid;

fn main() {
    println!("=== EBSS Memory System Demonstration ===\n");

    let mut memory = MemoryManager::new();
    let mut current_time = 0u64;

    // ===== Part 1: Episodic Memory =====
    println!("--- Part 1: Episodic Memory (Autobiographical Events) ---");

    // Record a discovery event
    let cave_episode = memory.record_event(
        EpisodeType::Discovery,
        "Discovered a dark cave entrance".to_string(),
        0.3, // Slightly positive (curiosity)
        Some((15, 20, 5)),
        vec![],
    );
    println!("✓ Recorded discovery: Dark cave at (15, 20, 5)");

    // Record a social interaction
    let friend_id = Uuid::new_v4();
    let social_episode = memory.record_event(
        EpisodeType::SocialInteraction,
        "Had a great conversation with a friend".to_string(),
        0.8, // Very positive
        Some((10, 10, 0)),
        vec![friend_id],
    );
    println!("✓ Recorded social interaction: Conversation with friend");

    // Record a threatening encounter
    current_time += 100;
    memory.tick(current_time);

    let combat_episode = memory.record_event(
        EpisodeType::Combat,
        "Fought off a wolf attack".to_string(),
        -0.7, // Negative (fear, stress)
        Some((20, 15, 0)),
        vec![],
    );
    println!("✓ Recorded combat: Wolf attack at (20, 15, 0)");

    println!("\nEpisodic Memory Stats:");
    let epi_stats = memory.episodic.stats();
    println!("  Total episodes: {}", epi_stats.total_episodes);
    println!("  Average strength: {:.2}", epi_stats.average_strength);
    println!();

    // ===== Part 2: Working Memory (Task Management) =====
    println!("--- Part 2: Working Memory (Current Tasks) ---");

    // Add tasks with different priorities
    let task1 = memory.add_task(
        "Gather firewood for the night".to_string(),
        TaskPriority::High,
    ).unwrap();
    println!("✓ Added task: Gather firewood (High priority)");

    let task2 = memory.add_task(
        "Explore the cave we found".to_string(),
        TaskPriority::Normal,
    ).unwrap();
    println!("✓ Added task: Explore cave (Normal priority)");

    let task3 = memory.add_task(
        "Find water source".to_string(),
        TaskPriority::Critical,
    ).unwrap();
    println!("✓ Added task: Find water (Critical priority)");

    let task4 = memory.add_task(
        "Craft a better spear".to_string(),
        TaskPriority::Low,
    ).unwrap();
    println!("✓ Added task: Craft spear (Low priority)");

    println!("\nWorking Memory Stats:");
    let wm_stats = memory.working.stats();
    println!("  Total tasks: {}", wm_stats.total_tasks);
    println!("  Pending tasks: {}", wm_stats.pending_tasks);
    println!("  Capacity used: {:.1}%", wm_stats.capacity_used);

    // Show task prioritization
    println!("\nTask Priority Order:");
    let pending = memory.working.pending_tasks();
    for (i, task) in pending.iter().enumerate() {
        println!("  {}. {} ({:?})", i + 1, task.description, task.priority);
    }
    println!();

    // ===== Part 3: Long-term Spatial Memory =====
    println!("--- Part 3: Long-term Spatial Memory ---");

    // Remember important locations
    memory.remember_location(
        SpatialMemoryType::Food,
        (5, 8, 0),
        "Found berry bushes".to_string(),
    );
    println!("✓ Remembered food location: Berry bushes at (5, 8, 0)");

    memory.remember_location(
        SpatialMemoryType::Water,
        (12, 3, 0),
        "Clear stream".to_string(),
    );
    println!("✓ Remembered water location: Stream at (12, 3, 0)");

    memory.remember_location(
        SpatialMemoryType::Shelter,
        (10, 10, 0),
        "Safe cave".to_string(),
    );
    println!("✓ Remembered shelter location: Cave at (10, 10, 0)");

    println!("\nSpatial Memory Stats:");
    println!("  Food locations: {}", memory.long_term.recall_locations(SpatialMemoryType::Food).len());
    println!("  Water locations: {}", memory.long_term.recall_locations(SpatialMemoryType::Water).len());
    println!("  Shelter locations: {}", memory.long_term.recall_locations(SpatialMemoryType::Shelter).len());
    println!();

    // ===== Part 4: Social Memory =====
    println!("--- Part 4: Social Memory (Relationships) ---");

    let ally_id = Uuid::new_v4();
    let stranger_id = Uuid::new_v4();
    let rival_id = Uuid::new_v4();

    // Build relationships
    memory.remember_interaction(
        ally_id,
        true,
        0.9,
        "Ally helped me survive the wolf attack".to_string(),
    );
    println!("✓ Positive interaction: Ally helped in combat");

    memory.remember_interaction(
        stranger_id,
        true,
        0.2,
        "Brief encounter with a stranger".to_string(),
    );
    println!("✓ Neutral interaction: Met stranger");

    memory.remember_interaction(
        rival_id,
        false,
        0.8,
        "Rival stole my food cache".to_string(),
    );
    println!("✓ Negative interaction: Rival stole food");

    println!("\nSocial Memory Stats:");
    println!("  Note: Social relationships are now tracked in Agent.relationships");
    println!("  Episodic memories of interactions are stored in memory.episodic");
    println!();

    // ===== Part 5: Context-Based Recall =====
    println!("--- Part 5: Context-Based Memory Recall ---");

    current_time += 500;
    memory.tick(current_time);

    println!("Agent returns to social gathering location (10, 10, 0)...");
    println!("Recalling relevant memories based on context:\n");

    let recalled = memory.recall_relevant(
        0.5, // Currently in neutral-positive mood
        Some((10, 10, 0)), // At the cave/gathering spot
        &[friend_id], // Friend is present
    );

    println!("Context-triggered memories ({}):", recalled.len());
    for (i, episode) in recalled.iter().enumerate() {
        println!("  {}. {} (valence: {:.2})",
            i + 1,
            episode.description,
            episode.emotional_valence);
    }
    println!();

    // ===== Part 6: Memory Consolidation =====
    println!("--- Part 6: Memory Consolidation ---");

    // Add an important life event
    current_time += 200;
    memory.tick(current_time);

    memory.record_event(
        EpisodeType::LifeEvent,
        "First successful solo hunt".to_string(),
        0.9,
        Some((25, 30, 0)),
        vec![],
    );
    println!("✓ Recorded important life event: First solo hunt");

    // Perform consolidation
    memory.consolidate();
    println!("✓ Performed memory consolidation");

    let stats_after = memory.episodic.stats();
    println!("\nAfter consolidation:");
    println!("  Total episodes: {}", stats_after.total_episodes);
    println!("  Consolidated (long-term): {}", stats_after.consolidated_episodes);
    println!();

    // ===== Part 7: Decision Making Context =====
    println!("--- Part 7: Memory-Based Decision Making ---");

    current_time += 100;
    memory.tick(current_time);

    let context = memory.get_decision_context(Some((20, 15, 0)));

    println!("Decision context analysis:");
    println!("  Recent emotional state: {:.2}", context.recent_emotion);
    println!("  Recent threats: {}", context.recent_threats);
    println!("  Location familiarity: {:.2}", context.location_familiarity);
    println!("  Trusted agents: {}", context.trusted_agents.len());
    println!("  Agents to avoid: {}", context.avoid_agents.len());
    println!("  Pending tasks: {}", context.pending_tasks);

    println!("\nDecision recommendations:");
    if context.should_be_cautious() {
        println!("  ⚠ Be cautious - recent threats detected");
    }
    if context.is_overwhelmed() {
        println!("  ⚠ Overwhelmed - too many pending tasks");
    }
    if context.in_familiar_territory() {
        println!("  ✓ In familiar territory - feel confident");
    }
    if context.is_positive_mood() {
        println!("  ✓ Positive mood - good time for social interaction");
    }
    if context.needs_social_support() {
        println!("  💬 Seek social support from trusted agents");
    }
    println!();

    // ===== Part 8: Experience Checking =====
    println!("--- Part 8: Experience and Association Queries ---");

    println!("Experience check:");
    println!("  Has combat experience? {}", memory.has_similar_experience(EpisodeType::Combat));
    println!("  Has discovery experience? {}", memory.has_similar_experience(EpisodeType::Discovery));
    println!("  Has achievement experience? {}", memory.has_similar_experience(EpisodeType::Achievement));

    println!("\nEmotional associations:");
    println!("  Association with friend: {:.2}", memory.emotional_association_with(friend_id));
    println!("  Association with ally: {:.2}", memory.emotional_association_with(ally_id));
    println!("  Association with rival: {:.2}", memory.emotional_association_with(rival_id));
    println!();

    // ===== Part 9: Memory Over Time =====
    println!("--- Part 9: Memory Decay and Persistence ---");

    println!("Simulating passage of time (2000 ticks)...");
    for _ in 0..2000 {
        current_time += 1;
        memory.tick(current_time);
    }

    let final_stats = memory.episodic.stats();
    println!("\nAfter 2000 ticks:");
    println!("  Episodes retained: {}", final_stats.total_episodes);
    println!("  Average strength: {:.2}", final_stats.average_strength);
    println!("  Consolidated memories: {}", final_stats.consolidated_episodes);

    println!("\nNote: Consolidated memories persist longer!");
    println!("Emotional intensity also affects decay rate.");
    println!();

    // ===== Part 10: Complete System Stats =====
    println!("--- Part 10: Complete Memory System Statistics ---");

    let total_stats = memory.stats();
    println!("Episodic Memory:");
    println!("  Total: {}", total_stats.episodic.total_episodes);
    println!("  Consolidated: {}", total_stats.episodic.consolidated_episodes);
    println!("  Avg strength: {:.2}", total_stats.episodic.average_strength);

    println!("\nWorking Memory:");
    println!("  Total tasks: {}", total_stats.working.total_tasks);
    println!("  Pending: {}", total_stats.working.pending_tasks);
    println!("  Active: {}", total_stats.working.active_tasks);
    println!("  Capacity: {:.1}%", total_stats.working.capacity_used);

    println!("\nLong-term Memory:");
    println!("  Spatial locations: {}", total_stats.spatial_locations);
    println!("  Social relationships: {}", total_stats.social_relationships);
    println!("  Knowledge items: {}", total_stats.knowledge_items);

    println!("\n=== Key Features Demonstrated ===");
    println!("✓ Episodic memory stores autobiographical events");
    println!("✓ Working memory manages current tasks with priorities");
    println!("✓ Long-term memory tracks locations, relationships, knowledge");
    println!("✓ Context-based recall retrieves relevant memories");
    println!("✓ Memory consolidation preserves important experiences");
    println!("✓ Emotional intensity affects memory strength");
    println!("✓ Memory-based decision making provides context");
    println!("✓ Memories decay naturally over time");
    println!("✓ Social relationships tracked with trust/affection");
    println!("✓ Experience queries for behavior learning");

    println!("\n=== Demonstration Complete ===");
}
