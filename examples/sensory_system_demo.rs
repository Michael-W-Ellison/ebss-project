// examples/sensory_system_demo.rs
//! Demonstration of the comprehensive sensory system including:
//! - Vision, hearing, and speech
//! - Smell and scent detection
//! - Attention and focus mechanisms
//! - Sensory memory
//! - Percept processing and salience calculation

use ebss::agents::{Agent, AgentConfig, Percept};
use ebss::agents::senses::{Scent, ScentType, Sound, SoundType};
use ebss::core::DriveType;
use uuid::Uuid;

fn main() {
    println!("=== EBSS Sensory System Demonstration ===\n");

    // Create an agent
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (50, 50, 0);

    println!("Agent created at position {:?}\n", agent.state.position);

    // === Part 1: Smell Detection ===
    println!("--- Part 1: Smell Detection ---");

    // Add some scents to the environment
    agent.senses.smell.detect_scent(Scent {
        source_position: (55, 52, 0),
        scent_type: ScentType::Food,
        strength: 0.8,
        age: 0,
    });

    agent.senses.smell.detect_scent(Scent {
        source_position: (48, 48, 0),
        scent_type: ScentType::Water,
        strength: 0.6,
        age: 0,
    });

    agent.senses.smell.detect_scent(Scent {
        source_position: (60, 50, 0),
        scent_type: ScentType::Blood,
        strength: 0.9,
        age: 0,
    });

    println!("Detected {} scents:", agent.senses.smell.detected_scents.len());
    for (i, scent) in agent.senses.smell.detected_scents.iter().enumerate() {
        println!("  {}. {:?} at {:?} (strength: {:.2})",
            i + 1, scent.scent_type, scent.source_position, scent.strength);
    }

    // Test smell-based resource finding
    if let Some(food_pos) = agent.senses.find_food_source(agent.state.position) {
        println!("\nFood source detected at: {:?}", food_pos);
    }

    if let Some(water_pos) = agent.senses.find_water_source(agent.state.position) {
        println!("Water source detected at: {:?}", water_pos);
    }

    println!("\nDanger detected: {}", agent.senses.senses_danger());
    println!("Threat level: {}", agent.senses.threat_level());

    // === Part 2: Hearing and Vision ===
    println!("\n--- Part 2: Vision and Hearing ---");

    // Add visible agents
    let agent_1 = Uuid::new_v4();
    let agent_2 = Uuid::new_v4();
    agent.senses.vision.visible_agents.insert(agent_1);
    agent.senses.vision.visible_agents.insert(agent_2);

    println!("Visible agents: {}", agent.senses.vision.visible_agents.len());

    // Add sounds
    agent.senses.hearing.hear_sound(Sound {
        source_position: (52, 51, 0),
        sound_type: SoundType::Speech,
        loudness: 0.5,
        age: 0,
    });

    agent.senses.hearing.hear_sound(Sound {
        source_position: (45, 50, 0),
        sound_type: SoundType::Combat,
        loudness: 0.8,
        age: 0,
    });

    println!("Heard sounds: {}", agent.senses.hearing.heard_sounds.len());
    for sound in &agent.senses.hearing.heard_sounds {
        println!("  {:?} at {:?} (loudness: {:.2})",
            sound.sound_type, sound.source_position, sound.loudness);
    }

    // === Part 3: Sensory Memory ===
    println!("\n--- Part 3: Sensory Memory ---");

    agent.senses.memory.remember_agent(agent_1, (52, 51, 0));
    agent.senses.memory.remember_agent(agent_2, (53, 50, 0));
    agent.senses.memory.remember_position((55, 52, 0), "Food cache".to_string());

    println!("Agents in memory: {}", agent.senses.memory.seen_agents.len());
    println!("Locations in memory: {}", agent.senses.memory.seen_positions.len());

    if let Some(pos) = agent.senses.memory.get_agent_position(agent_1) {
        println!("Last known position of agent 1: {:?}", pos);
    }

    // === Part 4: Attention System ===
    println!("\n--- Part 4: Attention and Focus ---");

    use ebss::agents::senses::Focus;
    agent.senses.attention.focus_on(Focus::Agent(agent_1));

    println!("Current focus: {:?}", agent.senses.attention.focus);
    println!("Attention span: {} ticks", agent.senses.attention.attention_span);
    println!("Distractibility: {:.2}", agent.senses.attention.distractibility);

    // Simulate some ticks
    for _ in 0..50 {
        agent.senses.attention.tick();
    }
    println!("After 50 ticks - Still focused: {}", agent.senses.attention.focus.is_some());

    // === Part 5: Percept Processing ===
    println!("\n--- Part 5: Percept Processing ---");

    // Set agent drives to affect salience
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.9; // Very hungry
    }
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.3; // Slightly thirsty
    }

    println!("Agent drives:");
    println!("  Hunger: {:.2}", agent.drives.get(DriveType::Hunger).unwrap().value);
    println!("  Thirst: {:.2}", agent.drives.get(DriveType::Thirst).unwrap().value);

    // Process percepts
    let percepts = agent.process_percepts();
    println!("\nTotal percepts detected: {}", percepts.len());

    for (i, percept) in percepts.iter().enumerate() {
        let salience = agent.percept_salience(percept);
        println!("  {}. {:?} (salience: {:.2})", i + 1, percept, salience);
    }

    // Get most salient percept
    if let Some(most_salient) = agent.most_salient_percept() {
        let salience = agent.percept_salience(&most_salient);
        println!("\nMost salient percept (salience: {:.2}):", salience);
        println!("  {:?}", most_salient);
    }

    // Filter by salience threshold
    let important_percepts = agent.filter_percepts_by_salience(0.7);
    println!("\nPercepts with salience >= 0.7: {}", important_percepts.len());
    for percept in important_percepts {
        println!("  {:?}", percept);
    }

    // === Part 6: Sensory Integration ===
    println!("\n--- Part 6: Sensory Integration Methods ---");

    println!("Danger detected via percepts: {}", agent.senses_danger_percept());

    let detected_agents = agent.get_detected_agents();
    println!("Detected agents: {}", detected_agents.len());

    let detected_resources = agent.get_detected_resources();
    println!("Detected resources: {}", detected_resources.len());
    for (resource_type, position) in detected_resources {
        println!("  {} at {:?}", resource_type, position);
    }

    if let Some((threat_type, severity)) = agent.get_primary_threat() {
        println!("\nPrimary threat: {:?} (severity: {:.2})", threat_type, severity);
    }

    // === Part 7: Sensory Impairment ===
    println!("\n--- Part 7: Sensory Impairment ---");

    agent.senses.vision.impaired = true;
    println!("Vision impaired: {}", agent.senses.vision.impaired);
    println!("Has any impairment: {}", agent.senses.has_impairment());

    // Process percepts with impairment
    let percepts_impaired = agent.process_percepts();
    println!("Percepts with impaired vision: {}", percepts_impaired.len());

    // Check for environmental condition percepts
    let env_percepts: Vec<_> = percepts_impaired.iter()
        .filter(|p| matches!(p, Percept::EnvironmentalCondition { .. }))
        .collect();
    println!("Environmental condition percepts: {}", env_percepts.len());

    println!("\n=== Demonstration Complete ===");
}
