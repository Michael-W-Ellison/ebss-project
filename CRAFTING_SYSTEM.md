# Crafting & Production System

This document describes the comprehensive crafting and production mechanics that allow agents to transform raw materials into useful tools, weapons, and other items.

## Overview

The crafting system enables agents to:
- Craft tools and weapons from gathered materials
- Gain crafting experience and improve skill levels
- Produce higher quality items as skills improve
- Progress from wooden tools → stone tools → iron tools
- Convert raw resources into finished goods

## The stone-age chain

**Location**: `src/environment/making.rs`

Everything below this section takes its inputs as `ResourceType` — things dug
or picked out of the ground — and its outputs as `ItemType`. That is a table of
one-step recipes and it cannot express a chain: there is no way to say that the
thing you made last is what you need for the thing you are making now.
Everything a people could do therefore had to be reachable in a single move
from raw material, which is why the whole of their toolmaking was three logs
into an axe.

`making.rs` takes named things and makes a named thing, so what one step
produces the next can consume:

```text
flax or cotton                  -> lashing
stone                           -> a knapped tip
stick + tip + lashing           -> spear
stick + tip + lashing           -> hand axe
tip + lashing                   -> stone knife
```

`what_to_do_first` works back from the thing wanted through what it is short
of and returns the first step whose makings are actually in the pack: a man who
wants a spear and holds flax is told to twist cordage. `what_is_wanting` is the
other half — when no step can be taken, it names the raw thing to go and fetch.
`Agent::what_i_would_make` and `Agent::what_i_must_find` are what the Utility
drive asks; `Action::Craft` resolves a named step here before falling through
to the table below.

A tool made this way is as good as the hands that made it: `Quality::from_hand`
and `how_long_this_one_lasts` set what it is worth and how long it lasts, from
the maker's hand at the trade the step calls for — a spear is only as good as
the man who lashed it, whoever ends up throwing it. Quality tells on how well
the tool works as well as on how long it lasts, squeezed into the band
`Agent::WHAT_GOOD_WORK_IS_WORTH` so that the tenth spear is half again the
first and not three times it. A step that costs more to do teaches more, so a
spear is worth two flakes of practice. `Making::obvious` marks what a
stone-age people arrives already knowing — everything in the table today, with
the field there for the things they will have to find out.

### What a people has to find out

`Making::obvious` marks what a stone-age people arrives already knowing. Three
steps are not obvious, and they are the specification's "rock + fire = ?":

```text
iron (a bright stone) + a fire   -> a shiny lump
shiny lump + a hammerstone       -> a metal blade
metal blade + lashing            -> a metal knife
```

Nothing about a bright stone tells anybody what it is for. `Making::over_a_fire`
and `Making::wants_in_hand` are the conditions half: a man learns nothing from
the stone until he is sitting at a burning fire holding it, and then he learns
it all at once. `Simulation::somebody_notices_something` runs once a tick and
rolls for exactly that — ingredients in the pack, conditions met, curiosity
above `CURIOUS_ENOUGH_TO_NOTICE`, at odds scaled by the hand at the trade.

Knowledge is one person's, kept in `Agent::found_out`. `Agent::knows_how_to`
gates both the doing and the planning: `what_to_do_first_knowing` will not
route a chain through a step this agent has never seen, so a pack full of iron
is not a step towards a knife nobody has seen made.

Two things close the loop. `Agent::what_i_would_try_out` makes a curious agent
repeat a trick it has just learned with no use in mind for what comes out —
without it nobody would ever hold a shiny lump, because nothing in a settlement
wants one, and the blade could never be found out. And what a pair of hands
wants is stated as the *work* it wants to be equipped for
(`WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO`) rather than as a named tool, so a man who
has seen metal wants a metal knife and a man who has not wants a stone one,
from the same line of code.

### What a tool is for

`making::EVERY_TOOL` says which work each tool helps with and by how much: an
axe is up to 1.8x on timber and 1.5x on stone, a spear 2.0x in a hunt and 1.6x
in the shallows, a knife 1.8x on a carcass and 1.3x at the bench. Half the gain
survives to the end of the tool's life and the other half wears away with it.
`Agent::how_much_my_tools_help` is what the gathering, hunting, butchering and
fishing paths multiply by; `Agent::wear_what_i_worked_with` is what takes a
piece of work out of the tool afterwards. Twenty-five to forty pieces of work
is a stone-age tool's whole life.

## System Components

### 1. Recipe System

**Location**: `src/world/production.rs`

Recipes define the transformation of input materials into output items.

```rust
pub struct Recipe {
    pub name: &'static str,
    pub job: JobType,
    pub inputs: Vec<ResourceRequirement>,
    pub outputs: Vec<ProductionOutput>,
    pub base_time: u32,  // Base time in ticks
}
```

**Resource Requirement**:
```rust
pub struct ResourceRequirement {
    pub resource_type: ResourceType,
    pub amount: u32,
}
```

**Production Output**:
```rust
pub struct ProductionOutput {
    pub item_type: ItemType,
    pub base_amount: u32,
}
```

### 2. Quality System

**Location**: `src/world/production.rs:8-51`

Quality is determined by the crafter's skill level (0-100):

| Quality | Skill Range | Output Multiplier | Time Multiplier | Description |
|---------|------------|-------------------|-----------------|-------------|
| Poor | 0-20 | 0.8x (80%) | 1.2x (slower) | Low skill work |
| Common | 21-40 | 1.0x (100%) | 1.0x (normal) | Average quality |
| Good | 41-60 | 1.2x (120%) | 0.85x (faster) | Above average |
| Excellent | 61-80 | 1.4x (140%) | 0.7x (faster) | High quality |
| Masterwork | 81-100 | 1.6x (160%) | 0.5x (fastest) | Master craftsman |

**Quality Effects**:
- **Output Multiplier**: Better quality = more items produced
- **Time Multiplier**: Better quality = faster crafting
- **Experience Gain**: Better quality = more skill improvement

### 3. Skill System

**Location**: `src/agents/skills.rs`

#### Crafting Skill

Agents have a **Crafting** skill that ranges from **-10 to 10**.

**Skill Progression**:
```rust
// Skill starts at -10 (complete beginner)
// Gain experience through crafting
// 100 experience points = 1 skill level
// Maximum skill level = 10 (master)
```

**Skill Level Conversion**:
```rust
// Agent skill level (-10 to 10) → Production skill (0 to 100)
skill_value = (skill_level + 10) * 5

// Examples:
// -10 → 0   (Poor quality)
//   0 → 50  (Common/Good quality)
//  10 → 100 (Masterwork quality)
```

**Experience Gain**:
```rust
match quality {
    Poor => 1 experience point,
    Common => 2 experience points,
    Good => 3 experience points,
    Excellent => 4 experience points,
    Masterwork => 5 experience points,
}
```

**Skill Categories**:
| Skill Level | Category | Title |
|------------|----------|-------|
| -10 to -6 | None | (No title) |
| -5 to -1 | Low | Apprentice |
| 0 to 5 | Medium | Journeyman |
| 6 to 10 | High | Master |

### 4. Available Recipes

**Location**: `src/analytics/mod.rs:604-669`

Currently implemented simple recipes (available to all agents):

#### Wooden Tools (3 wood each)
- **Wooden Axe** - Chopping tool
- **Wooden Pickaxe** - Mining tool
- **Wooden Hammer** - Building tool
- **Base Time**: 80 ticks

#### Stone Tools (2 stone + 1 wood each)
- **Stone Axe** - Better chopping
- **Stone Pickaxe** - Better mining
- **Base Time**: 90 ticks

#### Iron Tools (2 iron + 1 wood each)
- **Iron Axe** - Best chopping
- **Iron Pickaxe** - Best mining
- **Iron Hammer** - Best building
- **Base Time**: 100 ticks

#### Iron Weapons (3 iron + 1 wood)
- **Iron Sword** - Combat weapon
- **Base Time**: 120 ticks

### 5. Crafting Process

**Location**: `src/analytics/mod.rs:598-782`

#### Step-by-Step Flow

```
1. Agent's Utility drive accumulates → triggers Action::Craft
   ↓
2. Find recipe matching requested item type
   ↓
3. Check agent inventory for required materials
   ↓
4. If missing materials → ActionResult::failure
   ↓
5. Get agent's Crafting skill level (-10 to 10)
   ↓
6. Convert skill level to skill value (0 to 100)
   ↓
7. Determine quality based on skill value
   ↓
8. Calculate actual outputs (base_amount * quality_multiplier)
   ↓
9. Consume materials from inventory
   ↓
10. Add crafted items to inventory
   ↓
11. Grant experience points based on quality
   ↓
12. Update Crafting skill (auto-level if >= 100 exp)
   ↓
13. Return ActionResult with success message
```

#### Code Example

```rust
Action::Craft { item_type } => {
    // Find matching recipe
    let recipe = simple_recipes.iter().find(|r| {
        r.outputs.iter().any(|output| {
            format!("{:?}", output.item_type).to_lowercase() == item_type.to_lowercase()
        })
    });

    // Check materials
    for req in &recipe.inputs {
        if !agent.inventory.has_item(req.resource_type, req.amount) {
            return ActionResult::failure("Missing materials");
        }
    }

    // Get skill and determine quality
    let skill_level = agent.skills.get_skill_mut(SkillType::Crafting).level;
    let skill_value = ((skill_level + 10) * 5) as u8;
    let quality = ProductionQuality::from_skill(skill_value);

    // Calculate outputs
    let outputs = recipe.calculate_output(quality);

    // Consume materials and add crafted items
    for req in &recipe.inputs {
        agent.inventory.remove_item(req.resource_type, req.amount);
    }
    for (item, quantity) in outputs {
        agent.inventory.add_item(item, quantity);
    }

    // Grant experience
    agent.skills.get_skill_mut(SkillType::Crafting).gain_experience(exp);

    ActionResult::success()
}
```

### 6. Integration with Drives

**Drive System**: `src/analytics/mod.rs:186-211`

```rust
DriveType::Utility => Action::Craft { item_type: "woodenaxe" }
```

**Drive Satisfaction**:
- Successful crafting reduces Utility drive by **-0.2**
- Energy cost: **15.0** per craft
- Crafting satisfies the need to be productive

### 7. Crafting Statistics

**Location**: `src/bin/test_simulation.rs:310-349`

Test executable displays crafting statistics:

```
Crafting & Production:
  • Tools Crafted:      3
  • Weapons Crafted:    1
  • Total Items:        4
```

**Tracked Items**:
- **Tools**: woodenaxe, woodenpickaxe, woodenhammer, stoneaxe, stonepickaxe, stonehammer, ironaxe, ironpickaxe, ironhammer
- **Weapons**: woodenspear, ironsword

## Configuration

### Recipe Tuning

**Material Requirements**:
```rust
// Wooden tools (3 wood)
inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)]

// Stone tools (2 stone + 1 wood)
inputs: vec![
    ResourceRequirement::new(ResourceType::Stone, 2),
    ResourceRequirement::new(ResourceType::Wood, 1),
]

// Iron tools (2 iron + 1 wood)
inputs: vec![
    ResourceRequirement::new(ResourceType::Iron, 2),
    ResourceRequirement::new(ResourceType::Wood, 1),
]
```

**Crafting Times**:
```rust
Wooden tools:  80 ticks
Stone tools:   90 ticks
Iron tools:    100 ticks
Iron weapons:  120 ticks
```

### Quality Multipliers

**Output Quantity**:
```rust
Poor:       0.8x (produces 80% of base amount, rounded up)
Common:     1.0x (produces 100%)
Good:       1.2x (produces 120%)
Excellent:  1.4x (produces 140%)
Masterwork: 1.6x (produces 160%)
```

**Example**:
- Recipe outputs 1 axe
- Common quality: 1 axe (1.0 * 1 = 1)
- Masterwork quality: 2 axes (1.6 * 1 = 1.6 → 2)

### Skill Progression

**Experience Required**:
```rust
100 experience points = 1 skill level
Max level = 10
Total experience to master: 2000 points
```

**Leveling Speed**:
- Poor quality crafts: 1000 crafts to master
- Common quality crafts: 500 crafts to master
- Good quality crafts: 334 crafts to master
- Excellent quality crafts: 250 crafts to master
- Masterwork quality crafts: 200 crafts to master

**Note**: As skill improves, quality improves, which grants more experience, creating a positive feedback loop for learning.

## Usage Examples

### Running Crafting Tests

```bash
# Basic test with crafting enabled
cargo run --bin test_simulation -- --agents 10 --ticks 5000 --report 500

# Long simulation to see skill progression
cargo run --bin test_simulation -- --agents 20 --ticks 50000 --report 5000

# Debug logging to see crafting events
RUST_LOG=debug cargo run --bin test_simulation -- --agents 5 --ticks 1000
```

### Expected Debug Output

**Crafting Event**:
```
[DEBUG] Agent 81258bc5 crafted Craft Wooden Axe (quality: Common, skill: -5, exp: +2)
[DEBUG] Agent a4b3c2d1 crafted Craft Iron Sword (quality: Excellent, skill: 7, exp: +4)
```

**Failure Events**:
```
[DEBUG] Agent 81258bc5 failed to craft: No recipe found for super_tool
[DEBUG] Agent a4b3c2d1 failed to craft: Missing materials for Craft Stone Axe: 2 stone
```

## Integration with Other Systems

### Inventory System

```
Agent gathers wood (Action::Gather)
    ↓
Wood stored in agent.inventory
    ↓
Utility drive increases
    ↓
Action::Craft generated
    ↓
Check inventory for materials
    ↓
Consume materials, produce tool
    ↓
Tool stored in inventory
    ↓
Tool can be used or traded
```

### Skill System

```
Agent crafts item
    ↓
Quality determined by skill level
    ↓
Experience granted based on quality
    ↓
Skill.gain_experience(amount)
    ↓
If experience >= 100:
    - Level up
    - Reset experience
    - Better quality on next craft
```

### Resource Gathering

```
Agents gather materials
    ↓
Wood, Stone, Iron in inventory
    ↓
Crafting transforms materials
    ↓
Produced tools/weapons
    ↓
Inventory weight increases
```

### Drive System

```
Utility drive accumulates
    ↓
Reaches threshold (e.g., 0.8)
    ↓
generate_action_for_drive(Utility)
    ↓
Returns Action::Craft { "woodenaxe" }
    ↓
execute_action processes craft
    ↓
Drive satisfaction: Utility -= 0.2
```

## Error Handling

### Recipe Not Found

**Cause**: Item type doesn't match any recipe
```rust
ActionResult::failure("No recipe found for super_tool")
```

**Solution**:
- Use exact item type name from recipes
- Check spelling (case-insensitive matching)
- Valid items: woodenaxe, stoneaxe, ironaxe, ironsword, etc.

### Missing Materials

**Cause**: Agent doesn't have required materials
```rust
ActionResult::failure("Missing materials for Craft Stone Axe: 1 wood (have 0)")
```

**Solution**:
- Agent must gather materials first
- Check inventory with `agent.inventory.get_item()`
- Need exact quantities specified in recipe

### Inventory Full

**Cause**: Agent can't carry crafted item
```rust
debug!("Agent {} crafted {} but inventory full, item dropped")
```

**Solution**:
- Drop or use existing items to make space
- Increase agent inventory capacity
- Crafting still succeeds but item is lost

## Advanced Features (Defined, Not Yet Integrated)

The production module defines many advanced features ready for integration:

### Job-Specific Recipes

**Location**: `src/world/production.rs:108-488`

Over 20 professions with specialized recipes:

**Food Processing**:
- Miller: Grain → Flour
- Baker: Flour → Bread
- Brewer: Grain → Ale
- Butcher: Meat processing
- Cheesemaker: Milk → Cheese

**Material Processing**:
- Tanner: Hides → Leather
- Potter: Clay → Pottery
- Weaver: Flax/Cotton → Cloth/Linen
- Spinner: Wool → Cloth
- Glassblower: Sand → Glass

**Crafting Professions**:
- Carpenter: Wood → Furniture, wooden tools
- Stonemason: Stone tools
- Blacksmith: Iron tools and weapons
- Armorer: Leather/Iron/Steel armor

**Usage** (when professions are integrated):
```rust
let agent_job = agent.profession;
let recipes = get_job_recipes(agent_job);
// Returns profession-specific recipes only
```

### Recipe Functions

```rust
/// Get all recipes for a job
pub fn get_job_recipes(job: JobType) -> Vec<Recipe>

/// Get primary recipe for a job (most common)
pub fn get_primary_recipe(job: JobType) -> Option<Recipe>
```

## Implementation Status

### ✅ Completed

- [x] Recipe data structures
- [x] Quality system with skill-based quality
- [x] Simple recipe set (wooden, stone, iron tools)
- [x] Action::Craft implementation
- [x] Material checking and consumption
- [x] Output calculation with quality multipliers
- [x] Skill experience gain
- [x] Auto-leveling when experience reaches 100
- [x] Crafting statistics tracking
- [x] Integration with Utility drive
- [x] Debug logging for crafting events

### 🔄 Partial Integration

- [ ] **Profession-specific recipes** - Defined but not accessible (agents don't have professions yet)
- [ ] **Tool quality tracking** - Items crafted but quality not stored on item
- [ ] **Crafting time** - Base time defined but not enforced

### 📋 Potential Enhancements

#### Short-term
- **Recipe discovery**: Learn recipes through experimentation
- **Tool durability**: Crafted tools degrade with use
- **Batch crafting**: Craft multiple items at once
- **Quality persistence**: Store item quality with crafted items
- **Recipe variety**: More diverse recipes for each tier

#### Medium-term
- **Profession system**: Assign jobs, unlock profession recipes
- **Workshops**: Dedicated crafting buildings with bonuses
- **Tool requirements**: Require tools to craft (hammer for smithing)
- **Material quality**: Input material quality affects output
- **Failure chance**: Risk of wasting materials based on skill

#### Long-term
- **Custom recipes**: Agents experiment and create new recipes
- **Specialization bonuses**: Extra bonuses for focused crafting
- **Apprenticeship**: Learn faster from skilled crafters
- **Blueprints**: Share recipes between agents
- **Mass production**: Workshop-based bulk crafting
- **Repair system**: Fix damaged tools instead of replacing

## Code References

### Key Files
- `src/world/production.rs` - Recipe system, quality calculation
- `src/agents/skills.rs` - Skill tracking, experience gain
- `src/analytics/mod.rs:598-782` - Crafting execution
- `src/analytics/mod.rs:203` - Utility drive → Craft action
- `src/bin/test_simulation.rs:310-349` - Crafting statistics

### Integration Flow

```
Agent.drives.Utility increases
    ↓
Simulation.tick()
    ↓
generate_action_for_drive(Utility)
    → Action::Craft { item_type: "woodenaxe" }
    ↓
execute_action(&Action::Craft, agent_index)
    ↓
1. Find recipe in simple_recipes list
2. Check agent.inventory for materials
3. Get agent.skills.Crafting.level
4. Convert level (-10..10) to skill (0..100)
5. ProductionQuality::from_skill(skill_value)
6. recipe.calculate_output(quality)
7. Remove materials from inventory
8. Add crafted items to inventory
9. Grant experience based on quality
10. Skill auto-levels if exp >= 100
    ↓
ActionResult::success()
    → Drive: Utility -0.2
    → Energy: -15.0
    → Message: "Crafted Craft Wooden Axe (Common quality)"
    ↓
agent.apply_feedback() updates drives
```

## Troubleshooting

### Issue: Agents not crafting
**Possible causes**:
- Utility drive not high enough
- No materials in inventory
- Behavior trees not initialized

**Solution**:
- Check drive values with debug logging
- Agents must gather materials first (wood/stone/iron)
- Verify behavior tree execution

### Issue: All crafts fail with "No recipe found"
**Cause**: Item type doesn't match recipe outputs

**Solution**:
- Check exact item type in code
- Valid types: "woodenaxe", "stoneaxe", "ironaxe", "ironsword"
- Case-insensitive matching, but use lowercase

### Issue: Can't craft stone/iron tools
**Cause**: Missing required materials

**Solution**:
- Stone tools need: 2 stone + 1 wood
- Iron tools need: 2 iron + 1 wood
- Agent must gather both resource types

### Issue: Quality never improves
**Cause**: Not gaining experience or not leveling

**Solution**:
- Check experience gain in debug logs
- Verify Skill.gain_experience() is being called
- 100 exp needed per level
- Check starting skill level (should be -10)

### Issue: Crafted items disappear
**Cause**: Inventory full

**Solution**:
- Check agent inventory weight
- Default capacity: 50kg
- Tools weigh 5kg each
- Drop items or increase capacity

## Performance Considerations

### Recipe Matching
- O(n) linear search through recipes
- Currently 7 simple recipes
- Acceptable for hundreds of crafts per tick
- Could use HashMap for faster lookup

### Skill Calculation
- O(1) constant time
- Simple arithmetic conversion
- No performance concerns

### Inventory Operations
- Material checking: O(m) where m = materials
- Material consumption: O(m)
- Item addition: O(1) if space available
- Total: O(m) per craft, very fast

## Summary

The crafting system is **complete and functional**:

1. ✅ Recipe system with materials and outputs
2. ✅ Quality system based on skill levels
3. ✅ Skill progression with experience
4. ✅ Simple recipes for tools and weapons
5. ✅ Material consumption from inventory
6. ✅ Quality-based output quantities
7. ✅ Integration with Utility drive
8. ✅ Statistics tracking and display

The system integrates with:
- **Drive system**: Utility drive triggers crafting
- **Skill system**: Crafting skill improves with practice
- **Inventory system**: Materials consumed, items produced
- **Resource gathering**: Gathered materials used for crafting

This provides a foundation for:
- Tool and weapon creation
- Skill progression
- Resource transformation
- Economic production
- Agent specialization (when professions added)
- Complex crafting chains (when advanced recipes integrated)

The extensive profession-specific recipes in `production.rs` are ready to be activated when the profession system is integrated, enabling 20+ specialized crafting jobs with hundreds of unique recipes.
