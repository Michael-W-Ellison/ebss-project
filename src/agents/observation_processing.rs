// src/agents/observation_processing.rs
//! Observation processing system for automatic learning between agents.
//!
//! This module handles:
//! - Broadcasting actions to nearby observers
//! - Automatic observation recording
//! - Skill learning from adopted behaviors
//! - Behavior application and imitation

use uuid::Uuid;
use super::{Agent, ActionType, SkillType};

/// Represents an action that can be observed by other agents
#[derive(Debug, Clone)]
pub struct BroadcastAction {
    /// Who performed the action
    pub performer_id: Uuid,
    /// Position where action occurred
    pub position: (i32, i32, i32),
    /// Type of action performed
    pub action_type: ActionType,
    /// Whether the action succeeded
    pub success: bool,
    /// Details about the action
    pub details: String,
    /// When this happened
    pub timestamp: u64,
    /// Maximum observation distance (beyond this, action isn't visible)
    pub visibility_range: f32,
}

impl BroadcastAction {
    pub fn new(
        performer_id: Uuid,
        position: (i32, i32, i32),
        action_type: ActionType,
        success: bool,
        details: String,
        timestamp: u64,
    ) -> Self {
        Self {
            performer_id,
            position,
            action_type,
            success,
            details,
            timestamp,
            visibility_range: action_type.visibility_range(),
        }
    }

    /// Check if an observer at given position can see this action
    pub fn is_visible_from(&self, observer_position: (i32, i32, i32)) -> bool {
        let dx = (self.position.0 - observer_position.0) as f32;
        let dy = (self.position.1 - observer_position.1) as f32;
        let dz = (self.position.2 - observer_position.2) as f32;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        distance <= self.visibility_range
    }
}

impl ActionType {
    /// Get visibility range for this action type
    pub fn visibility_range(&self) -> f32 {
        match self {
            ActionType::Mining => 15.0,        // Loud, visible
            ActionType::Crafting => 10.0,      // Medium visibility
            ActionType::Building => 20.0,      // Very visible
            ActionType::Combat => 25.0,        // Loud, attracts attention
            ActionType::Cooking => 12.0,       // Smoke, smell
            ActionType::ToolUse => 10.0,       // Moderate visibility
            ActionType::Social => 8.0,         // Close-range interaction
            ActionType::Navigation => 15.0,    // Movement is visible
            ActionType::ProblemSolving => 8.0, // Subtle, requires close observation
            ActionType::Farming => 18.0,       // Done in the open, in a field
        }
    }
}

/// Process a broadcast action and record observations for nearby agents
pub fn process_observations(
    agents: &mut [Agent],
    broadcast: &BroadcastAction,
) {
    for agent in agents.iter_mut() {
        // Can't observe yourself
        if agent.id == broadcast.performer_id {
            continue;
        }

        // Check if agent can see the action
        if !broadcast.is_visible_from(agent.state.position) {
            continue;
        }

        // Check if performer is visible to this agent
        if !agent.senses.vision.visible_agents.contains(&broadcast.performer_id) {
            continue;
        }

        // Agent observes the action
        agent.observe_action(
            &broadcast.performer_id,
            broadcast.position,
            broadcast.action_type,
            broadcast.success,
            broadcast.details.clone(),
            broadcast.timestamp,
        );

        // The young pick something up every time they watch, long before they
        // have seen enough of it to take it up themselves. Adoption is the
        // moment a child starts doing a thing; this is the years of watching
        // that come first, and it counts for more when it is their own parent
        // they are watching.
        teach_by_watching(agent, broadcast);
    }
}

/// Skill experience a child gains from watching an adult work.
///
/// Only the young learn this way - a grown agent picks things up by doing them
/// - and a child learns most from its own parents, who it is with and paying
/// attention to.
fn teach_by_watching(watcher: &mut Agent, broadcast: &BroadcastAction) {
    use crate::agents::LifeStage;

    let learning_age = matches!(
        watcher.state.life_stage,
        LifeStage::Infant | LifeStage::Child | LifeStage::Adolescent
    );

    if !learning_age || !broadcast.success {
        return;
    }

    // Watching a stranger work teaches something; watching your mother teaches
    // more
    let from_a_parent = watcher.parent_ids.contains(&broadcast.performer_id);
    let attention = if from_a_parent { 3 } else { 1 };

    for (skill_type, experience) in get_skill_gains_for_action(broadcast.action_type) {
        // A fraction of what taking the behaviour up outright would give
        let learned = (experience * attention) / 10;
        if learned > 0 {
            watcher.skills.gain_experience(skill_type, learned);
        }
    }
}

/// Automatically check and adopt ready behaviors for an agent
pub fn auto_adopt_ready_behaviors(agent: &mut Agent) -> Vec<(Uuid, ActionType)> {
    let mut adopted = Vec::new();

    let opportunities = agent.check_learning_opportunities();

    for (teacher_id, action_type, _confidence) in opportunities {
        if agent.adopt_learned_behavior(&teacher_id, action_type) {
            adopted.push((teacher_id, action_type));

            // Apply skill learning when behavior is adopted
            apply_skill_learning(agent, action_type);
        }
    }

    adopted
}

/// Apply skill improvements when a behavior is adopted
///
/// When an agent adopts a behavior through observation, they gain
/// a skill boost in related areas
pub fn apply_skill_learning(agent: &mut Agent, action_type: ActionType) {
    let skill_gains = get_skill_gains_for_action(action_type);

    for (skill_type, experience) in skill_gains {
        agent.skills.gain_experience(skill_type, experience);
    }
}

/// Get skill gains for adopting a specific action type
fn get_skill_gains_for_action(action_type: ActionType) -> Vec<(SkillType, u32)> {
    match action_type {
        ActionType::Mining => vec![
            (SkillType::Mining, 15),
        ],
        ActionType::Crafting => vec![
            (SkillType::Crafting, 20),
        ],
        ActionType::Building => vec![
            (SkillType::Construction, 20),
        ],
        ActionType::Combat => vec![
            (SkillType::MeleeCombat, 25),
        ],
        ActionType::Cooking => vec![
            (SkillType::Cooking, 15),
        ],
        ActionType::Farming => vec![
            (SkillType::Farming, 20),
            (SkillType::Herbalism, 5),
        ],
        ActionType::ToolUse => vec![
            (SkillType::Crafting, 15),
        ],
        ActionType::Social => vec![
            (SkillType::Social, 20), // Social interactions build social skill
        ],
        ActionType::Navigation => vec![
            (SkillType::Navigation, 15), // Pathfinding builds navigation skill
        ],
        ActionType::ProblemSolving => vec![
            // General intelligence/problem-solving boost
            // Could add to multiple skills
            (SkillType::Crafting, 10),
        ],
    }
}

/// Check if an agent should imitate a learned behavior in current context
///
/// Returns the action type to perform if imitation is appropriate
pub fn should_imitate_behavior(
    agent: &Agent,
    current_context: &BehaviorContext,
) -> Option<ActionType> {
    let adopted_behaviors = agent.get_adopted_behaviors();

    if adopted_behaviors.is_empty() {
        return None;
    }

    // Filter behaviors relevant to current context
    let relevant_behaviors: Vec<_> = adopted_behaviors
        .iter()
        .filter(|(_, action_type, _)| is_relevant_to_context(*action_type, current_context))
        .collect();

    if relevant_behaviors.is_empty() {
        return None;
    }

    // Choose behavior with highest confidence
    relevant_behaviors
        .iter()
        .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, action_type, _)| *action_type)
}

/// Current behavioral context for decision-making
#[derive(Debug, Clone)]
pub struct BehaviorContext {
    /// What the agent needs right now
    pub current_need: NeedType,
    /// What resources are available
    pub available_resources: Vec<String>,
    /// What the agent is currently doing
    pub current_activity: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedType {
    Food,
    Resources,
    Shelter,
    Safety,
    Social,
    Exploration,
}

impl BehaviorContext {
    pub fn new(current_need: NeedType) -> Self {
        Self {
            current_need,
            available_resources: Vec::new(),
            current_activity: None,
        }
    }

    pub fn with_resources(mut self, resources: Vec<String>) -> Self {
        self.available_resources = resources;
        self
    }

    pub fn with_activity(mut self, activity: String) -> Self {
        self.current_activity = Some(activity);
        self
    }
}

/// Check if an action type is relevant to the current context
fn is_relevant_to_context(action_type: ActionType, context: &BehaviorContext) -> bool {
    match context.current_need {
        NeedType::Food => matches!(action_type, ActionType::Mining | ActionType::Cooking),
        NeedType::Resources => matches!(action_type, ActionType::Mining | ActionType::Crafting),
        NeedType::Shelter => matches!(action_type, ActionType::Building | ActionType::Crafting),
        NeedType::Safety => matches!(action_type, ActionType::Combat | ActionType::Building),
        NeedType::Social => matches!(action_type, ActionType::Social),
        NeedType::Exploration => matches!(action_type, ActionType::Navigation),
    }
}

/// Get learning statistics for an agent
pub fn get_learning_stats(agent: &Agent) -> LearningStats {
    let adopted_behaviors = agent.get_adopted_behaviors();
    let opportunities = agent.check_learning_opportunities();

    let mut teachers = std::collections::HashSet::new();
    for (teacher_id, _, _) in &adopted_behaviors {
        teachers.insert(*teacher_id);
    }

    let parent_learning = agent.learning_from_parents();

    LearningStats {
        total_adopted: adopted_behaviors.len(),
        ready_to_adopt: opportunities.len(),
        unique_teachers: teachers.len(),
        learning_from_parents: parent_learning.len(),
        learning_rate: agent.learning_rate(),
    }
}

/// Learning statistics for an agent
#[derive(Debug, Clone)]
pub struct LearningStats {
    pub total_adopted: usize,
    pub ready_to_adopt: usize,
    pub unique_teachers: usize,
    pub learning_from_parents: usize,
    pub learning_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentConfig;

    #[test]
    fn test_broadcast_action_visibility() {
        let action = BroadcastAction::new(
            Uuid::new_v4(),
            (0, 0, 0),
            ActionType::Mining,
            true,
            "mined stone".to_string(),
            0,
        );

        // Close position should be visible
        assert!(action.is_visible_from((5, 0, 0)));

        // Far position should not be visible
        assert!(!action.is_visible_from((100, 0, 0)));

        // Just at edge should be visible
        let edge_distance = action.visibility_range;
        assert!(action.is_visible_from((edge_distance as i32, 0, 0)));
    }

    #[test]
    fn test_action_visibility_ranges() {
        // Combat should have longest range
        assert!(ActionType::Combat.visibility_range() > ActionType::Social.visibility_range());

        // Building should be more visible than crafting
        assert!(ActionType::Building.visibility_range() > ActionType::Crafting.visibility_range());

        // Problem solving should be subtle
        assert!(ActionType::ProblemSolving.visibility_range() < ActionType::Mining.visibility_range());
    }

    #[test]
    fn test_skill_gains_mining() {
        let gains = get_skill_gains_for_action(ActionType::Mining);
        assert!(!gains.is_empty());
        assert!(gains.iter().any(|(skill, _)| *skill == SkillType::Mining));
    }

    #[test]
    fn test_skill_gains_combat() {
        let gains = get_skill_gains_for_action(ActionType::Combat);
        assert!(!gains.is_empty());
        assert!(gains.iter().any(|(skill, _)| *skill == SkillType::MeleeCombat));
    }

    #[test]
    fn test_behavior_context_relevance() {
        let food_context = BehaviorContext::new(NeedType::Food);
        assert!(is_relevant_to_context(ActionType::Mining, &food_context));
        assert!(is_relevant_to_context(ActionType::Cooking, &food_context));
        assert!(!is_relevant_to_context(ActionType::Building, &food_context));

        let shelter_context = BehaviorContext::new(NeedType::Shelter);
        assert!(is_relevant_to_context(ActionType::Building, &shelter_context));
        assert!(!is_relevant_to_context(ActionType::Combat, &shelter_context));
    }

    #[test]
    fn test_process_observations() {
        let mut agents = vec![
            Agent::new(AgentConfig::default()),
            Agent::new(AgentConfig::default()),
        ];

        let performer_id = agents[0].id;
        let observer_id = agents[1].id;

        // Set positions close together
        agents[0].state.position = (0, 0, 0);
        agents[1].state.position = (5, 0, 0);

        // Observer can see performer
        agents[1].senses.vision.visible_agents.insert(performer_id);

        let broadcast = BroadcastAction::new(
            performer_id,
            (0, 0, 0),
            ActionType::Mining,
            true,
            "mined stone".to_string(),
            0,
        );

        process_observations(&mut agents, &broadcast);

        // Check that observer recorded the observation
        let progress = agents[1].get_learning_from(&performer_id, ActionType::Mining);
        assert!(progress.is_some());
        assert_eq!(progress.unwrap().observation_count, 1);
    }

    #[test]
    fn test_auto_adopt_ready_behaviors() {
        let mut agent = Agent::new(AgentConfig::default());
        agent.set_learning_rate(1.5);

        let teacher_id = Uuid::new_v4();
        agent.senses.vision.visible_agents.insert(teacher_id);
        agent.state.position = (0, 0, 0);

        use crate::agents::{Relationship, RelationshipType};
        agent.relationships.add_relationship(Relationship::new(teacher_id, RelationshipType::Parent));

        // Observe many times to be ready to adopt
        for i in 0..10 {
            agent.observe_action(
                &teacher_id,
                (3, 0, 0),
                ActionType::Mining,
                true,
                format!("mined {}", i),
                i as u64,
            );
        }

        let adopted = auto_adopt_ready_behaviors(&mut agent);
        assert!(!adopted.is_empty());
        assert!(adopted.iter().any(|(id, action)| *id == teacher_id && *action == ActionType::Mining));
    }

    #[test]
    fn test_get_learning_stats() {
        let agent = Agent::new(AgentConfig::default());
        let stats = get_learning_stats(&agent);

        assert_eq!(stats.total_adopted, 0);
        assert_eq!(stats.ready_to_adopt, 0);
        assert_eq!(stats.learning_rate, 1.0);
    }
}
