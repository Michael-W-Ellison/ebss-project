# Combat & Damage System

This document describes the comprehensive combat and damage mechanics that allow agents to engage in combat and sustain injuries from various sources.

## Overview

The combat and damage system enables agents to:
- Attack other agents in melee combat
- Sustain injuries to specific body parts
- Experience environmental damage (exposure, falling, disease)
- Heal naturally over time or with medical assistance
- Die from combat or accumulated injuries

## System Components

### 1. Body System

**Location**: `src/agents/body.rs`

The body system provides anatomical structure with 7 body parts:

#### Body Parts
| Part | Base Health | Critical | Function |
|------|------------|----------|----------|
| Head | 50.0 | ✓ | Death if destroyed |
| Torso | 100.0 | ✓ | Death if destroyed |
| Back | 80.0 | ✗ | Protection |
| Left Arm | 60.0 | ✗ | Tool use, combat |
| Right Arm | 60.0 | ✗ | Tool use, combat |
| Left Leg | 70.0 | ✗ | Movement |
| Right Leg | 70.0 | ✗ | Movement |

**Total Body Health**: 490.0 HP (sum of all parts)

#### Body Part Status
```rust
pub enum BodyPartStatus {
    Healthy,      // >= 75% health
    Injured,      // 50-75% health
    Crippled,     // 25-50% health
    Disabled,     // < 25% health
    Missing,      // 0% health (destroyed)
}
```

### 2. Injury System

**Location**: `src/agents/body.rs:8-104`

#### Injury Types

**Minor Injuries**
- **Healing Rate**: 0.5 HP/tick
- **Recovery**: 100% (full recovery)
- **Examples**: Bruises, small cuts, scrapes
- **Duration**: ~20 ticks for 10 damage

**Major Injuries**
- **Healing Rate**: 0.1 HP/tick
- **Recovery**: 100% (full recovery but slow)
- **Examples**: Deep wounds, fractures
- **Duration**: ~300 ticks for 30 damage

**Crippling Injuries - Partial**
- **Healing Rate**: 0.05 HP/tick (very slow)
- **Recovery**: 70% (permanent impairment)
- **Examples**: Severe trauma, amputations
- **Duration**: Permanent damage remains

**Crippling Injuries - Full**
- **Healing Rate**: 0.0 HP/tick (no healing)
- **Recovery**: 0% (no recovery)
- **Examples**: Complete amputation, total organ failure
- **Duration**: Permanent

#### Injury Mechanics

```rust
pub struct Injury {
    pub injury_type: InjuryType,
    pub damage_taken: f32,
    pub healing_progress: f32,  // Tracks healing
    pub timestamp: u64,          // When injury occurred
}
```

**Healing Process**:
1. Each tick, injury heals by `injury_type.healing_rate()`
2. Healing progress tracks recovery up to max recoverable damage
3. Permanent damage = `damage_taken * (1.0 - max_recovery())`
4. When fully healed (to limit), injury is removed

### 3. Combat System

**Location**: `src/analytics/mod.rs:469-590`

#### Action::Attack

```rust
Action::Attack {
    target_agent_id: uuid::Uuid,
    weapon: Option<String>,
}
```

#### Combat Mechanics

**1. Range Checking**
- Melee range: 1 tile Manhattan distance
- Target must be adjacent (horizontally or vertically)
- Future: Ranged weapons can extend range

**2. Damage Calculation**

```rust
base_damage = if weapon.is_some() {
    rng.gen_range(10.0..25.0)  // Weapon damage
} else {
    rng.gen_range(5.0..15.0)   // Unarmed combat
}

actual_damage = base_damage * attacker_tool_efficiency
```

**Attacker Tool Efficiency**:
- Based on arm health (better arm used)
- Damaged arms reduce combat effectiveness
- 100% = full effectiveness
- 0% = cannot attack (both arms disabled)

**3. Body Part Targeting**

Weighted random selection:

| Body Part | Hit Chance | Notes |
|-----------|-----------|-------|
| Head | 10% | Critical hits |
| Torso | 30% | Most common target |
| Left Arm | 15% | Side targets |
| Right Arm | 15% | Side targets |
| Left Leg | 12% | Lower targets |
| Right Leg | 12% | Lower targets |
| Back | 6% | Hard to hit |

**4. Armor Protection**

Each body part has a `protection` value (0.0 to 0.95):

```rust
actual_damage = base_damage * (1.0 - part.protection)
```

Equipment provides protection to covered body parts:
- Helmet → Head
- Chestplate → Torso
- Leggings → Legs
- etc.

**5. Injury Assignment**

Based on damage dealt:

| Damage | Injury Type | Notes |
|--------|------------|-------|
| < 15.0 | Minor | Quick healing |
| 15.0 - 30.0 | Major | Slow healing |
| >= 30.0 | Crippling (30% chance) | Permanent impairment |
| >= 30.0 | Major (70% chance) | Alternative outcome |

**6. Health Reduction**

```rust
// Body part takes full damage
part.apply_injury(injury_type, actual_damage, timestamp);

// Agent overall health reduced by 20% of damage
agent.state.health -= actual_damage * 0.2;
```

**7. Death Check**

Agent dies if:
- Critical part (Head or Torso) destroyed (health = 0)
- Overall health drops to 0
- Both conditions checked after damage applied

#### Combat Flow

```
Attacker executes Action::Attack
    ↓
Find target agent
    ↓
Check range (must be ≤ 1 tile)
    ↓
Calculate base damage (weapon or unarmed)
    ↓
Modify by attacker arm efficiency
    ↓
Select random body part (weighted)
    ↓
Apply armor protection
    ↓
Determine injury type
    ↓
Apply injury to body part
    ↓
Reduce overall health
    ↓
Check if target died
    ↓
Return ActionResult (success/failure)
```

### 4. Environmental Damage

**Location**: `src/analytics/mod.rs:615-743`

Processed every tick via `Simulation::process_environmental_damage()`

#### Exposure Damage

**Cold Exposure**
- **Trigger**: `cold_insulation < 1.0`
- **Chance**: `(1.0 - cold_insulation) * 1%` per tick
- **Damage**: 1.0 - 5.0 HP
- **Targets**: Arms and Legs (extremities)
- **Injury Type**: Minor

**Heat Exposure**
- **Trigger**: `heat_resistance < 0.5`
- **Chance**: `(0.5 - heat_resistance) * 0.5%` per tick
- **Damage**: 2.0 - 8.0 HP
- **Targets**: Head and Torso
- **Injury Type**: Minor

#### Falling Damage

- **Frequency**: 0.01% chance per tick (~14 falls per million ticks)
- **Fall Height**: 1-5 units (random)
- **Damage**: `height * (3.0..8.0)` HP
- **Primary Targets**: Legs (70% chance)
- **Critical Fall**: Height >= 4 with 30% chance to hit Head/Torso

**Injury Severity**:
- Fall damage >= 25.0 → Crippling (Partial)
- Fall damage >= 12.0 → Major
- Fall damage < 12.0 → Minor

**Additional Effects**:
- Overall health reduced by `fall_damage * 0.15`
- Can cause instant death if critical part destroyed

#### Disease/Infection

- **Risk Factor**: Number of existing injuries
- **Chance**: `injury_count * 0.01%` per tick
- **Effect**: Adds Infected condition to random body part

**Infected Condition**:
```rust
Condition {
    condition_type: ConditionType::Infected,
    severity: 0.3..0.8,
    duration: 100..500 ticks,
}
```

Infected parts take ongoing damage per tick until condition expires.

#### Natural Healing

Each tick, all body parts process:
1. Injury healing (based on injury type healing rate)
2. Condition effects (bleeding, infection, etc.)
3. Status updates
4. Equipment wear

### 5. Body Conditions

**Location**: `src/agents/body.rs:212-227`

```rust
pub enum ConditionType {
    Bleeding,    // 0.5 damage per severity per tick
    Burned,      // 0.2 damage per severity per tick
    Frostbitten, // Future implementation
    Poisoned,    // 0.3 damage per severity per tick
    Infected,    // Applied by disease system
    Bruised,     // Visual/minor effect
    Fractured,   // Future: reduces functionality
}
```

**Condition Processing**:
- Each condition has severity (0.0 to 1.0) and duration (ticks)
- Conditions tick down each frame
- Expired conditions are removed
- Active conditions deal damage based on type and severity

### 6. Equipment & Protection

**Location**: `src/agents/body.rs:588-670`

#### Equipment Slots

Equipment covers specific body parts:

| Slot | Covered Parts |
|------|--------------|
| Head | Head |
| Torso | Torso, Back |
| Legs | Left Leg, Right Leg |
| Feet | (Future) |
| Hands | (Future) |

#### Protection Values

```rust
pub struct Equipment {
    pub name: String,
    pub slot: EquipmentSlot,
    pub armor_protection: f32,      // 0.0 to 0.95
    pub cold_insulation: f32,
    pub heat_resistance: f32,
    pub durability: f32,
}
```

**Armor Types** (examples from existing templates):
- Leather Tunic: 0.15 protection
- Iron Chestplate: 0.50 protection
- Fur Coat: 0.10 protection, high cold insulation
- Linen Shirt: 0.05 protection, high heat resistance

#### Equipment Wear

Equipment degrades over time:
- Each tick reduces durability slightly
- Durability 0.0 → item breaks and is removed
- Broken items provide no protection

### 7. Combat Statistics

**Location**: `src/bin/test_simulation.rs:251-308`

Test executable displays combat statistics:

```
Combat & Injuries:
  • Total Injuries:     15
  • Agents Injured:     8
  • Crippled Parts:     2
  • Disabled/Missing:   1
  • Avg Body Health:    87.3%
  • Agents w/ Armor:    5
```

**Statistics Tracked**:
- Total injuries across all agents and body parts
- Number of agents with at least one injury
- Count of crippled body parts
- Count of disabled/missing body parts
- Average body health percentage
- Number of agents wearing armor

## Configuration

### Damage Tuning

**Unarmed Combat**:
```rust
base_damage = rng.gen_range(5.0..15.0)
```

**Weapon Combat**:
```rust
base_damage = rng.gen_range(10.0..25.0)
```

**Environmental Damage Rates**:
```rust
cold_exposure_chance = (1.0 - insulation) * 0.01  // 1% per severity point
heat_exposure_chance = (0.5 - resistance) * 0.005 // 0.5% per severity
fall_chance = 0.0001  // 0.01% per tick
infection_chance = injury_count * 0.0001  // 0.01% per injury
```

### Healing Rates

**Injury Healing**:
```rust
Minor:              0.5 HP/tick
Major:              0.1 HP/tick
Crippling Partial:  0.05 HP/tick
Crippling Full:     0.0 HP/tick (no healing)
```

**Recovery Limits**:
```rust
Minor:              100% recovery
Major:              100% recovery
Crippling Partial:  70% recovery (30% permanent damage)
Crippling Full:     0% recovery (100% permanent damage)
```

## Usage Examples

### Running Combat Tests

```bash
# Basic test with combat enabled
cargo run --bin test_simulation -- --agents 10 --ticks 5000 --report 500

# Long simulation to see injuries accumulate
cargo run --bin test_simulation -- --agents 20 --ticks 50000 --report 5000

# Debug logging to see combat events
RUST_LOG=debug cargo run --bin test_simulation -- --agents 5 --ticks 1000
```

### Expected Debug Output

**Combat Event**:
```
[DEBUG] Agent 81258bc5 attacked Agent a4b3c2d1 (unarmed): 12.3 damage to Torso (survived)
[DEBUG] Agent a4b3c2d1 attacked Agent 81258bc5 (iron_sword): 23.7 damage to LeftArm (FATAL)
```

**Environmental Damage**:
```
[DEBUG] Agent 81258bc5 suffered cold exposure: 3.2 damage to LeftLeg
[DEBUG] Agent a4b3c2d1 suffered fall damage: 18.4 damage to RightLeg (Major)
[DEBUG] Agent f9e8d7c6 developed infection on Torso
```

## Integration with Other Systems

### Drive System Integration

```
DriveType::Safety accumulates → 1.0
    ↓
Action::Attack generated
    ↓
Combat executed successfully
    ↓
Drive satisfaction: -0.2 (reduced to 0.8)
Energy cost: -15.0
```

### Death System Integration

```
Agent takes damage
    ↓
Body part health → 0
    ↓
If critical part (Head/Torso)
    ↓
body.is_alive() → false
    ↓
Agent death triggered
    ↓
Population.tick() removes agent
    ↓
stats.total_deaths incremented
```

### Movement System Integration

```
Agent has leg injuries
    ↓
body.movement_speed_multiplier() → 0.5
    ↓
Movement actions take longer
    ↓
Pathfinding accounts for reduced speed
```

### Tool Use Integration

```
Agent has arm injuries
    ↓
body.tool_efficiency_multiplier() → 0.6
    ↓
Gathering takes longer
    ↓
Crafting quality reduced
    ↓
Combat damage reduced
```

## Error Handling

### Attack Failures

**Target Not Found**:
```
ActionResult::failure("Target agent not found")
```

**Out of Range**:
```
ActionResult::failure("Target too far away (distance: 5)")
```

**Self-Attack Prevented**:
```
ActionResult::failure("Cannot attack yourself")
```

### Damage Application

- Body parts cannot go below 0 health
- Overall health clamped to [0.0, 100.0]
- Protection values clamped to [0.0, 0.95] (max 95%)
- Healing cannot exceed max health

## Implementation Status

### ✅ Completed

- [x] Body system with 7 body parts
- [x] Injury types (Minor, Major, Crippling)
- [x] Natural healing system
- [x] Body part status tracking
- [x] Action::Attack implementation
- [x] Melee combat mechanics
- [x] Body part targeting (weighted random)
- [x] Armor protection system
- [x] Equipment system
- [x] Environmental damage (cold, heat, falling)
- [x] Disease/infection system
- [x] Combat statistics display
- [x] Death from combat
- [x] Critical part destruction

### 🔄 Potential Enhancements

#### Short-term
- **Weapon system**: Define actual weapon stats (damage, range, type)
- **Ranged combat**: Arrows, thrown weapons
- **Block/dodge**: Defensive actions
- **Stances**: Aggressive, defensive, balanced
- **Skill bonuses**: Combat skill affects damage/accuracy
- **Morale system**: Fear, courage affecting combat

#### Medium-term
- **Formations**: Group combat tactics
- **Mounted combat**: Fighting while riding animals
- **Siege weapons**: Catapults, ballistae
- **Medical system**: Bandaging, surgery, medicine
- **Pain system**: Injuries affect behavior
- **Blood loss**: Separate from general health

#### Long-term
- **Hit locations**: More precise body part hitboxes
- **Armor degradation**: Equipment damaged in combat
- **Shield blocking**: Active defense
- **Weapon types**: Slash, pierce, blunt with different effects
- **Combat animations**: Visual feedback
- **Veteran bonuses**: Experience-based improvements

## Code References

### Key Files
- `src/agents/body.rs` - Body part system, injuries, equipment
- `src/environment/mod.rs:150-154` - Action::Attack definition
- `src/analytics/mod.rs:469-590` - Combat execution
- `src/analytics/mod.rs:615-743` - Environmental damage
- `src/bin/test_simulation.rs:251-308` - Combat statistics display

### Integration Flow

```
Agent selects Action::Attack
    ↓
Simulation::execute_action()
    ↓
1. Find target agent by ID
2. Check if in range (≤ 1 tile)
3. Calculate base damage (weapon or unarmed)
4. Modify by attacker.body.tool_efficiency_multiplier()
5. Select target body part (weighted random)
6. Get body part armor protection
7. Apply protection: actual_damage = base * (1 - protection)
8. Determine injury type based on damage
9. Apply injury: target.body.get_part_mut(part).apply_injury()
10. Reduce overall health: target.state.health -= damage * 0.2
11. Check death: !target.body.is_alive() || target.state.health == 0
12. Return ActionResult with success/kill message
    ↓
Agent.apply_feedback() - reduces Safety drive
    ↓
Combat complete
```

```
Simulation.tick()
    ↓
population.tick() (handles deaths)
    ↓
Process agent actions (including combat)
    ↓
process_environmental_damage()
    ↓
1. For each agent:
2.   Check cold_insulation → apply cold damage
3.   Check heat_resistance → apply heat damage
4.   Random fall chance → apply fall damage
5.   Check injury_count → chance of infection
6.   body.tick() → process injuries, conditions, healing
    ↓
world.tick() (building construction, etc.)
    ↓
Log statistics
```

## Troubleshooting

### Issue: Agents not attacking
**Cause**: Safety drive not high enough or no behavior trees
**Solution**:
- Safety drive needs to accumulate to threshold
- Behavior trees must be initialized
- Action generation maps Safety → Attack

### Issue: All attacks miss/fail
**Possible causes**:
- Agents too far apart (need to be adjacent)
- Target agent not found (wrong ID)
- Attacker arms disabled

**Solution**:
- Check agent positions with debug logging
- Verify target_agent_id is valid
- Check attacker.body.tool_efficiency_multiplier()

### Issue: Damage seems too high/low
**Cause**: Damage tuning needs adjustment
**Solution**:
- Adjust base damage ranges in execute_action()
- Modify injury type thresholds
- Change armor protection values

### Issue: Agents dying too quickly to environment
**Cause**: Environmental damage rates too high
**Solution**:
- Reduce exposure damage chances
- Lower fall chance probability
- Increase healing rates

### Issue: Injuries not healing
**Cause**: Natural healing not being called
**Solution**:
- Verify process_environmental_damage() is called each tick
- Check body.tick() is being executed
- Confirm injury types allow healing (not Full Crippling)

## Performance Considerations

### Combat Execution
- O(n) to find target agent by ID (could use HashMap)
- O(1) for damage calculation and application
- Acceptable for hundreds of agents

### Environmental Damage
- O(n * p) where n = agents, p = body parts (7)
- Executed every tick
- ~35 operations per agent per tick
- Fine for hundreds of agents

### Injury Tracking
- Each injury tracked separately
- Natural healing processes all injuries
- Could accumulate over long simulations
- Consider injury cleanup for old/minor injuries

## Summary

The combat and damage system is **complete and functional**:

1. ✅ Full body system with anatomical structure
2. ✅ Three injury types with healing mechanics
3. ✅ Melee combat with body part targeting
4. ✅ Armor protection and equipment system
5. ✅ Environmental damage sources
6. ✅ Natural healing and recovery
7. ✅ Death from combat or injuries
8. ✅ Combat statistics tracking

The system integrates with:
- **Drive system**: Safety drive triggers attacks
- **Death system**: Combat can kill agents
- **Movement system**: Leg injuries slow movement
- **Tool use**: Arm injuries reduce effectiveness
- **Equipment**: Armor reduces damage taken

This provides a foundation for:
- Agent-to-agent combat
- Environmental hazards
- Injury management and healing
- Equipment and protection
- Long-term survival challenges
- Combat-based emergent behaviors
