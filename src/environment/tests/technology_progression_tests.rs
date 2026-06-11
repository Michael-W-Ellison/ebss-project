// src/environment/tests/technology_progression_tests.rs
//! Integration tests for technology progression from Stone Age to Bronze Age

use crate::environment::*;
use uuid::Uuid;

#[test]
fn test_stone_age_starting_knowledge() {
    let agent_id = Uuid::new_v4();
    let mut tech_knowledge = TechnologyKnowledge::new();

    // Agents start with basic Stone Age knowledge
    tech_knowledge.add_initial_technology("fire_making".to_string(), agent_id, 0);
    tech_knowledge.add_initial_technology("flint_knapping".to_string(), agent_id, 0);
    tech_knowledge.add_initial_technology("basic_shelter".to_string(), agent_id, 0);

    assert_eq!(tech_knowledge.known_technologies.len(), 3);
    assert_eq!(tech_knowledge.get_state("fire_making", 0), TechnologyState::Known);
    assert_eq!(tech_knowledge.get_state("flint_knapping", 0), TechnologyState::Known);
}

#[test]
fn test_accidental_metal_discovery() {
    let agent_id = Uuid::new_v4();
    let mut tech_knowledge = TechnologyKnowledge::new();

    // Agent starts with fire knowledge
    tech_knowledge.add_initial_technology("fire_making".to_string(), agent_id, 0);

    // Accidentally discovers lead melting (low melting point)
    tech_knowledge.discover_technology(
        "lead_melting".to_string(),
        agent_id,
        DiscoveryMethod::Accident,
        100, // timestamp
        true, // world first
    );

    let record = tech_knowledge.known_technologies.get("lead_melting").unwrap();
    assert_eq!(record.method, DiscoveryMethod::Accident);
    assert_eq!(record.confidence, 0.5); // Accidental discovery has lower initial confidence
    assert_eq!(tech_knowledge.original_discoveries.len(), 1);
}

#[test]
fn test_experimentation_improves_confidence() {
    let agent_id = Uuid::new_v4();
    let mut tech_knowledge = TechnologyKnowledge::new();

    // Discover via accident (low confidence)
    tech_knowledge.discover_technology(
        "copper_smelting".to_string(),
        agent_id,
        DiscoveryMethod::Accident,
        0,
        true,
    );

    let initial_confidence = tech_knowledge.known_technologies
        .get("copper_smelting")
        .unwrap()
        .confidence;

    // Experiment successfully to improve confidence
    tech_knowledge.record_attempt("copper_smelting", true);
    tech_knowledge.record_attempt("copper_smelting", true);
    tech_knowledge.record_attempt("copper_smelting", true);

    let new_confidence = tech_knowledge.known_technologies
        .get("copper_smelting")
        .unwrap()
        .confidence;

    assert!(new_confidence > initial_confidence);
    assert!(new_confidence > 0.7); // Should be quite confident after 3 successes
}

#[test]
fn test_knowledge_sharing_via_teaching() {
    let teacher_id = Uuid::new_v4();
    let student_id = Uuid::new_v4();

    let mut teacher_knowledge = TechnologyKnowledge::new();
    let mut student_knowledge = TechnologyKnowledge::new();

    // Teacher discovers and masters copper smelting
    teacher_knowledge.discover_technology(
        "copper_smelting".to_string(),
        teacher_id,
        DiscoveryMethod::Experimentation,
        0,
        true,
    );

    // Multiple successful attempts -> high confidence
    for _ in 0..10 {
        teacher_knowledge.record_attempt("copper_smelting", true);
    }

    let teacher_confidence = teacher_knowledge.teaching_confidence("copper_smelting");
    assert!(teacher_confidence > 0.8); // Very high confidence

    // Teacher teaches student (high trust)
    let trust_in_teacher = 0.9;
    student_knowledge.learn_from_agent(
        "copper_smelting".to_string(),
        student_id,
        DiscoveryMethod::Instruction,
        teacher_confidence,
        trust_in_teacher,
        100,
    );

    let student_record = student_knowledge.known_technologies.get("copper_smelting").unwrap();
    // Confidence should be product: ~0.9 * 0.9 = ~0.81
    assert!(student_record.confidence > 0.7);
    assert_eq!(student_record.method, DiscoveryMethod::Instruction);
}

#[test]
fn test_gossip_creates_rumors() {
    let _discoverer_id = Uuid::new_v4();
    let gossip_recipient_id = Uuid::new_v4();

    let mut recipient_knowledge = TechnologyKnowledge::new();

    // Recipient hears about copper working through gossip (low trust source)
    recipient_knowledge.learn_from_agent(
        "copper_working".to_string(),
        gossip_recipient_id,
        DiscoveryMethod::Gossip,
        0.6, // Teacher has moderate confidence
        0.4, // Low trust in gossiper
        50,
    );

    let record = recipient_knowledge.known_technologies.get("copper_working").unwrap();
    // Confidence should be low: 0.6 * 0.4 = 0.24
    assert!(record.confidence < 0.3);
    assert_eq!(record.get_state(0), TechnologyState::Rumored); // Not confident enough to attempt
}

#[test]
fn test_failed_attempts_reduce_confidence() {
    let agent_id = Uuid::new_v4();
    let mut tech_knowledge = TechnologyKnowledge::new();

    tech_knowledge.discover_technology(
        "iron_smelting".to_string(),
        agent_id,
        DiscoveryMethod::Experimentation,
        0,
        true,
    );

    let initial_confidence = tech_knowledge.known_technologies
        .get("iron_smelting")
        .unwrap()
        .confidence;

    // Multiple failures (trying to smelt iron without hot enough fire)
    for _ in 0..5 {
        tech_knowledge.record_attempt("iron_smelting", false);
    }

    let new_confidence = tech_knowledge.known_technologies
        .get("iron_smelting")
        .unwrap()
        .confidence;

    assert!(new_confidence < initial_confidence);
    assert!(new_confidence < 0.5); // Should lose confidence
}

#[test]
fn test_technology_prerequisites() {
    let mut registry = TechnologyRegistry::new();

    // Define technology tree
    let flint_knapping = Technology::new(
        "flint_knapping".to_string(),
        "Flint Knapping".to_string(),
    )
    .with_description("Shape flint into sharp tools".to_string())
    .with_discovery_chance(0.3); // Relatively easy to discover

    let copper_working = Technology::new(
        "copper_working".to_string(),
        "Copper Working".to_string(),
    )
    .with_description("Cold-work native copper into tools".to_string())
    .with_prerequisites(vec!["flint_knapping".to_string()]) // Need basic toolmaking first
    .with_required_materials(vec!["native_copper".to_string()])
    .with_discovery_chance(0.1); // Harder to discover

    let copper_smelting = Technology::new(
        "copper_smelting".to_string(),
        "Copper Smelting".to_string(),
    )
    .with_description("Extract copper from ore using fire".to_string())
    .with_prerequisites(vec!["fire_making".to_string(), "copper_working".to_string()])
    .with_required_materials(vec!["copper_ore".to_string()])
    .with_curiosity_threshold(0.4) // Needs curious agent
    .with_discovery_chance(0.05) // Rare discovery
    .with_accidental_discovery(0.01); // 1% chance per fire with copper ore

    registry.register(flint_knapping);
    registry.register(copper_working);
    registry.register(copper_smelting);

    // Agent only knows flint knapping
    let agent_id = Uuid::new_v4();
    let mut known_techs = std::collections::HashMap::new();

    let mut flint_record = DiscoveryRecord::new(
        "flint_knapping".to_string(),
        agent_id,
        DiscoveryMethod::Initial,
        0,
    );
    flint_record.success_count = 1; // Practiced
    known_techs.insert("flint_knapping".to_string(), flint_record);

    // Can discover copper_working (has prerequisite)
    let copper_working_tech = registry.get("copper_working").unwrap();
    assert!(copper_working_tech.can_discover(&known_techs));

    // Cannot discover copper_smelting (missing copper_working prerequisite)
    let copper_smelting_tech = registry.get("copper_smelting").unwrap();
    assert!(!copper_smelting_tech.can_discover(&known_techs));
}

#[test]
fn test_curiosity_threshold_for_discovery() {
    let tech = Technology::new("experimental_alloy".to_string(), "Experimental Alloy".to_string())
        .with_curiosity_threshold(0.6); // Needs high curiosity

    assert_eq!(tech.curiosity_threshold, 0.6);

    // Agent with low curiosity drive (0.3) shouldn't attempt
    // Agent with high curiosity drive (0.7) should attempt
    // This would be checked in the action system
}

#[test]
fn test_heat_source_temperature_gating() {
    let mut heat_source = HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0);
    heat_source.add_fuel("wood".to_string(), 10.0, 100);
    heat_source.light();
    heat_source.current_temperature = 700.0; // Campfire temp

    // Campfire (max 800°C) cannot smelt copper (1085°C)
    assert!(!heat_source.can_smelt(1085.0));

    // But can accidentally melt lead (327°C) or tin (232°C)
    assert!(heat_source.can_smelt(327.0));
    assert!(heat_source.can_smelt(232.0));

    // Bloomery (1200-1400°C) CAN smelt copper
    let mut bloomery = HeatSource::new(HeatSourceType::Bloomery, (0, 0, 0), 0);
    bloomery.add_fuel("charcoal".to_string(), 10.0, 200);
    bloomery.light();
    bloomery.current_temperature = 1300.0;

    assert!(bloomery.can_smelt(1085.0)); // Copper
    assert!(!bloomery.can_smelt(1538.0)); // Iron (needs higher temp)
}

#[test]
fn test_material_cold_working() {
    // Native copper can be cold-worked
    let native_copper = Material::new("native_copper".to_string(), "Native Copper".to_string())
        .with_melting_point(1085.0)
        .with_cold_working();

    assert!(native_copper.can_cold_work);
    assert_eq!(native_copper.melting_point, Some(1085.0));

    // Iron cannot be cold-worked (needs heat)
    let iron_ingot = Material::new("iron_ingot".to_string(), "Iron Ingot".to_string())
        .with_melting_point(1538.0)
        .with_workable_temp(1000.0);

    assert!(!iron_ingot.can_cold_work);
    assert_eq!(iron_ingot.workable_temp, Some(1000.0));
}

#[test]
fn test_ore_smelting_yield() {
    // Copper ore has 60% yield
    let copper_ore = Material::new("copper_ore".to_string(), "Copper Ore".to_string())
        .as_ore("copper_ingot".to_string(), 0.6);

    assert!(copper_ore.is_ore);
    assert_eq!(copper_ore.ore_metal_id, Some("copper_ingot".to_string()));
    assert_eq!(copper_ore.ore_yield, 0.6);

    // Smelting 10 copper ore should yield 6 copper ingots (10 * 0.6)
    let ore_quantity = 10;
    let expected_ingots = (ore_quantity as f32 * copper_ore.ore_yield) as u32;
    assert_eq!(expected_ingots, 6);
}

#[test]
fn test_full_technology_progression_scenario() {
    // Setup: Create registry with full tech tree
    let mut registry = TechnologyRegistry::new();

    // Stone Age
    registry.register(
        Technology::new("fire_making".to_string(), "Fire Making".to_string())
            .with_description("Create and maintain fire".to_string())
    );

    registry.register(
        Technology::new("flint_knapping".to_string(), "Flint Knapping".to_string())
            .with_prerequisites(vec![])
            .with_discovery_chance(0.3)
    );

    // Copper Age
    registry.register(
        Technology::new("native_copper_working".to_string(), "Native Copper Working".to_string())
            .with_prerequisites(vec!["flint_knapping".to_string()])
            .with_required_materials(vec!["native_copper".to_string()])
            .with_discovery_chance(0.1)
    );

    registry.register(
        Technology::new("copper_smelting".to_string(), "Copper Smelting".to_string())
            .with_prerequisites(vec!["fire_making".to_string()])
            .with_required_materials(vec!["copper_ore".to_string()])
            .with_accidental_discovery(0.01)
            .with_discovery_chance(0.05)
    );

    registry.register(
        Technology::new("bellows".to_string(), "Bellows".to_string())
            .with_prerequisites(vec!["copper_smelting".to_string()])
            .with_description("Increase fire temperature with forced air".to_string())
            .with_discovery_chance(0.1)
    );

    // Bronze Age
    registry.register(
        Technology::new("tin_smelting".to_string(), "Tin Smelting".to_string())
            .with_prerequisites(vec!["copper_smelting".to_string()])
            .with_required_materials(vec!["tin_ore".to_string()])
            .with_discovery_chance(0.05)
    );

    registry.register(
        Technology::new("bronze_casting".to_string(), "Bronze Casting".to_string())
            .with_prerequisites(vec!["copper_smelting".to_string(), "tin_smelting".to_string()])
            .with_description("Alloy copper and tin to create bronze".to_string())
            .with_discovery_chance(0.03)
    );

    // Simulate progression
    let agent_id = Uuid::new_v4();
    let mut knowledge = TechnologyKnowledge::new();

    // Start with Stone Age knowledge
    knowledge.add_initial_technology("fire_making".to_string(), agent_id, 0);
    knowledge.add_initial_technology("flint_knapping".to_string(), agent_id, 100);

    // Discover native copper working
    knowledge.discover_technology(
        "native_copper_working".to_string(),
        agent_id,
        DiscoveryMethod::Experimentation,
        200,
        true,
    );

    // Accidentally discover copper smelting (ore in fire)
    knowledge.discover_technology(
        "copper_smelting".to_string(),
        agent_id,
        DiscoveryMethod::Accident,
        300,
        true,
    );

    // Practice copper smelting
    for _ in 0..5 {
        knowledge.record_attempt("copper_smelting", true);
    }

    // Discover tin smelting
    knowledge.discover_technology(
        "tin_smelting".to_string(),
        agent_id,
        DiscoveryMethod::Experimentation,
        400,
        true,
    );

    // Discover bronze casting (combining known techs)
    knowledge.discover_technology(
        "bronze_casting".to_string(),
        agent_id,
        DiscoveryMethod::Experimentation,
        500,
        true,
    );

    // Verify progression
    assert_eq!(knowledge.known_technologies.len(), 6);
    assert_eq!(knowledge.original_discoveries.len(), 4); // Discovered 4 new techs

    // Check states
    assert_eq!(knowledge.get_state("fire_making", 0), TechnologyState::Known);
    assert_eq!(knowledge.get_state("copper_smelting", 5), TechnologyState::Practiced); // Skill 5, has successes
    assert_eq!(knowledge.get_state("bronze_casting", 0), TechnologyState::Known);

    // Verify prerequisites were met
    let bronze_tech = registry.get("bronze_casting").unwrap();
    assert!(bronze_tech.can_discover(&knowledge.known_technologies));
}

#[test]
fn test_world_first_discoverer_tracking() {
    let mut registry = TechnologyRegistry::new();

    let agent1 = Uuid::new_v4();
    let agent2 = Uuid::new_v4();

    // Agent 1 discovers copper smelting first
    let is_first = registry.record_first_discovery(
        "copper_smelting".to_string(),
        agent1,
        100,
    );
    assert!(is_first);

    // Agent 2 also discovers it, but not first
    let is_first = registry.record_first_discovery(
        "copper_smelting".to_string(),
        agent2,
        200,
    );
    assert!(!is_first);

    // Verify agent 1 is recorded as first discoverer
    let (discoverer, timestamp) = registry.first_discoverers.get("copper_smelting").unwrap();
    assert_eq!(*discoverer, agent1);
    assert_eq!(*timestamp, 100);
}

#[test]
fn test_mastery_progression() {
    let agent_id = Uuid::new_v4();
    let mut knowledge = TechnologyKnowledge::new();

    knowledge.discover_technology(
        "copper_working".to_string(),
        agent_id,
        DiscoveryMethod::Experimentation,
        0,
        true,
    );

    // Progress through states as skill increases
    assert_eq!(knowledge.get_state("copper_working", -5), TechnologyState::Known);

    knowledge.record_attempt("copper_working", true);
    assert_eq!(knowledge.get_state("copper_working", 0), TechnologyState::Practiced);

    // At skill level 6+, becomes Mastered
    assert_eq!(knowledge.get_state("copper_working", 6), TechnologyState::Mastered);
    assert_eq!(knowledge.get_state("copper_working", 10), TechnologyState::Mastered);
}
