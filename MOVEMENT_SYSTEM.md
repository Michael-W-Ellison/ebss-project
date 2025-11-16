# Movement & Pathfinding System

This document describes the comprehensive movement mechanics that allow agents to navigate the world, explore terrain, and relocate.

## Overview

The movement system enables agents to:
- Move between positions on the world grid
- Navigate towards target locations with simple pathfinding
- Have movement speed affected by leg injuries
- Consume energy based on distance and health
- Avoid obstacles (water, buildings, world boundaries)
- Explore the world when curiosity drives them
- Seek safety by relocating when threatened

## System Components

### 1. Action::Move

**Location**: `src/environment/mod.rs:150-154`

```rust
Action::Move {
    target: (i32, i32, i32),  // Target position (x, y, z)
}
```

**Parameters**:
- `target`: 3D coordinates of destination (z-coordinate currently unused)

### 2. Movement Execution

**Location**: `src/analytics/mod.rs:784-881`

#### Movement Flow

```
1. Get agent's current position
   ↓
2. Calculate distance to target
   ↓
3. Check if already at destination → Success
   ↓
4. Determine next step (simple pathfinding)
   ↓
5. Validate next position:
   - Within world bounds
   - Not water terrain
   - Not blocked by building
   ↓
6. Get movement speed multiplier from leg health
   ↓
7. Calculate energy cost (modified by speed)
   ↓
8. Update agent position
   ↓
9. Return success with drive satisfaction
```

#### Step-by-Step Execution

```rust
Action::Move { target } => {
    // 1. Get current position
    let current_pos = agent.state.position;
    let current_2d = Position::new(current_pos.0, current_pos.1);
    let target_2d = Position::new(target.0, target.1);

    // 2. Check if at destination
    if current_2d == target_2d {
        return ActionResult::success("Already at destination");
    }

    // 3. Calculate delta
    let dx = target.0 - current_pos.0;
    let dy = target.1 - current_pos.1;

    // 4. Normalize to single step
    let step_x = if dx > 0 { 1 } else if dx < 0 { -1 } else { 0 };
    let step_y = if dy > 0 { 1 } else if dy < 0 { -1 } else { 0 };

    // 5. Prioritize longer axis for movement
    let (next_x, next_y) = if dx.abs() >= dy.abs() {
        (current_pos.0 + step_x, current_pos.1)
    } else {
        (current_pos.0, current_pos.1 + step_y)
    };

    // 6. Validate position
    if !is_valid_position(next_x, next_y) {
        return ActionResult::failure("Invalid position");
    }

    // 7. Get movement speed
    let movement_speed = agent.body.movement_speed_multiplier();
    if movement_speed < 0.1 {
        return ActionResult::failure("Too injured to move");
    }

    // 8. Calculate energy cost
    let energy_cost = base_energy_cost / movement_speed;

    // 9. Update position
    agent.state.position = (next_x, next_y, target.2);

    ActionResult::success()
}
```

### 3. Simple Pathfinding

**Location**: `src/analytics/mod.rs:804-817`

The system uses **greedy best-first** approach:

```rust
// Calculate delta
let dx = target.0 - current.0;
let dy = target.1 - current.1;

// Normalize to -1, 0, or 1
let step_x = if dx > 0 { 1 } else if dx < 0 { -1 } else { 0 };
let step_y = if dy > 0 { 1 } else if dy < 0 { -1 } else { 0 };

// Move along longer axis first
if dx.abs() >= dy.abs() {
    move_x()  // Horizontal priority
} else {
    move_y()  // Vertical priority
}
```

**Pathfinding Characteristics**:
- **One step per action**: Moves one tile per execution
- **Axis-aligned**: Moves horizontally OR vertically (not diagonally)
- **Greedy**: Always moves towards target on longest axis
- **No obstacle avoidance**: If blocked, movement fails
- **Manhattan distance**: Uses Manhattan pathfinding

**Example Path**:
```
Start: (5, 5)
Target: (10, 8)

Step 1: (6, 5)  - Move X (dx=5 > dy=3)
Step 2: (7, 5)  - Move X
Step 3: (8, 5)  - Move X
Step 4: (9, 5)  - Move X
Step 5: (10, 5) - Move X
Step 6: (10, 6) - Move Y
Step 7: (10, 7) - Move Y
Step 8: (10, 8) - Arrived!
```

### 4. Movement Speed Modifiers

**Location**: `src/agents/body.rs:563-573`

Movement speed is calculated from leg health:

```rust
pub fn movement_speed_multiplier(&self) -> f32 {
    let left_leg = self.parts.get(&BodyPartType::LeftLeg)
        .map(|p| p.effectiveness())
        .unwrap_or(0.0);
    let right_leg = self.parts.get(&BodyPartType::RightLeg)
        .map(|p| p.effectiveness())
        .unwrap_or(0.0);

    // Average effectiveness of both legs
    (left_leg + right_leg) / 2.0
}
```

**Leg Effectiveness**:
| Leg Health | Effectiveness | Movement Speed |
|-----------|---------------|----------------|
| 100% | 1.0 | 100% (1.0x) |
| 75% | 0.75 | 75% (0.75x) |
| 50% | 0.5 | 50% (0.5x) |
| 25% | 0.25 | 25% (0.25x) |
| 0% (disabled) | 0.0 | 0% (can't move) |

**Both Legs Average**:
```
Left: 100%, Right: 100% → Speed: 1.0x (full speed)
Left: 100%, Right: 50%  → Speed: 0.75x (limping)
Left: 50%,  Right: 50%  → Speed: 0.5x (slow)
Left: 0%,   Right: 100% → Speed: 0.5x (one leg)
Left: 0%,   Right: 0%   → Speed: 0.0x (immobilized)
```

### 5. Energy Cost

**Location**: `src/analytics/mod.rs:846-853`

```rust
let base_energy_cost = 2.0;
let actual_energy_cost = if movement_speed > 0.1 {
    base_energy_cost / movement_speed
} else {
    // Can't move - legs too damaged
    return ActionResult::failure("Too injured to move");
}
```

**Energy Cost Examples**:
| Movement Speed | Energy Cost | Notes |
|---------------|-------------|-------|
| 1.0x (healthy) | 2.0 | Normal cost |
| 0.75x (minor injury) | 2.67 | 33% more energy |
| 0.5x (one leg) | 4.0 | 2x energy cost |
| 0.25x (severe injury) | 8.0 | 4x energy cost |
| < 0.1x (crippled) | N/A | Cannot move |

**Rationale**: Injured agents spend more energy to move the same distance.

### 6. Obstacle Avoidance

**Location**: `src/analytics/mod.rs:821-840`

#### World Boundary Check
```rust
let world_width = self.world.grid.width as i32;
let world_height = self.world.grid.height as i32;

if next_x < 0 || next_x >= world_width || next_y < 0 || next_y >= world_height {
    return ActionResult::failure("Cannot move outside world bounds");
}
```

#### Terrain Check
```rust
if let Some(tile) = self.world.grid.get_tile(&next_pos) {
    if tile.terrain.terrain_type == TerrainType::Water {
        return ActionResult::failure("Cannot move into water");
    }
}
```

#### Building Check
```rust
if self.world.is_position_occupied(&next_pos) {
    return ActionResult::failure("Position blocked by building");
}
```

**Impassable Terrain**:
- **Water**: Agents cannot cross water tiles
- **Buildings**: Cannot move into occupied positions
- **World edges**: Cannot move outside grid boundaries

**Passable Terrain**:
- **Plains**: Normal movement
- **Forest**: Normal movement
- **Mountain**: Normal movement (no penalty currently)

### 7. Drive Integration

**Location**: `src/analytics/mod.rs:187-224`

Movement is triggered by two drives:

#### Safety Drive
```rust
DriveType::Safety => {
    // Move to random nearby safe location
    let target_x = position.0 + rng.gen_range(-5..=5);
    let target_y = position.1 + rng.gen_range(-5..=5);
    Action::Move { target: (target_x, target_y, position.2) }
}
```

- **Range**: ±5 tiles from current position
- **Purpose**: Flee from danger, seek shelter
- **Drive satisfaction**: -0.05 when close to target

#### Curiosity Drive
```rust
DriveType::Curiosity => {
    // Explore by moving to random distant location
    let target_x = position.0 + rng.gen_range(-20..=20);
    let target_y = position.1 + rng.gen_range(-20..=20);
    Action::Move { target: (target_x, target_y, position.2) }
}
```

- **Range**: ±20 tiles from current position
- **Purpose**: Explore new areas, wander
- **Drive satisfaction**: -0.05 when far from origin

### 8. Movement Statistics

**Location**: `src/bin/test_simulation.rs:351-400`

Displays when agents have spread out (> 5 tile spread):

```
Movement & Exploration:
  • Position Range:
    X: 15 to 35 (spread: 20)
    Y: 18 to 32 (spread: 14)
  • Avg Distance from Center: 12.3 tiles
  • Avg Movement Speed:      0.87x
  • Agents w/ Leg Injuries:  2
```

**Tracked Metrics**:
- **Position Range**: Min/max X and Y coordinates
- **Spread**: Difference between min and max
- **Distance from Center**: Average Euclidean distance from world center
- **Movement Speed**: Average movement multiplier across all agents
- **Leg Injuries**: Count of agents with impaired movement

## Configuration

### Movement Parameters

```rust
// Energy cost
const BASE_ENERGY_COST: f32 = 2.0;

// Minimum movement speed
const MIN_MOVEMENT_SPEED: f32 = 0.1;

// Drive-based movement ranges
const SAFETY_RANGE: i32 = 5;      // ±5 tiles
const CURIOSITY_RANGE: i32 = 20;  // ±20 tiles
```

### World Defaults

```rust
// Default world size
const DEFAULT_WIDTH: usize = 50;
const DEFAULT_HEIGHT: usize = 50;

// World center (for statistics)
const CENTER_X: i32 = 25;
const CENTER_Y: i32 = 25;
```

## Usage Examples

### Running Movement Tests

```bash
# Basic test
cargo run --bin test_simulation -- --agents 10 --ticks 5000 --report 500

# Long simulation to see exploration
cargo run --bin test_simulation -- --agents 20 --ticks 50000 --report 5000

# Debug logging to see movement events
RUST_LOG=ebss::analytics=debug cargo run --bin test_simulation -- --agents 5 --ticks 1000
```

### Expected Debug Output

**Movement Event**:
```
[DEBUG] Agent 81258bc5 moved from (25, 25) to (26, 25) (distance to target: 8, speed: 1.00x)
[DEBUG] Agent a4b3c2d1 moved from (30, 20) to (29, 20) (distance to target: 5, speed: 0.75x)
```

**Failure Events**:
```
[DEBUG] Agent 81258bc5 failed to move: Cannot move into water
[DEBUG] Agent a4b3c2d1 failed to move: Position blocked by building
[DEBUG] Agent f9e8d7c6 failed to move: Too injured to move (legs crippled)
```

## Integration with Other Systems

### Body/Injury System

```
Agent takes leg damage (combat/falling)
    ↓
BodyPart(LeftLeg).health decreases
    ↓
body.movement_speed_multiplier() recalculated
    ↓
Movement becomes slower
    ↓
Energy cost increases
    ↓
Agent may become immobilized if both legs disabled
```

### Energy System

```
Agent executes Action::Move
    ↓
Calculate energy cost based on movement speed
    ↓
agent.state.energy -= energy_cost
    ↓
Low energy → cannot move effectively
    ↓
Must rest or eat to restore energy
```

### Drive System

```
Safety or Curiosity drive accumulates
    ↓
Reaches threshold (e.g., 0.8)
    ↓
generate_action_for_drive() returns Move action
    ↓
execute_action() processes movement
    ↓
Drive satisfaction reduces drive by -0.05
    ↓
Agent moves towards safety or exploration
```

### World/Terrain System

```
Agent attempts to move to position
    ↓
world.grid.get_tile(position) checked
    ↓
terrain_type == Water → blocked
    ↓
terrain_type == Plains/Forest/Mountain → allowed
    ↓
world.is_position_occupied() checked
    ↓
If building present → blocked
    ↓
Position updated if valid
```

## Error Handling

### Already at Destination
```rust
ActionResult::success("Already at destination")
```
- Not an error, just no movement needed
- Success with no energy cost

### Out of Bounds
```rust
ActionResult::failure("Cannot move outside world bounds")
```
- Target or next step exceeds grid size
- Agent stays at current position

### Water Obstacle
```rust
ActionResult::failure("Cannot move into water")
```
- Next tile is water terrain
- Agent cannot cross water (no swimming/boats)

### Building Obstacle
```rust
ActionResult::failure("Position blocked by building")
```
- Next tile occupied by building
- Agent must path around obstacles

### Too Injured
```rust
ActionResult::failure("Too injured to move (legs crippled)")
```
- Movement speed < 0.1 (both legs severely damaged)
- Agent is immobilized until healed

## Advanced Features (Potential Enhancements)

### Short-term
- **A* Pathfinding**: Intelligent obstacle avoidance
- **Diagonal movement**: 8-directional instead of 4
- **Terrain costs**: Mountains slower, plains faster
- **Swimming**: Allow water crossing with stamina cost
- **Sprint/walk modes**: Variable speed with energy tradeoff

### Medium-term
- **Mounted travel**: Riding animals for faster movement
- **Roads**: Built paths for faster travel
- **Bridges**: Cross water at specific points
- **Formation movement**: Groups move together
- **Patrol routes**: Repeating paths

### Long-term
- **Dynamic obstacles**: Moving agents avoid each other
- **Stealth movement**: Slower but quieter
- **Climbing**: Vertical movement in 3D
- **Flight**: Flying creatures or magic
- **Teleportation**: Instant travel between points
- **Waypoint navigation**: Plan multi-step journeys

## Code References

### Key Files
- `src/environment/mod.rs:150-154` - Action::Move definition
- `src/analytics/mod.rs:784-881` - Movement execution
- `src/analytics/mod.rs:187-224` - Drive-based movement generation
- `src/agents/body.rs:563-573` - Movement speed calculation
- `src/world/grid.rs:8-54` - Position and distance utilities
- `src/bin/test_simulation.rs:351-400` - Movement statistics

### Integration Flow

```
Agent.drives.Safety or Curiosity increases
    ↓
Simulation.tick()
    ↓
generate_action_for_drive(Safety/Curiosity)
    → Action::Move { target: random_position }
    ↓
execute_action(&Action::Move, agent_index)
    ↓
1. Get current position
2. Calculate distance to target
3. Determine next step (greedy pathfinding)
4. Validate next position (bounds, terrain, buildings)
5. Get movement speed from leg health
6. Calculate energy cost (base / speed)
7. Update agent.state.position
8. Return ActionResult with drive satisfaction
    ↓
agent.apply_feedback() updates drives
agent.state.energy -= energy_cost
```

## Troubleshooting

### Issue: Agents not moving
**Possible causes**:
- Safety/Curiosity drives not high enough
- Behavior trees not initialized
- No valid movement targets

**Solution**:
- Check drive values with debug logging
- Verify behavior tree execution
- Ensure world has passable terrain

### Issue: Agents moving very slowly
**Cause**: Leg injuries reducing movement speed

**Solution**:
- Check agent leg health with stats
- Allow time for natural healing
- Avoid combat/falling damage

### Issue: Movement fails constantly
**Possible causes**:
- Surrounded by water
- Trapped by buildings
- Target outside world bounds

**Solution**:
- Check terrain layout
- Generate more open spaces
- Validate target positions

### Issue: Agents stuck at edges
**Cause**: Random targets outside world bounds

**Solution**:
- Clamp random target generation to world size
- Add boundary padding to random ranges

### Issue: Too much energy loss
**Cause**: High energy cost due to injuries

**Solution**:
- Heal leg injuries
- Reduce movement frequency
- Adjust base energy cost parameter

## Performance Considerations

### Pathfinding
- O(1) per step (greedy, no search)
- Very fast for hundreds of agents
- No pathfinding data structures needed

### Position Validation
- O(1) grid lookup
- O(n) building check (n = buildings)
- Acceptable for dozens of buildings

### Movement Speed Calculation
- O(1) lookup of leg health
- Simple average computation
- No performance concerns

## Summary

The movement system is **complete and functional**:

1. ✅ Action::Move execution
2. ✅ Simple greedy pathfinding
3. ✅ Movement speed based on leg health
4. ✅ Energy cost scaling with injuries
5. ✅ Obstacle detection (water, buildings, bounds)
6. ✅ Drive integration (Safety, Curiosity)
7. ✅ Movement statistics tracking

The system integrates with:
- **Body system**: Leg injuries affect speed
- **Energy system**: Movement costs energy
- **Drive system**: Safety and Curiosity trigger movement
- **World/Grid**: Terrain and obstacles validated
- **Position tracking**: Agent coordinates updated

This provides a foundation for:
- Agent exploration and wandering
- Fleeing from danger
- Seeking resources/shelter
- World navigation
- Spatial distribution of population
- Injury-based mobility limitations

The simple greedy pathfinding is efficient but basic. For complex obstacle navigation, A* pathfinding can be added in the future when needed.
