# EBSS Simulation Feature Audit

**Last verified:** August 2026, against commit `0e3500d`
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

### The turning year
- A calendar a life fits inside. A tick is two hours, a day twelve ticks, a
  season twenty-four days and a year 1,152 ticks. A world opens in spring and
  an eight-thousand-tick run covers seven years and twenty-eight seasons; every
  run before this one ended on Year 0, Day 4, Winter, having never left the
  season it started in
- Two things the season now decides: the growth modifier on regrowth (spring
  ×1.5 through winter ×0.3), read every regeneration pass, and the length of
  the day, which plants feel directly - nine hours of winter sun against
  summer's fifteen. A winter is a winter for a plant whatever the weather is
  doing that hour
- The season also picks the weather. `WeatherGenerator` turns winter into snow,
  sleet and blizzards: measured over four years, winter is wintry a tenth of
  the time and no other season snows at all. That took a fix — weather
  durations were written in ticks back when a tick was thirty-six seconds, so
  500-2,000 of them meant five to twenty hours and now meant forty to a hundred
  and sixty days. A single blizzard outlasted the winter that started it and
  blew through the following summer, and snow fell in all four seasons in equal
  measure. Durations are given in hours now and converted through
  `TICKS_PER_DAY`
- What the season still does **not** reach is the temperature a tile reports —
  see **Built but not connected**

### Lifecycle
- Aging through infant, child, adolescent, adult and elderly stages, over eight
  or nine calendar years and thirty-odd seasons
- Mate selection, pregnancy with prenatal nutrition, birth, nursing, and
  developmental nutrition that modifies adult stats
- Inheritance of traits and behaviour trees from both parents

### Working the land
- Soil on every tile: a stock of nutrients, and two pools of dead matter — soft
  and woody — waiting to become more of it. What a tile starts with follows the
  country it is in, from marsh at 0.85 down to sand at 0.08
- Decay at a rate the ground decides. Humidity does most of it and density the
  rest: over two agent lifetimes a fallen tree in a swamp is more than half
  gone and the same tree in a desert has lost two parts in a thousand
- Around two hundred plants standing in a new world, each growing on whichever
  of water, light and nutrient it has least of. Foliage shades what is under it
  and sheds leaf fall onto the ground beneath, so a wood feeds itself
- Growth draws the ground down, so a settlement that farms hard works its soil
  from about 0.48 fertility towards 0.2 over twenty thousand ticks
- Agents break open grass into fields and sow them. A field gets at two and a
  half times as much of what the soil holds and carries a heavier crop; it does
  not grow anything faster than that plant's kind can grow
- Muck-spreading, which nobody is told about: an agent carrying food that has
  turned, standing on a field, tries tipping it out, sees whether the ground
  improves, keeps or drops the idea, and is watched doing it
- Water is fed by where it lies: a river carries it in from upstream, a spring
  in the hills gives whatever the weather does, a pool on open ground lives on
  the rain, and frozen ground gives up a quarter of what it otherwise would

### Family
- Parents keep their children close, going to one that has strayed and running
  to one that something is stalking - above their own coat and their own roof
- Children pick up skill experience every time they watch an adult work, and
  three times as much watching their own parents

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
- 15 drives with per-agent weights and thresholds. The six that look past this
  afternoon — a store of food, a field, tools, a building, comforts — run five
  times faster in an agent that is fed, watered, rested and warm, and a quarter
  as fast in one that is not
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
- Recolonisation: a species wiped out of a world, or hunted down to a quarter
  of the most that world ever held of it, is slowly replaced by animals
  wandering in from off the map - one small group every eight thousand ticks
  or so, and only species that have lived there
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
| Hearing (`senses::Hearing`) | Nothing feeds sounds from the world |
| Seasonal temperature | Computed three ways; the one agents read is frozen. `ClimateManager::get_biome` builds a `Biome` per tile on first touch, stamps the season and hour into it, and caches it for ever - `clear_biome_cache()` is called only from a test - so `get_temperature` returns that first-touch value plus the current weather modifier. Meanwhile `ClimateManager::tick` recomputes `base_climate.temperature` from the season and hour every tick and nothing reads it, and `SeasonalCalendar::apply_modifiers` has no caller outside its own test. Measured: winter and summer report the same temperature to a tenth of a degree |
| `world::zoning`, `world::territory` | Read by building placement scoring (`spatial_planning.rs`), but nothing outside tests ever calls `add_zone` or `claim_territory`, so both managers are always empty and every bonus they contribute is zero |

**Consequence for perception:** agents find the world by sight and smell.
`Percept::ResourceDetected` comes from scents; `visible_agents` is populated
each tick now, which is what observational learning is gated on — until it was,
the whole learning system ran over an empty list and no agent had ever recorded
seeing another do anything. Anything depending on sound is still a dead path.

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
  cannot be reproduced and five tests are intermittently flaky.
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
| Worlds with predators still alive at the end | 7 of 40 | 36 of 40 |
| Worlds with herds bounded (under 150) | — | 33 of 40 |
| Herbivores, as a multiple of the founding stock | 11x | 4.1x |
| Agents mauled by animals | 0 | 31 |
| Agent population | 903 | 938 |

Before this, predation sat behind one roll for the whole world per tick, a
predator only hunted when half starved, predators were stocked without regard
to what they could eat, and nothing but the hard population cap of a thousand
limited a herd. Nothing in the model let an animal touch an agent at all.

It is not stable everywhere. In the seven worlds where herds are not bounded,
the predators died out first. A world holding both at once is the common case
now rather than the rare one, which it was not before. Species that do go now
come back: 208 of the 301 species a world starts with are still there after
eight thousand ticks, and the rest are candidates for the slow trickle of
arrivals from off the map.

**Over a long run.** Every settlement tested used to be empty by thirty
thousand ticks. Two things were killing them, neither visible in the eight
thousand ticks everything above is measured over: children froze — they have
no clothing of their own and nobody makes any for them, so they ran two or
three degrees colder than the adults beside them, and nearly half of everyone
born died before growing up — and water was consumed without ever coming back,
so a world lost more than half of it in fifteen thousand ticks. With the young
kept warm by their carers and the rivers refilling, nine settlements of twelve
were still inhabited at thirty thousand ticks and thirteen of sixteen at
twenty thousand. The rest starve out: food regenerates about four times slower
than a grown population eats it, so a settlement that overshoots the land does
not settle back.

Farming, measured over twenty thousand ticks in five worlds:

| Measure | Result |
| --- | --- |
| Settlements still there at 20,000 ticks | 4 of 5 |
| Population at the end | 20, 36, 131, 147 (one world empty) |
| Fields broken | 23 to 101 per world |
| Edible resource in the world | 3,700 to 8,800 units |

Before fields, a settlement's food came only from what grew wild, which regrows
about four times slower than a grown population eats it: settlements capped
around a dozen people and a quarter starved out. The largest now overshoot and
correct rather than dying.

The land, measured over twenty thousand ticks in five worlds:

| Measure | Result |
| --- | --- |
| Settlements still there at 20,000 ticks | 5 of 5 |
| Population at the end | 91 to 189 |
| Fields | 75 to 101 per world |
| Field fertility, start to end | 0.48 → between 0.19 and 0.36 |
| Agents who tried muck-spreading | 126 |
| Agents who came to believe in it | 373 |

Three times as many settled on the practice as ever tried it themselves, which
is the shape of something spreading by being watched rather than being coded.
One world shows the whole arc: a boom to 165 people, soil worked from 0.48 down
to 0.19, the standing crop collapsing from 4,176 units to 80, and the
population falling back to 91.

**The turning year.** Twenty worlds of twelve, eight thousand ticks, measured
on the commit before this one and on this one:

| Measure | Winter-locked | Seasons turning |
| --- | --- | --- |
| Where a run ends | Year 0, Day 4, Winter | Year 6, Day 91, Winter |
| Seasons a run sees | 1 of 4 | 4 of 4 |
| A life, in calendar years | 0.011 | 7.8-9.5 |
| Worlds still inhabited | 19 of 20 | 20 of 20 |
| People at the end | 36.8 | 38.0 |
| Standing crop at the end | 4,925 | 5,220 |
| Mean temperature over the run | 14.1 °C | 17.3 °C |

Nothing had to be rebalanced for it, which was not obvious beforehand: the
season modifier on regrowth had been pinned at winter's ×0.3 and now averages
×0.95 over a year, so wild food comes back around three times as fast on
average. It does not run away, because a patch is capped by what the ground
under it will carry rather than by how fast it grows back, and the cap did not
move. What changed is the shape of the year rather than the total: a summer
hedgerow now outgrows a winter one, and a settlement has to get from one to
the next.

Over a long run, twelve worlds of twelve taken to thirty thousand ticks -
twenty-six years, a hundred and four seasons, three generations:

| Measure | Result |
| --- | --- |
| Settlements still inhabited | 11 of 12 (was 9 of 12) |
| People at the end | 77.7 on average, 0 to 120 |
| Highest the population got | 141.3 on average, 33 to 225 |
| Oldest person alive at the end | 8,049 to 10,827 ticks (7.0 to 9.4 years) |

The shape is the same one farming produced: a settlement that booms past a
hundred and fifty works the standing crop down to almost nothing - four
worlds ended under 130 units of it - and falls back. The ones that never
overshoot sit small and comfortable, 15 or 50 people against 5,000 units of
crop still standing. Nobody starves in a straight line; they starve after
having been too many.

**The season never reaches the temperature a tile reports.** Measured over
160,000 ticks, holding the weather constant so that only the season varies —
what `ClimateManager::get_temperature` returns for a plains tile under a clear
sky:

| Season | Clear-sky temperature |
| --- | --- |
| Spring | 18.667 °C |
| Summer | 18.667 °C |
| Fall | 18.667 °C |
| Winter | 18.667 °C |

Identical to three decimal places on about fourteen thousand samples each. All
the cold there is comes through the weather instead: winter snows nearly a
tenth of the time and no other season snows at all, which pulls a winter about
half a degree below the rest. See **Built but not connected** for why.

Mortality agrees. Deaths per ten thousand agent-ticks over six worlds run to
twenty-four thousand: spring 1.57, summer 1.42, autumn 1.42, winter 1.57.
Winter kills a tenth more than a summer, and so does spring to the second
decimal — inside the noise on about 290 deaths a season. Nothing in these
worlds has to survive a winter.

**Why a settlement that overshoots does not settle back.** Six worlds traced to
thirty thousand ticks, sampling the ground the settlement actually farms rather
than the map as a whole.

A settlement's food is not a stock it can over-draw and then let recover. It is
a flow that its own harvesting permanently reduces: `regenerate_in_ground`
takes 0.0015 nutrients out of a tile per unit grown and scales the rate by the
fertility that remains, so every unit eaten makes the next one slower to
arrive. Production decays with cumulative harvest towards zero, and the only
equilibrium is the one where hardly anybody lives there.

| tick | people | standing crop | fertility of the farmed ground |
| --- | --- | --- | --- |
| 0 | 12 | 1,414 | 0.529 |
| 10,000 | 50 | 6,138 | 0.509 |
| 20,000 | 111 | 3,875 | 0.304 |
| 24,500 | 219 | 1,367 | 0.106 |
| 30,000 | 81 | 24 | 0.025 |

The ground does not come back. Taking every agent out of a world that had
farmed for twenty-two thousand ticks and leaving it fallow for thirty thousand
more — twenty-six calendar years with nobody in it — returned 0.017 fertility
of the 0.166 that had been taken, a tenth, and slowing as it went. Depletion
under a hundred people runs about sixty-five times faster than recovery under
nobody.

Nothing brakes the population until the standing crop is gone.
`should_attempt_reproduction` looks only at whether the Hunger and Thirst
drives are above threshold right now; Hunger's is 0.7 and the measured value
sat at 0.5-0.6 throughout. The population doubled from 111 to 219 while the
crop fell from 3,875 to 1,367, and peaked about nine calendar years after the
ground had lost eighty per cent of its fertility.

Two things sharpen it. A spent field still counts as a field — `fields_within`
counts cultivated tiles rather than producing ones, and a tile carrying a
resource node cannot be tilled again — so six exhausted fields stop a
settlement breaking new ground for good, while the map around it averages 0.358
fertility against the 0.025 it has left itself. And nutrient only ever leaves:
food eaten is gone from the world, food that spoils in a pack is deleted rather
than dropped, and the one return path is muck-spreading.

Nobody has ever died of hunger. `is_starving()` needs 1,440 ticks without food,
health loss 4,320 and death 10,080, which is most of a life. Attributing every
death in four worlds over thirty thousand ticks by what was true of the agent
the tick before it died: old age 407, health gone with nothing else wrong 374,
thirst 235, cold 229, energy exhaustion 1, heat 0, **hunger 0**. In a
simulation whose central drama is food, going without it has never killed
anybody.

## Test coverage

1,139 library tests, 15 integration tests, 21 plugin tests, 1 doc test, plus
one ignored long-run test (`a_settlement_lasts_thirty_thousand_ticks`). All
pass, except the known flaky ones (`test_resource_clustering`,
`test_minimize_travel_time_from_agent_position`,
`test_production_building_placed_near_resources`, and now
`water_is_not_used_up`) that assert on properties a randomly generated world
does not always have. The third was measured at 4 failures in 120 runs on an
earlier commit, so it is not new; the fourth has a wider margin after the
calendar change than before it (98.4% of a world's water still there at six
thousand ticks, against 95.6% before, on a floor of 95%).

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
