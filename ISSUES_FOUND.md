# Known Issues

**Last verified:** August 2026, against commit `1b98aa4`.

Each entry below was reproduced before being written down, and each carries
the evidence. Entries are ordered by how much they block someone picking the
project up.

Every build configuration compiles today — default, `--features gui`,
`--features bevy_gui` and `--workspace` — so nothing here stops you building
and running the project.

---

## Correctness

### 1. Three tests fail intermittently

    world::tdd_tests::naturalistic_resource_tests::test_resource_clustering
    world::tdd_tests::spatial_planning_tests::test_minimize_travel_time_from_agent_position
    analytics::tests::agent_building_integration_tests::test_production_building_placed_near_resources

Measured failure rates of roughly 1-in-10 to 1-in-20 per run for the first two
and 4-in-120 for the third, all present long before recent work (measured on
unmodified code at 2/20, 3/15 and 4/120). All three build a world through
`World::new`, which draws from `thread_rng`, and then assert on a property a
random world does not always have — for example that clay deposits happen to
be clustered, or that a forge finds somewhere near the iron to stand.

The fix is to give world generation a seed, which the project wants anyway for
reproducible runs. Until then, a red build is not necessarily a real failure,
which is corrosive: check whether the failing test is one of these two before
assuming a regression.

### 2. No error recovery around a tick

One panicking agent ends the whole run and loses everything since the last
autosave. There is no isolation of per-agent failure and no attempt to
continue after an error. This mattered concretely: a probability bug in
conception crashed roughly one run in twenty-five until it was fixed, and each
crash took the entire simulation with it.

---

## Design gaps that show up as odd behaviour

### 3. Agents never make or wear clothing

Cold insulation comes only from equipment, and nothing drives an agent to
craft or equip anything. Insulation is therefore always zero, and agents spend
their lives cycling between being cold and sheltering. Clothing recipes
(`src/environment/clothing_recipes.rs`) and the equipment system both exist
and work when items are placed on an agent by hand.

This is the largest behavioural gap: cold is currently a condition agents
endure rather than a problem they solve, which is exactly the kind of
emergence the project exists to produce.

### 4. Fear is a hunger signal, not a danger signal

`calculate_survival_drive_emotion` derives fear from unmet hunger, thirst and
rest. Since hunger saturates between meals, fear sits at around 0.8 much of
the time. `should_flee` triggers above 0.6, so agents read as fleeing in
ordinary circumstances rather than in response to a threat.

Survival actions now outrank fleeing, so this no longer strands agents, but
the emotional model is still reporting something misleading, and anything
built on `should_flee` inherits that.

### 5. Agents still cannot see each other, or hear anything

Sight now discovers terrain, resources and buildings, but only through the
exploration path. The percept pipeline's own vision channel reads
`vision.visible_agents`, which nothing populates, so `Percept::AgentDetected`
is still never produced and agents do not perceive one another — social
behaviour works because `Population` computes proximity directly. Hearing is
unfed entirely, so every sound-derived percept is a dead path. See
SIMULATION_AUDIT.md.

### 6. Zoning and territory are never established

Building placement scoring reads zone and territory bonuses from
`World::zone_manager` and `World::territory_manager`, but nothing outside the
tests ever calls `add_zone` or `claim_territory`. Both managers are therefore
always empty in a live run and every bonus they contribute is zero, so
settlements have no planned structure and agents claim no ground.

### 7. Agents carry food they will never eat

Food that has turned is correctly refused, but stays in the inventory until
its freshness decays to zero and spoilage removes it. The same is true of food
an agent burns: a novice cook ruins about one batch in five, and the ruins ride
along in the pack. Both announce themselves as a decay scent to anyone nearby,
which is realistic and mildly useful, but nothing makes the carrier drop them:
carried weight still includes rot and cinders.

---

## Housekeeping

### 8. Committed backup file

`src/analytics/mod.rs.backup` is checked into the repository.

### 9. Build warnings

15 warnings on `cargo build`, all unused variables and imports. `cargo fix`
handles most.

### 10. Placeholder package metadata

`Cargo.toml` still declares `authors = ["Your Name <your.email@example.com>"]`
and `repository = "https://github.com/yourusername/ebss-project"`.

---

## Recently fixed

Listed so nobody re-investigates them. Each has regression tests in
`src/analytics/tests/`.

- **Death mechanics not integrated.** `Simulation::tick` bypassed
  `Population::tick`, so aging, starvation and death never ran. This was the
  critical issue in the previous version of this document. Fixed.
- **Survival subsystems never invoked.** `Population::tick` reached past
  `tick_with_time`, so nutrition metabolism, food spoilage and awake fatigue
  had no effect on a live run despite full unit-test coverage.
- **Agents starved holding food.** Eating ignored inventory, goals outranked
  hunger indefinitely, and action selection was gated on having a matching
  behaviour tree.
- **Agents were permanently hypothermic.** Thermoregulation was weaker than
  environmental heat transfer, so core temperature settled near ambient;
  shelter did not affect body temperature; exposure damage never decreased.
- **Agents never drank.** Thirst was reachable only through the drive fallback
  that hunger monopolised. Agents went thousands of ticks without water beside
  a river.
- **Survival damage was erased.** Health was overwritten from body condition
  every tick, discarding starvation, dehydration and exposure damage.
- **The Bevy front end did not compile.** A statistics CSV exporter read
  life-stage and construction fields that a refactor had dropped from
  `HistoryPoint`. The fields are back, populated from the snapshot data that
  was already available at the sampling site.
- **The bundled plugin crate did not compile.** It was written against an
  `Action` descriptor struct that no longer exists; `Action` is now an enum
  with no cost data. The plugin now registers enum actions keyed by id and
  keeps their costs and requirements in an `ActionProfile` beside them, which
  also gives `ActionType`, `ActionEffects` and `ActionRequirements` a user
  again — they were orphaned.
- **Sight did nothing.** `process_exploration_with_world`, the only path that
  discovers the world by line of sight, had no callers, so agents found food by
  smell alone. It now runs each tick from `Simulation::tick`, scaled by the
  agent's visual acuity, and what an agent sees of food and water reaches the
  spatial memory that foraging reads. A new `Blind` trait sets sight range to
  zero.
- **Sensory traits never reached the senses.**
  `apply_trait_sensory_modifications` had no callers, so a `Deaf` agent heard
  perfectly well. It now runs at creation and when a newborn inherits traits.
- **Nothing ever cooked.** Heat sources, fuel, lighting and the whole
  preparation model existed, and nothing in a run had ever lit a fire, so every
  meal was eaten raw at about a third of its value and the strongest smell in
  the model was unreachable. Agents now gather wood, light campfires and cook
  at them, which is what moved the fraction of fed agents from 96.0% to 99.7%.
  Cooking is restricted to food a fire improves — meat, fish, grain — and
  anything else put over one is ruined, as is anything cooked twice.
- **Smell found everything, and sight found nothing twice.** Every resource
  emitted the same full-strength scent, so an agent smelled a berry patch from
  twenty-five tiles and sight was decoration. Scent strength now depends on
  what the thing is and what has been done to it — a berry carries about two
  tiles, water three, flesh six, rot nine to twenty, cooking the full range —
  and sight reaches twenty-five so that looking is what finds food. Sight also
  stopped being a one-off: exploration reports a tile only the first time it is
  looked at, so an agent used to stop noticing a patch once it had walked past
  it, and nothing brought the memory back when it faded.
- **Conception crashed the simulation.** Fertility could exceed 1.0 and was
  used as a probability; 44.7% of adult pairs produced odds above 1.0, which
  panics the sampler. Only reachable once agents could feed and water
  themselves well enough to reproduce.
