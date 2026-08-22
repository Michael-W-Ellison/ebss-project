# Known Issues

**Last verified:** August 2026, against commit `0e3500d`.

Each entry below was reproduced before being written down, and each carries
the evidence. Entries are ordered by how much they block someone picking the
project up.

Every build configuration compiles today — default, `--features gui`,
`--features bevy_gui` and `--workspace` — so nothing here stops you building
and running the project.

---

## Correctness

### 1. Six tests fail intermittently

    world::tdd_tests::naturalistic_resource_tests::test_resource_clustering
    world::tdd_tests::spatial_planning_tests::test_minimize_travel_time_from_agent_position
    analytics::tests::agent_building_integration_tests::test_production_building_placed_near_resources
    analytics::tests::agent_building_integration_tests::test_production_chain_buildings_cluster
    analytics::tests::agent_building_integration_tests::test_different_building_types_use_appropriate_strategies
    analytics::tests::longevity_tests::water_is_not_used_up

Measured failure rates of roughly 1-in-10 to 1-in-20 per run for the first two,
4-in-120 for the third and 1-in-30 to 1-in-40 for the next two, all present long
before recent work (measured on unmodified code at 2/20, 3/15, 4/120, 1/40 and
1/30). The last was seen to fail once and then pass six times running; it
asserts that a world holds 95% of its water after six thousand ticks, and
across twelve worlds the worst case sits at 98.4% — on the commit before the
calendar was fixed it sat at 95.6%, so the margin got wider rather than
narrower, and the tail is simply thin. All six build a world through
`World::new`, which draws from `thread_rng`, and
then assert on a property a random world does not always have — for example
that clay deposits happen to be clustered, or that a forge finds somewhere near
the iron to stand.

The fix is to give world generation a seed, which the project wants anyway for
reproducible runs. Until then, a red build is not necessarily a real failure,
which is corrosive: check whether the failing test is one of these six before
assuming a regression.

### 2. No error recovery around a tick

One panicking agent ends the whole run and loses everything since the last
autosave. There is no isolation of per-agent failure and no attempt to
continue after an error. This mattered concretely: a probability bug in
conception crashed roughly one run in twenty-five until it was fixed, and each
crash took the entire simulation with it.

---

## Design gaps that show up as odd behaviour

### 3. A settlement that overshoots slides instead of settling back

Traced over six worlds to thirty thousand ticks. A settlement grows, strips the
ground it farms, and then slides — it does not find a smaller level and hold
there. One world went 12 → 219 people and was down to 81 and still falling at
thirty thousand ticks, on a standing crop of twenty-four units.

Three things make it a slide rather than a correction.

**Growing food takes nutrient out of the tile, and regrowth is proportional to
what is left.** `ResourceNode::regenerate_in_ground` draws `0.0015` nutrients
per unit grown and scales the rate by `soil.fertility()`. Every unit eaten
makes the next one slower to arrive, so production decays with cumulative
harvest towards zero. The only equilibrium is the one where almost nobody
lives there. Measured on the ground a settlement actually farms:

| tick | people | standing crop | fertility of the farmed ground |
| --- | --- | --- | --- |
| 0 | 12 | 1,414 | 0.529 |
| 10,000 | 50 | 6,138 | 0.509 |
| 20,000 | 111 | 3,875 | 0.304 |
| 24,500 | 219 | 1,367 | 0.106 |
| 30,000 | 81 | 24 | 0.025 |

**The ground does not come back on any timescale the simulation reaches.**
Twenty-two thousand ticks of settlement took farmed ground from 0.528 to 0.362.
Thirty thousand further ticks with every agent removed from the world returned
0.017 of it — a tenth, over a span longer than the run that did the damage, and
slowing as it went, because the litter that feeds the recovery is running out
too. Depletion under a hundred people runs about sixty-five times faster than
recovery under nobody.

**Nothing brakes the population until the standing crop is gone.**
`should_attempt_reproduction` suppresses breeding only while the Hunger or
Thirst drive is above threshold. Hunger's threshold is 0.7 and the measured
value sits at 0.5-0.6 for the whole run: a shrinking stock that still yields a
meal today reads as "not hungry". The population went 111 → 219 while the crop
fell from 3,875 to 1,367, and peaked about nine calendar years after the ground
had already lost eighty per cent of its fertility.

Two things make it worse.

**A spent field still counts as a field.** `fields_within` counts cultivated
tiles, not producing ones, and `farming_action` stops at `FIELDS_WANTED` within
`FIELD_WALK_RADIUS`. A tile that already carries a resource node cannot be
tilled again. So six exhausted fields inside twelve tiles stop a settlement
breaking new ground for ever. The farmed tiles in the run above ended at 0.025
fertility while the map around them averaged 0.358 — one fourteenth of the
ground they were standing on. The world is not short of nutrient; the four per
cent of it that anybody farms is.

**Nutrient only ever leaves.** Food eaten is gone from the world. Food that
spoils in a pack is deleted outright by `tick_food_spoilage` rather than
falling to the ground as litter. The single return path is muck-spreading,
which needs an agent to be carrying rotting food, standing on a field, and to
have learned the practice.

Worth knowing: **nobody has ever died of hunger.** `is_starving()` needs 1,440
ticks without food, health loss 4,320 and death 10,080 — most of a lifetime.
Attributing every death in four worlds over thirty thousand ticks by what was
actually true of the agent the tick before it died:

| Cause | Deaths |
| --- | --- |
| Old age | 407 |
| Health gone, nothing else wrong | 374 |
| Thirst (over 4,320 ticks without water) | 235 |
| Cold (core under 33 °C) | 229 |
| Energy exhaustion | 1 |
| **Hunger (over 4,320 ticks without food)** | **0** |
| Heat | 0 |

In a simulation whose central drama is food, going without it has never killed
anybody. What kills them in a collapse is old age, cold, accumulated damage
that never heals off, and — once the near ground is bare and they range further
to forage — thirst.

A second thing this turned up, not yet chased down: mean health across a
settled population sits at 65-70 and never recovers. Measured over a thousand
ticks at a population of 25, agents lose about 430 health and heal back about
200. Neither exposure (22 of it) nor attacks (none) accounts for the
difference; the residue is many small repeated hits, most likely
`process_environmental_hazards` putting cold, heat and fall injuries on body
parts, infections on top of those, and `state.health` being clamped instantly
to `body.overall_health()` but clawed back at 0.02 a tick. A population
carrying a permanent thirty-point health deficit has no reserve for a bad
winter, which is what a mass die-off looks like when it comes.

### 4. Winter is not cold: the tile temperature is frozen at first touch

`ClimateManager::get_biome` builds a `Biome` for a position the first time
anybody asks about it, stamps the current season and hour into it, and caches
it for the rest of the run. `clear_biome_cache()` exists and is called only
from a test. So `get_temperature` — the temperature agents actually feel, via
exposure and body temperature — is that first-touch value plus whatever the
weather is doing now, and the season never reaches it again.

Measured over 15,600 ticks spanning every season, for a plains tile:

| Season | Mean | Lowest | Highest |
| --- | --- | --- | --- |
| Spring | 20.75 °C | 19.3 | 21.3 |
| Summer | 20.68 °C | 19.3 | 21.3 |
| Fall | 20.75 °C | 19.3 | 21.3 |
| Winter | 20.79 °C | 19.3 | 21.3 |

Winter is the warmest season by four hundredths of a degree. Mortality agrees:
deaths per ten thousand agent-ticks over six worlds to twenty-four thousand
come out at 1.62, 1.58, 1.47 and 1.71 for spring, summer, autumn and winter.

Two correct seasonal-temperature paths are computed and thrown away.
`ClimateManager::tick` sets `base_climate.temperature = base_temp * season_mod
* time_mod` every tick and nothing reads it. `SeasonalCalendar::apply_modifiers`
does the same job and has no caller outside its own test. The live path is the
frozen one.

The seasons do reach the world by two other routes, both working: the growth
modifier on regrowth, and the `WeatherGenerator`, which turns winter into snow,
sleet and blizzards. What does not reach it is the baseline swing — and with it
the reason a settlement would need to store food, put on a coat or get indoors
at one time of year rather than another.

Fixing it is not just a cache invalidation: making winter genuinely cold is a
real change to the balance and would need measuring before and after.

### 5. The ecology settles in most worlds, not all

Over forty worlds, predators are still alive at the end in thirty-six and
herds stay bounded in thirty-three. In the seven that run away the predators
died out first, and although animals do wander back in from off the map, the
trickle is slow enough — by design — that a world can spend thousands of ticks
with its herds climbing unopposed before a replacement pack arrives.

### 6. Clothing and hunting cost about what they return

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

### 7. Fear is a hunger signal, not a danger signal

`calculate_survival_drive_emotion` derives fear from unmet hunger, thirst and
rest. Since hunger saturates between meals, fear sits at around 0.8 much of
the time. `should_flee` triggers above 0.6, so agents read as fleeing in
ordinary circumstances rather than in response to a threat.

Survival actions now outrank fleeing, so this no longer strands agents, but
the emotional model is still reporting something misleading, and anything
built on `should_flee` inherits that.

### 8. Agents still cannot hear anything

Sight discovers terrain, resources and buildings, and agents now see one
another — `vision.visible_agents` is populated each tick, which is what
observational learning is gated on. Hearing is unfed entirely, so every
sound-derived percept is still a dead path. See SIMULATION_AUDIT.md.

### 9. Zoning and territory are never established

Building placement scoring reads zone and territory bonuses from
`World::zone_manager` and `World::territory_manager`, but nothing outside the
tests ever calls `add_zone` or `claim_territory`. Both managers are therefore
always empty in a live run and every bonus they contribute is zero, so
settlements have no planned structure and agents claim no ground.

### 10. Agents carry food they will never eat

Food that has turned is correctly refused, but stays in the inventory until
its freshness decays to zero and spoilage removes it — or until the agent takes
it onto a field and tips it out, which some of them work out for themselves. The same is true of food
an agent burns: a novice cook ruins about one batch in five, and the ruins ride
along in the pack. Both announce themselves as a decay scent to anyone nearby,
which is realistic and mildly useful, but nothing makes the carrier drop them:
carried weight still includes rot and cinders.

---

## Housekeeping

### 11. Committed backup file

`src/analytics/mod.rs.backup` is checked into the repository.

### 12. Build warnings

15 warnings on `cargo build`, all unused variables and imports. `cargo fix`
handles most.

### 13. Placeholder package metadata

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
- **Nothing grew out of the ground.** Growth was a number per species times the
  weather, with nothing taken out of the soil and nothing put back, so a patch
  picked bare regrew as fast on bare rock as in river silt. And the flora system
  — species, growth stages, regrowth timers, biome lists, a cultivation flag —
  had never held a single plant: its spawners had no callers outside the world's
  own pass-through wrappers.
- **Every plant in the world was in drought whenever it was not raining.**
  Growth took the hour's rainfall as its water term rather than what the ground
  holds, which cut it to a fifth on any clear day and made a marsh no wetter
  than a dune.
- **Nobody could see anybody.** Nothing populated `vision.visible_agents`, and
  observation is gated on it, so the whole observational learning system —
  broadcast, record, adopt, teach — ran every twenty ticks over an empty list.
  No agent had ever recorded seeing another do anything, in any run, ever.
- **Wild food could not feed a settlement.** It regrows about four times slower
  than a grown population eats it, and nothing else produced food at all.
  Agents break ground into fields now; crops on them grow eight times faster
  than the same thing wild.
- **Children froze to death.** A child has no clothing of its own — it cannot
  gather flax, has no skill to sew and nobody makes anything for it — so it ran
  two or three degrees colder than the adults beside it. One traced child had a
  perfect body, no injuries, full energy and a core temperature of 32.9. Nearly
  half of everyone ever born died before growing up, which no birth rate can
  carry. The young are now kept warm by whoever is looking after them.
- **Water was consumed and never came back.** It had no regeneration rate and
  did not count as renewable, so every drink took a unit out of the world for
  good and a lake drunk dry was deleted outright. A world lost more than half
  its water in fifteen thousand ticks. Together with the above, this is what
  emptied every settlement by thirty thousand ticks.
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
