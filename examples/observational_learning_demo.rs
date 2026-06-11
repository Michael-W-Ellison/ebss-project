// examples/observational_learning_demo.rs
//! Demonstration of the observational learning system
//!
//! This example shows:
//! - Children learning from parents
//! - Skill acquisition through observation
//! - Automatic behavior adoption
//! - Population-wide learning statistics
//! - Broadcasting actions to nearby observers

use ebss::agents::{
    Agent, AgentConfig, Population, ActionType,
    Relationship, RelationshipType, LifeStage,
    get_learning_stats, auto_adopt_ready_behaviors,
};

fn main() {
    println!("=== EBSS Observational Learning Demonstration ===\n");

    // Create a population
    let mut population = Population::new();

    // Create a parent agent (skilled miner)
    let mut parent = Agent::new(AgentConfig::default());
    parent.state.position = (0, 0, 0);
    parent.state.life_stage = LifeStage::Adult;
    parent.skills.gain_experience(ebss::agents::SkillType::Mining, 100); // Experienced miner
    parent.set_learning_rate(1.0); // Adult learning rate

    let parent_id = parent.id;

    // Create a child agent
    let mut child = Agent::new(AgentConfig::default());
    child.state.position = (3, 0, 0); // Close to parent
    child.state.life_stage = LifeStage::Child;
    child.set_learning_rate(1.5); // Children learn faster

    let child_id = child.id;

    // Establish parent-child relationship
    parent.relationships.add_relationship(Relationship::new(child_id, RelationshipType::Child));
    child.relationships.add_relationship(Relationship::new(parent_id, RelationshipType::Parent));

    // Set up vision (child can see parent)
    child.senses.vision.visible_agents.insert(parent_id);

    println!("--- Part 1: Family Setup ---");
    println!("Parent ID: {}", parent_id);
    println!("Parent mining skill level: {}", parent.skills.get_skill_if_exists(ebss::agents::SkillType::Mining)
        .map(|s| s.level).unwrap_or(0));
    println!("Child ID: {}", child_id);
    println!("Child learning rate: {}", child.learning_rate());
    println!();

    // Add agents to population
    population.agents.push(parent);
    population.agents.push(child);

    println!("--- Part 2: Parent Performs Mining Actions ---");

    // Simulate parent mining multiple times (child observes)
    for i in 0..8 {
        let timestamp = i * 100;

        // Broadcast parent's mining action
        population.broadcast_action(
            parent_id,
            (0, 0, 0),
            ActionType::Mining,
            true, // Successful
            format!("Successfully mined stone chunk #{}", i + 1),
            timestamp,
        );

        println!("  Broadcast #{}: Parent mined successfully", i + 1);
    }

    println!();

    // Check learning progress
    println!("--- Part 3: Learning Progress ---");

    let child_stats = get_learning_stats(&population.agents[1]);
    println!("Child learning statistics:");
    println!("  Total behaviors adopted: {}", child_stats.total_adopted);
    println!("  Ready to adopt: {}", child_stats.ready_to_adopt);
    println!("  Learning from parents: {}", child_stats.learning_from_parents);

    if let Some(progress) = population.agents[1].get_learning_from(&parent_id, ActionType::Mining) {
        println!("\nMining observation details:");
        println!("  Observations: {}", progress.observation_count);
        println!("  Successes: {}", progress.success_count);
        println!("  Success rate: {:.1}%", progress.success_rate() * 100.0);
        println!("  Average quality: {:.2}", progress.avg_quality());
        println!("  Confidence: {:.2}", progress.confidence);
        println!("  Already adopted: {}", progress.adopted);
    }

    println!();

    // Check if ready to adopt
    println!("--- Part 4: Learning Opportunities ---");

    let opportunities = population.agents[1].check_learning_opportunities();
    println!("Child has {} learning opportunities:", opportunities.len());

    for (teacher_id, action_type, confidence) in &opportunities {
        println!("  - {:?} from teacher {} (confidence: {:.2})",
            action_type, teacher_id, confidence);
    }

    println!();

    // Auto-adopt ready behaviors
    println!("--- Part 5: Automatic Behavior Adoption ---");

    let initial_mining_skill = population.agents[1].skills.get_skill_if_exists(ebss::agents::SkillType::Mining)
        .map(|s| s.level).unwrap_or(0);
    println!("Child's mining skill BEFORE adoption: level {}", initial_mining_skill);

    let adopted = auto_adopt_ready_behaviors(&mut population.agents[1]);

    println!("\nChild adopted {} behaviors:", adopted.len());
    for (teacher_id, action_type) in &adopted {
        println!("  - {:?} from teacher {}", action_type, teacher_id);
    }

    let final_mining_skill = population.agents[1].skills.get_skill_if_exists(ebss::agents::SkillType::Mining)
        .map(|s| s.level).unwrap_or(0);
    println!("\nChild's mining skill AFTER adoption: level {}", final_mining_skill);
    println!("Skill improvement: +{} levels", final_mining_skill - initial_mining_skill);

    println!();

    // Check adopted behaviors
    println!("--- Part 6: Adopted Behaviors ---");

    let adopted_behaviors = population.agents[1].get_adopted_behaviors();
    println!("Child has {} adopted behaviors:", adopted_behaviors.len());

    for (teacher_id, action_type, confidence) in &adopted_behaviors {
        println!("  - {:?} learned from {} (confidence: {:.2})",
            action_type, teacher_id, confidence);
    }

    // Check parent learning
    let parent_learning = population.agents[1].learning_from_parents();
    println!("\nChild learning from parents:");
    for (parent_id, actions) in &parent_learning {
        println!("  Parent {}: {:?}", parent_id, actions);
    }

    println!();

    // ===== Part 7: Multi-Agent Learning =====
    println!("--- Part 7: Multi-Agent Learning Scenario ---");

    // Create a third agent (stranger)
    let mut stranger = Agent::new(AgentConfig::default());
    stranger.state.position = (2, 0, 0);
    stranger.state.life_stage = LifeStage::Adult;
    stranger.skills.gain_experience(ebss::agents::SkillType::Crafting, 80);
    let stranger_id = stranger.id;

    // Child can see stranger
    population.agents[1].senses.vision.visible_agents.insert(stranger_id);

    population.agents.push(stranger);

    println!("Added stranger (ID: {}) who is skilled in crafting", stranger_id);

    // Stranger performs crafting
    for i in 0..3 {
        population.broadcast_action(
            stranger_id,
            (2, 0, 0),
            ActionType::Crafting,
            true,
            format!("Crafted item #{}", i + 1),
            1000 + i * 100,
        );
    }

    println!("Stranger performed 3 crafting actions");

    let child_opportunities_after = population.agents[1].check_learning_opportunities();
    println!("\nChild now has {} learning opportunities (from multiple teachers)",
        child_opportunities_after.len());

    println!();

    // ===== Part 8: Population-Wide Statistics =====
    println!("--- Part 8: Population Learning Statistics ---");

    let pop_stats = population.get_population_learning_stats();
    println!("Total behaviors adopted: {}", pop_stats.total_behaviors_adopted);
    println!("Total ready to adopt: {}", pop_stats.total_ready_to_adopt);
    println!("Agents learning from parents: {}", pop_stats.agents_learning_from_parents);
    println!("Average unique teachers: {:.1}", pop_stats.average_unique_teachers);

    let active_learners = population.get_active_learners();
    println!("\nActive learners: {}", active_learners.len());
    for (agent_id, opportunity_count) in &active_learners {
        println!("  Agent {} has {} opportunities", agent_id, opportunity_count);
    }

    let parent_child_learning = population.get_parent_child_learning();
    println!("\nParent-child learning pairs: {}", parent_child_learning.len());
    for (child_id, parent_id, actions) in &parent_child_learning {
        println!("  Child {} learning from parent {}: {} action types",
            child_id, parent_id, actions.len());
    }

    println!();

    // ===== Part 9: Observational Learning Over Time =====
    println!("--- Part 9: Learning Over Multiple Ticks ---");

    // Process observational learning through population system
    println!("Running 5 population ticks with observational learning...");

    for tick in 0..5 {
        population.process_observational_learning();

        let stats_after_tick = population.get_population_learning_stats();
        println!("  Tick {}: {} adopted, {} ready",
            tick + 1,
            stats_after_tick.total_behaviors_adopted,
            stats_after_tick.total_ready_to_adopt);
    }

    println!();

    // ===== Part 10: Final Summary =====
    println!("--- Part 10: Final Summary ---");

    println!("\nChild agent final state:");
    let child_agent = &population.agents[1];
    println!("  Total adopted behaviors: {}", child_agent.get_adopted_behaviors().len());
    println!("  Mining skill: level {}", child_agent.skills.get_skill_if_exists(ebss::agents::SkillType::Mining)
        .map(|s| s.level).unwrap_or(0));
    println!("  Crafting skill: level {}", child_agent.skills.get_skill_if_exists(ebss::agents::SkillType::Crafting)
        .map(|s| s.level).unwrap_or(0));

    println!("\nKey takeaways:");
    println!("  ✓ Children learn faster than adults (1.5x learning rate)");
    println!("  ✓ Parent-child relationships accelerate learning (50% fewer observations needed)");
    println!("  ✓ Successful actions are more easily learned than failed ones");
    println!("  ✓ Skill improvements are automatically applied when behaviors are adopted");
    println!("  ✓ Agents can learn from multiple teachers simultaneously");
    println!("  ✓ Observation quality decreases with distance");
    println!("  ✓ Trust and relationship strength affect learning speed");

    println!("\n=== Demonstration Complete ===");
}
