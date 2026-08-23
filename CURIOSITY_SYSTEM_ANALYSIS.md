# Curiosity System Analysis - EBSS Project

## Overview

This analysis examines the curiosity system implementation in the EBSS (Evolutionary Behavior Simulation System) project. The curiosity system is a multi-layered architecture that integrates drives, perception, discovery, learning, and analytics.

---

## Key Findings Summary

### 1. Curiosity is Defined as a Secondary Drive

**Location**: `src/core/drives.rs`, line 33-118

- **Type**: `DriveType::Curiosity` (one of 15 core drives)
- **Threshold**: 0.2 (activates easily - lowest among survival drives at 0.6-0.8)
- **Accumulation Rate**: 0.004 per tick (0.4% - slow compared to hunger at 1%)
- **Weight**: 0.5-1.5 (variable per agent - lower than survival drives at 1.5-2.5)

**Design Philosophy**: Curiosity is intentionally a "luxury" drive that:
- Activates before survival needs are met
- Accumulates slowly so other drives take priority
- Has variable importance across the population
- Gets satisfied through exploration and discovery

### 2. Percepts Are Separate from Discoveries

**Percept System** (`src/agents/sensory_processing.rs`):
- Converts raw sensory input (vision, hearing, smell, speech) into 5 meaningful percept types
- Calculates salience (0.0-1.0) based on agent's current drives
- Filters percepts below attention threshold
- Does NOT currently have explicit "discovery percepts"

**Discovery System** (`src/agents/exploration.rs`):
- Parallel to percepts - tracks discoveries in `ExplorationKnowledge`
- 4 discovery types: Terrain, Resource, Building, AreaExplored
- Maintained as history in agent's exploration knowledge
- Converted to rewards via `calculate_exploration_reward()`

### 3. Satisfaction Mechanisms Are Multi-Layered

**Reward Calculation**:
- Resource discovery: 0.3 satisfaction (highest)
- Building discovery: 0.2 satisfaction
- Terrain discovery: 0.1 satisfaction
- Area explored: 0.01 per tile, capped at 0.5

**Application**:
- `curiosity_drive.partial_satisfy(reward)` reduces drive value
- Tracked via `SatisfactionTracker` to record sources
- Sources enable agents to form attachments to discovery locations

### 4. Integration Points Are Extensive

**Exploration System**:
- `should_explore()` decision logic based on drive state
- Drives exploration goals and pathfinding

**Technology System**:
- Recipe discoveries mentioned in satisfaction description
- Integrated via `ObservableEventType::Discovery`

**Knowledge System**:
- `PersonalKnowledge` tracks learned resources
- `KnowledgeSource` categorizes reliability (Personal, Direct, Overheard)
- Decay mechanism reduces reliability over time

**Emotion System** (Separate from Drive):
- `EmotionType::Curiosity` exists independently
- Decay rate 0.002/tick (PERSISTS - lowest decay rate)
- Affected by traits (Curious, Bookworm, Suspicious, Uncaring, Stoic)

**Learning System**:
- `ObservableEventType::Discovery` event for observational learning
- Family members teach +50%, trusted +20%
- Young agents learn faster

**Analytics System**:
- `DriveSnapshot[Curiosity]` per agent per tick
- Population aggregates and trend analysis
- Emergence detection for exploration waves

### 5. Type Structures Are Well-Organized

| Category | Types | Interfaces |
|----------|-------|-----------|
| **Drives** | Drive, DriveType, DriveState | is_active(), urgency(), satisfy() |
| **Exploration** | Discovery, DiscoveryType, ExplorationKnowledge | explore_tile(), discover_resource() |
| **Percepts** | Percept (5 types), DetectionMethod | process_sensory_input(), calculate_salience() |
| **Satisfaction** | SatisfactionRecord, DriveSatisfactionTracker | record(), get_importance() |
| **Knowledge** | ResourceKnowledge, PersonalKnowledge, KnowledgeSource | observe_resource(), learn_from_agent() |
| **Learning** | ObservableEvent, ObservableEventType, LearningResult | observe_and_learn() |
| **Emotions** | Emotion, EmotionType, EmotionalState | increase(), decrease(), decay() |

---

## System Architecture

```
AGENT DECISION LAYER
├─ DriveState[Curiosity] = 0.35
│  ├─ value: 0.35
│  ├─ threshold: 0.2
│  └─ weight: 0.8
│
PERCEPTION LAYER
├─ Sensory Input (Vision, Hearing, Smell, Speech)
├─ Percept Processing (5 types)
├─ Salience Calculation (0.0-1.0)
└─ Percept Filtering
│
EXPLORATION DECISION
├─ should_explore() = true (if conditions met)
├─ Find nearest unexplored tile
└─ Move toward it
│
DISCOVERY TRACKING
├─ ExplorationKnowledge
│  ├─ explored_tiles: HashSet
│  ├─ known_resources: HashMap
│  ├─ discoveries: Vec (history)
│  └─ total_tiles_explored: count
│
SATISFACTION LAYER
├─ calculate_exploration_reward(discovery) = 0.3
├─ curiosity_drive.partial_satisfy(0.3)
├─ SatisfactionTracker::record()
│  └─ Record source + amount for grief/attachment
│
KNOWLEDGE SYSTEM
├─ PersonalKnowledge::observe_resource()
├─ ObservableEvent::Discovery
└─ Nearby agents learn (1.5x if family)
│
ANALYTICS
├─ DriveSnapshot[Curiosity]
└─ Population aggregates
```

---

## Critical Observations

### Strengths
1. Clean separation between perception (percepts) and discovery tracking
2. Multi-level satisfaction tracking enables emotional attachment to locations
3. Flexible learning system with relationship bonuses
4. Both drive and emotion aspects captured
5. Well-tested core functionality

### Current Gaps
1. **No explicit discovery percept**: Discoveries aren't represented as Percept objects
   - Could enhance percept filtering for more intelligent exploration

2. **Recipe discovery underimplemented**: Mentioned in satisfaction description but mechanism unclear
   - Technology system integration needs verification

3. **Curiosity emotion ↔ drive disconnect**: Both exist but integration unclear
   - Could use curiosity emotion to modulate exploration likelihood

4. **No curiosity-specific percept salience**: ResourceDetected salience scales by hunger/thirst, not curiosity
   - Curious agents don't prioritize novel discoveries in perception

5. **Learning from discovery incomplete**: ObservableEvent::Discovery exists but full implementation unclear
   - Need to verify when/how recipes and knowledge propagate

---

## Files Referenced in Analysis

### Core System Files
- `/home/user/ebss-project/src/core/drives.rs` - 376 lines
- `/home/user/ebss-project/src/agents/exploration.rs` - 393 lines
- `/home/user/ebss-project/src/agents/sensory_processing.rs` - 414 lines
- `/home/user/ebss-project/src/agents/senses.rs` - 1038 lines
- `/home/user/ebss-project/src/agents/drive_satisfaction.rs` - 287 lines

### Integration Files
- `/home/user/ebss-project/src/agents/knowledge.rs` - 308 lines
- `/home/user/ebss-project/src/core/learning.rs` - ~200 lines
- `/home/user/ebss-project/src/core/emotions.rs` - ~200 lines
- `/home/user/ebss-project/src/core/traits.rs` - 584 lines
- `/home/user/ebss-project/src/analytics/mod.rs` - Multi-module

### Test Files
- `/home/user/ebss-project/src/core/tests/drive_satisfaction_tests.rs` - 321 lines
- Integrated tests in exploration.rs, sensory_processing.rs, knowledge.rs

---

## Key Parameters for Tuning

```rust
// Drive parameters (src/core/drives.rs)
Curiosity::threshold = 0.2
Curiosity::accumulation_rate = 0.004

// Exploration rewards (src/agents/exploration.rs)
Terrain = 0.1
Resource = 0.3
Building = 0.2
Area = min(0.01 * tiles, 0.5)

// Exploration triggers (src/agents/exploration.rs)
HIGH_CURIOSITY = 0.6
MODERATE_CURIOSITY = 0.3
MINIMUM_UNEXPLORED = 5 tiles
LOW_CURIOSITY = 0.2
EXPLORATION_RECENCY = 1000 ticks

// Percept salience ranges (src/agents/sensory_processing.rs)
DANGER = 0.7-1.0
RESOURCE = 0.0-0.9 (scaled by hunger/thirst)
AGENT = 0.0-0.8 (scaled by social)
COMMUNICATION = 0.4-0.7
ENVIRONMENT = 0.3-0.6
```

---

## Recommended Enhancements

1. **Add Discovery Percept Type**
   - Create `Percept::DiscoveryDetected` for novel items/areas
   - Enable exploration decisions from perception layer

2. **Complete Recipe Discovery**
   - Define recipe learning events in ObservableEventType
   - Integrate with crafting/technology system

3. **Curiosity-Aware Salience**
   - Modify `calculate_salience()` to boost unknown resource types
   - Let curious agents prioritize novel discoveries

4. **Emotion-Drive Feedback Loop**
   - Curiosity emotion should increase exploration likelihood
   - Create feedback between satisfaction and emotional state

5. **Knowledge Clustering**
   - Group discovered resources by location/type
   - Enable agents to form exploration strategies

---

## Conclusion

The curiosity system is a sophisticated, multi-layered implementation that successfully integrates drives, perception, discovery, learning, and analytics. While the core mechanisms are solid and well-tested, several integration points could be strengthened to fully realize the system's potential. The current implementation provides an excellent foundation for agent-driven exploration and knowledge acquisition.

**Project Context**: This analysis was conducted on commit `82d1067` (Merge pull request #13 - consolidate duplicate systems).

---

Generated: 2025-11-18
Analyzer: Claude Code File Search & Analysis Tool
