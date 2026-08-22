# Known Issues

**Last verified:** August 2026, against commit `5c74481`.

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

### 3. Settlements die out over a long run

Every population tested was gone by thirty thousand ticks — three worlds of
three, on this commit and on the one before any of the ecology work, so the
animals are not the cause. Nobody has looked at what is killing them.

Up to eight thousand ticks, which is what everything else here is measured
over, settlements grow. What happens between there and thirty thousand is
uncharacterised.

### 4. The ecology settles in most worlds, not all

Over forty worlds, predators are still alive at the end in thirty and herds
stay bounded in thirty-two. In the eight that run away the predators died out
first; in some others the herd goes and takes the predators with it. Nothing
recolonises: once a species is gone from a world it is gone for good, so a
world that loses its predators early spends the rest of the run with herds
climbing to the population cap.

### 5. Clothing and hunting cost about what they return

Over forty worlds, clothing halves how often agents are cold (28% to 16%) and
warms cores by half a degree, at three points of the fed population and three
percent of the population itself. The material is scarce and the climate is
mild, so the time spent gathering flax is close to break-even against the time
it would have spent on food. Nothing in the model weighs the two: the ordering
in `generate_non_emotional_action` is fixed, and an agent picks material by
warmth against distance rather than by what its stores can afford.

An inventory stack also carries one quality for the whole stack, which is why
making and wearing had to become a single act: a better second coat merged
into the first and was recorded as no better than it.

Hunting is the same shape. Over forty worlds it puts 44 agents of 862 into
fur, hide or leather — which nothing else can — at two points of the fed
population and about eight percent of the population itself. A world starts
with under a dozen animals, so most agents never find one.

### 6. Fear is a hunger signal, not a danger signal

`calculate_survival_drive_emotion` derives fear from unmet hunger, thirst and
rest. Since hunger saturates between meals, fear sits at around 0.8 much of
the time. `should_flee` triggers above 0.6, so agents read as fleeing in
ordinary circumstances rather than in response to a threat.

Survival actions now outrank fleeing, so this no longer strands agents, but
the emotional model is still reporting something misleading, and anything
built on `should_flee` inherits that.

### 7. Agents still cannot see each other, or hear anything

Sight now discovers terrain, resources and buildings, but only through the
exploration path. The percept pipeline's own vision channel reads
`vision.visible_agents`, which nothing populates, so `Percept::AgentDetected`
is still never produced and agents do not perceive one another — social
behaviour works because `Population` computes proximity directly. Hearing is
unfed entirely, so every sound-derived percept is a dead path. See
SIMULATION_AUDIT.md.

### 8. Zoning and territory are never established

Building placement scoring reads zone and territory bonuses from
`World::zone_manager` and `World::territory_manager`, but nothing outside the
tests ever calls `add_zone` or `claim_territory`. Both managers are therefore
always empty in a live run and every bonus they contribute is zero, so
settlements have no planned structure and agents claim no ground.

### 9. Agents carry food they will never eat

Food that has turned is correctly refused, but stays in the inventory until
its freshness decays to zero and spoilage removes it. The same is true of food
an agent burns: a novice cook ruins about one batch in five, and the ruins ride
along in the pack. Both announce themselves as a decay scent to anyone nearby,
which is realistic and mildly useful, but nothing makes the carrier drop them:
carried weight still includes rot and cinders.

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
- **Herbivores had nothing holding them down.** The world was ticked twice per
  simulation tick; predation sat behind one roll for the whole world per tick;
  a predator only hunted when half starved; predators were stocked without
  regard to whether anything they ate lived there; the default world got two
  herds in total; and nothing but the hard population cap limited a herd. All
  six are fixed, and predators now survive in thirty worlds of forty rather
  than seven. A starving predator also widens what it will take and will turn
  on a settlement — nothing in the model let an animal touch an agent before.
- **Nothing ever hunted.** `Action::Hunt` and the fauna model worked, and the
  one place the action appeared passed a nil animal id the executor could not
  resolve, so meat, hides and wool never reached an inventory. Three things
  had to be fixed with it: kills dropped names nothing downstream knew
  (mutton, deer_meat, thick_hide) and are butchered now; the odds read the
  MeleeCombat skill with no floor, so an untrained agent's chance was exactly
  zero and the first kill it made locked it out of hunting for life; and a
  hunter could kill an animal on the far side of the map without moving.
- **Insulation was always zero.** Clothing recipes, equipment slots and cold
  insulation all existed and worked when a garment was put on an agent by
  hand; nothing drove an agent to make or wear anything, so cold was endured
  rather than solved. Agents now gather flax, cotton and bark, make garments
  and wear them, and just over half the population ends a run dressed. Four
  things had to be fixed before it worked at all: wood being burned on boots
  instead of fires, garments piling up unworn because a stack carries one
  quality, coats being replaced for ordinary wear, and cast-offs being carried
  around at two kilos each.
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
