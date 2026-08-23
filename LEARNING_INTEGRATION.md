# Learning Loop Integration Points

## Critical Integration Points for Learning Loop Implementation

### 1. BEHAVIOR TREE EXECUTION PIPELINE

**Current Code** (from `/home/user/ebss-project/src/core/behavior_tree.rs:126-137`):
```rust
pub fn execute(&mut self) -> ExecutionResult {
    self.total_executions += 1;
    let result = self.execute_node(&mut self.root.clone());
    
    if result == ExecutionResult::Success {
        self.total_successes += 1;
    }
    
    self.root.update_weight(result);
    result
}
```

**Key Points:**
- Already tracks total_executions and total_successes
- Automatically updates weights on each execution
- Returns ExecutionResult for outcome processing
- **LEARNING ALREADY INTEGRATED** - Just needs action execution wrapped around it

---

### 2. DRIVE SELECTION FOR BEHAVIOR TREES

**Current Code** (from `/home/user/ebss-project/src/core/drives.rs:235-241`):
```rust
pub fn most_urgent(&self) -> Option<&Drive> {
    self.drives
        .iter()
        .filter(|d| d.is_active())
        .max_by(|a, b| a.urgency().partial_cmp(&b.urgency()).unwrap())
}
```

**Integration Point:**
This is where agent decision-making MUST happen:
```rust
// Pseudocode for integration
if let Some(urgent_drive) = agent.drives.most_urgent() {
    // Find behavior tree matching this drive
    let tree_index = match urgent_drive.drive_type {
        DriveType::Hunger => find_tree_for_hunger(&agent),
        DriveType::Rest => find_tree_for_rest(&agent),
        // ... etc
    };
    
    // Execute the tree
    let result = agent.behavior_trees[tree_index].execute();
    
    // Tree weights already updated by execute()
    // Now: convert result to action and execute against environment
}
```

---

### 3. WEIGHT REINFORCEMENT MECHANISM

**Current Code** (from `/home/user/ebss-project/src/core/behavior_tree.rs:64-83`):
```rust
pub fn update_weight(&mut self, result: ExecutionResult) {
    self.execution_count += 1;
    
    match result {
        ExecutionResult::Success => {
            self.success_count += 1;
            self.weight *= 1.1; // Increase weight by 10%
        }
        ExecutionResult::Failure => {
            self.weight *= 0.9; // Decrease weight by 10%
        }
        ExecutionResult::Running => {
            // No weight change for running state
        }
    }
    
    // Clamp weight between 0.1 and 10.0
    self.weight = self.weight.clamp(0.1, 10.0);
}
```

**Current Learning Behavior:**
- Success → weight multiplied by 1.1 (exponential increase)
- Failure → weight multiplied by 0.9 (exponential decrease)
- Running → no change
- Automatic clamping prevents runaway values

**For Learning Loop Integration:**
- This mechanism is ALREADY ACTIVE
- Just need to ensure realistic Success/Failure feedback from environment

---

### 4. DRIVE SATISFACTION & FEEDBACK

**Current Code** (from `/home/user/ebss-project/src/core/drives.rs:181-189`):
```rust
pub fn satisfy(&mut self) {
    self.value = 0.0;
}

pub fn partial_satisfy(&mut self, amount: f32) {
    self.decrease(amount);
}
```

**Integration Point:**
After action execution succeeds, application must:
```rust
// After agent performs action successfully
match action_type {
    Action::Eat(food) => {
        agent.drives.get_mut(DriveType::Hunger)?.partial_satisfy(0.3);
    }
    Action::Sleep => {
        agent.drives.get_mut(DriveType::Rest)?.satisfy();
    }
    // ... etc
}
```

---

### 5. GENETIC INHERITANCE (Already Implemented)

**Current Code** (from `/home/user/ebss-project/src/core/behavior_tree.rs:202-208`):
```rust
pub fn clone_with_pruning(&self, min_weight: f32) -> Self {
    let mut cloned = self.clone();
    cloned.id = Uuid::new_v4(); // New ID for offspring
    cloned.prune(min_weight);
    cloned
}
```

**Already Available:**
- Clone parent's behavior tree
- Remove low-weight branches
- Assign new ID for genetic tracking
- Mutation can be added here: modify weights before returning

---

## LEARNING LOOP ARCHITECTURE

### Complete Flow with Integration Points:

```
┌─────────────────────────────────────────────────────────────────┐
│ SIMULATION TICK                                                  │
└─────────────────────────────────────────────────────────────────┘

1. UPDATE DRIVES (Already Implemented)
   ├─ drive_state.tick()  [drives.rs:244-248]
   └─ Each drive increases by base_accumulation_rate

2. SELECT MOST URGENT DRIVE (Already Implemented)
   ├─ most_urgent_drive = drives.most_urgent()  [drives.rs:235-241]
   └─ Returns Option<&Drive> with highest urgency

3. FIND MATCHING BEHAVIOR TREE (MISSING - TO IMPLEMENT)
   ├─ tree = find_tree_for_drive(agent, urgent_drive)
   └─ Match drive type to behavior tree

4. EXECUTE BEHAVIOR TREE (Already Implemented)
   ├─ result = tree.execute()  [behavior_tree.rs:126-137]
   ├─ Updates weights automatically
   ├─ Increments execution counters
   └─ Returns ExecutionResult::Success/Failure/Running

5. EXECUTE ACTION IN ENVIRONMENT (MISSING - TO IMPLEMENT)
   ├─ action = tree_result_to_action(result, tree_type)
   ├─ Apply action to world (move, gather, craft, etc.)
   └─ Get outcome (success/partial success/failure)

6. PROVIDE FEEDBACK (MISSING - TO IMPLEMENT)
   ├─ Update tree weights based on environment feedback
   ├─ urgent_drive.partial_satisfy(reward_amount)
   └─ Log statistics for analytics

7. HANDLE LEARNING UPDATES (Partially Implemented)
   ├─ Tree weights already updated in step 4
   ├─ Success rate calculated: success_count / execution_count
   └─ Pruning available for offspring via clone_with_pruning()
```

---

## DATA STRUCTURES READY FOR LEARNING

### Agent State for Learning
```rust
pub struct Agent {
    pub id: Uuid,                        // Track individual
    pub state: AgentState,               // Position, health
    pub drives: DriveState,              // 15 drives with values
    pub behavior_trees: Vec<BehaviorTree>, // Multiple strategies
    pub memory: Memory,                  // (Stub - for future use)
}
```

### Behavior Tree Learning Metrics (Already Collected)
```rust
pub struct BehaviorNode {
    pub id: Uuid,
    pub node_type: NodeType,
    pub weight: f32,           // Learning parameter (0.1 to 10.0)
    pub children: Vec<BehaviorNode>,
    pub execution_count: u32,   // Total executions
    pub success_count: u32,     // Successful outcomes
}
```

### Per-Tree Learning Metrics
```rust
pub struct BehaviorTree {
    pub id: Uuid,
    pub root: BehaviorNode,
    pub name: String,
    pub total_executions: u32,   // Track learning progress
    pub total_successes: u32,    // Calculate success rate
}
```

---

## GAPS TO FILL FOR COMPLETE LEARNING LOOP

### Tier 1: CRITICAL (Must Have)
1. **Action Selection Logic**
   - File: `/home/user/ebss-project/src/agents/agent.rs`
   - Implement: Select behavior tree based on most urgent drive

2. **Action Execution**
   - Files: Need new action execution system
   - Implement: Convert tree results to world actions

3. **Feedback Integration**
   - File: `/home/user/ebss-project/src/analytics/mod.rs`
   - Implement: Simulation tick loop

### Tier 2: HIGH PRIORITY
4. **World Grid System**
   - File: `/home/user/ebss-project/src/world/mod.rs`
   - Implement: Spatial partitioning, resource locations

5. **Environment Interaction**
   - File: `/home/user/ebss-project/src/environment/mod.rs`
   - Implement: Action success determination

### Tier 3: MEDIUM PRIORITY
6. **Memory System**
   - File: `/home/user/ebss-project/src/core/memory.rs`
   - Implement: Store learned locations, recipes, agent info

7. **Analytics**
   - File: `/home/user/ebss-project/src/analytics/mod.rs`
   - Implement: Logging, statistics, emergence detection

---

## CODE SNIPPETS FOR INTEGRATION

### Template 1: Agent Decision Loop
```rust
// In agent.rs or a new decision module
pub fn decide_and_act(agent: &mut Agent) -> ExecutionResult {
    // Step 1: Get most urgent drive
    if let Some(urgent_drive) = agent.drives.most_urgent() {
        // Step 2: Find tree for this drive (TODO)
        let tree_idx = find_best_tree_for(agent, urgent_drive.drive_type);
        
        if let Some(tree) = agent.behavior_trees.get_mut(tree_idx) {
            // Step 3: Execute tree (ALREADY WORKS)
            let result = tree.execute();
            
            // Step 4: Weights auto-updated by tree.execute()
            
            // Step 5: Return result for action execution
            return result;
        }
    }
    
    ExecutionResult::Failure
}
```

### Template 2: Feedback Integration
```rust
// After action execution in environment
pub fn apply_learning_feedback(
    agent: &mut Agent,
    drive_type: DriveType,
    satisfaction_amount: f32,
) {
    // Reduce the drive that was satisfied
    if let Some(drive) = agent.drives.get_mut(drive_type) {
        drive.partial_satisfy(satisfaction_amount);
    }
    
    // Tree weights already updated by tree.execute()
    // Success rate automatically calculated
}
```

### Template 3: Genetic Inheritance
```rust
// Create offspring from parent
pub fn reproduce(parent: &Agent) -> Agent {
    let mut offspring = Agent::new(AgentConfig::default());
    
    // Inherit behavior trees
    offspring.behavior_trees = parent.behavior_trees
        .iter()
        .map(|tree| {
            // Clone with pruning (removes low-weight branches)
            tree.clone_with_pruning(0.5) // 0.5 = minimum weight threshold
        })
        .collect();
    
    // Offspring gets copy of parent drives with possible mutations
    offspring.drives = parent.drives.clone();
    
    offspring
}
```

---

## TESTING POINTS FOR LEARNING

1. **Drive Accumulation**
   ```
   Test: drives.tick() increases all values
   Expected: value increases by base_accumulation_rate each tick
   ```

2. **Drive Activation**
   ```
   Test: is_active() returns true when value >= threshold
   Expected: Hunger (0.7 threshold) activates at 0.7+
   ```

3. **Weight Reinforcement**
   ```
   Test: Tree weights change based on execution result
   Expected: Success increases, Failure decreases
   ```

4. **Pruning**
   ```
   Test: clone_with_pruning removes low-weight branches
   Expected: Offspring has fewer/simpler trees
   ```

5. **Most Urgent Drive**
   ```
   Test: most_urgent() returns highest urgency active drive
   Expected: Returns drive with max(value * weight) where is_active
   ```

---

## EXPECTED LEARNING BEHAVIORS (After Implementation)

1. **Trial & Error**
   - Agent tries random actions
   - Successful ones reinforced (weight up)
   - Failed ones discouraged (weight down)
   - Tree naturally prunes unsuccessful branches

2. **Drive-Driven Specialization**
   - Agents develop trees for each drive
   - Social drive → follow other agents
   - Hunger → seek food locations (memory)
   - Curiosity → explore new areas

3. **Genetic Evolution**
   - Offspring inherit parent's learned behaviors
   - Pruning removes weak strategies
   - Population becomes more specialized over generations
   - Emergent division of labor

4. **Personality Variation**
   - Random drive weights create different priorities
   - Some agents social, others exploratory
   - Affects which trees develop and are reinforced

