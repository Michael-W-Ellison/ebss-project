# Known Issues

**Last verified:** August 2026, against commit `0d11751` and the work since.

Each entry below was reproduced before being written down, and each carries
the evidence. Entries are ordered by how much they block someone picking the
project up.

Every build configuration compiles today — default, `--features gui`,
`--features bevy_gui` and `--workspace` — so nothing here stops you building
and running the project.

---

## Correctness

### 1. Eight tests fail intermittently

    world::tdd_tests::naturalistic_resource_tests::test_resource_clustering
    world::tdd_tests::spatial_planning_tests::test_minimize_travel_time_from_agent_position
    analytics::tests::agent_building_integration_tests::test_production_building_placed_near_resources
    analytics::tests::agent_building_integration_tests::test_production_chain_buildings_cluster
    analytics::tests::agent_building_integration_tests::test_different_building_types_use_appropriate_strategies
    analytics::tests::longevity_tests::water_is_not_used_up
    analytics::tests::clothing_tests::a_cold_agent_ends_up_dressed
    analytics::tests::news_tests::honest_agents_do_not_end_up_accused

Measured failure rates of roughly 1-in-10 to 1-in-20 per run for the first two,
4-in-120 for the third and 1-in-30 to 1-in-40 for the next two, all present long
before recent work (measured on unmodified code at 2/20, 3/15, 4/120, 1/40 and
1/30). The last was seen to fail once and then pass six times running; it
asserts that a world holds 95% of its water after six thousand ticks, and
across twelve worlds the worst case sits at 98.4% — on the commit before the
calendar was fixed it sat at 95.6%, so the margin got wider rather than
narrower, and the tail is simply thin. The seventh was found while checking
whether a change had broken it: it fails about one run in six *and does so on
the commit before the change too*, so it was an undocumented flake rather than
a regression — the test itself says a random world does not always let an agent
reach the flax it can see. The eighth was found the same way: it failed once
in a full-suite run and then passed ten times running on its own, and what it
asserts — that no honest agent is ever accused of lying — depends on where a
random world happens to put its resources and who happens to walk over them.
All eight build a world through
`World::new`, which draws from `thread_rng`, and
then assert on a property a random world does not always have — for example
that clay deposits happen to be clustered, or that a forge finds somewhere near
the iron to stand.

The fix is to give world generation a seed, which the project wants anyway for
reproducible runs. Until then, a red build is not necessarily a real failure,
which is corrosive: check whether the failing test is one of these eight before
assuming a regression.

### 2. Three fifths of everything a settlement does fails — mostly fixed

The drive hierarchy made the drives ask for the right things. Almost nothing
underneath them can deliver. Measured over two worlds of fifteen thousand ticks
(2.1M actions), by share of all actions taken and how often each one failed:

| Action | Share of all actions | Failed |
| --- | --- | --- |
| Gather | 21.5% | **85.6%** |
| Mate | 19.7% | **99.9%** |
| Build | 12.1% | **100.0%** |
| Store | 4.7% | **100.0%** |
| Craft | 4.2% | **99.3%** |
| Move, Sleep, SeekShelter, Eat | 36.1% | 0% |

That is **about three fifths of every action a settlement takes coming to
nothing**. Each of the five has a distinct and nameable cause, and none of them
is subtle:

**Store never works at all.** `generate_action_for_drive` maps Preparedness to
`Action::Store { item_type: "resource" }` — a placeholder string the executor
does not recognise. 13,713 failures in four thousand ticks, every one of them
`Unknown item type: resource`. The drive is answerable; the action is a stub.

**Craft cannot bootstrap.** Utility maps to `Action::Craft { item_type:
"woodenaxe" }`, which needs Crafting at −5. Skill starts at −10 and rises only
by *doing*, so no agent can ever make its first axe: `insufficient skill (need
-5, have -8)`. Some also want a technology nobody has. This is why the world
has no tools in it, which issue #5 records as a gap in crafting — it is not
that nothing makes tools, it is that the one recipe agents reach for is behind
a skill gate they cannot climb.

**Build is attempted with nothing to build from.** `Missing resources for
SmallHouse: 38 wood (have 12), 30 stone` — and the stone line has no "have"
at all, because they have none. Nothing checks materials before choosing to
build and nothing makes an agent gather *towards* a build, so the Construction
drive spends an eighth of the settlement's life restating that it has not got
enough wood.

**Mate is aimed at whoever is nearest.** `resolve_action_target` fills a nil
target with the nearest agent, not a viable mate, so the top reasons are
`Target cannot reproduce (too young, too old, or pregnant)` and `Agents too far
apart for mating`. One birth per thousand-odd attempts.

**Gather fails because there is no water.** The single largest failure in the
whole simulation, 131,436 in one pair of worlds: `Gather: No water sources
nearby`. Thirst maps to `Action::Gather { resource_type: "water" }` and the
agent is nowhere near any. Thirst is a primary drive that outranks nearly
everything, so an agent away from water spends its turns asking for it and
being told no, rather than walking to it.

The common shape is the same in all five: **a drive is answered by naming an
action, and nothing checks that the action can succeed from where the agent is
standing with what it is carrying.** Before the drive hierarchy these drives
rarely won a turn, so the actions were rarely attempted and the failures were
invisible. Ranking the drives properly is what exposed it.

It also corrects something recorded earlier in this document: `Action::Build`
and `Action::Socialize` becoming non-zero after the hierarchy was reported as
progress. Build became non-zero and has **never once succeeded**.

**Since largely fixed.** Founders arrive with the hands of grown people and
stone tools, so crafting is no longer behind a skill only crafting can teach; a
skin tent is the first shelter, so building no longer needs thirty stone a
stone-age people cannot quarry; putting something by names a thing out of the
agent's own pack instead of a placeholder; mating chooses somebody who could
actually bear a child and whom the agent trusts; building is not attempted with
an empty pack; and, generally, an action that keeps failing gets chosen less
often, per particular thing tried rather than per kind of undertaking.

**Craft now never fails.** The recipe chain finished the job. Measured over
eight worlds a side of ten thousand ticks against commit `ec9399a`: craft was
attempted 17,724 times a world and failed 17,577 of them — 99.2%, every one of
them `Cannot craft woodenaxe`, either a missing technology or a skill gate. It
is now attempted 529 times a world and **fails none of them**, because Utility
asks for a step the pack will actually carry rather than for a named end
product. All actions failing fell from 11.6% (se 0.9) to 6.0% (se 0.5), and
stayed at 6.0% (se 0.8) once tools began wearing out.

The largest remaining failure is the one this entry opened with: `Gather: No
water sources nearby`, 12,000-20,000 a world, which is now roughly two thirds
of everything that still fails.

| Measure | Before | After |
| --- | --- | --- |
| All actions failing | 58.1% | **6.6%** |
| Population at 15,000 ticks | 136 | 218 |
| Move failing | 73% | 0.1% |
| Mate, share of all actions | 22.5% | below the top twelve |
| Build, Store | 100% failing | out of the reckoning |

What is left is a thirsty agent nowhere near water, and children born during a
run who have not yet learned to craft — the second of which is correct and
self-correcting. The cost of all this is issue #3.

### 3. The nutrient loop does not keep up with a settlement that works

`the_farmed_ground_holds_up_longer` asks that farmed ground keep at least half
its fertility over ten thousand ticks of a settlement working it. It passed
three times in three before the work of issue #2 and passes about three times
in six after.

This is not a flake and it is not a bug in the loop. It is what the loop costs
when the people on top of it stop wasting their lives. Three fifths of every
action a settlement took used to come to nothing; it is now under a tenth, and
the population at fifteen thousand ticks went from 136 to 218 on the same land.
Twice as many people, each of them twice as effective, take a great deal more
off the ground.

The mechanism was isolated rather than guessed at. Granting founders the
`wooden_tools` technology - which clears twenty-five thousand crafting failures
at a stroke - took the test from three-in-four to one-in-five on its own: a
people who can put a handle on a stone strip the land far faster. That grant
was removed for exactly this reason, and it is only half the story; the rest is
the general rise in effectiveness.

**It wants a decision rather than a patch**, and the decision is not the
simulation's to make:

- Let the settlement press harder and rely on the brakes the model already has
  - breeding gated on surplus, and migration when the ground fails - to find
  the new equilibrium, and move the threshold this test asserts.
- Or put more back: the fishery is the one input that is not paid for out of
  the same ground (issue #6 and the audit), and a settlement that fished harder
  could carry more people on the same soil.
- Or accept a smaller settlement and slow the birth rate.

Left failing rather than quietly weakened, because the assertion is measuring
something real and the number it is measuring has genuinely changed.

### 4. No error recovery around a tick

One panicking agent ends the whole run and loses everything since the last
autosave. There is no isolation of per-agent failure and no attempt to
continue after an error. This mattered concretely: a probability bug in
conception crashed roughly one run in twenty-five until it was fixed, and each
crash took the entire simulation with it.

---

## Design gaps that show up as odd behaviour

### 5. A settlement that overshoots slides instead of settling back

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
have learned the practice. **Since fixed** — see the four return paths below.

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

A second thing this turned up: mean health across a settled population sat at
65-70 and never recovered, and neither exposure nor attacks accounted for it.
**Since found and fixed** - it was newborns. Both survival clocks are kept as a
tick the agent last ate or drank on, and both start at zero; that is right for
the twelve people a world begins with and wrong for everybody born afterwards,
who arrived having last drunk at the beginning of the world. An infant born
after about four thousand ticks was two days past the point where dehydration
takes health, lost 1.65 a tick from its first breath, and died at sixty-one -
at full energy, unhurt, beside its mother, being nursed. Every second-generation
agent that survived at all did so carrying the damage, which is what the
population-wide deficit was. Newborns now start both clocks at their birth
tick; mean health across a settlement runs at 90-96 instead of 65-70.

**Since measured.** Six things were put in against this: worked-out ground now
carries a proportionally smaller crop rather than flooring at four tenths; a
denied drive presses harder the longer it waits, up to fourfold; breeding waits
on a surplus or a long settled stretch rather than on a full stomach; children
have a quarter to a half of an adult's reserves against a famine; ten days of
unanswered hunger sends an agent out of the country it is in; and agents record
how their attempts turn out and drop what does not pay.

Six worlds to thirty thousand ticks afterwards: peaks fell from a mean of 141
to 93, five of six worlds ended within a tenth of their own peak rather than a
third below it, and the farmed ground ended at 0.179 fertility rather than
0.025. Four settled outright. Two still worked their ground out, and in both
the migration pressure was firing on a quarter to a half of the population by
the end.

**Then it stopped holding.** That measurement was taken while every settlement
was losing its second generation to the newborn dehydration bug described above.
Fixing that roughly doubled how many people a world grows, and the brakes turn
out to be calibrated for the smaller number. The same six worlds, all three
states of the code:

| Measure | Before any of it | With the survival pressure | With healthy newborns |
| --- | --- | --- | --- |
| Worlds still inhabited | 11 of 12 | 6 of 6 | 6 of 6 |
| People at the end | 77.7 | 76.0 | 78.2 |
| Highest reached | 141.3 | 93.2 | 211.5 |
| End over peak | 0.55 | 0.82 | 0.37 |
| Fertility of the farmed ground | 0.025 | 0.179 | 0.055 |

Every world is still inhabited at thirty thousand ticks, which no version of
this held before. But the overshoot-and-slide is back and steeper than it
started. The soil economics are untouched and were always the binding
constraint; breeding-on-a-surplus and migration bought a settlement of ninety a
soft landing and buy a settlement of two hundred nothing.

**And then the ground got a way back.** Four return paths went in against this:
what a body passes after a meal, what spoils in somebody's pack falling to the
ground rather than being deleted, what a body is when it stops, and the roots,
stalk and leaf a plant leaves in the tile it grew in.

The first three did essentially nothing. Three worlds to thirty thousand ticks
with all of them in came out at mean farmed fertility 0.058, against 0.055 with
no return path at all. The reason is spatial and obvious in hindsight: what
goes through a person comes out where the person is standing, and agents range
over the whole map. That is not a fault to correct. It is the fact that makes
carting muck onto a field worth an agent's time, and it is why the one return
path that already existed had to be a learned practice.

The fourth was decisive, and was the one missing longest. The model had been
treating every plant as though the whole of it were carried off. Most of a
plant never leaves the field: only the grain does. Four worlds to thirty
thousand ticks with crop residue staying put:

| Measure | Before any of it | Survival pressure | Healthy newborns | Agent-side returns | Crop residue too |
| --- | --- | --- | --- | --- | --- |
| Worlds run | 12 | 6 | 6 | 3 | 4 |
| Still inhabited | 11 | 6 | 6 | 3 | 4 |
| People at the end | 77.7 | 76.0 | 78.2 | 53.0 | **154.0** |
| Highest reached | 141.3 | 93.2 | 211.5 | 212.3 | 226.2 |
| End over peak | 0.55 | 0.82 | 0.37 | 0.25 | **0.69** |
| Fertility of the farmed ground | 0.025 | 0.179 | 0.055 | 0.058 | **0.268** |

The two runs do not overlap on either measure. Every world with residue ended
between 140 and 176 people on ground between 0.175 and 0.457 fertility; every
world without ended between 25 and 107 on ground between 0.031 and 0.103. Map
litter now holds near 430 rather than draining from 1,035 to 368.

What matters about the last column is the peak. Every earlier measure that
improved end-over-peak did it by holding the population down — the survival
pressure took the peak from 141 to 93 to buy its 0.82. This one leaves the peak
where it was, at 226 against 211, and the settlement holds two thirds of it
instead of a third. The population is not being braked; the ground under it is
no longer collapsing.

It is not a closed loop and cannot be. Rot keeps three fifths of what it works
on and loses the rest, so every turn of the cycle is smaller than the last, and
farmed fertility is still falling at thirty thousand ticks in three of the four
worlds. What changed is the slope. A settlement that overshoots now settles
back onto ground that can still carry it, rather than sliding to the level
where hardly anybody lives there.

**And then a fishery, which reverses it.** Everything above is a return: the
ground gets back some part of what it already paid out, minus what rot loses,
so the best a farming people can do is run down slowly. A fish is not grown on
the land. It is grown at sea, fed on a whole catchment, and it comes up the
river under its own power whatever last year's fishing left behind — so what is
left of one, put on a field, makes the country richer than it was.

Four worlds to thirty thousand ticks with a fishery in the model:

| Measure | No return path | Crop residue | Residue and a fishery |
| --- | --- | --- | --- |
| Worlds run | 6 | 4 | 4 |
| Still inhabited | 6 | 4 | 4 |
| People at the end | 78.2 | 154.0 | 150.5 |
| Highest reached | 211.5 | 226.2 | 220.8 |
| End over peak | 0.37 | 0.69 | 0.69 |
| Fertility of the farmed ground | 0.055 | 0.268 | **0.607** |

**All four worlds ended with better ground than they started on** — a mean of
0.545 at tick zero against 0.607 at thirty thousand, and every world
individually up, from 0.539→0.594 at worst to 0.544→0.641 at best. Map
nutrients rose from about 800 to between 1,049 and 1,103 rather than sitting
flat. Standing crop ended between 4,900 and 6,000 units.

The peak and the end-over-peak are the part to read twice. They did not move:
226 and 0.69 without the fishery, 221 and 0.69 with it. The settlement still
overshoots and still settles back onto what the ground will carry. What changed
is that the ground it settles back onto is no longer poorer each time. Nothing
was made easier for the people; something was added to the country.

Twelve to thirty-four people in a settlement of a hundred and fifty had settled
into fishing as a matter of course by the end, having each worked it out from
their own record of whether it paid.

What remains: a spent field still counts as a field, so a settlement still will
not break new ground while exhausted ones sit inside its radius, and nobody has
still ever died of hunger.

### 6. Winter is not cold: the tile temperature is frozen at first touch

`ClimateManager::get_biome` builds a `Biome` for a position the first time
anybody asks about it, stamps the current season and hour into it, and caches
it for the rest of the run. `clear_biome_cache()` exists and is called only
from a test. So `get_temperature` — the temperature agents actually feel, via
exposure and body temperature — is that first-touch value plus whatever the
weather is doing now, and the season never reaches it again.

Measured over 160,000 ticks, holding the weather constant so that only the
season varies — the temperature `get_temperature` reports for a plains tile
under a clear sky:

| Season | Clear-sky temperature |
| --- | --- |
| Spring | 18.667 °C |
| Summer | 18.667 °C |
| Fall | 18.667 °C |
| Winter | 18.667 °C |

Identical to three decimal places, on about fourteen thousand samples each. The
season contributes nothing at all; every degree of variation in the number an
agent feels comes from the weather type sitting over it.

Two correct seasonal-temperature paths are computed and thrown away.
`ClimateManager::tick` sets `base_climate.temperature = base_temp * season_mod
* time_mod` every tick and nothing reads it. `SeasonalCalendar::apply_modifiers`
does the same job and has no caller outside its own test. The live path is the
frozen one.

The seasons do reach the world by three other routes, all working: the growth
modifier on regrowth, the length of the day that plants feel, and the
`WeatherGenerator`, which turns winter into snow, sleet and blizzards. That
last one carries the only cold there is — a winter runs about half a degree
below the other seasons because it snows, not because winter is cold. What
never arrives is the baseline swing, and with it most of the reason a
settlement would store food, put on a coat or get indoors at one time of year
rather than another.

Mortality says the same thing. Deaths per ten thousand agent-ticks, six worlds
run to twenty-four thousand ticks each, with snow correctly confined to winter:

| Season | Deaths per 10k agent-ticks |
| --- | --- |
| Spring | 1.57 |
| Summer | 1.42 |
| Fall | 1.42 |
| Winter | 1.57 |

Winter kills about a tenth more than a summer — and so does spring, to the
second decimal. On roughly 290 deaths a season that is inside the noise. There
is no winter in these worlds, only a slightly snowier stretch of the same
weather.

Fixing it is not just a cache invalidation: making winter genuinely cold is a
real change to the balance and would need measuring before and after.

### 7. Three drives ask for things the world cannot give

The design document's Appendix A gives each drive a list of **increase
conditions** — Safety on "hostile entity proximity, recent injury, darkness",
Construction on "buildable templates seen, others building, drive synergy",
Sustenance on "low food stockpile, crop depletion". None of them existed:
`base_accumulation_rate` returned one flat number per drive per tick and that
was the whole of it, including for the line `DriveType::Safety => 0.02, //
Spikes with threats`, whose comment described the specification and whose code
was a constant.

Because those drives' satisfying actions are chosen rarely, they climbed to
their ceiling and stayed there: nine of fifteen at 1.00 and active every tick
after eight thousand ticks, which left the per-agent weight as the only thing
telling them apart.

**Since fixed.** The nine now read the conditions the document gives them, and
move towards what the situation calls for rather than up a clock. Six of the
nine came unpinned:

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

**The three that stayed high were the finding at the time, and issue #2 has
since named the reason.** It is not that the world has no way to answer them:
Store is a stub that cannot succeed, and the one thing agents try to craft is
behind a skill gate they cannot climb. Preparedness asks for stockpiled
food, materials and tools; Utility for tools in working order; Luxury for
something fine. Counting what thirty agents were carrying at eight thousand
ticks: 102 wood, 21 food, 17 leather, 14 horn, 12 flax, 11 cotton, 8 wool, and
**no tools and nothing decorative at all** — zero equipped items across the
whole settlement. Those three drives are now reading the world correctly and
the world has no path to satisfy them. That is a gap in crafting and
tool-making, not in the drive system, and it was invisible while every drive
sat at its ceiling for reasons of its own.

**And the pegging is gone, though the gap is not.** The drive hierarchy puts
each drive behind the one it depends on — Preparedness cannot build until
Hunger and Thirst are reliably answered, Utility until Construction or
Industry is — so a drive the world cannot satisfy no longer sits at 1.00
shouting over everything else. Luxury fell from above its threshold 98.9% of
the time to 0.5%, Preparedness from 98.2% to 0%, Utility from 84.6% to 0.6%.
The world still has no tools and nothing decorative in it; what changed is
that the absence no longer drowns out the drives that *can* be answered.

### 8. The ecology settles in most worlds, not all

Over forty worlds, predators are still alive at the end in thirty-six and
herds stay bounded in thirty-three. In the seven that run away the predators
died out first, and although animals do wander back in from off the map, the
trickle is slow enough — by design — that a world can spend thousands of ticks
with its herds climbing unopposed before a replacement pack arrives.

### 9. Clothing and hunting cost about what they return

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

**Tools now cost and return something, and a settlement cannot keep up with
them.** Until recently a tool was a thing an agent counted: `Inventory` had
carried durability fields since the beginning, only clothing used them, and a
man with a stone axe felled timber at exactly the rate of a man with his bare
hands. An axe is now worth up to 1.8x on timber and 1.5x on stone, a spear
counts in a hunt and in the shallows, a knife nearly doubles what comes off a
carcass — and each of them wears out in twenty-five to forty pieces of work,
sooner if the hand that made it was clumsy. What the hand that made it could do
also decides how well it works, so a man's tenth spear is half again the spear
his first was and lasts twice as long, which is what "repeating the action
increases the quality of the outcome" comes to.

Measured over eight worlds of ten thousand ticks, the settlement works with
tools that are visibly used up: the mean condition of every tool held is 0.72
of new (se 0.02) and about one living agent in three is carrying something
worn through (21.3 a world, se 6.5). Toolmaking rises by a quarter to keep up —
529 crafts a world without the wear, 668 with it (se 60) — which is the
replacement cycle running.

It buys nothing measurable yet. Population is 80.0 alive at the baseline (se
4.8) and 70.2 across the sixteen worlds run with the chain and the tools (se
4.6): a difference of 9.8 at 6.6, which is noise with a hint of a decline in
it. That is roughly what the specification asks for — "everything should be
slow and inefficient", "wood and stone tools should wear out quickly" — but it
means the multipliers are only worth having if a people can keep a stock of
tools, and nothing yet makes one ahead of needing it.

### 10. Agents still cannot hear anything

Sight discovers terrain, resources and buildings, and agents now see one
another — `vision.visible_agents` is populated each tick, which is what
observational learning is gated on. Hearing is unfed entirely, so every
sound-derived percept is still a dead path. See SIMULATION_AUDIT.md.

### 11. Zoning and territory are never established

Building placement scoring reads zone and territory bonuses from
`World::zone_manager` and `World::territory_manager`, but nothing outside the
tests ever calls `add_zone` or `claim_territory`. Both managers are therefore
always empty in a live run and every bonus they contribute is zero, so
settlements have no planned structure and agents claim no ground.

### 12. Agents carry food they will never eat

Food that has turned is correctly refused, but stays in the inventory until
its freshness decays to zero and spoilage removes it — or until the agent takes
it onto a field and tips it out, which some of them work out for themselves. The same is true of food
an agent burns: a novice cook ruins about one batch in five, and the ruins ride
along in the pack. Both announce themselves as a decay scent to anyone nearby,
which is realistic and mildly useful, but nothing makes the carrier drop them:
carried weight still includes rot and cinders.

### 13. Personality exists and reaches the drives, and still decides nothing

The project's stated purpose is emergent social behaviour out of drives and
personality. Both halves are now live: everybody has a personality and it bends
what their drives argue for. What is still missing is anywhere for that to show,
because the action-selection ladder decides nearly everything before a drive is
consulted. The history below is worth keeping, because each layer only became
visible once the one under it was fixed.

**No agent held a trait.** `Agent::new` set `traits: TraitSet::default()`,
which is empty, and the only `add_trait` on any live path was the 1.5 per cent
congenital infertility roll in `with_parents`. Inheritance in `reproduction.rs`
worked, but it inherited from founders who had none, so it propagated nothing.
Measured over three worlds: **zero traits held across a hundred and
twenty-one surviving agents**, out of sixty-odd defined. **Since fixed** — see
point 1 below.

Worse, that one roll never survived either: `give_birth_internal` assigned
`offspring.traits = inherit_traits(..)` straight over the top of it, so
congenital infertility was thrown away on every live birth. The one trait
anything in the running simulation ever assigned, and it never once reached a
living agent. Also since fixed: what a child is born *with* now survives what
it is born *to*.

Everything downstream of traits is therefore dormant: the trait-to-job
affinities in `job_happiness.rs`, the gossip distortion in `gossip.rs`, the
`update_relationship_from_traits` affinity model, the emotional modifiers
(`add_fear_with_traits` and its siblings), and the religious effects. All of it
compiles, all of it is tested, none of it has an input that varies.

**And the traits would not change behaviour even if they were assigned.** Read
the enum: `Lazy` is "constant happiness decrease when working", `Builder` is
"happiness from building structures", `Glutton` is "increases happiness from
favorite food". Nearly every one of the sixty is defined as a modifier on how
an agent *feels* about what happened, not on what it *does*. `src/core/drives.rs`
contains no reference to traits at all, and `analytics/mod.rs` — where actions
are chosen — reads `.traits` ten times, all of them for gossip distortion,
infertility, religion, or passing the set to somebody else. Not once in the
priority chain. A lazy agent and a diligent one pick the same action; the lazy
one is only sadder about it.

**So the relationship graph carries no information.** Bonds are updated from
traits, and everybody's traits are identical (empty), so everybody converges on
the same footing. Measured in settlements of 45 to 68 people:

| | per agent |
| --- | --- |
| Relationships held | 32 to 44 |
| Of those, close (bond above 0.5) | 29 to 39 |
| Hostile | **0.0** |
| Attempts at `Undertaking::Dealing` | **0 in the whole run** |

Every agent is on close terms with two thirds of the settlement, nobody
dislikes anybody, and no agent ever undertakes a social act as such. There is
nothing for a personal interaction to be *about*.

The three things that would fix it, in order of what they buy:

1. ~~**Assign traits at spawn**~~ — **done.** `Population::spawn_agent` now
   draws three to five compatible traits for a founder; everybody born
   afterwards inherits, which the existing code already did. Forty founders
   between them hold sixty-odd distinct traits and no two are the same person.
   The draw is in `spawn_agent` rather than `Agent::new` deliberately: a bare
   `Agent::new` stays the same agent every time, which several dozen tests of
   other machinery rely on, and a personality is something somebody has on
   entering a world rather than a property of a body.
2. ~~**Let traits reach the drives**~~ — **done, and it was not enough.**
   `Trait::leanings()` now says which drives a trait argues for and against, as
   a multiplier on how loudly the drive argues and on how much of the need it
   takes before the agent acts. `DriveState::lean_towards` applies it, and
   every path that picks a drive honours it. A Lazy person needs more pushing
   before starting work and drops it sooner; a Coward starts running at a
   smaller wolf; an Extrovert at six tenths of loneliness is already looking
   for company where an Introvert is content.

   **It changed almost nothing about what anybody does.** Fourteen worlds to
   six thousand ticks, 777 surviving agents, comparing holders of a trait
   against everybody else on the matching undertaking:

   | | holders | others | ratio | |
   | --- | --- | --- | --- | --- |
   | Lazy, foraging | 206.3 | 269.4 | 0.77× | 1.4 se |
   | Diligent, foraging | 247.1 | 267.4 | 0.92× | 0.4 se |
   | Curious, foraging | 222.8 | 269.2 | 0.83× | 0.8 se |
   | Glutton, fishing | 21.1 | 18.9 | 1.11× | 0.4 se |
   | Builder, **building** | **0.0** | **0.0** | — | |
   | Extrovert, **dealing** | **0.0** | **0.0** | — | |

   Nothing above 1.4 standard errors. (A six-world run had Lazy foraging at
   2.04× the rest; at fourteen worlds it is 0.77×, having crossed over. Six
   worlds is not enough to say anything here either.)

   I said this was "one hook in `DriveState`, and it is what turns sixty labels
   into sixty people". That was wrong, and the reason is worth writing down.

   **What blocked it was the action-selection ladder, not the drives, and
   that has since been rebuilt.** `generate_non_emotional_action` used to be
   thirteen fixed priorities with drives consulted only at the thirteenth,
   after survival, protection, clothing, cooking, muck, farming, fishing,
   hunting, percepts, plans and goals had all had their turn. Seventy-nine per
   cent of everything a settlement did was `Foraging`, almost all of it off
   that ladder rather than out of a drive, so leaning on the Industry drive
   barely moved it.

   And when the thirteenth priority *was* reached, three drives took it every
   time. Over four thousand agent-samples, Luxury stood above its threshold
   **98.9%** of the time, Preparedness **98.2%**, Utility **84.6%** — because
   nothing in the world could answer them (issue #7). Construction was above
   its threshold 12.7% of the time and Social 38.1%, so they were not quiet;
   they simply never won, and `Action::Build` and `Action::Socialize` were
   chosen **zero** times in 777 agent-lives.

   The drive hierarchy inverted that ladder: the drives are now ranked first
   and the highest-ranked one that this agent can actually answer chooses the
   action, with the old fixed order kept only as a fallback for the drives
   that have no answer. Foraging fell from 79% of everything to 25%, Luxury
   from 98.9% pegged to 0.5%, Preparedness from 98.2% to 0%, Utility from
   84.6% to 0.6%, and `Action::Build` and `Action::Socialize` became non-zero
   for the first time.

   So the hook has the room it was waiting for, and whether a personality now
   tells has not been re-measured since. The fourteen-world reading above was
   taken against the old ladder and should not be quoted as the current state.

3. **Give agents a reason to need each other.** Everyone still does
   everything, so no agent is ever the one who has what another wants. The
   fishery is the first thing in the model that is *place-bound* — you must be
   at the water — which is the raw material for a real division of labour.

**And a fourth, which assigning traits revealed — since fixed.** The
relationship graph was undifferentiated: every bond saturated at close and
**none was hostile**, in any settlement, ever. It was arithmetic rather than
affection. `Population::update_relationships` added up to 0.10 in proximity
bonus to every nearby pair on every tick with no ceiling, so a bond saturated
within a day of standing beside somebody;
`Relationship::update_from_trait_interaction` ran on the same per-tick
schedule and moved a bond 0.035 a tick for two people who got on. Both were
rates being read as amounts.

Both are rates now. A season of never leaving somebody's side makes them a
familiar face and stops; a season of getting on with them makes them a friend
and stops. Anything past that has to be earned by what the two of them have
actually done. On top of that, a grudge weighs on the bond at eight times what
keeping company is worth, and a blow costs a quarter of the whole scale at
once. `settle_what_we_are` then puts a name to the number, so
`RelationshipType::Rival` and `Enemy` — which had been constructed nowhere
outside a test file in the project's history — appear in live settlements.

Measured at fifteen thousand ticks, three worlds:

| | Before | After |
| --- | --- | --- |
| Mean bond across a settlement | 0.901 | 0.78–0.83 |
| Relationships named Rival | 0 | 10–14 |
| Relationships named Enemy | 0 | 40–83 |
| Relationships named Friend | 0 | 3,646–5,101 |

The interesting negative: zeroing the grudge weight and running again leaves
the enemy count where it was. What makes enemies in a settlement is being hit,
not being lied to — the grudge mechanism is wired and correct per grudge, and
grudge-generating events are simply rarer than blows.

At eight worlds a side, relationships named rival or enemy went from **0.00
per agent to 1.53**, at 5.30 standard errors — one of the few results in this
project that clears the bar decisively — and the mean bond from 0.90 to 0.82.
It costs fourteen more deaths (1.81 se) on a settlement with slightly more
births and a slightly higher peak, so more turnover rather than failure, and
eight of eight settlements were still inhabited.

Nobody undertaking a social act is fixed separately: `Undertaking::Dealing`
was attempted zero times in a whole run before the drive hierarchy let drives
choose actions, and `Socialize` now runs at about 0.6% of everything a
settlement does.

**What is left of it.** Close relationships are still the large majority — 86
of 108 per agent. Proximity and temperament are capped now, so what carries a
bond past friendship is `positive_interaction` from social acts, at 0.01 to
0.05 apiece; over fifteen thousand ticks in a settlement where everybody is
within reach of everybody, that adds up for every pair alike. It is the same
shape of defect as the two that were fixed — an unbounded accumulator over a
long run — and the principle has not yet been applied to it.

### 14. Skill measured how far you had walked, and bought nothing

**Since fixed**, and recorded because the shape of it recurs.

Experience was granted for *looking*. The resource-discovery pass filtered on
the tick a thing was found and ran every tick, so a thing seen once paid out on
ten consecutive ticks — fifty Farming experience for walking past a grain
field, half a level, in a settled world holding ninety of them. A level cost a
flat hundred wherever you stood, so the last step from journeyman to master was
as cheap as the first away from knowing nothing. Nothing ever took a skill
back.

Between them, skill level measured how much of the map somebody had wandered
over. Across 298 agents at eight thousand ticks:

| trade | mean level (−10 to 10) | at journeyman or better |
| --- | --- | --- |
| Farming | **9.9** | 297 of 298 |
| Herbalism | 1.0 | 293 |
| Woodcutting | 0.5 | 265 |
| Leatherworking | **−9.2** | 0 |
| Hunting | **−9.9** | 0 |

Everybody was a master farmer and nobody had farmed. The trades nothing could
be *discovered* for stayed on the floor however much of them was done. And
none of it mattered anyway: `Skill::speed_multiplier`, `Skill::perform_check`
and `Skill::determine_quality` were all built and had no callers anywhere, and
the harvest site carried the comment "determine harvest amount based on
resource type and skill" above code that did not consult the skill. A lifetime
at a trade brought back exactly what a first day did.

Four things went in. Finding a thing pays once and pays a pittance; a level
costs more the higher it goes, sized against the roughly two hundred and fifty
goes at anything an agent gets in a working life; a trade not practised for a
year begins to go, and keeps going, though never below apprentice; and what a
hand is worth — half at the bottom, double at the top — now decides what comes
off a field per trip and whether a garment is finished or spoiled in the
making.

The spread that came out, same measurement:

| | before | after |
| --- | --- | --- |
| Best trade per agent | 9.9 | −2.4 |
| Trades off the floor per agent | 5.8 of 8 | 4.6 of 8 |
| Reached journeyman in anything | 297 of 298 | 92 of 284 |

Most people are now mediocre at several things and a minority are genuinely
good at one, which is the point. A settlement is unaffected: six worlds to
fifteen thousand ticks came out at 100.5 people on 0.561 farmed fertility,
matching the eight-world baseline exactly.

---

## Housekeeping

### 15. Committed backup file

`src/analytics/mod.rs.backup` is checked into the repository.

### 16. Build warnings

15 warnings on `cargo build`, all unused variables and imports. `cargo fix`
handles most.

### 17. Placeholder package metadata

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
- **Every drive was equal, and the ladder decided everything.** The nine drives
  had weights but no order, so nothing said that a man dying of thirst should
  stop hunting; and `generate_non_emotional_action` consulted them only at the
  thirteenth of thirteen fixed priorities, so almost nothing a settlement did
  came out of a drive at all. Drives now carry a rank — primary, secondary,
  tertiary — and inside the primary band the one that would kill soonest wins,
  computed live from how long this agent could actually last. Each drive is
  also gated behind the one it follows in the specification's chains, so
  Preparedness cannot build while its owner is hungry. Foraging fell from 79%
  of everything a settlement does to 25%, three permanently pegged drives came
  off their ceilings, and `Action::Build` and `Action::Socialize` were chosen
  for the first time in the project's history.
- **A shut-out drive drained forty times faster than it filled.** Gating a
  drive behind its predecessor needed a way for the gated drive to fall quiet,
  and `fall_quiet` used one flat rate for all nine. For Reproduction, which
  accumulates at 0.001 a tick, that rate was 0.004 — so the 9.9% of ticks an
  agent spent with its primaries unanswered cost it half its total accumulation
  and halved the birth rate. Measured at eight worlds a side, this alone was
  the difference between a settlement of 45 and one of 30. A drive now fades at
  the pace it would have grown.
- **News had no age, no room, and no shelf life.** A claim carried who made it
  and nothing else, so a man who honestly reported a patch he saw last season
  was called a liar the moment somebody found it picked. Telling was strictly
  two-handed - one speaker, one listener, nobody else hearing a word of it
  however many were standing round - and a liar weighed only the man in front
  of him. And what an agent remembered was bounded by nothing, so a settlement
  that talks carried the whole map in every head. A claim now says when the
  speaker saw the thing and he is answerable for two days; being out of date
  costs a sixth of a lie and no anger. Speech reaches everybody in earshot,
  and a liar picks ground nobody present has walked lately. An agent holds
  ninety-six places and lets go of what answers no need it has. Two things
  turned up on the way: the periodic sweep over remembered claims was making
  every accusation in the model, all of them false, and is retired in favour
  of the sight pass; and the first cut of "count the room" abolished lying
  outright by letting anybody who had *ever* walked the ground contradict it.
- **Nobody could be disbelieved, and nobody could lie.** Trust lived in three
  books that never met — a verified track record in the knowledge base, an
  enum on the relationship, and a sum of trait modifiers that mixed "do I
  believe people" with "do people believe me" — and the channel that actually
  carries information between agents consulted none of them. Resource and
  building locations went into `exploration_knowledge`, which is what foraging
  reads, from anybody at all, and could not be wrong: `would_lie_to` weighs
  honesty and the relationship, and its only caller was itself never called.
  Agents now decide whose word to take, a liar can name a place that is not
  there, and the lie is found out by walking to it. What it costs him depends
  on what he lied about, weighed by how hard that need is pressing on the man
  he lied to. Two things had to be fixed first: an agent's map mixed what it
  had seen with what it had been told, so it read its own hearsay back as
  confirmation and every lie verified as true; and agents passed hearsay on as
  first hand, which laundered a lie so thoroughly that a hundred and fifty of
  them produced four thousand accusations against honest people.
- **Fear was a hunger reading and anger was nothing at all.**
  `calculate_survival_drive_emotion` derived fear from how high a survival
  drive's value stood, so it originally sat near 0.8 between meals and — once
  the survival drives were being answered — inverted to nearly zero. Anger was
  written only by the resolution of a blow that had already landed, and
  measured at exactly 0.00 over three worlds. `should_flee` and
  `should_attack` therefore never fired in a settlement's whole life, so the
  emotional branch of `generate_action` was dead code. Both are appraisals
  now: what is in front of the agent is weighed against what the agent can do
  about it, and the answer comes out as anger where it can be fought and fear
  where it cannot. `ThreatAssessment` had always been able to make that
  judgement; nothing had ever asked it about anything but a wound. What
  happened in past fights scales the estimate, so an agent that has been
  beaten runs where one that has won stands.
