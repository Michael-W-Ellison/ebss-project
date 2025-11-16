# Profession System Documentation

## Overview

The profession system assigns specialized roles to agents when they reach adulthood, unlocking profession-specific crafting recipes and providing a framework for economic specialization within the simulation. Each profession grants access to unique recipes and represents a distinct role in the agent society.

## System Architecture

### Core Components

1. **JobType Enum** (`src/agents/profession.rs`)
   - Defines 40+ profession types
   - Used as agent profession identifier
   - Includes Unemployed state for children/unassigned agents

2. **Agent Profession Field** (`src/agents/agent.rs:574`)
   ```rust
   pub struct Agent {
       pub profession: JobType,
       // ... other fields
   }
   ```

3. **Assignment Logic** (`src/agents/agent.rs:1260-1318`)
   - `assign_profession()` - Weighted random selection
   - `should_assign_profession()` - Checks for assignment eligibility

4. **Population Integration** (`src/agents/population.rs:107-112`)
   - Triggers profession assignment during population tick
   - Called after agent updates, before other processing

5. **Recipe Integration** (`src/analytics/mod.rs:621-707`)
   - Profession-specific recipes via `get_job_recipes()`
   - Combined with simple recipes for all agents

## Profession Assignment

### When Professions Are Assigned

Professions are assigned when an agent:
1. Reaches Adult life stage (transitions from Adolescent)
2. Has profession == JobType::Unemployed
3. Is processed during population tick

```rust
// Triggered in population.rs tick() method
for agent in &mut self.agents {
    if agent.should_assign_profession() {
        agent.assign_profession();
    }
}
```

### Assignment Algorithm

The system uses weighted random selection to ensure realistic profession distribution:

```rust
pub fn assign_profession(&mut self) {
    let profession_weights = vec![
        (JobType::Farmer, 15),       // 15% weight - Most common
        (JobType::Woodcutter, 10),   // 10% weight - Common
        (JobType::Miner, 8),         // 8% weight - Common
        (JobType::Carpenter, 8),     // 8% weight - Common
        (JobType::Stonemason, 6),    // 6% weight - Common
        (JobType::Blacksmith, 5),    // 5% weight - Important
        (JobType::Baker, 5),         // 5% weight - Important
        (JobType::Hunter, 4),        // 4% weight - Useful
        (JobType::Herder, 4),        // 4% weight - Useful
        (JobType::Fisher, 3),        // 3% weight - Useful
        (JobType::Miller, 3),        // 3% weight - Support
        (JobType::Butcher, 3),       // 3% weight - Support
        (JobType::Tanner, 2),        // 2% weight - Support
        (JobType::Weaver, 2),        // 2% weight - Support
        (JobType::Potter, 2),        // 2% weight - Support
        (JobType::Brewer, 1),        // 1% weight - Luxury
        (JobType::Cook, 1),          // 1% weight - Luxury
        (JobType::Armorer, 1),       // 1% weight - Specialized
        (JobType::Laborer, 10),      // 10% weight - Fallback
    ];

    let total_weight: u32 = profession_weights.iter().map(|(_, w)| w).sum();
    let roll = rng.gen_range(0..total_weight);
    let mut cumulative = 0;

    for (job, weight) in &profession_weights {
        cumulative += weight;
        if roll < cumulative {
            self.profession = *job;
            return;
        }
    }

    self.profession = JobType::Laborer; // Fallback
}
```

### Profession Categories

**Common Professions (High Weight)**
- Farmer (15) - Agricultural production, food growing
- Woodcutter (10) - Wood harvesting and processing
- Laborer (10) - General labor, multiple tasks
- Miner (8) - Mining and ore extraction
- Carpenter (8) - Wood construction and furniture

**Important Professions (Medium Weight)**
- Stonemason (6) - Stone construction
- Blacksmith (5) - Metal tool and weapon crafting
- Baker (5) - Bread and baked goods production
- Hunter (4) - Hunting and meat gathering
- Herder (4) - Animal husbandry

**Support Professions (Low Weight)**
- Fisher (3) - Fishing and aquatic resources
- Miller (3) - Grain processing
- Butcher (3) - Meat processing
- Tanner (2) - Leather processing
- Weaver (2) - Cloth and textile production
- Potter (2) - Ceramic production

**Luxury/Specialized Professions (Very Low Weight)**
- Brewer (1) - Alcoholic beverage production
- Cook (1) - Food preparation and cooking
- Armorer (1) - Armor crafting

## Recipe Integration

### How Professions Affect Crafting

When an agent performs `Action::Craft`, the system:

1. **Retrieves profession-specific recipes**
   ```rust
   let agent_profession = self.population.agents[agent_index].profession;
   let mut profession_recipes = get_job_recipes(agent_profession);
   ```

2. **Combines with simple recipes**
   ```rust
   let simple_recipes: Vec<Recipe> = vec![
       // Basic tools anyone can craft
       Recipe::simple_recipe("wooden_axe", /* ... */),
       Recipe::simple_recipe("stone_pickaxe", /* ... */),
   ];
   profession_recipes.extend(simple_recipes);
   ```

3. **Searches combined recipe list**
   - Agent can craft any simple recipe (universal access)
   - Agent can craft profession-specific recipes
   - Other profession recipes are unavailable

### Recipe Access Examples

**Farmer** can craft:
- Simple recipes (wooden tools, stone tools, basic items)
- Farmer recipes (farming tools, crop processing equipment)
- Cannot craft blacksmith recipes, carpenter recipes, etc.

**Blacksmith** can craft:
- Simple recipes (wooden tools, stone tools, basic items)
- Blacksmith recipes (advanced metal tools, weapons, armor)
- Cannot craft farmer recipes, baker recipes, etc.

**Unemployed** (children, unassigned) can craft:
- Simple recipes only
- No profession-specific recipes

## Profession Recipe System

### Recipe Categories

The `get_job_recipes()` function in `src/world/production.rs` returns profession-specific recipes:

```rust
pub fn get_job_recipes(job: JobType) -> Vec<Recipe> {
    match job {
        JobType::Farmer => get_farmer_recipes(),
        JobType::Blacksmith => get_blacksmith_recipes(),
        JobType::Carpenter => get_carpenter_recipes(),
        JobType::Baker => get_baker_recipes(),
        JobType::Miner => get_miner_recipes(),
        JobType::Woodcutter => get_woodcutter_recipes(),
        // ... 40+ more professions
        JobType::Unemployed => vec![], // No special recipes
        _ => vec![], // Unknown professions get no recipes
    }
}
```

### Example Profession Recipes

**Blacksmith Recipes** (from production.rs):
- Iron tools (iron axe, iron pickaxe, iron hammer)
- Steel weapons (steel sword, steel spear, steel mace)
- Advanced armor (steel helmet, steel breastplate, steel greaves)
- Quality: Skill-based, Poor → Masterwork

**Farmer Recipes**:
- Farming implements (hoe, sickle, scythe)
- Crop processing tools (grain basket, seed bag)
- Animal care items (feed trough, water bucket)

**Carpenter Recipes**:
- Furniture (chair, table, bed, chest)
- Structural components (door, window frame, beam)
- Wooden mechanisms (wheelbarrow, ladder)

**Baker Recipes**:
- Bread types (white bread, rye bread, wheat bread)
- Baked goods (rolls, biscuits, flatbread)
- Pastries (if advanced ingredients available)

## Statistics and Monitoring

### Test Executable Integration

The test simulation (`src/bin/test_simulation.rs:402-441`) displays profession statistics:

```
Professions & Specialization:
  • Adults with Professions: 45/50
  • Top Professions:
    - Farmer: 8
    - Laborer: 6
    - Woodcutter: 5
    - Miner: 4
    - Carpenter: 4
    - Blacksmith: 3
    - Baker: 3
    - Stonemason: 3
    - Hunter: 2
    - Herder: 2
```

### Statistics Calculated

1. **Total Adults** - Count of agents with Adult life stage
2. **Adults with Professions** - Adults with profession != Unemployed
3. **Profession Distribution** - Count of agents per profession
4. **Top Professions** - Sorted by count, shows top 10
5. **Other Professions** - Summary of remaining professions

## Integration with Other Systems

### Crafting System

**Location**: `src/analytics/mod.rs:611-791`

Professions directly affect recipe availability:
```rust
Action::Craft { item_type } => {
    let agent_profession = self.population.agents[agent_index].profession;
    let mut profession_recipes = get_job_recipes(agent_profession);

    // Combine with simple recipes
    let simple_recipes = vec![/* ... */];
    profession_recipes.extend(simple_recipes);

    // Find matching recipe in combined list
    let recipe = all_recipes.iter().find(|r| /* ... */);
}
```

### Life Stage System

**Location**: `src/agents/agent.rs` (LifeStage enum)

Professions are assigned when transitioning to Adult:
- Infant → Child: No profession (Unemployed)
- Child → Adolescent: No profession (Unemployed)
- Adolescent → Adult: **Profession assigned here**
- Adult → Elder: Keeps existing profession

### Population Management

**Location**: `src/agents/population.rs:107-112`

Population tick triggers profession assignment:
```rust
pub fn tick(&mut self) {
    // ... update agents ...

    // Assign professions to new adults
    for agent in &mut self.agents {
        if agent.should_assign_profession() {
            agent.assign_profession();
        }
    }

    // ... process deaths, reproduction, etc ...
}
```

## Technical Implementation Details

### Data Structures

**Agent Struct** (`src/agents/agent.rs:574`):
```rust
pub struct Agent {
    pub id: Uuid,
    pub state: AgentState,
    pub drives: DriveState,
    pub body: Body,
    pub skills: Skills,
    pub inventory: Inventory,
    pub memory: Memory,
    pub relationships: RelationshipManager,
    pub profession: JobType,  // <-- Profession field
}
```

**Initialization** (`src/agents/agent.rs:604`):
```rust
impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: AgentState::new(config.state_config),
            // ... other initializations ...
            profession: JobType::Unemployed, // Start unemployed
        }
    }
}
```

### Assignment Check

**Location**: `src/agents/agent.rs:1311-1318`

```rust
pub fn should_assign_profession(&self) -> bool {
    use super::LifeStage;

    // Assign when:
    // 1. Agent is an adult
    // 2. Agent is currently unemployed
    self.state.life_stage == LifeStage::Adult &&
    self.profession == super::profession::JobType::Unemployed
}
```

This ensures:
- Only adults get professions
- Professions are assigned exactly once
- No re-assignment after initial assignment
- Children and adolescents remain unemployed

### Weighted Selection Algorithm

**Location**: `src/agents/agent.rs:1260-1309`

**Algorithm Steps**:
1. Define profession weights (total: 100)
2. Generate random number in range [0, total_weight)
3. Iterate through professions, accumulating weights
4. When cumulative weight exceeds random number, assign that profession
5. Fallback to Laborer if something goes wrong

**Weight Distribution**:
- Total weight: 100
- Highest: Farmer (15%)
- Lowest: Brewer, Cook, Armorer (1% each)
- Ensures realistic economic distribution

**Example Roll**:
```
Weights: Farmer(15), Woodcutter(10), Miner(8), ...
Total: 100
Roll: 42

Cumulative:
  0-14:   Farmer (no, 42 >= 15)
  15-24:  Woodcutter (no, 42 >= 25)
  25-32:  Miner (no, 42 >= 33)
  33-40:  Carpenter (no, 42 >= 41)
  41-45:  Stonemason (YES! 42 < 46)

Result: Stonemason assigned
```

## Usage Examples

### Example 1: New Agent Becomes Adult

```
Tick 1000: Agent (ID: a1b2c3, Age: 12) - Child
  - profession: Unemployed
  - life_stage: Child

Tick 5000: Agent (ID: a1b2c3, Age: 18) - Adolescent
  - profession: Unemployed
  - life_stage: Adolescent

Tick 7200: Agent (ID: a1b2c3, Age: 21) - Becomes Adult
  - Before tick: profession = Unemployed
  - During tick: should_assign_profession() = true
  - assign_profession() called
  - Weighted random selection: Farmer
  - After tick: profession = Farmer
```

### Example 2: Crafting with Profession

```
Agent: Adult Blacksmith (skill: Crafting level 5)

Action: Craft { item_type: "iron_sword" }

Step 1: Get profession recipes
  - get_job_recipes(Blacksmith) → [iron_sword, steel_sword, ...]

Step 2: Add simple recipes
  - profession_recipes.extend(simple_recipes)
  - Total: [iron_sword, steel_sword, wooden_axe, stone_pickaxe, ...]

Step 3: Find recipe
  - Search for "iron_sword" in combined recipes
  - Found: iron_sword recipe (from Blacksmith recipes)

Step 4: Check materials
  - Required: 3 iron_ingot
  - Agent inventory: 5 iron_ingot ✓

Step 5: Calculate quality
  - Skill level: 5
  - Skill value: (5 + 10) * 5 = 75
  - Quality: Excellent (70-84 range)

Step 6: Craft
  - Consume: 3 iron_ingot
  - Produce: 1 iron_sword (Excellent quality)
  - Experience: +4 (Excellent tier)

Result: SUCCESS - Iron Sword (Excellent) crafted
```

### Example 3: Crafting Without Profession Access

```
Agent: Adult Farmer (skill: Crafting level 5)

Action: Craft { item_type: "iron_sword" }

Step 1: Get profession recipes
  - get_job_recipes(Farmer) → [hoe, sickle, scythe, ...]

Step 2: Add simple recipes
  - profession_recipes.extend(simple_recipes)
  - Total: [hoe, sickle, wooden_axe, stone_pickaxe, ...]

Step 3: Find recipe
  - Search for "iron_sword" in combined recipes
  - NOT FOUND (iron_sword is Blacksmith-only)

Result: FAILURE - Recipe not known
```

## Profession List

### All Available Professions

From `src/agents/profession.rs`:

1. **Primary Production**
   - Farmer - Agriculture and crop production
   - Woodcutter - Lumber and wood harvesting
   - Miner - Ore and mineral extraction
   - Fisher - Fishing and aquatic resources
   - Hunter - Hunting and game procurement
   - Herder - Animal husbandry and livestock

2. **Processing & Crafting**
   - Miller - Grain processing and flour production
   - Butcher - Meat processing and preparation
   - Tanner - Leather processing and tanning
   - Weaver - Cloth and textile production
   - Potter - Ceramic and pottery crafting
   - Brewer - Alcoholic beverage production
   - Baker - Bread and baked goods production
   - Cook - Food preparation and cooking

3. **Construction & Fabrication**
   - Carpenter - Wooden construction and furniture
   - Stonemason - Stone construction and masonry
   - Blacksmith - Metal tool and weapon smithing
   - Armorer - Armor crafting and metalworking

4. **Services & Support**
   - Laborer - General labor and multiple tasks
   - Unemployed - No profession (default state)

5. **Additional Professions** (20+ more)
   - See `src/agents/profession.rs` for complete list
   - Includes specialized, luxury, and niche professions
   - Each with unique recipe sets and capabilities

## Future Enhancements

### Potential Improvements

1. **Skill-Based Assignment**
   - Assign professions based on agent skills
   - Higher skill in relevant area → higher profession weight
   - Example: High combat skill → more likely to become Hunter

2. **Profession Progression**
   - Allow profession advancement (Apprentice → Journeyman → Master)
   - Unlock additional recipes at higher tiers
   - Skill bonuses increase with profession level

3. **Economic Balancing**
   - Adjust profession weights based on population needs
   - More Farmers when food is scarce
   - More Blacksmiths when tools are needed

4. **Profession Changes**
   - Allow agents to change professions under certain conditions
   - Skill reset or transfer mechanics
   - Retirement system for Elder life stage

5. **Profession Buildings**
   - Require specific buildings for profession assignment
   - Blacksmith needs Forge, Miller needs Mill, etc.
   - Building construction unlocks profession access

6. **Apprenticeship System**
   - Youth learn from adult profession holders
   - Skill transfer through relationships
   - Higher chance of same profession as mentor

7. **Profession Skills**
   - Add profession-specific skill bonuses
   - Blacksmiths craft faster/better quality
   - Farmers harvest more resources

## Troubleshooting

### Issue: Professions Not Assigned

**Symptoms**:
- All agents remain Unemployed
- Adult agents never get professions
- Statistics show 0/X adults with professions

**Possible Causes**:
1. Population tick not calling assignment logic
2. Agents not reaching Adult life stage
3. Assignment conditions never satisfied

**Solution**:
Check `src/agents/population.rs:107-112`:
```rust
// This code should exist in tick() method
for agent in &mut self.agents {
    if agent.should_assign_profession() {
        agent.assign_profession();
    }
}
```

### Issue: All Agents Get Same Profession

**Symptoms**:
- Every agent becomes Farmer
- No profession diversity
- Weighted selection not working

**Possible Causes**:
1. RNG not seeded properly
2. Weight calculation error
3. Cumulative weight logic broken

**Solution**:
Verify weighted selection in `src/agents/agent.rs:1260-1309`:
```rust
let total_weight: u32 = profession_weights.iter().map(|(_, w)| w).sum();
let roll = rng.gen_range(0..total_weight);  // Must be random each time
```

### Issue: Profession Recipes Not Available

**Symptoms**:
- Blacksmith can't craft blacksmith recipes
- Agents can only craft simple recipes
- Profession-specific recipes always "not found"

**Possible Causes**:
1. Recipe integration not working
2. get_job_recipes() returning empty vec
3. Recipe name mismatch in crafting action

**Solution**:
Check recipe integration in `src/analytics/mod.rs:621-707`:
```rust
let agent_profession = self.population.agents[agent_index].profession;
let mut profession_recipes = get_job_recipes(agent_profession);
profession_recipes.extend(simple_recipes);  // Must combine both
```

### Issue: Compilation Error - JobType Not Found

**Symptoms**:
```
error[E0433]: failed to resolve: use of undeclared type `JobType`
```

**Possible Causes**:
1. Missing import in test executable
2. JobType not exported from module
3. Incorrect module path

**Solution**:
Add import to file using JobType:
```rust
use ebss_sim::agents::profession::JobType;
```

## Performance Considerations

### Profession Assignment

- **Cost**: O(1) per agent per tick
- **Triggered**: Only when agent becomes adult (once per lifetime)
- **Impact**: Negligible - happens infrequently
- **Optimization**: Already optimal, no improvement needed

### Recipe Lookup

- **Cost**: O(R) where R = number of recipes
- **Triggered**: Every crafting action
- **Impact**: Low - recipe lists are small (<50 items)
- **Optimization**: Consider HashMap for large recipe sets

### Statistics Collection

- **Cost**: O(N) where N = number of agents
- **Triggered**: Every display update (configurable interval)
- **Impact**: Low - simple HashMap counting
- **Optimization**: None needed for current scale

## Conclusion

The profession system provides:
- **Economic Specialization** - Agents have distinct roles
- **Recipe Gating** - Advanced recipes require specific professions
- **Realistic Distribution** - Weighted selection ensures balance
- **Scalability** - Supports 40+ profession types
- **Extensibility** - Easy to add new professions and recipes

Professions integrate seamlessly with:
- Crafting system (recipe access)
- Life stage system (assignment timing)
- Population management (tick integration)
- Statistics display (monitoring)

The system is production-ready and fully functional.
