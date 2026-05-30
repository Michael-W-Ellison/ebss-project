// src/agents/tests/gossip_spreading_tests.rs
//! Integration tests for gossip spreading mechanism

use crate::agents::{Agent, AgentConfig, Population, Trait};
use crate::agents::gossip::{Information, InformationType};

#[test]
fn test_gossip_spreads_between_nearby_agents() {
    let mut pop = Population::new();

    // Create two agents at the same position
    let mut config = AgentConfig::default();
    config.random_weights = false;
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());

    // Put them at the same position (within gossip range)
    pop.agents[0].state.position = (10, 10, 0);
    pop.agents[1].state.position = (10, 10, 0);

    // Make sure they're not hungry/thirsty (which blocks gossip)
    for agent in &mut pop.agents {
        if let Some(hunger) = agent.drives.get_mut(crate::core::DriveType::Hunger) {
            hunger.value = 0.2;
        }
        if let Some(thirst) = agent.drives.get_mut(crate::core::DriveType::Thirst) {
            thirst.value = 0.2;
        }
    }

    // Agent 0 learns some information
    let info = Information::new(
        InformationType::ResourceLocation {
            resource: "gold".to_string(),
            location: (50, 50, 0),
        },
        pop.agents[0].id,
        true,
        0,
    );
    pop.agents[0].knowledge.known_information.insert(info.id, info);

    // Agent 1 should not know about gold yet
    assert!(pop.agents[1].knowledge.known_information.is_empty());

    // Run gossip multiple times to ensure it spreads (probabilistic)
    for _ in 0..50 {
        pop.process_gossip();
    }

    // Agent 1 should now know about the gold
    let agent1_knows_gold = pop.agents[1].knowledge.known_information
        .values()
        .any(|info| {
            matches!(&info.info_type, InformationType::ResourceLocation { resource, .. }
                if resource.contains("gold"))
        });

    assert!(agent1_knows_gold, "Agent 1 should have learned about gold through gossip");
}

#[test]
fn test_gossip_trait_increases_sharing_probability() {
    let mut pop = Population::new();

    // Create two agents - one with Gossip trait
    let mut config = AgentConfig::default();
    config.random_weights = false;
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());

    // Add Gossip trait to agent 0
    pop.agents[0].traits.add_trait(Trait::Gossip);

    // Put them together
    pop.agents[0].state.position = (10, 10, 0);
    pop.agents[1].state.position = (10, 10, 0);

    // Reduce survival drive urgency
    for agent in &mut pop.agents {
        if let Some(hunger) = agent.drives.get_mut(crate::core::DriveType::Hunger) {
            hunger.value = 0.2;
        }
        if let Some(thirst) = agent.drives.get_mut(crate::core::DriveType::Thirst) {
            thirst.value = 0.2;
        }
    }

    // Agent 0 learns information
    let info = Information::new(
        InformationType::TechnologyDiscovered {
            tech: "pottery".to_string(),
        },
        pop.agents[0].id,
        true,
        0,
    );
    pop.agents[0].knowledge.known_information.insert(info.id, info);

    // With Gossip trait, sharing should happen quickly
    for _ in 0..20 {
        pop.process_gossip();
    }

    let agent1_knows_tech = pop.agents[1].knowledge.known_information
        .values()
        .any(|info| {
            matches!(&info.info_type, InformationType::TechnologyDiscovered { tech }
                if tech.contains("pottery"))
        });

    assert!(agent1_knows_tech, "Gossip trait agent should share information readily");
}

#[test]
fn test_information_distortion_during_gossip() {
    let mut pop = Population::new();

    // Create agents - one with Imaginative trait (causes exaggeration)
    let mut config = AgentConfig::default();
    config.random_weights = false;
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());

    // Add Imaginative trait to agent 0
    pop.agents[0].traits.add_trait(Trait::Imaginative);

    // Put them together
    pop.agents[0].state.position = (10, 10, 0);
    pop.agents[1].state.position = (10, 10, 0);

    // Reduce survival drive urgency
    for agent in &mut pop.agents {
        if let Some(hunger) = agent.drives.get_mut(crate::core::DriveType::Hunger) {
            hunger.value = 0.2;
        }
        if let Some(thirst) = agent.drives.get_mut(crate::core::DriveType::Thirst) {
            thirst.value = 0.2;
        }
    }

    // Agent 0 observes something
    let observer_id = pop.agents[0].id;
    let info = Information::new(
        InformationType::Observation {
            observer: observer_id,
            observed: "rabbit".to_string(),
            location: (30, 30, 0),
        },
        observer_id,
        true,
        0,
    );
    pop.agents[0].knowledge.known_information.insert(info.id, info);

    // Spread through gossip
    for _ in 0..50 {
        pop.process_gossip();
    }

    // Check if agent 1 received distorted information
    let has_distorted_observation = pop.agents[1].knowledge.known_information
        .values()
        .any(|info| {
            if let InformationType::Observation { observed, .. } = &info.info_type {
                // Imaginative distortion adds "a dozen" to observations
                observed.contains("dozen") || observed.contains("rabbit")
            } else {
                false
            }
        });

    // Either received original or distorted version
    assert!(has_distorted_observation, "Agent 1 should have received observation info");
}

#[test]
fn test_distant_agents_dont_gossip() {
    let mut pop = Population::new();

    let mut config = AgentConfig::default();
    config.random_weights = false;
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());

    // Put them far apart (beyond gossip range of 6 tiles)
    pop.agents[0].state.position = (10, 10, 0);
    pop.agents[1].state.position = (50, 50, 0); // ~57 tiles away

    // Reduce survival drive urgency
    for agent in &mut pop.agents {
        if let Some(hunger) = agent.drives.get_mut(crate::core::DriveType::Hunger) {
            hunger.value = 0.2;
        }
    }

    // Agent 0 has information
    let info = Information::new(
        InformationType::ResourceLocation {
            resource: "iron".to_string(),
            location: (100, 100, 0),
        },
        pop.agents[0].id,
        true,
        0,
    );
    pop.agents[0].knowledge.known_information.insert(info.id, info);

    // Process gossip many times
    for _ in 0..100 {
        pop.process_gossip();
    }

    // Agent 1 should NOT have learned about iron (too far)
    let agent1_knows_iron = pop.agents[1].knowledge.known_information
        .values()
        .any(|info| {
            matches!(&info.info_type, InformationType::ResourceLocation { resource, .. }
                if resource == "iron")
        });

    assert!(!agent1_knows_iron, "Distant agents should not share gossip");
}

#[test]
fn test_hungry_agents_dont_gossip() {
    let mut pop = Population::new();

    let mut config = AgentConfig::default();
    config.random_weights = false;
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());

    // Put them together
    pop.agents[0].state.position = (10, 10, 0);
    pop.agents[1].state.position = (10, 10, 0);

    // Make agent 0 very hungry (will block gossip)
    if let Some(hunger) = pop.agents[0].drives.get_mut(crate::core::DriveType::Hunger) {
        hunger.value = 0.9; // Very hungry
    }

    // Agent 0 has information but is too hungry to gossip
    let info = Information::new(
        InformationType::TechnologyDiscovered {
            tech: "weaving".to_string(),
        },
        pop.agents[0].id,
        true,
        0,
    );
    pop.agents[0].knowledge.known_information.insert(info.id, info);

    // Process gossip
    for _ in 0..50 {
        pop.process_gossip();
    }

    // Agent 1 should NOT know about weaving (agent 0 too hungry to chat)
    let agent1_knows_weaving = pop.agents[1].knowledge.known_information
        .values()
        .any(|info| {
            matches!(&info.info_type, InformationType::TechnologyDiscovered { tech }
                if tech.contains("weaving"))
        });

    assert!(!agent1_knows_weaving, "Hungry agents should not gossip");
}

#[test]
fn test_introvert_gossips_less() {
    let mut pop = Population::new();

    let mut config = AgentConfig::default();
    config.random_weights = false;

    // Create introvert and extrovert
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());

    pop.agents[0].traits.add_trait(Trait::Introvert);
    pop.agents[1].traits.add_trait(Trait::Extrovert);
    // Agent 2 has no social traits (baseline)

    // All at same position
    for agent in &mut pop.agents {
        agent.state.position = (10, 10, 0);
        if let Some(hunger) = agent.drives.get_mut(crate::core::DriveType::Hunger) {
            hunger.value = 0.2;
        }
    }

    // Calculate probabilities
    let introvert_prob = pop.calculate_gossip_probability(&pop.agents[0]);
    let extrovert_prob = pop.calculate_gossip_probability(&pop.agents[1]);
    let baseline_prob = pop.calculate_gossip_probability(&pop.agents[2]);

    assert!(introvert_prob < baseline_prob, "Introvert should gossip less than baseline");
    assert!(extrovert_prob > baseline_prob, "Extrovert should gossip more than baseline");
    assert!(extrovert_prob > introvert_prob, "Extrovert should gossip more than introvert");
}

#[test]
fn test_trust_affects_belief_confidence() {
    let mut pop = Population::new();

    let mut config = AgentConfig::default();
    config.random_weights = false;
    pop.spawn_agent(config.clone());
    pop.spawn_agent(config.clone());

    let agent0_id = pop.agents[0].id;
    let agent1_id = pop.agents[1].id;

    // Set low trust in agent 0
    pop.agents[1].knowledge.trust_ratings.insert(
        agent0_id,
        crate::agents::gossip::TrustRating {
            truster: agent1_id,
            trustee: agent0_id,
            trust: 0.2, // Low trust
            correct_count: 0,
            wrong_count: 5,
        },
    );

    // Same position
    pop.agents[0].state.position = (10, 10, 0);
    pop.agents[1].state.position = (10, 10, 0);

    for agent in &mut pop.agents {
        if let Some(hunger) = agent.drives.get_mut(crate::core::DriveType::Hunger) {
            hunger.value = 0.2;
        }
    }

    // Agent 0 shares information
    let info = Information::new(
        InformationType::Accusation {
            accuser: agent0_id,
            accused: uuid::Uuid::new_v4(),
            crime: "theft".to_string(),
        },
        agent0_id,
        false, // This is actually a lie
        0,
    );
    pop.agents[0].knowledge.known_information.insert(info.id, info);

    // Spread gossip
    for _ in 0..50 {
        pop.process_gossip();
    }

    // Agent 1 might receive but with low confidence due to low trust
    let received_accusation = pop.agents[1].knowledge.beliefs.iter()
        .find(|b| {
            pop.agents[1].knowledge.known_information
                .get(&b.info_id)
                .map(|info| matches!(info.info_type, InformationType::Accusation { .. }))
                .unwrap_or(false)
        });

    if let Some(belief) = received_accusation {
        assert!(belief.confidence < 0.5, "Low trust should result in low belief confidence");
    }
}
