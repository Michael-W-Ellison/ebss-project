# Known Issues

**Last verified:** August 2026, against commit `b8e557e`.

Each entry below was reproduced before being written down, and each carries
the evidence. Entries are ordered by how much they block someone picking the
project up.

Every build configuration compiles today — default, `--features gui`,
`--features bevy_gui` and `--workspace` — so nothing here stops you building
and running the project.

---

## Correctness

### 1. Two tests fail intermittently

    world::tdd_tests::naturalistic_resource_tests::test_resource_clustering
    world::tdd_tests::spatial_planning_tests::test_minimize_travel_time_from_agent_position

Measured failure rates of roughly 1-in-10 to 1-in-20 per run, present long
before recent work (measured on unmodified code at 2/20 and 3/15 for related
cases). Both build a world through `World::new`, which draws from
`thread_rng`, and then assert on a property a random world does not always
have — for example that clay deposits happen to be clustered.

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
its freshness decays to zero and spoilage removes it. Harmless, but it means
carried weight includes rot.

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
- **Conception crashed the simulation.** Fertility could exceed 1.0 and was
  used as a probability; 44.7% of adult pairs produced odds above 1.0, which
  panics the sampler. Only reachable once agents could feed and water
  themselves well enough to reproduce.
