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

### Drives
- Fifteen drives with per-agent weights and thresholds. Nine of them read the
  conditions the design document specifies rather than a clock: Shelter on
  exposure, cold, nightfall, weather and predators; Safety on threat, injury and
  darkness, answered by cover and a weapon; Preparedness, Industry, Sustenance
  and Utility on what the agent has put by and what the ground round about is
  bearing; Construction on room to build, neighbours building, and the shelter
  drive itself, which is the document's "drive synergy"; Luxury on idleness and
  the lack of anything fine; Protection on having children and on one of them
  having strayed. Each moves towards what its situation calls for and falls away
  when nothing does
- Hunger, Thirst, Rest, Curiosity, Social and Reproduction still build with
  time, which is what the document says of them
- A need presses harder the longer it is denied. Every drive counts how long it
  has been asking without being answered, and that count multiplies both how
  fast it builds and how loudly it argues in action selection, up to fourfold.
  A person who missed a meal is a little distracted; one who has not eaten in
  three days is not thinking about anything else. Being fed halves the count
  rather than clearing it
- Every drive also counts how long it has *not* had to ask, which is the only
  forward-looking thing an agent has and is what breeding waits on
- Breeding waits on a surplus rather than on a full stomach: immediate needs
  met, no recent stretch of going short, and either food in hand for two or a
  long stretch in which feeding itself was simply not a problem
- Children have less to live on than adults. Every starvation threshold is
  measured against what the body has stored - a quarter of an adult's for an
  infant, under half for a child, three fifths for the elderly - so a hungry
  year takes a generation rather than an even slice
- A settlement can leave. Ten days of hunger going unanswered and an agent
  stops working the fields it has and walks, to the furthest food it remembers
  or failing that on a bearing of its own. Nobody decides it on the
  settlement's behalf; it falls out of the drive
- Agents learn what works. Every action's outcome is recorded against the kind
  of undertaking it was and shifts a running belief about whether that kind of
  thing pays for this agent. Failures count for more than successes, nothing is
  written off before five attempts, and something that has gone badly nearly
  every time is dropped - a hunter who never catches anything stops hunting.
  The same outcomes drive the behaviour-tree weights, which were built to be
  this record and had never had a caller

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
- A fishery, which is the only food in the model the land does not pay for. Fish
  are not grown in the tile they are caught from and do not regrow out of what
  is left of them: they come up the river on the season, heaviest in spring and
  autumn, thinnest in a frozen winter, so a reach fished down to nothing fills
  again from the catchment inside a year. What is left of one, put on a field,
  is worth forty times a unit of crop, because the crop is giving back what the
  ground already paid out and the fish is bringing in what the sea grew it with
- Growth draws the ground down, and four things put matter back: what a body
  passes after a meal, what spoils in somebody's pack, what a body is when it
  stops, and — much the largest — the roots, stalk and leaf a plant leaves in
  the tile it grew in, since only the part somebody carries away leaves the
  field. Rot keeps three fifths of what it works on and loses the rest, so the
  loop turns and loses on every turn
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
- Drives are ranked primary, secondary and tertiary, and within the primary
  band the one that would kill this agent soonest wins, computed live from how
  much it has left rather than from a fixed table. Each drive is gated behind
  the one it follows in the specification's chains — hunger before
  preparedness before luxury, safety before shelter before protection, all
  four primaries before reproduction — so a drive cannot build while the one
  it depends on is unanswered
- Action selection: a child in trouble, then freezing with a roof in reach,
  then the highest-ranked drive this agent has an answer for. Perception,
  plan, goal and the old fixed ordering follow, for the drives that have no
  answer to offer
- Fear and anger are appraisals, not timers. What is in front of the agent is
  weighed against what the agent can do about it, and what past fights taught
  it, so the same wolf angers one person and frightens another
- And they reach the agent's hands. A frightened agent puts ground between
  itself and the thing; an angry one strikes at what is within arm's reach and
  closes the last pace or two, but does not cross the map looking for a fight.
  Grudges against people work the same way: whether an agent squares up to
  somebody it cannot stand or keeps clear of them is the same appraisal, and
  nobody raises a hand to a child, to their own parent, or to their own
  children
- Obstacle-aware movement (greedy step, then a bounded breadth-first route
  search), committed search legs when looking for something out of range

### Social
- Relationships and bonds. Being about the same place as somebody takes a
  season to make them a familiar face and stops there; getting on with them
  takes a season to make them a friend and stops there. Anything past that is
  earned by what the two of them have done
- What one agent holds against another reaches what it thinks of them: a
  grudge weighs on the bond at eight times what keeping company is worth, a
  blow costs a quarter of the whole scale at once, and the relationship is
  renamed to match — a settlement now contains rivals and enemies, which no
  settlement in this project's history had ever contained
- Social interactions, gossip and information spread. Whose word an agent
  takes depends on what the two of them are to each other, whether that one
  has been right before, and what sort of people the two of them are — and an
  agent that would rather lie can name a place that is not there. A lie is
  found out by walking to it, and what it cost the man who was lied to depends
  on what it was about
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
| `core::drive_progression` (`DriveProgression`) | Basic → Intermediate → Advanced → Luxury tiers for every drive, with tests, and no caller outside its own module |
| `agents::drive_satisfaction` (`SatisfactionTracker`) | Fed only from tests, so the grief-on-death code in `Population` that asks which agent was a drive's satisfaction source always gets nothing |
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
  Nothing weighs the two against each other, and the material an agent will
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

**What the pressure changed.** Six worlds of twelve, thirty thousand ticks,
against the same measurement before the drives were given any of it:

| Measure | Before | After |
| --- | --- | --- |
| Worlds still inhabited | 11 of 12 | 6 of 6 |
| People at the end | 77.7 | 76.0 |
| Highest the population reached | 141.3 | 93.2 |
| Still at or near that peak at the end | — | 5 of 6 |
| Fertility of the farmed ground at the end | 0.025 (traced world) | 0.179 |

The population figure barely moves; the shape behind it is different. Before,
a settlement ran up past two hundred and was down to a third of that and still
falling — end over peak, 0.55. Now the peak is a third lower and five of six
worlds are within a tenth of it when the run ends: 0.82. They are holding
rather than having crashed from a high.

Four of the six settle outright — populations of 57 to 102 on ground still at
0.21 to 0.26 fertility, carrying two to two and a half thousand units of crop,
births and deaths level. The other two work their ground out anyway (0.081 and
0.028) and go into famine, and in both the pressure shows: 27 of 91 people in
one and 23 of 42 in the other have had hunger denied past the point where they
stop working the fields and walk.

Worth being plain about what has not been fixed. The ground is still mined,
just more slowly, because there are fewer people on it: nothing yet puts
nutrient back at the rate it is taken. And leaving is a real option only in
proportion to how much unfarmed country is left — on a fifty-by-fifty map with
ninety-odd fields already broken, an agent that walks out does not have far to
walk.

**And it did not hold.** The measurement above was taken while a settlement's
whole second generation was quietly dying of the newborn dehydration bug (see
below). Fixing that roughly doubled how many people a world can grow, and at the
larger scale the brakes are not enough. The same six worlds, thirty thousand
ticks, across all three states of the code:

| Measure | Before any of it | With the survival pressure | With healthy newborns |
| --- | --- | --- | --- |
| Worlds still inhabited | 11 of 12 | 6 of 6 | 6 of 6 |
| People at the end | 77.7 | 76.0 | 78.2 |
| Highest the population reached | 141.3 | 93.2 | **211.5** |
| End over peak | 0.55 | 0.82 | **0.37** |
| Fertility of the farmed ground | 0.025 | 0.179 | **0.055** |

Every world is still inhabited at thirty thousand ticks, which is the thing that
matters most and which no version of this before held. But the shape is the
overshoot-and-slide again, and worse than the shape it started as: a settlement
now runs to two hundred and eleven and is down to a third of that by the end,
having mined its ground to 0.055.

The reading is that breeding-on-a-surplus, migration and the rest bought a
settlement of ninety a soft landing, and buy a settlement of two hundred
nothing. The soil economics are unchanged and were always the binding
constraint: production decays with cumulative harvest, recovery runs about
sixty-five times slower than depletion, and nothing puts nutrient back. Curing
the infant mortality removed a brake nobody had intended, and what it revealed
was that the intended brakes are calibrated for a population half the size.

**Then the ground got a way back, and the shape changed.** Four return paths
went in: what a body passes after a meal, what spoils in a pack falling to the
ground instead of being deleted, what a body is when it stops, and the roots,
stalk and leaf a plant leaves in the tile it grew in.

The three agent-side paths made no measurable difference — three worlds to
thirty thousand ticks came out at mean farmed fertility 0.058 against the 0.055
of no return path at all. What goes through a person comes out where the person
is standing, and agents range over the whole map, so the matter lands
everywhere except the fields. That is why muck-spreading has to be a practice
somebody learns rather than something that happens by itself.

The fourth was the one that mattered, and was missing longest: the model had
been treating every plant as though all of it were carried off, when in fact
only the grain leaves the field. Four worlds, thirty thousand ticks:

| Measure | Before any of it | Survival pressure | Healthy newborns | Agent-side returns | Crop residue too |
| --- | --- | --- | --- | --- | --- |
| Worlds run | 12 | 6 | 6 | 3 | 4 |
| Still inhabited | 11 | 6 | 6 | 3 | 4 |
| People at the end | 77.7 | 76.0 | 78.2 | 53.0 | **154.0** |
| Highest the population reached | 141.3 | 93.2 | 211.5 | 212.3 | 226.2 |
| End over peak | 0.55 | 0.82 | 0.37 | 0.25 | **0.69** |
| Fertility of the farmed ground | 0.025 | 0.179 | 0.055 | 0.058 | **0.268** |

The last two columns do not overlap on either measure: every world with residue
ended between 140 and 176 people on ground between 0.175 and 0.457, every world
without between 25 and 107 on ground between 0.031 and 0.103.

The peak is the part worth reading. Every earlier measure that improved
end-over-peak did it by holding the population down — the survival pressure
bought its 0.82 by taking the peak from 141 to 93. This one leaves the peak
where it was and changes what happens after it: the settlement keeps two thirds
of its highest number instead of a third. The best of the four worlds, sampled
the way the decline table further down is sampled — set the two side by side and
the difference is the whole of this section:

| tick | people | standing crop | fertility of the farmed ground |
| --- | --- | --- | --- |
| 0 | 12 | 1,560 | 0.541 |
| 10,000 | 32 | 4,601 | 0.559 |
| 20,000 | 134 | 4,502 | 0.491 |
| 25,000 | 170 | 4,264 | 0.455 |
| 30,000 | 140 | 4,306 | 0.457 |

That one barely dips. The worst of the four still ended at 0.175 with 148 people
on 1,134 units of standing crop, which is a working settlement rather than the
twenty-four units and eighty-one people the decline table ends on.

It is not a closed loop and is not meant to be. Rot keeps three fifths of what
it works on, so every turn is smaller than the last, and farmed fertility is
still falling at thirty thousand ticks in three worlds of the four. What
changed is the slope, and with it the shape: a settlement that overshoots now
comes back onto ground that can still carry it.

**A fishery turns the slope the other way.** Everything above is a return - the
ground getting back part of what it already paid out, less what rot takes. A
fish is not grown on the land at all: it is grown at sea, fed on a whole
catchment, and it comes up the river under its own power whatever was taken out
of that reach last year. What is left of one, put on a field, is the only thing
in the model that makes a country richer than it was. Four worlds, thirty
thousand ticks:

| Measure | No return path | Crop residue | Residue and a fishery |
| --- | --- | --- | --- |
| Worlds run | 6 | 4 | 4 |
| Still inhabited | 6 | 4 | 4 |
| People at the end | 78.2 | 154.0 | 150.5 |
| Highest the population reached | 211.5 | 226.2 | 220.8 |
| End over peak | 0.37 | 0.69 | 0.69 |
| Fertility of the farmed ground | 0.055 | 0.268 | **0.607** |

Every one of the four ended on better ground than it started on: 0.545 at tick
zero against 0.607 at the end, and no world went down. Map nutrients climbed
from about 800 to between 1,049 and 1,103. The best of them:

| tick | people | standing crop | fertility of the farmed ground |
| --- | --- | --- | --- |
| 0 | 12 | 1,402 | 0.544 |
| 10,000 | 46 | 4,459 | 0.594 |
| 20,000 | 184 | 5,372 | 0.577 |
| 30,000 | 136 | 6,030 | 0.641 |

The peak did not move, and neither did end-over-peak: 226 and 0.69 without a
fishery, 221 and 0.69 with one. The settlement still overshoots and still
settles back onto what the ground will carry. What changed is that the ground
is no longer poorer every time it does. Nothing was made easier for the people;
something was added to the country - and somebody has to stand in a river to
fetch it, which twelve to thirty-four of every hundred and fifty had settled
into doing by the end, each having worked out for themselves that it paid.

**Why a settlement that overshoots did not settle back.** Six worlds traced to
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
fertility against the 0.025 it has left itself. And nutrient only ever left:
food eaten was gone from the world, food that spoiled in a pack was deleted
rather than dropped, and the one return path was muck-spreading. That last is
the one since addressed, in the four return paths described above.

Nobody has ever died of hunger. `is_starving()` needs 1,440 ticks without food,
health loss 4,320 and death 10,080, which is most of a life. Attributing every
death in four worlds over thirty thousand ticks by what was true of the agent
the tick before it died: old age 407, health gone with nothing else wrong 374,
thirst 235, cold 229, energy exhaustion 1, heat 0, **hunger 0**. In a
simulation whose central drama is food, going without it has never killed
anybody.

## The drive system against its specification

The design document (Appendix A) specifies thirteen drives, each with a list of
**increase conditions** and **decrease conditions**. The increase conditions are
what make a drive a motivation rather than a timer: Safety is meant to rise on
"hostile entity proximity, recent injury, darkness", Construction on "buildable
templates seen, others building, drive synergy", Sustenance on "low food
stockpile, crop depletion".

None of them existed. `DriveType::base_accumulation_rate` returned one flat
number per drive per tick and that was the whole of it — including for the line
`DriveType::Safety => 0.02, // Spikes with threats`, whose comment described the
specification, whose code was a constant, and whose 0.02 was the highest flat
rate any drive had. Because the satisfying actions for those drives are chosen
rarely, nine of the fifteen sat pinned at 1.00 and active every tick from early
in a run, which left the per-agent weight as the only thing telling them apart.

**They read their conditions now.** A drive that reads the world moves towards
what the situation calls for instead of climbing: it settles where the
conditions put it and falls away when they stop. The gap closes by a share of
itself each tick, so a drive with a high base rate answers a change quickly —
Safety is most of the way to a new level within a day of a predator appearing —
and one with a low rate takes seasons. Hunger, Thirst, Rest, Curiosity, Social
and Reproduction still build with time, which is what the document says of them
and what the rest of the survival loop is built on.

Half the conditions are things an agent knows about itself — what it is
carrying, whether it is armed, whether it is cold — and half are things only the
world knows. `Simulation::read_the_situation` gathers the second kind once per
agent per tick into `Surroundings`; the agent folds in the first kind when its
drives are ticked. "Drive synergy", named in the document as an increase
condition for Construction and never implemented in any form, is the shelter
drive being passed in: wanting to be out of the weather is a reason to build
something.

Measured at eight thousand ticks, before and after:

| Drive | Before | After |
| --- | --- | --- |
| Shelter | 1.00, active 100% | 0.25, active 0% |
| Safety | 0.99, active 100% | 0.26, active 13% |
| Construction | 1.00, active 100% | 0.15, active 9% |
| Protection | 0.93, active 100% | 0.09, active 9% |
| Industry | 0.96, active 96% | 0.29, active 65% |
| Sustenance | 1.00, active 100% | 0.52, active 87% |
| Preparedness | 1.00, active 100% | 0.88, active 100% |
| Utility | 1.00, active 100% | 0.60, active 100% |
| Luxury | 1.00, active 100% | 0.98, active 100% |

Six of the nine came unpinned. **The three that did not are the finding.**
Preparedness asks for stockpiled food, materials and tools; Utility for tools in
working order; Luxury for something fine. Counting what thirty agents were
carrying at eight thousand ticks: 102 wood, 21 food, 17 leather, 14 horn, 12
flax, 11 cotton, 8 wool — and no tools and nothing decorative at all, with zero
equipped items across the whole settlement. Those three are reading the world
correctly and the world has no way to answer them. That is a gap in crafting and
tool-making rather than in the drives, and it was invisible while every drive
sat at its ceiling for reasons of its own.

Two smaller things fall out of it. Luxury is specified to rise on "idle time"
as well as on lack, and reading the lack alone put it at the top of the fallback
for half the population; folding in the idleness is what the document actually
says. And Protection, which is not in the document, is answered by being where
the children are, so it now asks for nothing when there are none, where before
it climbed for ever and mapped to `Action::Wait` whenever it won.

The denial pressure works on the drives that are answered often enough for its
counter to move — Hunger, Thirst, Rest — and is still saturated on the three
that cannot be answered at all.

**What it cost to find out.** Unpinning the drives changed which of them wins
the fallback, agents stopped clustering, and settlements collapsed: three of six
worlds empty at thirty thousand ticks against six of six before, with peaks of
30 against 93. Chasing that turned up something older and worse than the drives.
Both survival clocks are kept as a tick the agent last ate or drank on, and both
start at zero — right for the twelve people a world begins with, wrong for
everybody born afterwards, who arrived having last drunk at the beginning of the
world. An infant born after about four thousand ticks was two days past the
point where dehydration takes health, lost 1.65 a tick from its first breath,
and died at sixty-one: at full energy, unhurt, beside its mother, being nursed.
Newborns now start both clocks at their birth tick.

That also closes the open question in the previous version of this document.
Mean health across a settled population sat at 65-70 and never recovered, and
neither exposure nor attacks accounted for it. It was this: every
second-generation agent that survived at all did so carrying the damage. A
settlement now runs at 90-96.

## The drive hierarchy

The specification says what the document's flat list did not: that drives are
ranked, that the one which will kill fastest has the highest priority, and that
they lead from one to the next. *An agent will not continue hunting if it will
die from dehydration, even if it resolves its hunger drive.*

Three things were needed and none of them existed.

**A rank.** `DriveType::rank` puts the five that bear on immediate survival
(Hunger, Sustenance, Thirst, Rest, Safety) in a primary band, the five that
bear on longer-term survival and wellbeing (Curiosity, Social, Reproduction,
Shelter, Preparedness) in a secondary band, and the five that bear on comfort
and standing (Luxury, Utility, Construction, Industry, Protection) in a
tertiary one. A drive that is asking outranks every drive in a lower band; a
drive that is quiet does not, or a primary at 0.05 would outrank a secondary
at 0.95 and nothing but foraging would ever happen — which is exactly what the
first attempt did.

**A clock.** Within the primary band, precedence is nearness of death and
nothing else. `AgentState::ticks_before_this_kills_me` works it out from what
the agent actually has: ticks without water against the dehydration threshold
and the rate health falls afterwards, ticks without food against the same for
starvation, energy against the rate it drains. It returns `None` where death
is not in prospect — an agent above 25 energy is not dying of tiredness, and
computing Rest's clock from a full tank put every agent in the settlement
about 2,800 ticks from death and gave Rest 79.9% of every turn.

**A chain.** `DriveType::unlocked_by` encodes the specification's chains, and
`DriveState::is_unlocked` walks them. A drive cannot build until the drive it
follows has been *reliably* answered, which is `RELIABLY = 24` ticks — two
days — of the earlier drive sitting quiet. The recursion has to test the lock
before the answer or a chain unlocks itself from the far end: a drive that is
locked is also quiet, and a quiet drive read as answered would unlock the one
after it.

Then `generate_non_emotional_action` was turned inside out. It used to be
thirteen fixed priorities with drives consulted last; it now asks the ranked
drives first and takes the first one this agent has an answer for. The old
ladder survives as the fallback for drives with nothing to offer, and two
things still come before everything: a child in trouble, and freezing with a
roof within reach.

Measured over agent-samples from three worlds a side, the same runs that
gave the drive-pegging figures above:

| Measure | Before | After |
| --- | --- | --- |
| Foraging as a share of all actions | 79% | 25% |
| Luxury above its threshold | 98.9% | 0.5% |
| Preparedness above its threshold | 98.2% | 0% |
| Utility above its threshold | 84.6% | 0.6% |
| `Action::Build` chosen | 0 in 777 lives | non-zero |
| `Action::Socialize` chosen | 0 in 777 lives | non-zero |

**And what it cost to find out.** Gating a drive means it has to be able to
fall quiet, and `fall_quiet` drained every drive at one flat 0.004 a tick.
Reproduction accumulates at 0.001. An agent spends about 9.9% of its ticks
with a primary unanswered, and at four times the fill rate that 9.9% cost it
half of everything it had accumulated — so the birth rate halved and with it
the settlement. Two four-world samples of *identical* code differed by more
than the effect being chased (end populations 45.2 and 30.2), which is why
this was found at eight worlds a side and not at four. A drive now fades at
the rate it would have grown.

## Emotion as appraisal

The specification asks two questions and they are the same question twice:

> Does a thing threaten my ability to satisfy my drives? Can I combat it? If
> not, increase fear. If so, increase anger. Does a thing prevent my ability
> to satisfy my drives? Can I combat it? If not, fear. If so, anger.

`ThreatAssessment` has been able to answer that since it was written. Nothing
ever asked it about anything except the resolution of a blow that had already
landed, so a wolf ten paces off and closing produced no feeling at all until
it bit somebody. Fear, meanwhile, was `calculate_survival_drive_emotion`
reading how high a survival drive's value stood — which meant a well-fed agent
with a full larder and a rising appetite was as frightened as a starving one.
When that was written the survival drives saturated between meals and fear sat
near 0.8; once they were being answered it inverted to nearly nothing. Over
three worlds: mean fear 0.01 to 0.06, mean anger exactly 0.00, and not one
agent in 170 ever above the 0.6 that `should_flee` wants. The branch of
`generate_action` that lets an agent run or fight was unreachable code.

**The threat question.** `feel_about_what_stands_in_the_way` runs each tick
over the creatures in sight, scales each one's strength by how near it is —
nothing beyond ten tiles registers, and a wolf at ten tiles is worth a tenth
of a wolf at your elbow — and appraises it against `own_strength`: health,
build, armour, weapon, combat skill and nerve. Because a wolf that stands
still is one wolf however long it stands, the appraisal *sets* the feeling
rather than adding to it, and `nothing_is_stalking_me` clears it when the wolf
is gone. Without the set, anger accumulated a tick at a time to a mean of
0.644 and 61% of the settlement wanted to fight something; without the
distance falloff, 24% of it wanted to fight a wolf ten tiles away.

**The prevention question** goes to the drives.
`calculate_survival_drive_emotion` reads `denied_ticks` — how long the need has
actually gone unanswered — against how soon it would kill, so a need that keeps
being met frightens nobody however loudly it asks, and one that has gone
unanswered frightens in proportion to how close the end is. Eleven days from
starving is worrying; one day from it is not.

**History decides the marginal cases.** `what_fighting_has_taught_me` reads the
`Fighting` record and multiplies `own_strength` by it: down to 0.6 for an agent
beaten every time, up to 1.5 for one that has never lost, 1.0 for one that has
never fought. Two agents of identical build appraise the same wolf differently,
and the one that has been beaten runs where the one that has won stands. A
fight only counts as won if the agent came out of it having lost less than a
quarter of its health — on the obvious test, *survived at all*, every survivor
was a winner and the record taught nothing.

Measured over three worlds of eight thousand ticks, 41,556 agent-samples:

| Measure | Before | After |
| --- | --- | --- |
| Mean fear | 0.01–0.06 | 0.044 |
| Mean anger | 0.00 | 0.298 |
| Samples that would flee | 0% | 1.94% |
| Samples that would fight | 0% | 22.80% |
| Fights fought | — | 51 |

The lost-a-fight branch is rare in the wild, because most agents that come off
badly against a predator do not survive to draw the lesson: of 229 survivors,
11 reckoned themselves better in a fight for what had happened and none worse.
Both directions are therefore demonstrated deterministically in
`src/agents/tests/appraisal_tests.rs` rather than left to the sample.

**It does not yet change what a settlement is worth.** Eight worlds a side at
fifteen thousand ticks, against the commit before the appraisal:

| Measure | Before | After | Shift |
| --- | --- | --- | --- |
| End population | 86.8 ± 9.5 | 94.1 ± 6.3 | 0.65 se |
| Peak population | 100.9 ± 6.6 | 108.5 ± 6.3 | 0.83 se |
| Births | 149.6 ± 14.9 | 162.1 ± 12.4 | 0.64 se |
| Deaths | 87.9 ± 6.5 | 93.0 ± 8.2 | 0.49 se |
| Soil fertility | 0.39 | 0.39 | −0.07 se |
| Settlements still inhabited | 8 of 8 | 8 of 8 | — |

Every measure drifts upward and not one of them is above a single standard
error, which at eight worlds a side means nothing has been shown. That was the
expected result rather than a disappointment: at that point an angry agent was
one that *would* attack, and nothing read the feeling. Building what reads it
is the next section.

## Fight or flight

Both branches of action selection that read fear and anger were keyed on
`recent_attacker` — another agent who has just landed a blow. An agent
terrified of a wolf ten paces off fell straight through the flight branch and
went on foraging; an agent furious at a neighbour fell through the attack
branch and did the same. The appraisal decided what an agent felt and stopped.

**Building the creature half first turned up the more interesting half.** Of
22,802 samples that read as ready to fight, anger at creatures came to 0.025
and anger at people to **0.806**. Nearly all the anger in this model is a
grudge — somebody lied to you, somebody betrayed you, somebody was the cause
of a death — held against that person for life, decaying at one per cent a
tick, with nothing whatever downstream of it. Half the time the person
resented was within ten tiles, and 6.9% of the time within arm's reach.

**Creatures.** `run_from_what_frightens_me` reads the strongest `Creature`
fear source, finds the nearest of that kind in sight, and heads the other way
far enough not to arrive back inside the range it started worrying at.
`round_on_what_angers_me` strikes at one within arm's reach and walks at one
within five; anything further off is left alone, because the appraisal already
scales a creature by how near it is, so a thing that angers an agent past the
threshold is close by anyway.

**People.** `square_up_to_the_people_i_resent` asks the specification's
question about a person rather than a wolf: this is somebody you cannot stand
and they are in front of you — can you take them? If you can, the grudge stays
anger and it may come to blows. If you cannot, the same grudge comes out as
fear and the agent keeps clear. The grudge itself is never touched, only which
feeling it turns into, and it is read per person rather than off the total:
`should_attack` sums every source, so three mild grudges of 0.2 read as a man
ready to fight nobody in particular. Nobody raises a hand to a child, to their
own parent, or to their own children.

`Action::Fight` is new and deliberately not `Hunt`. Hunting is how an agent
goes after food and skins and reads the Hunting skill; standing your ground
reads MeleeCombat, teaches `Undertaking::Fighting` rather than Hunting, and is
worth doing on a full stomach. Whether the blow lands is `own_strength`
against the creature's, on the same scale the appraisal used to decide to be
there at all — so the record of past fights, which scales `own_strength`,
decides both whether an agent stands and whether it wins.

Measured over three worlds of eight thousand ticks:

| Measure | Before | After |
| --- | --- | --- |
| `Action::Attack` chosen | 216 | 2,192 |
| `Action::Fight` chosen | — | 335 |
| Fleeing | 0 | 8,494 (0.80% of all actions) |
| Fights on the record | 51 | 1,105 |
| Survivors who reckon themselves better for a fight | 11 | 28 |
| Survivors who reckon themselves worse | **0** | **23** |

That last row is the one that matters. The history mechanic has always had two
directions and only ever one of them populated in the wild, because an agent
that came off badly against a predator usually died before it could draw the
lesson. Agents that lose fights to each other survive them, so "having been
beaten makes running look better" is now something a settlement actually
learns rather than something only a test demonstrates.

**And it costs the settlement a little.** Eight worlds a side at fifteen
thousand ticks, against the commit before:

| Measure | Before | After | Shift |
| --- | --- | --- | --- |
| End population | 97.0 ± 7.9 | 82.1 ± 7.6 | −1.36 se |
| Peak population | 110.3 ± 4.5 | 101.6 ± 6.3 | −1.12 se |
| Births | 160.9 ± 8.3 | 150.8 ± 12.8 | −0.66 se |
| Deaths | 88.9 ± 3.1 | 93.6 ± 8.5 | +0.53 se |
| Soil fertility | 0.40 | 0.39 | −0.93 se |
| Settlements still inhabited | 8 of 8 | 8 of 8 | — |

Nothing here clears the bar this project uses — one standard error at eight
worlds a side shows nothing — but every population measure moves the same way
and deaths move the other, which is a coherent direction rather than the
scatter of a null result. If it is real the cost is about fifteen per cent of
the end population, and it is the expected cost rather than a defect: a
settlement whose members hit each other and run from each other spends time
and health on it, and no settlement was lost.

The thing to watch is that the cost stays a cost and does not become a spiral,
which is a question about the next piece of work rather than this one. A
grudge currently never reaches the relationship — `Relationship` and
`EmotionState` keep separate books, so a man who has just been hit still
counts the man who hit him a close friend — so there is nothing yet that lets
one blow lead to the next.

## The relationship graph

`EmotionState` and `Relationship` kept separate books. A grudge lived in
`anger_sources`, was read by action selection and by nothing else, and never
touched the bond; a blow dealt damage, wrote anger, broke a bone and left the
relationship exactly where it found it. A man who had just been hit went on
counting the man who hit him a close friend.

And nothing could have shown through if it had. Measured at fifteen thousand
ticks before any of this: 82 to 105 relationships apiece, nine in ten of them
at 0.6 or better, mean bond **0.901**, and `RelationshipType::Rival` and
`Enemy` constructed nowhere outside a test file in the whole project's
history — so `get_hostile_relationships` and the inspector's hostile count
read zero in every run there had ever been, including runs in which eighty-six
bonds in one settlement stood below zero.

**Two rates were being read as amounts.** `Population::update_relationships`
added up to 0.10 in proximity bonus to every nearby pair every tick, with no
ceiling, so a bond saturated within a day of standing beside somebody.
`Relationship::update_from_trait_interaction` ran on the same schedule and
moved a bond 0.035 a tick for two people who got on and 0.065 for two who
clashed — inseparable in three days, sworn enemies in a week, both regardless
of anything that had happened between them.

Both are dispositions rather than events, and both now have a pace and a
ceiling. A season of never leaving somebody's side makes them a familiar face
(0.3) and no more. A season of getting on with them makes them a friend (0.5)
and no more. Friction keeps its floor at the bottom of the scale, because
friction is friction. What takes two people past friendship is what they have
actually done: meals shared, help offered, gifts given, children raised.

**On top of that, the feelings land.** `let_grudges_tell_on_the_bond` runs
each tick over everybody — not over pairs standing near each other, because a
grudge is an opinion and not a proximity effect, and doing it the other way
would leave a hole exactly where fear now puts one: an agent that resents a
man it dare not face keeps away from him, and would therefore have gone on
counting him a friend. A blow costs `WHAT_A_BLOW_COSTS` — a quarter of the
whole scale, at once — with a share of that for the one who threw it.

**And the number gets a name.** `settle_what_we_are` maps the bond onto the
type: Enemy below −0.6, Rival below −0.2, Friend above 0.5, Acquaintance
between. Blood is not renamed — a brother you cannot stand is a brother.

Measured over three worlds at fifteen thousand ticks:

| Measure | Before | After |
| --- | --- | --- |
| Mean bond across a settlement | 0.901 | 0.78–0.83 |
| Named Rival | 0 | 10–14 |
| Named Enemy | 0 | 40–83 |
| Named Friend | 0 | 3,646–5,101 |
| Bonds below zero | 39–86 | 57–112 |

**The interesting negative.** Setting the grudge weight to zero and running
again leaves the enemy count where it was, inside the scatter. What makes
enemies in this settlement is being hit, not being lied to: lies and betrayals
are rare events, blows are not, and a blow is worth thirty ticks of a
full-blown grudge in one go. The grudge mechanism is wired and correct per
grudge — a lie costs about a quarter of the scale over the life of the anger
it creates — and its contribution to the settlement statistics is not
resolvable at three worlds. Recorded as measured rather than tuned until the
number moved.

**At eight worlds a side**, against the commit before, this is one of the few
changes in this project that clears the bar decisively:

| Measure | Before | After | Shift |
| --- | --- | --- | --- |
| Relationships named rival or enemy, per agent | **0.00 ± 0.00** | **1.53 ± 0.29** | **5.30 se** |
| Mean bond across a settlement | 0.90 ± 0.01 | 0.82 ± 0.02 | −3.83 se |
| Bonds below zero, per agent | 1.09 ± 0.21 | 1.96 ± 0.36 | 2.08 se |
| Deaths | 84.8 ± 3.7 | 99.0 ± 7.0 | 1.81 se |
| Relationships per agent | 103.6 ± 8.2 | 107.7 ± 9.9 | 0.32 se |
| Close relationships per agent | 92.9 ± 7.8 | 85.8 ± 8.9 | −0.60 se |
| Peak population | 95.8 ± 5.6 | 99.9 ± 7.8 | 0.43 se |
| End population | 85.3 ± 6.2 | 77.5 ± 9.5 | −0.68 se |
| Births | 145.0 ± 8.3 | 151.5 ± 16.1 | 0.36 se |
| Soil fertility | 0.40 | 0.40 | 0.15 se |
| Settlements still inhabited | 8 of 8 | 8 of 8 | — |

Every agent now has one or two people it has fallen out with, where in eight
worlds of fifteen thousand ticks apiece there had previously been not one such
relationship anywhere. The graph is meaningfully less saturated, and the
number of soured bonds has nearly doubled.

The cost is deaths, up fourteen at 1.81 se — the clearest downward signal
these measurements have produced, though still short of the bar. Note what
does *not* move with it: births are up slightly, peak population is up
slightly, end population is not clearly down, and no settlement was lost. This
is a settlement with more turnover rather than one that is failing, which is
what a society whose members occasionally beat each other should look like.

Close relationships are still the large majority — 86 of 108 per agent — and
that is the next thread. Proximity and temperament are capped now, so what
carries a bond past friendship is `positive_interaction` from social acts at
0.01 to 0.05 apiece, and over fifteen thousand ticks in a settlement where
everybody is within reach of everybody, that adds up for every pair alike. The
principle applied to the first two rates has not yet been applied to the
third.

## Distrust

Trust was kept in three books that never met. `TrustRating` in the knowledge
base held a verified track record, read when a belief was filed and nowhere
else. `Relationship::trust_level` mapped the bond onto an enum, read in one
place, to decide whether a gift would be accepted.
`TraitSet::combined_trust_modifier` summed every trust-flavoured trait an
agent had, which mixes two different things — Paranoid is about whether *this*
agent believes people, Charismatic is about whether people believe *them* — so
a paranoid charmer trusted everybody slightly less for the wrong reason.

**And the channel that actually carries information consulted none of the
three.** Resource and building locations pass straight into
`exploration_knowledge`, which is what foraging reads. They went in from
anybody at all, including somebody the agent had just named an enemy. And they
could not be wrong: `would_lie_to` weighs honesty and the relationship, and its
only caller — `prepare_information_to_share` — was itself never called. **No
lie had ever been told in a running settlement.** Beside all this sat the
gossip apparatus, writing into a `known_information` map that changed no
behaviour whatever.

`Agent::how_far_i_trust` answers it once, from the four things the
specification names: what the two of them are to each other (weighted
heaviest — you believe your friends), whether this one has been right before,
what sort of person is listening, and what sort is talking. The listener
decides whether to take the word; the speaker decides whether it is true.

**What was lied about decides what the lie costs.**
`what_a_lie_about_this_costs` reads which need the subject answers and how hard
that need is pressing on this agent — the same `how_hard_it_presses` the drive
hierarchy ranks needs by — so a lie about food to a man who is not hungry is a
small thing and the same lie to one who is starving is not. Then what the two
of them were to each other, because being deceived by somebody you trusted is
worse than by somebody you did not; then whether the agent is vengeful,
forgiving, trusting, or already half expecting it. It had been a flat 0.2
whatever the lie was about.

**Two things had to be fixed before any of it could work.**

An agent's map of where things are is fed both by looking and by being told,
and the two went into the same map with nothing to tell them apart. So a man
walked to the place he had been told about, found bare ground, and read his
own hearsay back off the map as confirmation: every lie verified as true, and
the lie-detection apparatus could not detect anything. `who_told_me` keeps the
source, and a lie is found out at the only moment it can be — the agent
looking at the spot with nothing on it. Hooking that to `Action::Explore`
first caught almost nothing, because `Action::Explore` is chosen about never;
it belongs in the per-tick sight pass, where agents actually look around.

And agents passed on hearsay as though they had seen it, which launders a lie:
the man who invented a place is never blamed, because everybody heard it from
somebody honest who heard it from somebody honest. Measured, a hundred and
fifty lies produced **four thousand** accusations, nearly all against people
telling the truth as they understood it. An agent now passes on only what it
has been to and looked at.

**Two more found by measuring rather than reasoning.** Reading an emptied patch
as a lie had agents calling four thousand honest tips falsehoods — a renewable
node is kept when picked bare precisely because it will bear again, so an empty
patch is a stale tip, not a lie. And `known_information` and `beliefs` were
never pruned, so once agents started telling each other things, a settlement of
a hundred carried tens of thousands of remembered claims and scanned all of
them every hundred ticks; it is a rolling window of sixty-four now, which is
enough to hold a grudge about and not a ledger.

Measured over two worlds of fifteen thousand ticks:

| Measure | Before | After |
| --- | --- | --- |
| Lies told in a settlement | **0** — impossible | 169–247 |
| Lies found out | — | 74–113 |
| A caught-out man's credit with the one he lied to | — | 0.15, from a neutral 0.5 |
| Pairs who will not take each other's word | **0%** — nothing was consulted | 2.7–6.5% |

Fewer lies are found out than are told, which is right: a lie about a distant
place stands until somebody walks to it. Switching lying off entirely drops
detections from four thousand to three, so what is being detected is lies and
not staleness.

**And at eight worlds a side** it costs a settlement almost nothing:

| Measure | Before | After | Shift |
| --- | --- | --- | --- |
| Mean trust between any two agents | **0.00** — nothing consulted any | 0.70 ± 0.01 | 49.68 se |
| Pairs who will not take each other's word | **0.00%** | 7.43% ± 1.75 | 4.24 se |
| People an agent has caught lying to it | **0.00** | 0.93 ± 0.18 | 5.02 se |
| Places an agent knows of | 239.9 ± 1.9 | 231.8 ± 3.1 | −2.21 se |
| End population | 73.5 ± 10.2 | 78.0 ± 9.1 | 0.33 se |
| Peak population | 97.0 ± 7.2 | 98.6 ± 5.9 | 0.17 se |
| Births | 137.9 ± 13.8 | 143.9 ± 13.3 | 0.31 se |
| Deaths | 89.4 ± 5.3 | 90.9 ± 6.2 | 0.18 se |
| Soil fertility | 0.40 | 0.40 | 0.94 se |
| Settlements still inhabited | 8 of 8 | 8 of 8 | — |

The whole price of a settlement that does not believe everything it is told is
eight places in two hundred and forty — three per cent of what an agent knows
of the map — some refused because the speaker was not credible, some struck off
after being walked to and found empty. Nothing else moves at all: population,
births, deaths and the ground are within a standard error either way. Nearly
every agent has caught somebody out at least once by fifteen thousand ticks.

That the cost is so small is itself the finding. Sight reaches twenty-five
tiles and smell finds what is close, so being told where things are was never
what kept a settlement alive — which is precisely why the channel could go
ungated and unfalsifiable for the project's whole history without anybody
noticing.

## News: age, room and shelf life

Three things were missing from the way agents tell each other where things
are, and each was making the trust work behave badly.

**A claim carried who made it and nothing else.** So a man who honestly
reported a patch he saw last season was called a liar the moment somebody
found it picked, and there was no way for him not to be. `Hearsay` now carries
when the speaker says he saw the thing: a liar says he walked past it this
morning, an honest man says when he actually did, and a man is answerable for
two days. Being out of date costs a sixth of what a lie costs and no anger at
all — the difference between a man whose news keeps badly and a man who
invented a place.

**Walking past a thing again is seeing it again.** The sighting tick was set
once, on first discovery, and never touched — so "a patch I just passed" was a
claim nobody in this model could make. It needed a second map rather than a
refresh of the first: skill experience is paid on the discovery tick being the
current one, so writing today's tick there paid an agent Farming experience
every tick it stood near a field, which is the exact defect that map was
cleaned up for once already. `last_seen_ticks` answers what an agent can vouch
for; `resource_discovery_ticks` answers what it learned from.

**Telling was strictly two-handed.** One speaker, one listener, and nobody
else heard a word of it however many people were standing round.
`say_it_out_loud` reaches everybody within earshot, and each of them decides
for themselves whether the speaker is worth believing — so a settlement can
take one man at his word and disbelieve him in the same breath.

**And a liar counts the people who can hear him.** He picks ground nobody
present has walked lately, because somebody who was there will contradict him
on the spot; he weighs the room by whoever in it is worth deceiving; and every
extra pair of ears is another person who may go and look, so a crowd of five
is about a third as tempting as a private word.

**What an agent keeps is what it has some use for.** Nothing bounded it
before, which did not matter while the only way to learn a place was to walk
past it and matters a great deal now that news travels. Ninety-six places, and
what goes first is what answers no need this agent has — hearsay before
first-hand, older before newer at equal interest. A thirsty man holds on to
every waterhole he has heard of and lets the flax go.

**Two things measured rather than reasoned about**, both of which had the
previous piece of work wrong in ways that were not guessable:

The periodic sweep over remembered claims is retired. Run with lying switched
off entirely it still made every agent a proven liar to **twenty-seven**
others — every one of those accusations false — because `verify_resource_claim`
reads the agent's own map as ground truth and an agent's map holds what it has
been told. The sight pass, the honest test, fired **not once** in the same run.
There is one detection path now and it is the one where somebody is standing on
the spot.

And the first cut of "count the room" abolished lying outright. Vetoing a lie
when anybody present had *ever* walked the ground meant **four** lies told in a
whole world's life, because over fifteen thousand ticks a settlement walks over
nearly everything. Only a sighting inside the last season lets a man contradict
you.

Eight worlds before, ten after:

| Measure | Before | After | Shift |
| --- | --- | --- | --- |
| Places an agent holds in mind | 239.5 ± 1.6 | **96.0 ± 0.0** | −87.86 se |
| People caught lying, per agent | 1.35 ± 0.35 | 0.16 ± 0.04 | −3.41 se |
| Mean trust between two agents | 0.70 ± 0.02 | 0.69 ± 0.01 | −0.82 se |
| Pairs who will not take each other's word | 7.77% ± 2.22 | 7.47% ± 1.68 | −0.11 se |
| Births | 149.4 ± 7.0 | 130.2 ± 13.6 | −1.26 se |
| Deaths | 94.6 ± 7.6 | 82.8 ± 5.1 | −1.30 se |
| End population | 79.8 ± 4.6 | 72.4 ± 10.0 | −0.67 se |
| Peak population | 100.9 ± 4.7 | 96.7 ± 6.3 | −0.54 se |
| Soil fertility | 0.40 | 0.40 | 0.75 se |
| Settlements still inhabited | 8 of 8 | 10 of 10 | — |

Nobody carries the map any more, and accusations fall eightfold — which is the
point, because nearly all of them were false. What does *not* move is the
telling detail: mean trust and the share of pairs who will not take each
other's word are unchanged. Trust was never really being driven by the
evidence; it was driven by the bond and by disposition, and the accusations
were noise on top. Now there are eight times fewer of them and they are real.

Births and deaths both fall by about the same amount, neither clearing a
standard error and neither alone meaning anything, but moving together — a
slightly smaller, slower settlement rather than a failing one, and every world
still inhabited. Some of that is the memory cap: an agent that knows ninety-six
places rather than two hundred and forty walks further to feed itself.

## Test coverage

1,279 library tests, 15 integration tests, 21 plugin tests, 1 doc test, plus
two ignored long-run tests (`a_settlement_lasts_thirty_thousand_ticks` and
`a_river_settlement_keeps_its_ground`). All
pass, except the known flaky ones (`test_resource_clustering`,
`test_minimize_travel_time_from_agent_position`,
`test_production_building_placed_near_resources`,
`water_is_not_used_up` and `a_cold_agent_ends_up_dressed`) that assert on
properties a randomly generated world
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

The suites written against the subsystems this document describes, each of
which drives a whole `Simulation` or a fully-built `Agent` rather than a
function in isolation:

| Suite | Covers |
| --- | --- |
| `src/analytics/tests/nutrient_loop_tests.rs` | what a body passes, what spoils and what dies going back into the ground |
| `src/analytics/tests/fishery_tests.rs` | fish running on the season, a fished-out reach filling again, offal on a field |
| `src/analytics/tests/personality_tests.rs` | founders drawn with compatible traits, newborns taking after both parents |
| `src/core/tests/drive_leaning_tests.rs` | a trait scaling a drive's weight and moving its threshold |
| `src/analytics/tests/specialisation_tests.rs` | mastery costing more the higher it goes, unused skills rusting, a practised hand producing more |
| `src/core/tests/drive_hierarchy_tests.rs` | rank, nearness of death deciding among the primaries, a drive gated behind the one before it |
| `src/agents/tests/appraisal_tests.rs` | the same wolf angering one agent and frightening another, and what past fights change about that |
| `src/analytics/tests/fight_or_flight_tests.rs` | running from what you are afraid of, striking at what is in reach, and a grudge deciding between the two |
| `src/analytics/tests/relationship_graph_tests.rs` | a grudge weighing on a bond, a blow landing on it, and what two people are following what they think of each other |
| `src/analytics/tests/distrust_tests.rs` | whose word an agent takes, what a lie costs by what it was about, and a lie being found out by walking to it |
| `src/analytics/tests/news_tests.rs` | a week-old sighting not being a lie, a crowd making a man think twice, and a thirsty man keeping the waterholes |

---

## Superseded

An earlier version of this document listed auto-save, deterministic replay,
configuration validation, `SimulationConfig` and error recovery as missing.
Auto-save, replay, config validation and `SimulationConfig` have since been
implemented — the first three are running, replay is built but unconnected.
Error recovery (isolating a panicking agent so one failure does not end the
run) is still absent.
