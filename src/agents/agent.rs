// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, DriveState, DriveType, Memory, BehaviorNode, NodeType};
use crate::environment::{Action, ActionResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub random_weights: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { random_weights: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub health: f32,
    pub position: (i32, i32, i32),
    pub energy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub state: AgentState,
    pub drives: DriveState,
    pub behavior_trees: Vec<BehaviorTree>,
    pub memory: Memory,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        let mut agent = Self {
            id: Uuid::new_v4(),
            state: AgentState {
                health: 100.0,
                position: (0, 0, 0),
                energy: 100.0,
            },
            drives: if config.random_weights {
                DriveState::with_random_weights()
            } else {
                DriveState::new()
            },
            behavior_trees: Vec::new(),
            memory: Memory::new(),
        };

        // Initialize default behavior trees for each drive
        agent.initialize_behavior_trees();
        agent
    }

    /// Initialize default behavior trees for each drive type
    fn initialize_behavior_trees(&mut self) {
        for drive_type in DriveType::all() {
            let tree = Self::create_default_tree_for_drive(drive_type);
            self.behavior_trees.push(tree);
        }
    }

    /// Create a default behavior tree for a specific drive
    fn create_default_tree_for_drive(drive_type: DriveType) -> BehaviorTree {
        let root = match drive_type {
            DriveType::Hunger => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("eat_stored_food".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("gather_food".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("hunt".to_string())));
                selector
            }
            DriveType::Rest => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_shelter".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("sleep".to_string())));
                sequence
            }
            DriveType::Shelter => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("find_shelter".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("build_shelter".to_string())));
                selector
            }
            DriveType::Construction => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_materials".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("build_structure".to_string())));
                sequence
            }
            DriveType::Industry => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("mine_resources".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("process_materials".to_string())));
                selector
            }
            DriveType::Curiosity => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("explore".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("experiment".to_string())));
                selector
            }
            DriveType::Social => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("find_agents".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("socialize".to_string())));
                selector
            }
            DriveType::Utility => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_resources".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("craft_tools".to_string())));
                sequence
            }
            DriveType::Preparedness => {
                BehaviorNode::new(NodeType::Action("store_resources".to_string()))
            }
            DriveType::Sustenance => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("plant_crops".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("harvest".to_string())));
                selector
            }
            DriveType::Safety => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("seek_shelter".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("craft_weapon".to_string())));
                selector
            }
            DriveType::Reproduction => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_resources".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("reproduce".to_string())));
                sequence
            }
            DriveType::Luxury => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("seek_luxury".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("decorate".to_string())));
                selector
            }
        };

        BehaviorTree::new(format!("{:?}_tree", drive_type), root)
    }

    /// Select the most appropriate behavior tree based on current drive state
    pub fn select_behavior_tree(&mut self) -> Option<&mut BehaviorTree> {
        // Get the most urgent drive
        let most_urgent_drive = self.drives.most_urgent()?;

        // Find the behavior tree for this drive type
        self.behavior_trees
            .iter_mut()
            .find(|tree| tree.name.starts_with(&format!("{:?}", most_urgent_drive.drive_type)))
    }

    /// Convert a behavior tree action into an actual environment action
    pub fn action_from_tree_result(&self, action_name: &str) -> Action {
        match action_name {
            "eat_stored_food" | "gather_food" | "hunt" => Action::Eat { food_type: "generic".to_string() },
            "sleep" => Action::Sleep { duration: 10 },
            "find_shelter" | "seek_shelter" => Action::Move { target: self.find_nearest_shelter() },
            "build_shelter" | "build_structure" => Action::Build {
                structure_type: "shelter".to_string(),
                position: self.state.position
            },
            "mine_resources" | "gather_resources" => Action::Gather { resource_type: "generic".to_string() },
            "process_materials" => Action::Craft { item_type: "processed_material".to_string() },
            "explore" | "experiment" => Action::Explore { direction: self.random_direction() },
            "find_agents" | "socialize" => Action::Socialize { target_agent_id: Uuid::nil() },
            "craft_tools" | "craft_weapon" => Action::Craft { item_type: "tool".to_string() },
            "store_resources" => Action::Store { item_type: "resource".to_string(), amount: 1 },
            "plant_crops" | "harvest" => Action::Gather { resource_type: "food".to_string() },
            "reproduce" => Action::Wait, // Special handling needed
            "seek_luxury" | "decorate" => Action::Gather { resource_type: "luxury".to_string() },
            _ => Action::Wait,
        }
    }

    /// Process feedback from action execution
    pub fn apply_feedback(&mut self, action_result: &ActionResult, drive_type: DriveType) {
        // Update drive satisfaction
        if let Some(drive) = self.drives.get_mut(drive_type) {
            if action_result.success {
                drive.partial_satisfy(action_result.drive_satisfaction);
            }
        }
    }

    /// Tick function for agent updates
    pub fn tick(&mut self) {
        // Update all drives
        self.drives.tick();

        // Update energy
        self.state.energy = (self.state.energy - 0.1).max(0.0);
    }

    // Helper methods
    fn find_nearest_shelter(&self) -> (i32, i32, i32) {
        // Placeholder: return a position near the agent
        (self.state.position.0, self.state.position.1, self.state.position.2)
    }

    fn random_direction(&self) -> (i32, i32, i32) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (
            rng.gen_range(-1..=1),
            rng.gen_range(-1..=1),
            0
        )
    }
}
