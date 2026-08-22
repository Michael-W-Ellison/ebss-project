# EBSS Simulation Feature Audit

**Last verified:** August 2026, against commit `5c74481`
**Method:** every claim below was checked by reading the call chain from
`Simulation::tick()` outward, or by running the simulation and measuring the
result. Claims that could not be verified either way are marked as such.

---

## Why this audit is organised around "is it wired in"

The recurring shape of defects in this codebase has not been missing features.
It has been complete, well-tested subsystems that nothing in the live
simulation loop calls. Nutrition metabolism, food spoilage and awake fatigue
each had thorough unit tests and zero effect on a running simulation, because
`Population::tick` reached past the function that invoked them. Health
regeneration had no callers at all. The food database was never instantiated.

So this audit separates three states, and the middle one is where the risk
lives:

| State | Meaning |
| --- | --- |
| **Running** | Reached from `Simulation::tick()` and observable in a run |
| **Built, not connected** | Implemented and tested, but no live caller |
| **Absent** | Not implemented |

A subsystem being "implemented" and "tested" says nothing about whether it
does anything. Check the call chain.

---

## Running

Verified reachable from `Simulation::tick()`.

### Survival
- Hunger, thirst and energy, with agents foraging, drinking and eating from
  inventory or the land
- Nutrition: energy reserves, protein and micronutrients, with deficiency
  penalties; food carries nutrition and spoils, and agents refuse food that
  would make them sick
- Body temperature with insulation, wind chill and heat index; exposure
  (hypothermia, hyperthermia, frostbite, sunburn, windburn, dehydration) that
  accumulates, is capped, and recovers
- Shelter: buildings and woodland moderate temperature and let exposure heal
- Fatigue and sleep, with sleep quality from shelter, safety, health and hunger
- Injury, healing, and death from starvation, dehydration, exposure, injury
  and old age
- Fire and cooking: agents gather wood, build and light campfires, and cook at
  them. Only meat, fish and grain are improved; anything else over a fire is
  ruined, as is anything already cooked or preserved. Ruined food has nothing
  left in it, is unsafe to eat and smells of decay. How often a cook burns a
  batch falls from one in five to none with practice
- Hunting: agents go after animals for the skins and eat what comes with them.
  A hunter has to be within a spear's throw; an unarmed one leaves anything
  that fights back alone, and dangerous prey that gets away leaves a mark. A
  kill is butchered into meat, hides, leather and wool - the meat carrying
  nutrition, so it can be cooked and eaten
- Clothing: agents gather flax, cotton and bark, make garments and wear them,
  and insulation is no longer zero. A garment is worth what its material is
  worth (fur and wool best, plant fibre next, bark last) and what the hand
  that made it could manage. Wood goes into clothes only once a fire's worth
  is set aside, and the coat a new one replaces is left behind

### Lifecycle
- Aging through infant, child, adolescent, adult and elderly stages
- Mate selection, pregnancy with prenatal nutrition, birth, nursing, and
  developmental nutrition that modifies adult stats
- Inheritance of traits and behaviour trees from both parents

### Perception
- Sight, which is how food is actually found: agents discover terrain,
  resources and buildings within 25 tiles, refreshed every tick rather than
  once per tile, and what they see of food and water reaches the same spatial
  memory that foraging reads. The `Blind` trait sets sight range to zero,
  leaving such agents to smell and word of mouth
- Smell, scaled to what a thing gives off rather than a flat full-strength
  scent on everything. As a fraction of the nose's 25-tile range:

  | Source | Strength | Reaches |
  | --- | --- | --- |
  | Berries, grain, herbs on the land | 0.08 | ~2 tiles |
  | Water | 0.12 | ~3 tiles |
  | Meat, fish | 0.24 | ~6 tiles |
  | Rotting food, wherever carried | 0.35-0.80 | 9-20 tiles |
  | Food over a lit fire | 1.00 | 25 tiles |

  Rot is emitted as `ScentType::Decay`, not food: it reports that something is
  off, and does not send an agent over to eat it. Burnt food smells the same
  way. The cooking scent used to be dormant because nothing ever lit a fire;
  agents light them now, so it is something a nose can actually meet.
- Agents share what they know of resource locations with neighbours, so a
  blind agent can be told where the food is

### Behaviour
- 14 drives with per-agent weights and thresholds
- Behaviour trees with weight-based learning and pruning
- Goals and multi-step plans, abandoned when no longer relevant
- Action selection ordered: starvation, emotional response, shelter,
  perception, plan, goal, drive
- Obstacle-aware movement (greedy step, then a bounded breadth-first route
  search), committed search legs when looking for something out of range

### Social
- Proximity-based relationships and bonds, decay at distance
- Social interactions, gossip and information spread
- Observational learning between agents
- Shared knowledge, technology discovery and spread

### World
- Terrain, climate, weather, seasons, day/night
- Resources with regeneration; renewable nodes persist when emptied so they
  regrow, non-renewable deposits are removed
- Buildings, construction and maintenance
- Crafting, smelting, technology progression
- Fauna: movement, hunger, breeding, and predation that is an actual brake -
  every hungry predator hunts on its own account, a herd is limited by the
  ground it grazes as well as by what eats it, a starving predator widens what
  it will take, and one beside a settlement turns on the people
- Flora (growth, regrowth)
- Combat between agents and hunting of animals

### Persistence and display
- Save and load via MessagePack; autosave with checkpoint rotation
- ASCII renderer
- egui GUI (`cargo run --features gui --bin ebss_gui`)

---

## Built, not connected

Each of these is implemented and has tests. None is driven by
`Simulation::tick()`.

| Component | State |
| --- | --- |
| `analytics::web_api` (`ApiServer`) | **Zero call sites in the entire repo.** An HTTP API with no server started and no front end |
| `analytics::events` (`EventBus`) | Constructed only inside its own tests |
| `analytics::replay` (`SessionRecorder`) | Constructed only inside its own tests |
| `analytics::storage` (`StorageManager`) | Constructed only inside its own tests |
| `analytics::metrics` (`SimulationMetrics`) | Works when driven; `examples/ascii_simulation.rs` and `examples/phase4_analytics.rs` show how |
| `analytics::emergence` (`EmergenceDetector`) | Same — driven by those two examples only |
| `analytics::performance` (`PerformanceMonitor`) | Same — driven by those two examples only |
| Vision, as a percept channel (`senses::Vision`) | Sight now drives exploration and resource discovery, but nothing calls `update_visible_agents` or `update_visible_positions`, so `visible_agents` stays empty and agents still never see *each other* |
| Hearing (`senses::Hearing`) | Nothing feeds sounds from the world |
| `world::zoning`, `world::territory` | Read by building placement scoring (`spatial_planning.rs`), but nothing outside tests ever calls `add_zone` or `claim_territory`, so both managers are always empty and every bonus they contribute is zero |

**Consequence for perception:** agents find the world by sight and smell, but
the percept pipeline itself is still fed only by smell. `Percept::
ResourceDetected` comes from scents; `Percept::AgentDetected` comes from
`visible_agents`, which nothing populates, so agents do not see each other and
social proximity is computed directly by `Population` rather than perceived.
Anything depending on sound is likewise a dead path.

Sight reaches 25 tiles and every smell food gives off where it lies reaches
between 2 and 6, so looking is what finds dinner and smelling is what warns
you the pack has turned. Spatial memory is fed by both, which is why a blind
agent still eats: rot, a fire, and what the neighbours tell it. The dials are
`BASE_SIGHT_RANGE` in `Agent::sight_range`,
`ResourceType::raw_scent_strength` and `FoodData::scent_strength`.

---

## Absent

- **Trading warmth for food.** Clothing halves how often agents are cold and
  costs about three points of the fed population (see **Measured behaviour**).
  Nothing weighs the two against each other; the ordering in
  `generate_non_emotional_action` is fixed, and the material an agent will
  cross the map for is chosen by warmth over distance, not by what its stores
  can afford.
- **Seeded world generation.** `World::new` draws from `thread_rng`, so runs
  cannot be reproduced and two tests are intermittently flaky.
- **Long-run characterisation.** Nobody has studied population dynamics,
  technology spread or settlement patterns past a few tens of thousands of
  ticks.

---

## Measured behaviour

From forty independent worlds, twelve starting agents each, eight thousand
ticks (`Simulation::tick` driven directly, no GUI), measured at each of the
last three steps:

| Measure | Flat scent, no fire | Human nose, no fire | With cooking |
| --- | --- | --- | --- |
| Populations dying out | 0 of 40 | 0 of 40 | 0 of 40 |
| Population at the end | 985 from 480 | 944 from 480 | 1055 from 480 |
| Agents fed at the end | 94.8% | 96.0% | 99.7% |
| Agents hydrated at the end | 98.5% | 99.3% | 99.1% |
| Agents critically exposed | 0 | 0 | 0 |
| Typical core temperature | 35-37 °C | 35-37 °C | 35-37 °C |

Cooking is what moved feeding: raw food gives up about a third of what is in
it and cooked food nearly all, so the same forage feeds far more agents.

Clothing, measured on the commit before it and the commit that added it (a
different harness from the runs above, so read each column against its own
pair rather than against the table before it):

| Measure | Before clothing | With clothing |
| --- | --- | --- |
| Populations dying out | 0 of 40 | 0 of 40 |
| Population at the end | 1054 from 480 | 1021 from 480 |
| Agents wearing anything | 0 | 536 (53%) |
| Average cold insulation | 0.00 | 0.13 |
| Agents cold at the end | 297 (28.2%) | 164 (16.1%) |
| Average core temperature | 35.8 °C | 36.3 °C |
| Agents fed at the end | 96.8% | 93.6% |
| Agents hydrated at the end | 99.1% | 99.0% |

Clothing is not free, and in this climate it is close to an even trade: it
halves how often an agent is cold and warms cores half a degree, and the time
and material go somewhere — three points of the fed population and three
percent of the population itself. It would pay better in a colder world.

Hunting, measured the same way, the two runs made back to back:

| Measure | Without hunting | With hunting |
| --- | --- | --- |
| Populations dying out | 0 of 40 | 1 of 40 |
| Population at the end | 941 from 480 | 862 from 480 |
| Agents dressed in skins | 0 | 44 |
| Average cold insulation | 0.12 | 0.14 |
| Agents cold at the end | 15.0% | 13.7% |
| Agents fed at the end | 98.8% | 96.8% |
| Animals alive at the end | 11,386 | 2,878 |

Two things there are worth reading twice. Hunting is what makes fur, hide and
leather reachable at all, and 44 agents of 862 get there — most never find an
animal, because a world starts with under a dozen. And hunters hold the
herbivores down by about four times: the fauna population runs away without
them, which it did long before agents could hunt and is nobody's design.

Worth knowing before reading too much into a single run: the spread between
worlds is wide. Twenty-world samples of the same build came out anywhere
between 92% and 98% fed, which is why the comparisons above are over forty.

Thermal model behaviour, by settled core temperature of an unclothed agent:

| Ambient | Core | Verdict |
| --- | --- | --- |
| 20 °C | 37.0 | comfortable |
| 10 °C | 37.0 | comfortable |
| 0 °C | 35.1 | marginal |
| −20 °C | 29.6 | hypothermic |
| −20 °C with 0.8 insulation | 37.0 | comfortable |
| 60 °C | 42.0 | hyperthermic |

---

The ecology, measured the same way on the commit before it and the one that
added it:

| Measure | Before | After |
| --- | --- | --- |
| Worlds with predators still alive at the end | 7 of 40 | 30 of 40 |
| Worlds with herds bounded (under 150) | — | 32 of 40 |
| Herbivores, as a multiple of the founding stock | 11x | 4.8x |
| Agents mauled by animals | 0 | 31 |
| Agent population | 903 | 817 |

Before this, predation sat behind one roll for the whole world per tick, a
predator only hunted when half starved, predators were stocked without regard
to what they could eat, and nothing but the hard population cap of a thousand
limited a herd. Nothing in the model let an animal touch an agent at all.

It is not stable everywhere. In the eight worlds where herds are not bounded,
the predators died out first; in some others the herd goes instead and takes
the predators with it. A world holding both at once is the common case now
rather than the rare one, which it was not before.

## Test coverage

1,101 library tests, 15 integration tests, 21 plugin tests, 1 doc test. All
pass, except three known flaky tests (`test_resource_clustering`,
`test_minimize_travel_time_from_agent_position`,
`test_production_building_placed_near_resources`) that assert on properties a
randomly generated world does not always have. The third was measured at
4 failures in 120 runs on the commit before this one, so it is not new.

Coverage is dense at the unit level and thin at the "does this run in a real
simulation" level, which is precisely how the wiring defects survived. The
regression tests added for survival, shelter and thirst
(`src/analytics/tests/`) deliberately drive a whole `Simulation` for thousands
of ticks and assert on the outcome, rather than calling a subsystem directly.
More tests of that shape would be the single best defence against this class
of bug.

---

## Superseded

An earlier version of this document listed auto-save, deterministic replay,
configuration validation, `SimulationConfig` and error recovery as missing.
Auto-save, replay, config validation and `SimulationConfig` have since been
implemented — the first three are running, replay is built but unconnected.
Error recovery (isolating a panicking agent so one failure does not end the
run) is still absent.
