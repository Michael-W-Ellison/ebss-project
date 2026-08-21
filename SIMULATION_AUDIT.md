# EBSS Simulation Feature Audit

**Last verified:** August 2026, against commit `b8e557e`
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

### Lifecycle
- Aging through infant, child, adolescent, adult and elderly stages
- Mate selection, pregnancy with prenatal nutrition, birth, nursing, and
  developmental nutrition that modifies adult stats
- Inheritance of traits and behaviour trees from both parents

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
- Fauna (movement, hunger, breeding, predation) and flora (growth, regrowth)
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
| Vision (`senses::Vision`) | Nothing calls `update_visible_agents` or `update_visible_positions`, so agents never see anything |
| Hearing (`senses::Hearing`) | Nothing feeds sounds from the world |
| `world::zoning`, `world::territory` | Read by building placement scoring (`spatial_planning.rs`), but nothing outside tests ever calls `add_zone` or `claim_territory`, so both managers are always empty and every bonus they contribute is zero |

**Consequence for perception:** the only sensory channel the simulation feeds
is smell, and only for food and water. Agents locate resources by scent and
by remembering where they have been. They do not see each other; social
proximity is computed directly by `Population`, not perceived. Any reasoning
that depends on `Percept::AgentDetected` or on sound is a dead path in a live
run.

---

## Absent

- **Clothing behaviour.** Clothing recipes, equipment slots and cold
  insulation all exist and work when equipment is present. Nothing drives an
  agent to make or wear anything, so insulation is always zero and agents
  cycle between cold and shelter for their whole lives.
- **Seeded world generation.** `World::new` draws from `thread_rng`, so runs
  cannot be reproduced and two tests are intermittently flaky.
- **Long-run characterisation.** Nobody has studied population dynamics,
  technology spread or settlement patterns past a few tens of thousands of
  ticks.

---

## Measured behaviour

From ten independent worlds, twelve starting agents each, eight thousand ticks
(`Simulation::tick` driven directly, no GUI):

| Measure | Result |
| --- | --- |
| Populations dying out | 0 of 10 |
| Population trajectory | 12 → 18-37, by live births |
| Agents fed at the end | 296 of 299 |
| Agents hydrated at the end | 291 of 299 |
| Agents critically exposed | 0 |
| Typical core temperature | 35-37 °C |
| Typical time since last drink | ~30 ticks |

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

## Test coverage

1,055 library tests, 15 integration tests, 1 doc test. All pass, except two
known flaky tests (`test_resource_clustering`,
`test_minimize_travel_time_from_agent_position`) that assert on properties a
randomly generated world does not always have.

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
