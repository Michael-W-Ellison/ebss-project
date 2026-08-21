# Known Issues

**Last verified:** August 2026, against commit `b8e557e`.

Each entry below was reproduced before being written down, and each carries
the evidence. Entries are ordered by how much they block someone picking the
project up.

---

## Blocking

### 1. The Bevy front end does not compile

**Where:** `src/bevy_gui/ui/mod.rs:1001-1013`
**Reproduce:** `cargo check --features bevy_gui` — 9 errors

The panel reads fields that no longer exist on `HistoryPoint`: `infants`,
`children`, `adolescents`, `adults`, `elderly`, `avg_health`, `avg_energy`,
`avg_happiness`, `buildings_construction`. The struct was refactored and this
call site was not updated with it.

Mechanical to fix: either restore the fields on `HistoryPoint` or read the
values from wherever they moved. The egui front end (`--features gui`) is
unaffected and works.

### 2. The bundled plugin crate does not compile

**Where:** `plugins/minecraft_survival/src/lib.rs`
**Reproduce:** `cargo check --workspace` — 17 errors

The crate is written against an older API in which `environment::Action` was a
struct with `new()`, `.effects`, `.id` and `.action_type`. `Action` is now an
enum. This breaks `cargo check --workspace` and `cargo test --workspace` for
everyone, whether or not they care about the plugin.

The in-tree `src/environment/minecraft_survival.rs` is the current, working
version of the same environment. The plugin crate is a stale copy of it, so
the cheapest fix may be to delete the crate or rewrite it against the enum.

---

## Correctness

### 3. Two tests fail intermittently

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

### 4. No error recovery around a tick

One panicking agent ends the whole run and loses everything since the last
autosave. There is no isolation of per-agent failure and no attempt to
continue after an error. This mattered concretely: a probability bug in
conception crashed roughly one run in twenty-five until it was fixed, and each
crash took the entire simulation with it.

---

## Design gaps that show up as odd behaviour

### 5. Agents never make or wear clothing

Cold insulation comes only from equipment, and nothing drives an agent to
craft or equip anything. Insulation is therefore always zero, and agents spend
their lives cycling between being cold and sheltering. Clothing recipes
(`src/environment/clothing_recipes.rs`) and the equipment system both exist
and work when items are placed on an agent by hand.

This is the largest behavioural gap: cold is currently a condition agents
endure rather than a problem they solve, which is exactly the kind of
emergence the project exists to produce.

### 6. Fear is a hunger signal, not a danger signal

`calculate_survival_drive_emotion` derives fear from unmet hunger, thirst and
rest. Since hunger saturates between meals, fear sits at around 0.8 much of
the time. `should_flee` triggers above 0.6, so agents read as fleeing in
ordinary circumstances rather than in response to a threat.

Survival actions now outrank fleeing, so this no longer strands agents, but
the emotional model is still reporting something misleading, and anything
built on `should_flee` inherits that.

### 7. Perception is smell-only

Agents smell food and water. Nothing feeds vision or hearing from the world,
so `Percept::AgentDetected` and every sound-derived percept are dead paths in
a live run. Social behaviour works because `Population` computes proximity
directly rather than perceiving it. See SIMULATION_AUDIT.md.

### 8. Zoning and territory are never established

Building placement scoring reads zone and territory bonuses from
`World::zone_manager` and `World::territory_manager`, but nothing outside the
tests ever calls `add_zone` or `claim_territory`. Both managers are therefore
always empty in a live run and every bonus they contribute is zero, so
settlements have no planned structure and agents claim no ground.

### 9. Agents carry food they will never eat

Food that has turned is correctly refused, but stays in the inventory until
its freshness decays to zero and spoilage removes it. Harmless, but it means
carried weight includes rot.

---

## Housekeeping

### 10. Committed backup file

`src/analytics/mod.rs.backup` is checked into the repository.

### 11. Build warnings

15 warnings on `cargo build`, all unused variables and imports. `cargo fix`
handles most.

### 12. Placeholder package metadata

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
- **Conception crashed the simulation.** Fertility could exceed 1.0 and was
  used as a probability; 44.7% of adult pairs produced odds above 1.0, which
  panics the sampler. Only reachable once agents could feed and water
  themselves well enough to reproduce.
