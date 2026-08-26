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

### 1. Twenty tests fail intermittently

    world::tdd_tests::naturalistic_resource_tests::test_resource_clustering
    world::tdd_tests::spatial_planning_tests::test_minimize_travel_time_from_agent_position
    analytics::tests::agent_building_integration_tests::test_production_building_placed_near_resources
    analytics::tests::agent_building_integration_tests::test_production_chain_buildings_cluster
    analytics::tests::agent_building_integration_tests::test_different_building_types_use_appropriate_strategies
    analytics::tests::longevity_tests::water_is_not_used_up
    analytics::tests::clothing_tests::a_cold_agent_ends_up_dressed
    analytics::tests::news_tests::honest_agents_do_not_end_up_accused
    analytics::tests::working_tests::nobody_works_more_than_they_have_a_use_for
    analytics::tests::predator_prey_tests::predators_hold_a_herd_down
    analytics::tests::fluid_tests::nobody_proposes_a_fluid_working_with_a_dry_pack
    analytics::tests::relationship_tests::a_settlement_ends_up_with_enemies_in_it
    analytics::tests::nutrient_loop_tests::what_a_settlement_eats_reaches_the_ground_it_stands_on
    analytics::tests::survival_pressure_tests::the_children_of_a_settlement_live_past_infancy
    analytics::tests::personality_tests::a_congenital_trait_survives_inheritance
    analytics::tests::midden_tests::a_settlement_fouls_the_ground_it_stands_on
    analytics::tests::survival_loop_tests::population_feeds_itself_over_a_long_run
    analytics::tests::clay_tests::a_curious_agent_with_clay_tries_molding_it
    analytics::tests::barter_tests::two_people_with_opposite_problems_trade
    analytics::tests::asking_tests::being_told_lets_you_try_it_rather_than_making_you_believe_it

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
The sixteenth was found in a full-suite run and characterised the same way as
the eighth: **0 failures in 15 runs on its own, and 0 in 15 on the commit
before the change**, so it is a full-suite-parallelism flake rather than a
regression. It asks that eight people living for a month leave some fouling on
the ground, which depends on where a random world puts the food they eat.
The nineteenth was found in a full-suite run and characterised the same way as
the rest: **0 failures in 15 runs on its own here, and 1 in 15 on the commit
before the change**.
The twentieth is the second with a known mechanism rather than a shrug about
world layout, and it is the same mechanism as the eighteenth: it asserts once
that somebody takes a stranger's word for something, and `would_take_their_word`
turns on trust built out of traits drawn at random when a founder is made. **1
failure in 15 on this commit against 3 in 15 on the one before.** Both want the
same fix - sample rather than assert once - and both are tests written in this
run, which is worth saying plainly: two of the twenty are mine.
The eighteenth is the only one of the set with a **known mechanism** rather
than a shrug about world layout, and it is worth stating because the same trap
is waiting for any test of this shape. It asserts that a curious agent with
clay in the pack proposes molding it — and the gate it goes through,
`Lessons::will_try_this_again`, is *probabilistic by design*: an untried thing
is tried with probability `NEVER_QUITE_CERTAIN`, which is 0.95. So a test that
asserts the gate returns `Some` fails one run in twenty, exactly. Measured at
**1 failure in 20 on this commit and 2 in 20 on the one before**, which is that
rate and not a regression. The fix, when somebody wants it, is for the test to
sample rather than to assert once.
The seventeenth was found in a full-suite run and characterised the same way:
**0 failures in 15 runs on its own, and 0 in 15 on the commit before the
change**. It was worth a hard look rather than a shrug, because it landed in
the same batch as a change to how gathering is chosen - but the arms measured
either side of that change have population and winter store *up* rather than
down, so what it is is another full-suite-parallelism flake.
The last five were found the same way and characterised the same way: each
was run twenty-odd times in the working tree and the same number of times on
the commit before it, in a worktree, and each came out at the same rate on
both sides — `nobody_works_more_than_they_have_a_use_for` at 1-in-20 and
1-in-20, `predators_hold_a_herd_down` at 2-in-24 and 2-in-24,
`nobody_proposes_a_fluid_working_with_a_dry_pack` at 1-in-12 and 1-in-12, and
`a_settlement_ends_up_with_enemies_in_it` at 3-in-20 against 1-in-20, which
is the same order and not a difference this sample can see. The fifth,
`what_a_settlement_eats_reaches_the_ground_it_stands_on`, is worth a note on
method: it first read 2-in-15 in the working tree against 0-in-15 on the
previous commit, which looks like a regression and is not one. Taken out to
fifty-five runs a side it is 3-in-55 against 3-in-55 — the clean baseline was
luck, and fifteen runs is not enough to tell a 5% flake from a new bug. When
it does fail it fails at 0.250 litter falling to 0.143, which is the tile
decaying with nothing added at all: ten agents pinned to a tile still wander
off it during the tick they are pinned for, and where what they leave behind
lands is up to the world. The sixth,
`the_children_of_a_settlement_live_past_infancy`, came out at 1-in-15 on both
sides the same way. The seventh,
`a_congenital_trait_survives_inheritance`, failed once in a full-suite run and
then passed fifteen times running on its own, on both sides — too rare to put
a rate on, which is worth saying rather than inventing one. None of the seven
is a regression; all seven had simply never been written down.

All fifteen build a world through
`World::new`, which draws from `thread_rng`, and
then assert on a property a random world does not always have — for example
that clay deposits happen to be clustered, or that a forge finds somewhere near
the iron to stand.

The fix is to give world generation a seed, which the project wants anyway for
reproducible runs. Until then, a red build is not necessarily a real failure,
which is corrosive: check whether the failing test is one of these fifteen
before assuming a regression, and if it is not, check it against the previous
commit in a worktree before calling it one.

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

**And the obvious explanation for it is wrong.** The reading above - that an
agent away from water spends its turns asking for it because nothing joins the
drink it had yesterday to the bank it drank from - was tested directly by
giving agents exactly that memory (see `agents::patterns`). It made no
difference: eight worlds a side of ten thousand ticks put the refusal at 3.7%
of all actions without the memory and 4.7% with it, a difference of 0.010 at a
standard error of 0.017. The count rises with the population rather than with
anything else.

Nor does the memory pay off anywhere else. Population came out 16 higher at ten
thousand ticks (se 11) and 14 *lower* at twelve thousand (se 10), eight worlds
a side each time: two runs pointing opposite ways at the same size, which is
noise. It also widened the spread - eight worlds ran from 89 to 119 people
without it and from 41 to 121 with it.

What is left is a decision that was true when it was made. `water_action`
chooses to drink when a source with water in it stands within the foraging
radius; the action is carried out later in the same tick, by which time some
other agent may have drunk the last of it. Every agent that decided to drink at
a spring holding one unit fails but the first. That would explain a failure
that scales with population, cannot be removed by better memory, and never
falls however well the settlement is doing - but it has not been confirmed, and
confirming it is the next thing anybody should do to this entry.

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

**Hunting and fishing are now slow work, and it costs nothing.** A throw used
to land six times in ten for anybody at all and take two thirds of an animal
out of it, which made a deer a thing you walked up to rather than a thing you
stalked. A throw now lands 22.5% of the time (from 60.2%) and takes a third,
so a kill is three or four throws; each costs the same whether or not it lands,
and a wounded animal that gets away no longer feeds anybody. Spear-fishing the
same: 40.1% of casts take, from 57.5%, for two fish rather than three, and
standing in the water costs whether or not anything comes past.

Measured over eight worlds a side of ten thousand ticks, the settlement absorbs
it: 72.1 alive before and 77.9 after, a difference of 6 at a standard error of
10. What changes is what people do. They throw three times as many spears
(462 to 1,400 across eight worlds, because a kill takes several) and they fish
half as much (13,314 casts to 6,275) - the learning mechanism turning away from
work that stopped paying. Fewer of them end up in hide or fur, 3.0 a world
against 1.9, which is the cost of it.

**A midden smells, is walked away from, and comes up in berries - slowly.**
Waste had been going into the ground as leaf litter since the nutrient loop was
built, and that was all it did. It now also leaves the two things that make a
midden a midden: a smell that reaches a few tiles and is emitted as
`ScentType::Decay`, and seed that came through whole. A man who wants to lie
down on fouled ground steps off it first, which over a settlement's life is
what puts the midden at the edge of a camp rather than in it. Once the smell
has gone - which happens an order of magnitude faster than the matter breaks
down - whatever was in it comes up as food nobody planted.

Measured at ten thousand ticks: about a thousand tiles carry fouling, a dozen
to seventeen are foul enough at once to be walked away from, and every one of
them carries seed. Food nodes went from 25 to 27 in one world of two and stayed
at 25 in the other. So the loop closes, and it closes rarely, for a reason that
is correct rather than broken: a camp keeps its own midden too foul to grow
anything for as long as the camp is there, and what comes up comes up on ground
the people have moved on from. Watching a settlement that never moves will
never show it.

One number had to be found by measuring. `ENOUGH_TO_COME_UP` was five times
higher to begin with, and of a thousand tiles carrying seed not one carried
enough: people move about, and no single tile ever caught up.

**What comes off a carcass now depends on the time of year.** A deer killed at
the end of the autumn carries a quarter more than the book says; the same deer
at the end of the winter carries a third less. The curve runs continuously
round the year - `SeasonalCalendar::how_fat_the_beasts_are` - and is not
straight: an animal loses most of what it is going to lose in the first hard
weeks and puts nothing back in the first weeks of the spring, because running
both as straight lines put a deer in midwinter in the same condition as a deer
in midsummer.

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

**And there is now something past the stone age to find out.** Three steps —
a bright stone held in a fire, a lump beaten out with a hammerstone, a blade
given a handle — are marked as things nobody arrives knowing, and are found out
only by an agent who happens to be holding the makings in the right conditions
while curious enough to notice. Measured over four worlds of ten thousand
ticks: iron reached agents' packs in all four (36 to 55 of them carrying some),
two worlds worked out what a fire does to it, two got as far as a metal blade,
and one finished a metal knife. So metalworking is a thing that happens to some
settlements and not others, which is what it should be, but it is rare enough
that a run has to be watched for it rather than expected.

It buys nothing measurable yet. Population is 80.0 alive at the baseline (se
4.8) and 70.2 across the sixteen worlds run with the chain and the tools (se
4.6): a difference of 9.8 at 6.6, which is noise with a hint of a decline in
it. That is roughly what the specification asks for — "everything should be
slow and inefficient", "wood and stone tools should wear out quickly" — but it
means the multipliers are only worth having if a people can keep a stock of
tools, and nothing yet makes one ahead of needing it.

### 10. Farming is learned now, and the plant nobody can reach

Breaking ground used to be something every founder was born knowing, and a
field was sown and then forgotten: nothing came on in it, nothing took it, and
what it grew was the same whether anybody went near it again. Three things
changed and were measured over eight worlds of ten thousand ticks each,
against `44f7019`.

**A field goes over if nobody works it.** Weeds and vermin come on in ground
that is growing something, at a rate that takes an unattended field to about
half overrun in a season and right over in three. What they leave is what the
farmer gets, down to a tenth of the crop. Going round the field is an action
with a cost and a skill behind it; a practised hand gets round three times as
much of it in a turn as a beginner. Over eight worlds the settlements worked
their fields 388 times each on average — an action that did not exist before —
and held cultivated ground at 0.15 of overrun.

**Nobody is born believing in it.** Farming is a `Practice` now, like spreading
muck: an agent breaks ground out of curiosity until something proves it works,
and two things prove it. One is standing in your own field and seeing a crop in
it. The other is the midden — a people that voids the pips of what it eats in
one place walks past a season later to find the same plants standing in its own
refuse, and whoever is within six tiles takes the lesson. At ten thousand ticks
about 45% of a living settlement (mean 31 of 69) has farming as settled
practice and about 70% have some opinion of it, where before the number was
not defined because everybody simply farmed.

**Population did not move.** 60.9 people at ten thousand ticks before (se 8.3),
68.8 after (se 8.9): a difference of 7.9 at a standard error of 12.2, which is
noise. Fields broken went 81 to 91. What is standing on broken ground at any
moment went the other way, 743 to 392, for the reason in the next paragraph.

**The plant nobody can reach.** What goes in the ground is what is in the pack,
and of what is in the pack the crop the agent's own record rates best. Grain
carries three times what the ground would otherwise and a berry bush in rows
is still a berry bush, so this is the mechanism by which a people finds out
which plants are worth sowing — and in eight worlds it never fired. Every field
in every world was sown with berries. A default world places six wild grain
patches against twenty-five of generic food, foraging takes the nearest edible,
and grain therefore almost never reaches a pack at all. The suitability
machinery is built, tested and idle. Making it fire means putting more wild
grain in the world, which is a world-generation change with its own measurement
to do, not a line to tune here.

Partly answered since, by accident rather than by design: grain that gets wet
sprouts, and a sprouted grain that falls out of a pack takes root — so grain
patches now propagate, and eight worlds finished with 12.9 of them against 7.8.
That is more grain in the country than a world starts with, and it still is not
enough for grain to be the thing a settlement reaches for. See #12.

The first cut of the sowing rule let an agent sow anything in its pack. Over
eight worlds the people put in flax and cotton — they carry it for clothing —
and the food standing on the map fell by ninety per cent while they farmed
linen. A field broken to answer hunger now only takes something a person can
eat.

Also found on the way: `Gather { resource_type: "grain" }` was not a request
the executor understood. It fell through to "unknown resource type" and failed.
Grain only ever arrived as an edible substitute for a request for food.

### 11. What an action wants in the hand was never asked in one place

Thirty-odd actions, each with an executor arm that resolved its own target its
own way and checked its own preconditions its own way or not at all. `Cook`
looked for a fire. `Craft` looked for a fire only when the step wanted one.
`Gather` consulted a tool for a multiplier and then proceeded bare-handed if
there wasn't one. `TendField` asked for nothing. There was no answer to "what
does this verb want in its hands", because nothing ever asked.

There is a table now — `src/environment/verbs.rs` — carrying sixty-eight verbs
across the twelve families, each declaring what it targets, what it wants in
hand (bare hands, a hand free, any tool for a trade, or one named thing), what
it changes, and which action performs it. The executor asks the table before
every action, so the requirement is declared once and enforced once. Two things
are enforced that were not: a hunt wants a spear — the specification's own
"hunting = spear + animal" — and stitching wants a hand free.

The table is honest about what it does not do. Twenty-five of the sixty-eight
verbs have something performing them; forty-three are declared and idle, and
the whole chemical and fluid family is declaration. A test fails if the table
stops saying so, because a matrix that quietly implied sixty-eight working
verbs would be worse than no matrix.

Three of the idle ones have since been built, and building them showed what
the matrix is worth: smashing, cutting and scraping are a table of workings
that say what turns into what, and not one word about what they want in the
hand — the verb says that, and the executor enforces it without knowing
anything about stone. Over eight worlds against `c6218ae` the settlements
smashed 866 cores and scraped 901 sticks apiece, spears carried went from 33
to 65 because a struck flake is half the stone of a raw core, fires standing
went from 37 to 43, and about two thirds of a living settlement worked out
what shavings are for. Population 85.5 to 79.3, a difference of 6.3 at a
standard error of 7.6.

Cutting hides into leather fires almost never — nought to twelve times in a
world — because hides are scarce in a pack and what there is goes straight
into clothing. The verb works; the material does not reach it.

**Handing things over**, which also settles the barter mechanism asked for a
long way back and never delivered. A trade wants an abundance on both sides,
each of which the other is short of. A gift wants only one, and costs the
giver, and is worth more to the bond because it leaves somebody owing. What
either counts as wanting is the raw stuff every step and every working asks
for, minus what is in the pack.

Over eight worlds a settlement gives 328 times apiece and barters once or
twice. That is not a mechanism failing to fire: a people that gives freely has
little left to bargain over, and generalised reciprocity is what a band of
forty who all know each other actually runs on. Population 71.3 to 74.1, mean
bond 0.74 to 0.79.

The first cut measured abundance against a number — six of a thing on one side
and fewer than six on the other — and a settlement traded once in eight worlds
of ten thousand ticks. Abundance is a comparison, not a threshold: what makes
a thing worth handing over is that they have markedly less of it than you do.
The same cut also made a gift require a match on both sides, which is what a
trade is and not what a gift is.

**Things lying on the ground**, which the manipulation verbs needed and the
world did not have. A thing was either in a pack or nowhere. Nothing could be
put down and taken up again, and when somebody died everything they had
carried went out of the world with them — so an axe existed for exactly as
long as its owner did, and a people that spent a season making them had
nothing to show for it the morning after the man who made them drowned.

A pack falls where its owner does and stays the thing it was: a worn axe on
the ground is a worn axe when the next person picks it up. Food left lying
goes into the soil in a few weeks and everything else weathers away in a
season and a half, so a world does not silt up with everything anybody ever
put down. Over eight worlds against `566f18d` settlements stooped for
something 45 times apiece and finished with thirteen things lying about, two
or three of them tools. Population 68.1 to 64.4, a difference of 3.7 at a
standard error of 9.9 — noise, and so is everything else measured.

One ordering mistake worth recording: scavenging first went ahead of
everything else the Utility drive does, so a man who could have made a spear
out of what was in his pack walked twelve tiles for a stick instead. It
belongs beside going out to fetch a thing, which is what it is a substitute
for, and not ahead of making one.

**A throw parts you from the spear.** Half the throws that miss put the shaft
on the ground somewhere out past where the hunter was standing, and it is a
spear again as soon as somebody walks over and picks it up — which is what
makes a missed throw cost more than the walking, and which the ground store
above had to exist before it could. Over eight worlds against `25903eb`
settlements finished with two or three spears lying in the bracken and the
number of times anybody stooped for anything went from 39 to 79. Hunts 95 to
108. Population 68.4 to 69.1.

**And some verbs are not decisions.** Nobody chooses to get a spear between
himself and a wolf; it is what happens when the wolf arrives and there is a
spear in his hand. The matrix carries that kind now — `happens_when` beside
`done_by` — so a verb the world performs is not filed alongside the ones
nothing performs at all. A spear turns about half of a blow and an axe about a
third, and both are the worse for it. The effect is not separable at the
settlement level: mean health was 93.5 before and 93.6 after, because
predators reach very few people in a world.

**Looking closely at a strange thing**, which turned out to be the piece the
discovery chain had been missing all along. The deepest steps — the shiny
lump out of a fire, the blade beaten out of it, the knife and the axe hafted
from that — were reported twice in this file as built, tested and idle,
because the only ways in were repeating a trick you had already stumbled on
and putting the wrong thing where a part goes, and both of those want you to
have got there first.

Turning over a thing you are already carrying costs a turn and no materials,
so it is the cheapest experiment there is and pays off least often: six per
cent, scaled by the hand doing the turning. Over eight worlds against
`1d1d863` the agents who know what a bright stone does went from 0.9 a world
to 15, those who know what to beat one into from 0.1 to 5.5, and metal tools
existed in seven worlds of eight where in every measurement before this they
existed in none. Population 67.9 to 67.3.

The first cut let an agent look at anything. Examining a length of cord
announced the metal knife, because a metal knife happens to be lashed
together, and 30 to 44 agents a world "worked out" steps they had no business
reaching. A thing that is already part of something everybody understands
raises no questions, however much else it goes into; only something outside
all of that does.

**A basket, a bowl and a handful of flour** finished the reducing verbs.
Weaving is the one of them that wants nothing in the hand, which is a thing
the matrix is the right place to say. A basket raises what a pack holds;
carried weight went from 61 to 68 over eight worlds, and settlements finished
with 59 baskets apiece.

The bowl is the more interesting one. `Inventory::fill_containers`,
`InventoryItem::new_container` and the whole business of drinking from what
you are carrying were written long ago, and nothing in the world had ever
made a container — so an agent could only ever drink where the water was.
Settlements now finish with 84 bowls between them, and twenty to fifty
agents a world have worked out how to hollow one. Whether it shows in thirst
is not yet clear: agents going dry sat at 51 before and 49 after, which is
noise.

Crushing grain into flour is built, tested and idle, and idle for the reason
already recorded in #10: grain barely reaches a pack in this world, so
nobody has three of it to grind. Population 67.5 to 71.3.

**And then the fluid family**, which was entirely declaration until there was
something to hold water in. Soaking, fermenting and boiling all want a vessel
with something in it, and the order those things had to come in is why that
family sat idle in the matrix while every other family got built.

Flax left in water lets go of its fibre and gives three times the cordage: the
cordage a settlement carries went from 31 to 46 over eight worlds, and about
eighteen agents a world work out how to ret. Fruit and water left alone turn
into something that keeps a fortnight where berries keep hours, and a pot of
flour and water over a fire is bread — whole grain improves in the embers and
ground grain turns to ash, a distinction the food tables already drew and
nothing had ever used. Population 66.6 to 79.0, a difference of 12.4 at a
standard error of 9.8.

Fermenting fires about twice across eight worlds and the whole grain branch —
crush, then boil — not at all, both for reasons already recorded: berries are
eaten as fast as they are picked, and grain barely reaches a pack. Grain
scarcity now blocks three verbs rather than one.

**The order of a table was deciding what a whole people ever found out.**
Curiosity offered the first unknown working whose materials were to hand, and
retting sits above fermenting in the list, so as long as anybody had flax
nobody ever fermented anything. Where a man starts in that list is his own
business now, drawn off his own name.

Three of my own tests turned out to be asserting deterministic outcomes from
random gates, and only failed in full-suite runs where the seeds differ:
whether two strangers trust each other enough to trade is drawn with their
traits, whether a pack has room for what a test hands it depends on what else
is in it, and `Lessons::will_try_this_again` is a roll by design — it is what
stops anybody doing the same thing for ever.

Measured over eight worlds against `1b9aa40`: population 73.3 to 77.4, a
difference of 4.1 at a standard error of 5.9. What did move is the spread —
standard deviation from 15.4 to 5.9. Agents dressed 47 to 50, spears carried
45 to 50, knives 33 to 40, hunts 93 to 104.

Two wrong turns on the way, both caught by measuring rather than by reading.

**Sewing wanted an edge.** True of sewing and false of this economy: stone
knives wear through faster than a people replaces them, so a requirement for a
knife is one most people cannot meet most of the time. Over eight worlds it
took the agents who finished dressed from 47 to 23 and drove clothing attempts
from 774 to 5,694, almost all refusals. Sewing wants a hand free instead —
still a real requirement, and one the economy can carry. What a knife is worth
to the work is what it always was: how well the garment comes out.

**A pack is not a pair of hands.** The first free-hand rule counted a hand as
full for every kind of tool in the pack, so a man who owned an axe and a spear
had no hands at all and could never stitch again. That made it worse, not
better: clothing attempts went to 8,913 and the people finished in their
shirtsleeves anyway. A tool is carried and taken out when it is wanted. What
actually leaves somebody with nothing to work with is being loaded to the limit
of what they can carry, and that is what the rule measures now. It bites on one
or two agents in a world, which is about right.

### 12. Four accidents that teach farming, and two that hardly ever happen

Farming had one route in and two teachers. It has four of each now, and none of
them is anybody's idea about agriculture — which is the point, because nobody
gets ideas about agriculture before there is any.

**Grain gets wet and stops being grain.** A pack carried across a marsh, along
a riverbank, or through a downpour on open ground has grain coming up in it.
What sprouted works its way out of the pack, and what falls on ground that can
carry it grows where somebody happened to be standing. Whoever is within six
tiles of that learns what seed does. Over eight worlds the grain patches
standing at the end went from 7.8 to 12.9.

**Somebody moves the bush instead of walking to it.** A person who walks half a
morning to the same berry patch lifts a slip of it and puts it in beside the
tents. It is not a theory about growing things, it is an opinion about the
walk. This turned out to be the strongest of the four by a long way: eight
worlds lifted 269 slips apiece and put 193 of them in, and the food standing on
the map went from 1,319 units to 5,526.

That last number wants watching. The first cut took three units off the parent
plant, left the parent as big as it was, and grew into a plant carrying forty:
transplanting was not moving food about, it was manufacturing it, and the map
carried six times what it had. A slip now comes off the parent's carrying
capacity rather than only off this year's crop, and grows into somewhat more
than it cost rather than thirteen times. Four times the standing food is still
a large change for one mechanism and it is what a people that plants things
would do.

**Population did not move.** 70.5 at ten thousand ticks before, 75.0 after: a
difference of 4.5 at a standard error of 7.2. Fields went 93 to 101 and the
share of a living settlement with farming as settled practice went from 34 to
43 of about 70.

**Some plants are things nobody has tried, and hardly anybody ever does.** Four
sorts grow in a world; which are supper is drawn when the country is made and
written nowhere anybody in it can read. A curious agent with nothing pressing
walks over and eats one, and it costs him between a bad afternoon and his life.
Everybody standing round him learns it for nothing. Over eight worlds this
happened three times in total, in three different worlds — twice the plant was
food. It is built, it is tested, and it is barely exercised: curiosity rarely
wins a turn against everything else a person wants, and sixteen patches in a
hundred-tile country are not often underfoot.

The first cut of it never fired at all, in eight worlds of ten thousand ticks.
Two things were wrong: it asked the agent to be standing exactly on the plant,
and it rolled the odds again on every tick of the walk, so a small chance was
compounded against itself until nobody ever arrived. The roll is made once now,
to set out.

**Putting the wrong thing where a part goes** fires in about half of worlds and
has never yet produced anything. The good substitutions want a metal blade in
hand, and a metal blade is three discoveries deep already — so what actually
gets tried is a hide where the flax goes, over and over, by people who have
hides. The mechanism is sound and the tree above it is too tall for ten
thousand ticks.

Found on the way: a discovered thing nobody can make again is not a discovery.
A metal axe existed only as the outcome of a substitution and had no step in
the table, so the man who found one could never deliberately make a second.
Both new tools are steps now, marked as things nobody is born knowing, and the
substitution is how you come to know them.

### 13. There is no camp for nomadism to be a departure from

A people that cannot farm has nothing it can do to make this ground carry
more, so it should go where the ground already carries something, and a people
that can farm should stop. That decision now exists: an agent with no
established farming practice, standing on ground carrying less than four units
a head within foraging reach, with somewhere three times better between twenty
and sixty tiles off, picks up and goes. A standing crop on broken ground nearby
cancels it, and so does knowing how to farm.

It is built, it is tested, and over sixteen worlds it changed nothing
measurable. The camp walked 1,301 tiles in ten thousand ticks before and 1,331
after. Net displacement of the centroid was 28 tiles either way. Population
73.1 before and 64.6 after, a difference of 8.5 at a standard error of 6.5 -
consistently negative across two batches and not significant in either.

The reason is the finding. There is no camp. A settlement in this model is not
an entity with a location that people belong to; it is however many agents
happen to be standing near each other, and each of them is already dragged
across the map by its own foraging. They walk thirteen hundred tiles a run
whatever they believe about farming, and they finish about fourteen tiles from
each other's centre. Making the ground under a people something it can decide
to leave means first making the people a thing that is somewhere, which is a
larger piece of work than a decision function - a settlement with a seat, a
hearth the camp follows, and a way for a move to be agreed rather than taken
one agent at a time.

The first cut of the rule asked for an absolute standard of good ground -
twenty-five units a head - which no ground in the world meets for a settlement
of any size. It fired every tick of every life. Foraging fell forty per cent,
the food standing on the map went up four and a half times because nobody was
eating it, the camp ended up no further from where it started, and it cost
about twelve people. What replaced it is relative: somewhere three times better
than here. That stops the moment the camp arrives, because it is then standing
on the best ground it knows of.

### 14. Agents still cannot hear anything

Sight discovers terrain, resources and buildings, and agents now see one
another — `vision.visible_agents` is populated each tick, which is what
observational learning is gated on. Hearing is unfed entirely, so every
sound-derived percept is still a dead path. See SIMULATION_AUDIT.md.

### 15. Zoning and territory are never established

Building placement scoring reads zone and territory bonuses from
`World::zone_manager` and `World::territory_manager`, but nothing outside the
tests ever calls `add_zone` or `claim_territory`. Both managers are therefore
always empty in a live run and every bonus they contribute is zero, so
settlements have no planned structure and agents claim no ground.

### 16. Agents carry food they will never eat

Food that has turned is correctly refused, but stays in the inventory until
its freshness decays to zero and spoilage removes it — or until the agent takes
it onto a field and tips it out, which some of them work out for themselves. The same is true of food
an agent burns: a novice cook ruins about one batch in five, and the ruins ride
along in the pack. Both announce themselves as a decay scent to anyone nearby,
which is realistic and mildly useful, but nothing makes the carrier drop them:
carried weight still includes rot and cinders.

### 17. Personality exists and reaches the drives, and still decides nothing

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

### 18. Skill measured how far you had walked, and bought nothing

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

### 19. Running away taught agents they could win a fight

Fleeing and theft were the last two verbs in the matrix that the world was
already doing under other names, or not doing at all.

**Running had no name.** Flight went out as an `Action::Move` like any other
walk, so the matrix could not tell a bolt from a stroll, it cost what a stroll
costs, and — worse — an agent who had escaped four wolves had no record of
having escaped anything, because `learn_from` had nothing to file it under.
Running is its own verb now: further in a turn than a walk, `WHAT_RUNNING_COSTS`
in energy, refused if there is nothing to land on, and recorded.

**And filing it under fighting cost the settlement dearly.** The first cut sent
`FleeFrom` to `Undertaking::Fighting` on the reasoning that standing your ground
and running are two answers to one question. They are, and they are not one
lesson. `what_fighting_has_taught_me` scales an agent's estimate of what it can
beat by its own record, so every successful escape raised the estimate: a man
who had outrun four wolves went and picked a fight with the fifth. Over eight
worlds it showed up as 97 attacks a settlement against 35 before the change —
nearly three times as many fights, from a change that was supposed to be a
rename. Getting away is `Undertaking::Fleeing` now and the count went back to
39.8, which is baseline within the noise. A test asserts that a dozen
successful escapes leave a man exactly as confident about a fight as he was.

This one is worth keeping in view because of how it was found. Nothing about
the change looked behavioural — it named a thing the world was already doing —
and the only reason the regression surfaced is that the batch was measured
against a baseline anyway.

**Theft is built and almost never chosen.** Somebody with nothing more pressing
to do, standing next to somebody he does not think well of who is carrying
something worth having, takes half of it; the victim and everybody within six
tiles knows who did it, and it costs the bond and the trust rating in
proportion to the share taken. Honesty makes it rarer, greed and an empty
stomach make it likelier, and every onlooker divides the odds.

Over eight worlds of ten thousand ticks it happened once. The reason is not a
bug in the decision: mean bond strength across a settlement is 0.78, the bar
for not robbing somebody is 0.4, and so the filter that excludes people you
think well of excludes nearly everybody standing near enough to rob. A band of
forty who all grew up together has no strangers in it. The machinery is there
for a world that produces them — a second settlement, an outsider, a season bad
enough to drive the trust down — and this world does not.

**Everything else held.** Population 71.4 at ten thousand ticks against 71.4,
with the spread between worlds narrowing from a standard error of 9.0 to 4.3.
Standing crop 5,080 against 4,573 (t = 1.4). Deaths 19.0 against 16.0.

### 20. A free hand was a question about the pack, and nobody ever had one

The verb matrix has said since it was written that some actions want a hand
free — stitching a coat, lashing a haft, weaving a basket — and until now
there was nothing in the model that could answer the question. Two cuts were
tried and both were wrong. The first counted a hand as full for every kind of
tool in the pack, so a man who owned an axe and a spear had no hands at all
and could never stitch anything again. The second asked whether the pack had
spare carrying capacity, which was written down at the time as a fudge.

It was a worse fudge than it looked. A settlement lives at or over the limit
of what it can carry — measured mean load 70 against a capacity of 50 — so on
that rule nobody in the model ever had a hand free for anything, and every
action the matrix said wanted one was being quietly refused for the whole
population. Removing that single test, changing nothing else, takes a
settlement from 65.6 people at ten thousand ticks to 79.8 (se 4.7 and 3.5).
That is the largest single effect measured in this project, and it came from
deleting a line.

**So the hands are real now.** Two of them, holding named things. Taking a
tool out is `Action::Equip` and costs a turn; it is chosen not when somebody
has a spare moment but in the moment before the work, because the first cut
put it at the bottom of the Utility chain where it fired half a time in a
world of ten thousand ticks — there is always some material wanting fetching,
so nothing ever reached it. A job that wants a hand free and finds both full
becomes `Action::Unequip` rather than a failure. A hand is reconciled against
the pack every tick, because everything an agent loses — given away, stolen,
worn through, eaten — leaves through the inventory, which knows nothing about
hands.

**And what a tool in the bag is worth had to be measured twice.** The first
cut said two thirds of one in the hand, which sounded modest and was not: a
settlement owns four or five tools and a person can hold two, so most work is
done with something fetched out of the bag, and taxing all of it a sixth cost
the settlement a quarter of its standing crop (4,777 to 3,420) and 40 per cent
of its tools (110 to 65). At nine tenths both recover — crop 5,139, tools 91 —
and the settlement lands at 72.8 against the 79.8 of the fix alone. That gap
is 0.8 standard errors and its sign is what you would expect: the model got
more honest about tools and the people got slightly poorer for it. What is
bought with it is that `equip`, `unequip`, `hold`, `use` and `carry` stop
being declarations.

**Carrying costs something now as well.** It cost nothing at all before: a man
walked as easily under sixty pounds of stone as under nothing, which made a
full pack pure gain and a basket a thing with no downside. Nothing up to two
fifths of what the arms will hold, rising to about twice the energy a step at
the limit.

### 21. Nobody carries food, so nobody can go without it

The specification asks for agents that can sacrifice themselves for their
family and loved ones. Two forms of that were built. One works and one has
never fired.

**Standing in the way works.** A wolf within two paces of somebody this agent
loves, who could not fight it themselves, brings the agent at it regardless of
the odds — which is the point, and the only place in the model where an agent
knowingly takes the worse of two options. Everything else in the
fight-or-flight tree is about picking the better one, and a parent with a wolf
standing over their child is not picking anything. Tests hold both halves: the
same man with four wolves at his elbow runs when the child is elsewhere and
goes at them when it is not, and he does not lay down his life for a grown man
who could fight them himself.

**Going without has no occasion.** The other form — handing over food you are
going to want yourself, to somebody of your own who will not last the week —
is built, tested, and fired zero times in eight worlds of ten thousand ticks.
It is not a bug in the decision. Probing the settlement directly, at ten
thousand ticks:

| | world 1 | world 2 | world 3 |
|---|---|---|---|
| alive | 69 | 70 | 59 |
| carrying any food | 5 | 3 | 4 |
| starving | 0 | 0 | 0 |

Sampling every twenty-five ticks over a whole run, only 262 agent-samples out
of roughly 28,000 had so much as a meal in the pack, and on not one of those
occasions was a bonded neighbour standing next to them starving — because
nobody is ever starving. Everything gathered is eaten within a few ticks;
there is no larder and no scarcity for a sacrifice to be a sacrifice against.

Two things would give it an occasion, and neither is in this batch's scope: a
reason to carry a meal rather than eat it where it was found, and a winter bad
enough that somebody goes short. The machinery is there for a settlement that
has either.

### 22. A herd of deer counted as a pack of wolves

Introduced by the change immediately before it, and worth writing up because
of how it happened rather than because of what it cost.

Once several of a thing began adding up rather than only the worst of them
counting, the question of *what counts as a thing* stopped being harmless.
The filter was `attack_damage > 0.0`. A rabbit has an `attack_damage` of 1.0
and a deer of 5.0, because both will defend themselves if you pick them up;
reindeer travel in groups of five to twenty. So a herd of reindeer standing in
a field came to about a wolf, and the settlement spent its days running from
its own dinner.

What menaces somebody who has done nothing to it is a thing that comes after
people. What merely defends itself is a question for whoever attacks it. The
model already carried `AnimalBehavior` and nothing had ever consulted it:
Passive now counts for nothing at all, Neutral a quarter, Defensive two fifths,
and Aggressive and Territorial the whole of it.

Over twenty-four worlds: fleeing 465 times a settlement against 213 (se 76 and
66), and freezing 194 against 27 — most of that last being children hemmed in
by deer, which is the freeze branch firing for entirely the wrong reason.
Population, standing crop and burials are unchanged.

**A correction.** The commit that introduced this reported that hunting had
fallen from 91 a world to 44 and called it the price of agents being properly
afraid. That does not survive a larger sample: at twenty-four worlds hunting
sits at 51.9 before the fix and 52.3 after, and re-measuring the earlier
baseline gave 69.8 where the first run had said 91.2. The drop was mostly the
noise in an n=8 comparison and it was over-read. The bug above is real
regardless — a herd of reindeer registering as a pack of wolves is wrong
whatever it does to the hunting figures — but it was not costing the
settlement its dinner.

### 23. The larder is built, and a settlement that never starves has no use for one

Issue #21 above found that not one agent in sixty-five was carrying food and
not one was starving, so there was no occasion for anybody to go without for
their family. Half of that has a cause that could be built: there was nowhere
to put food. `Agent::what_i_can_spare` explicitly excluded anything anybody
eats, and the only place to put anything was `World::storehouse_inventory` — a
single global bag of counts with no position that nothing ever spoils in.

**So a settlement can dig now.** A pit takes a stone tool and a real morning's
work (22 energy, the most expensive single act in the model). Food goes in,
the earth goes back over it, and while it is covered its ageing clock is held
back on three ticks in four, so it keeps four times as long as it would in a
pack. Hunger looks in the nearest store before it walks out to a berry bush.
That makes `excavate` and `cover` live verbs and gives `Preparedness` a way to
want food, which it never had.

**Two rounds of measurement each found a real defect.**

*The first cut put food by all year.* A settlement dug and foraged for a
larder in the middle of summer with berries on every bush: 351 gathering trips
a world, and ten fewer people (76.9 to 66.4, t = -2.1) for the effort. Nobody
puts food by in June. Deliberate foraging for the store is now autumn work; a
genuine surplus already in the hand is buried whatever month it is.

*The second asked for a pit wherever somebody was standing.* The decision
checked whether a pit was within fourteen paces; the executor checked whether
the ground would take one. Between them a settlement made a hundred attempts a
world and finished with 1.7 pits — ninety-eight turns spent trying to dig a
hole in a lake. The decision asks the ground first now, and attempts fell to
fifteen.

**Where it ends up, over sixteen worlds against sixteen:**

| | before | after |
|---|---|---|
| people at ten thousand ticks | 76.9 ± 3.9 | 69.7 ± 3.5 |
| pits standing | 0 | 1.4 |
| units in the ground | 0 | 11.6 |
| carrying food | 2.5 | 3.6 |
| starving | 0.06 | 0.00 |
| went without for family | 0.5 | 0.0 |

The machinery works and it does not pay. Seven people is not significant at
this sample (t = -1.4) but the sign is the same across all three arms, and the
reason is plain in the last two rows: nobody was starving before and nobody is
starving now. A store is insurance, and this world has nothing to insure
against. Until a lean season can actually bite — a winter that strips the
land, a crop that fails — a larder is a cost with a theoretical benefit, and
the sacrifice that #21 wanted still has no occasion.

That is the thing to build next, and it is upstream of both.

### 24. Food was on a clock a hundred and twenty times too slow

This is the root of #21 and #23 and of a good deal else, and it was a units
mistake.

Every spoilage time in `FoodDatabase::register_all_foods` was written as a
day-count and stored as a number of ticks at 1440 ticks to the day. The
calendar was later put on a scale a life fits inside — `TICKS_PER_DAY` is 12,
a season is twenty-four days, a year is 1,152 ticks — and the food tables were
not brought with it. The comments still say what the author meant:

| | written as | actually lasted |
|---|---|---|
| meat | "1 day raw" | 120 days — 1.25 years |
| fish | "0.5 day — spoils very fast" | 60 days |
| berries | "1.5 days" | 180 days |
| grain | "10 days — lasts long when dry" | 1,200 days — 12.5 years |
| ale | "14 days" | 17.5 years |

Nothing in this world spoiled. Everything downstream followed: nobody ever
went hungry (#21), a larder was insurance against nothing (#23), and six of
the nine `PreparationState` variants — Dried, Smoked, Salted, Pickled,
Fermented — had never once been reached by anything, because there was no
reason to preserve anything. `set_preparation` had no caller at all.

**The first cut used the day-counts as written**, and that was wrong in a
different way. A tick on this calendar is an *action*, not a minute: an agent
gets twelve of them in a day, and walking out to a berry patch and back is
thirty or forty. Food that lasts two days lasts less than the trip that
fetches it. Nobody ever held a surplus, so nothing was ever dried or buried —
digs, burials and stores all went to exactly zero — and the settlement lost a
fifth of its people for nothing. The times are on the scale of the season
instead, which is the unit a store is actually against: meat ten days, berries
twelve, grain sixty.

**What it costs, sixteen worlds against sixteen:**

| | before | after |
|---|---|---|
| people at ten thousand ticks | 66.2 ± 4.9 | 52.4 ± 7.0 |
| food dried or smoked | 0 | 3.6 |
| food buried | 7.1 | 23.8 |
| still in the ground at the end | 6.9 | 0.1 |
| carrying food | 2.8 | 0.5 |
| hunts | 53.6 | 10.4 |
| burials | 18.5 | 20.8 |

A fifth of the population, at t = -1.6. It is worth being plain that this is
the price of correcting a bug rather than the bug itself: food that never
rots was wrong, and the user asked three times for it to decay. But two of
those rows point at structural things the clock only exposed, and neither is
fixed here.

**Hunting fell five-fold and it is not the learning system.** Probing a
settlement directly: forty of forty-seven living agents still believe hunting
pays, and there were thirty-five attempts in ten thousand ticks. Hunting sits
last in the Hunger chain, behind `food_action`, and `food_action` almost
always finds a berry bush. Agents are hungry far more often now, so Hunger
wins the drive contest far more often, and every time it does the chain stops
at the nearest bush. A permanently hungry people forages and never hunts,
which is the opposite of what hunger should do. The chain wants restructuring
and that is its own piece of work with its own measurement.

**Nothing stays in the store.** Twenty-four burials a world and a tenth of a
unit still in the ground at the end. Only 3.6 of those burials were of dried
food, so most of what goes in is raw berries at half a season, doubled by
bare earth to exactly one season — a store laid down in autumn is empty by
spring. Lining a pit with a bowl would double it again, and agents almost
never have a bowl to spare.

Also worth recording, because it wasted a measurement round: winding a food's
`created_tick` backwards to age it faster is a silent no-op for anything
created at tick zero, because `saturating_sub` on a `u32` at zero does
nothing. Weathering is counted on the `Dropped` record itself now.

### 25. A settlement that can pick fruit in the snow

The last three batches all built machinery against a scarcity this world has
never produced. This is the reason: growth was seasonal from the beginning and
what was *standing* was not. A berry bush that had grown all summer still had
its berries on it in February.

Every edible thing bears in its own season now and carries nothing outside it.
Spring gives wild leaf and shoot — `ResourceType::Greens`, almost no energy in
it and a great deal of what a body needs a little of, which is exactly what
somebody who has lived on stored grain all winter is short of. Summer gives
the first roots and pods, which is not a harvest. Autumn is when everything
else comes on at once. Winter gives nothing whatever. Things that do not grow —
stone, water, a standing tree — bear all year, because they are not bearing at
all.

**It works, and by a wide margin.** Standing edible food in winter, averaged
over sixteen worlds:

| | before | after |
|---|---|---|
| standing edible in winter | 3,849 ± 148 | 492 ± 64 |

t = -21, which is the largest effect measured anywhere in this project.

**Two rounds of measurement each found a real defect.**

*Fruit hung on the branch for three months.* The first cut shed a twentieth of
what a plant carried each pass, which left 472 units of berries on bushes in
midwinter — most of a season's crop still on the branch in the snow. That is
not a lean season, it is autumn with worse weather. A quarter a pass leaves a
hedgerow four fifths bare within five days of the season turning, which is what
fruit does.

*And the larder was in a deadlock.* Digging a pit wanted a surplus in hand;
gathering a surplus for the store wanted a pit to put it in. Neither could
happen first, and while food was abundant this never showed, because somebody
was always carrying a few spare berries. The moment the land actually went
bare, burials fell from 10.8 a world to 1.8 — the store stopped being used
exactly when it was worth most. Autumn with nowhere to put anything is reason
enough to dig, and digging went from 2.3 attempts a world to 40.

**What it costs, and what it did not fix.**

| | before | after |
|---|---|---|
| people at ten thousand ticks | 48.8 ± 8.9 | 34.3 ± 3.8 |
| starving in winter | 0.2 | 0.0 |
| burials | 20.4 | 16.4 |
| food dried or smoked | 0.0 | 3.1 |
| pits dug | 6.3 | 40.1 |
| still in the ground at the end | 0.0 | 0.0 |
| shelters built | 0 | 0 |

The settlement is a third smaller and *nobody is hungrier* — starvation in
winter is zero in both arms and burials are down. It is not dying of the
change, it is equilibrating smaller: fewer surpluses, so fewer births.

And the store still never holds anything. Forty pits a world get dug and not
one of them has anything in it at the end. Pits are dug now and filled from
whatever somebody happens to be carrying, which is exactly the thing seasonal
food removes. Nothing in this model ever gathers *for the winter*; it gathers
because it is hungry, and then puts the leftovers away. That is the last link
missing, and it is upstream of the larder, of the sacrifice in #21 and of the
"efficiency" question — a people that spends its whole year finding today's
dinner never builds anything, which is why `shelters built` is zero in both
columns and always has been.

### 26. Nothing ever gathered for the winter

The last four batches all ended at the same wall: the store was built, the
scarcity was built, and the store stayed empty. Forty pits a world dug and not
one of them with anything in it. This is why, and it was three deadlocks in a
row rather than a number that wanted tuning.

Probed directly in autumn: of 3,254 agent-samples, **108 were carrying any
food at all** — three in a hundred. There was never a load to carry home,
because nothing in this model gathered *for the winter*. It gathered because
it was hungry, ate what it picked in the same breath, and put away whatever
happened to be left over.

**First: putting by waited on farming.** `Preparedness` stood behind
`Sustenance` in the drive chain, on the reasoning that a people puts by what
it grows. It does not — a people puts by what it *finds*, and it has been
doing that far longer than it has been growing anything. Behind Sustenance,
Preparedness could not build until food production was answered, and food
production is never answered in a settlement that forages. Measured: it sat
below its threshold in eight agents out of eight, at values of 0.00 to 0.14
against thresholds of 0.26 to 0.40, for the whole of a settlement's life. It
now waits on Hunger and Thirst — on being neither hungry nor parched today,
and on nothing else.

**Second: a harvest is not supper, and nothing knew the difference.** `Hunger`
is a primary drive and wins every contest it enters, so the instant an agent
had food in the pack it ate it. Carrying food past your own mouth is what
provisioning *is*, so it has to sit above Hunger — as a preempt, alongside the
one that has a parent go without for a starving child. In autumn, with a store
within reach and no real hunger pressing, what is in the pack is a harvest: it
does not get eaten and it does not get cooked, because cooking a thing stops
it being dried and drying is worth twenty times what cooking is. Once the load
is worth carrying it goes home.

The first cut of that guard used the desperation line — the same 0.85 that
decides whether somebody will rob a neighbour — and that is far too late. It
had agents carrying food past their own mouths until they were nearly done
for, and burials went from 13.8 a world to 17.9. Being a bit peckish is the
price of eating in February; being hungry is not.

**Third: the keep-back ate the harvest.** `Cover` kept back three days' food
before burying anything, which is nonsense when you are standing on your own
larder — you can take more out tomorrow, that is what it is for. A settlement
living hand to mouth rarely holds three of anything, so `Cover` was refused
**1,513 times out of 1,525** for want of anything to bury. It keeps back one
meal now.

**What it does, sixteen worlds against sixteen:**

| | before | after |
|---|---|---|
| units in the ground through winter | 0.0 ± 0.0 | 42.4 ± 4.3 |
| pits standing | 2.1 | 10.0 |
| burials | 4.3 | 86.4 |
| food dried or smoked | 2.8 | 666.5 |
| pits dug | 34.2 | 72.4 |
| spears made | 48.2 | 38.4 |
| people at ten thousand ticks | 33.3 ± 4.9 | 29.1 ± 3.5 |
| burials of people | 13.8 ± 1.2 | 17.1 ± 1.5 |

The store holds through the lean season for the first time (t = 10), and every
part of the chain that had been built and never used — the pit, the covering,
the drying, the smoking — is now routine.

It costs something and the cost is the thing worth looking at: **spears made
fell from 48 to 38**. That is the efficiency trade showing up on its own. A
settlement that spends its autumn laying in stores makes a fifth fewer tools
for it, which is exactly what a time budget ought to do and what could not be
seen while nobody provisioned at all. Burials of people are up 13.8 to 17.1
(t = 1.7, not significant at this sample) and population is down four (t =
-0.7); both are consistent in sign and neither is established.

### 27. Nobody was born knowing how to dry a fish, and it showed

The preservation states were reachable from the last batch, and the way in was
`Action::Dry`: an agent decided to preserve a thing and the thing was
preserved. That is a rule handed down from nowhere. Nothing in this world had
ever *shown* anybody that laying food out keeps it, and the weather — which is
the thing that actually does the drying — had no opinion about food at all.
Everything lying on the ground aged at one flat penalty whatever the sky was
doing.

What the weather does now depends on what is under it. Rain rots. Sun dries a
thing thin enough to dry through and ruins a thing that is not: a whole fish
in the sun goes off, and the same fish opened out and cut into strips dries.
Berries, greens, grain and roots dry as they are. Shade is the middle case and
still costs something, because nothing keeps outdoors.

That makes the discovery possible, and the discovery is the point. `cut fish`
and `cut meat` are obvious workings — anybody with an edge works out that a
fish comes apart — and worth exactly nothing on their own. The value is
entirely in what happens afterwards, which nobody can predict and everybody
can watch. An agent carrying more than it can eat, with no store within reach
and a clear sky, puts it down; that is an ordinary thing to do and it is the
beginning of every preserved thing this people will ever own. When the world
converts something from raw to dried, everyone within six paces is told, the
same way the four routes into farming work. And `Action::Dry` is now gated on
having seen it: an agent that has never watched food dry cannot choose to dry
food.

**Leaving that last gate out cost more than half the store.** The first cut
let anybody choose `Dry` while only the executor checked the discovery, so
agents spent turns on an action that came back refused. Sixteen worlds a side
against the same baseline: winter store **41.9 → 17.5 (t = -2.9)** — worse
than doing nothing at all. Putting the same check inside the decision, where
it belongs, turned it round:

| | before | after |
|---|---|---|
| units in the ground through winter | 41.9 ± 8.1 | **84.3 ± 8.9** |
| burials of food | 83.4 ± 16.0 | 498.1 ± 72.7 |
| agents who know what drying is | 0.0 | 4.3 ± 1.2 |
| fish and meat cut into strips | 0.0 | 14.2 ± 3.8 |
| food deliberately laid out | 0.0 | 1.8 ± 0.6 |
| `Dry` attempted | 582.5 ± 102.6 | 42.8 ± 13.8 |
| people at ten thousand ticks | 25.8 ± 4.0 | 27.9 ± 3.3 |
| burials of people | 17.7 ± 1.1 | 18.1 ± 1.7 |

The winter store doubles (t = 3.5) and burials of food go up sixfold (t =
5.6), on a population and a death rate that do not move (t = 0.4 and 0.2).

**The `Dry` line is the interesting one and it went the other way on purpose.**
Deliberate drying fell by more than nine tenths, because 582 attempts a world
was almost entirely agents trying to dry things they had no idea how to dry.
The preserving now happens in the weather rather than in a turn, which is both
cheaper and the right place for it: what an agent contributes is cutting the
fish up and choosing to leave it somewhere sunny. That number also counts
watching somebody else's food dry, so the deliberate share of the remaining 43
is smaller still.

**What is still missing.** Salting is written, tested and unreachable, because
there is no salt anywhere in this world — no resource, no deposit, no
evaporation. Fermenting is reachable only through a recipe nobody chooses.
Rain rots things at the same rate whatever the intensity, so a downpour and a
drizzle are the same event. And nothing yet distinguishes food under a roof
from food in the open: the shade case is a constant rather than a question
about where the thing is lying.

### 28. Agents ate two-kilo lumps of raw beast, and nothing in this world could make anybody ill

Two questions, and the answer to both was no.

**"How are agents eating meat? Are they cooking it first? Can they just absorb
an entire side of beef?"** They ate it raw, in two-kilo lumps, with nothing in
the way. `eat_food_item` removed exactly one unit per `Eat`, and a unit off a
carcass is two kilos. The only gates were `is_harmful` and `is_spoiled`.
Cooking was worth 2.7 times the nutrition — 0.35 raw utilization against 0.95
cooked — and nothing else at all, so a fire you had to fetch wood for was a
convenience rather than a necessity. Nothing anywhere required a knife, and
piece size affected nothing: a whole deer dried as fast as a strip.

**"Eating raw meat, spending time near dead bodies or fresh waste, and eating
spoiling food should have a chance to cause sickness."** There was no illness
in this project at all. The only health consequence anywhere in it was a flat
ten damage for eating something past `is_harmful`, taken in one tick and over
with. A settlement could live on raw flesh and sleep in its own midden and
never know the difference.

**What was built.** `nutrition::Piece` reads how big a thing is off its own
name — whole, a joint, a strip, or already the size of a mouthful. A whole
beast can be neither eaten nor put over a fire; a joint can be both. Everybody
is born knowing a carcass comes apart, but knowing it is not the same as
having an edge to do it with, and the decision checks for one so that choosing
to cut without a knife does not spend the turn. Strips come off a joint rather
than off the animal, dry in two days where a joint takes most of a week, and
twice as many of them fit over a fire.

Illness is a state that lasts days rather than a hit that lands. Raw flesh
tells about one meal in twelve; food between 0.3 and 0.5 freshness tells more
often the further gone it is; a day on fouled ground tells one time in twenty
at the worst. A corpse now fouls the ground it falls on, which is what makes a
body a thing to be away from rather than a nutrient deposit.

**Salt exists.** `PreparationState::Salted` had been written, tested and
unreachable for the whole life of the project because there was no salt in the
world. There are now three new grounds — sea, salt marsh and salt flat — with
salt on the flats and in rare seams in the hills, and boiling the sea for it
when a people has neither. Sea and marsh water is a drink that costs more than
it gives: it slakes the thirst on the tick and raises it for days after.
Everybody knows better than to touch it, and nobody who is dying of thirst
does.

**Sixteen worlds a side, against the commit before:**

| | before | after | t |
|---|---|---|---|
| carcasses cut into joints | 8.3 | **357.6** | 10.8 |
| joints cut down into strips | 0.0 | 2.3 | 4.5 |
| food salted | 0.0 | 68.3 | 9.3 |
| the sea boiled for salt | 0.0 | 161.8 | 6.7 |
| people who know what drying does | 5.1 | 27.9 | 8.0 |
| illnesses | 0.0 | 4.8 | 7.8 |
| people at ten thousand ticks | 33.5 | 33.0 | -0.1 |
| burials of people | 15.1 | 15.6 | 0.3 |
| **times anybody cooked** | **284.4** | **111.8** | **-4.4** |
| units in the ground through winter | 97.3 | 71.1 | -1.8 |

Population and deaths do not move. What it costs is **cooking, down by
three fifths**, which is the efficiency trade showing up again: turns spent
quartering a deer, boiling the sea and rubbing salt in are turns not spent at
the fire. Winter stores are down a quarter and that is *not* established at
this sample (t = -1.8).

**Four rounds of measurement, and the third found something worth writing
down.** The first cut of the portioning work correctly refused to dry a whole
fish — and the winter store collapsed by 63%, because *a settlement's entire
preservation output had been drying whole fish*, which the specification says
should rot. The honest route was blocked behind a fourth circular
precondition of the same family as the three the provisioning work turned up:
you had to have seen food dry before you would lay any out, and the only route
to seeing it sat behind an autumn gate and two pit branches and effectively
never ran. Laying food out now comes before burying it, for anybody who has
not yet learned what it does; once they have, the drying branch catches it
first and this goes quiet.

The fourth round found the ordering error that mattered most: with all four
preservation branches ahead of burying, a settlement spent some two thousand
turns a world cutting, boiling, salting and drying and put a *third* as much
in the ground as it had before any of it existed. Every mechanism working, and
the settlement worse off. Burying a thing is one turn and it is what actually
gets food to February; preserving is several and only pays if the food is
somewhere it will keep. Burying goes first now.

**What is still wrong.** Preserved food does not accumulate: agents hold 0.56
units of dried or salted food through winter, which is nothing. What gets
preserved gets eaten rather than kept, because nothing in `find_best_food_to_eat`
prefers raw food over a thing somebody spent three turns making keep. That is
the next thing to fix and it is why the store has not recovered.

### 29. Clay was in every world and nobody could pick any of it up

`ResourceType::Clay`, `Pottery` and `Bricks` were three enum variants with
nothing whatever behind them. Clay had been spawning on every riverbank and
every marsh in every world since the project began, and no agent could ever
touch any of it: `"clay"` was missing from the vocabulary `Action::Gather`
answers to, which is the only vocabulary it has.

That vocabulary turned out to be **two** vocabularies, in two places, that had
drifted. The decision layer maps a request string to a `ResourceType`; the
executor maps a `ResourceType` back to an item name, and its table ended in
`_ => "generic"`. Greens and roots have been going into packs as `"generic"`
since the day they were added, three batches ago. There is one table now.

**Nobody is handed pottery.** There was also no reason for anybody to gather
clay even once it was possible, because every material in the chain is
gathered by somebody who already wants the thing it makes — and nobody can
want a pot before anybody has made one. Curiosity is the drive for that: a
curious agent within a short walk of a material nobody here has ever done
anything with goes and gets a handful. It is a detour, not an expedition, and
somebody who has tried everything clay does walks past the clay.

From there the chain is three separate things to find out, none of them handed
down:

- **clay holds a shape** (`mold clay`) — no tool, no fire, no water, just
  somebody turning a lump over in their hands. What comes out is worth almost
  nothing: an unfired shape holds nothing and comes apart in the rain.
- **fire stops it being clay** (`fire claypot`) — and what comes out holds
  water, which is the first thing this people can make that keeps something
  else.
- **and clay fired in a block is a brick** (`fire clay`) — a separate
  discovery off the same material and the same fire. A people that has fired a
  pot has not thereby learned to make a wall.

**And there is an accident.** "An agent 'cooks' some clay which causes it to
harden into stoneware." Nobody intends it: somebody is sitting at a fire with
clay in their pack because they picked it up walking past a riverbank, a lump
finds the embers about one day in fifty, and in the morning it is not clay any
more. Everyone round that fire sees it. It is the same shape as the drying
discovery two batches back and for the same reason — a people at this stage
does not reason its way to firing clay, it notices that firing has happened.

`MOLD` had sat in the verb matrix since the matrix existed with nothing
carrying it out. `FIRE` is new, and had to be: `heat` was already spoken for
by `Craft`, and holding a thing in a fire until it stops being what it was is
not the same act as warming it on the way to making something else.

### 30. An agent could be mauled at a ford and go back the next morning

The map an agent carries had explored tiles, resource positions with an age
and a source, buildings, storage and terrains — a real picture of the world's
*things* — and nothing whatever about danger, and nothing about people. There
was nowhere for "there are wolves in that wood" to live, so nobody could know
it.

Danger goes on the map now, with the same discipline the resource knowledge
already had: it has a place, a name, a time and a strength, and it **fades**.
A pack works a wood for a season and then moves on, and a man who avoids that
wood for the rest of his life is not being careful, he is being wrong — so a
fright is gone entirely after a season, and a bad place is three tiles wide
either way, because "there are wolves in that wood" is not a fact about one
tile. Nobody carries more than thirty-two of them about.

What goes on it is what somebody would actually notice: everything in sight
that means them harm, **taken together**. Together rather than one at a time,
because that is what the specification said a threat was — "a man encountering
4 wolves should see them as a threat" — and judging each wolf separately would
have him walk into the pack four times unafraid. One wolf is not much to a man
with a spear; four of them are a different afternoon.

Two callers, so it is not another library with nobody to use it. A patch of
food in a wood where this agent saw wolves last month is **further away than
it measures** — twelve paces further at full strength, fading with the memory —
so a settlement works its safe ground first and its bad ground only when
there is nothing else. And a fleeing agent picks between straight away and a
quarter-turn either side by what it knows of the ground, rather than bolting
headlong into the wood the pack lives in.

**A real ordering bug turned up while testing it.** The sight pass ran at the
end of the tick, after the beasts had moved — and a wolf pack that has just
been frightened off by the man it walked up to is nine paces away by then. The
man never learned there were wolves there at all. Everybody looks round before
the beasts move now.

**Sixteen worlds a side, both this and the clay above:**

| | before | after | t |
|---|---|---|---|
| people at ten thousand ticks | 27.6 ± 3.0 | **38.2 ± 3.2** | 2.4 |
| the most it ever held | 33.3 | 43.2 | 2.7 |
| handfuls of clay gathered | 0.0 | 106.9 | 7.9 |
| clay molded into a shape | 0.0 | 34.1 | 7.1 |
| things put in a fire to harden | 0.0 | 315.6 | 7.4 |
| people who know what a fire does to clay | 0.0 | 22.6 | 8.6 |
| pots standing at the end | 0.0 | 53.1 | 6.8 |
| bricks | 0.0 | 16.2 | 5.8 |
| bad places remembered | 0.0 | 55.3 | 5.1 |
| people remembered | 0.0 | 1793.5 | 9.0 |
| **times anybody ran** | **9015.2** | **131.5** | **-2.3** |
| burials of people | 14.6 | 15.8 | 1.0 |

**Population is up by a third, and the reason is the last line.** A settlement
was spending nine thousand turns a world running away — three per cent of
every turn anybody took, each one costing fourteen energy on top of the turn
itself. Agents that avoid the bad wood while choosing where to forage do not
have to run out of it. That was not a number anybody had looked at, and it was
the largest single waste left in the model.

The clay chain is doing real work at the same time and the two cannot be
separated at this sample; both are in one commit and the population figure is
the two together.

### 31. Three things that were meant to fix the store, and what each actually did

Three of the open issues, taken together, and the results are not uniform. They
are written up separately because two of them worked and one of them is a null.

**Hunting had been put behind everything, and now is not.** It sat behind
eating what you carry, behind foraging, behind walking to a known patch, behind
moving the whole camp, behind walking back to ground that fed you once — and
then behind being *desperate* on top of all that. It was never reached.
Measured before this, forty agents in forty-seven still believed hunting paid
and none of them had ever done any, which is what a belief with nothing to
update it looks like.

The rule that makes sense of it is narrow on purpose: **a deer at your feet
beats a berry patch twelve tiles off.** Not a deer across the valley — that is
the expedition that does not pay, and measured long ago it starved two
settlements in forty. Five paces, and only when there is nothing to pick up
where you stand, and only for somebody the lessons have not put off it.

Hunting attempts went from **6.8 a world to 148.1 (t = 5.0)**. Population and
burials did not move (t = -1.1 and -1.0). So it is reached, and it is close to
free — which is the honest reading, not that it is now profitable: hides held
at the end are 0.31 either way. What has changed is that the belief now has
something to update it.

**A drizzle and a thunderstorm were the same event.** `WHAT_THE_WEATHER_ADDS`
was a constant, and the intensity the weather has always reported was thrown
away at the first comparison — `precipitation_intensity() > 0.0`. Shade is the
floor now and the open sky under a downpour is the ceiling, with a drizzle
between them. And food under a roof is a question about where the thing is
lying rather than a constant: a roof keeps the rain off, and — cutting both
ways — stops the sun drying anything under it.

That second half found a live trap in the tests. A default world puts exactly
one building at the middle of the map, which is where several fixtures stand
somebody and drop food, so `whoever_is_standing_near_learns_what_the_sun_did`
began failing 10 runs out of 10 the moment a roof meant something. The code was
right and the fixture was unlucky; the weather fixtures clear the buildings now.

**And the one that did not work.** The reading was that agents ate the food
they had spent three turns preserving: a settlement held 0.56 units of dried or
salted food through a whole winter. The fix is right in itself and is tested —
`find_best_food_to_eat` weighted freshness alone, which is exactly backwards
for a people with a store, and now weights by `spoilage_multiplier` too, so a
dried strip is a twentieth as attractive as today's supper and exactly as
attractive in February when there is nothing else. Agents demonstrably prefer
the perishable thing now.

**It made no difference at settlement level:** preserved food carried through
winter went 0.81 → 0.94 (t = 0.7), and the store went 105.2 → 97.9 (t = -0.7).
The diagnosis was wrong rather than the fix. What gets preserved does not stay
in packs to be eaten — it goes **into the ground**, and the store has been
holding around a hundred units since the ordering was fixed two batches ago.
`carried` was never a measure of the problem. The change is kept because it is
correct and costs nothing, but it should not be described as having fixed
anything.

### 32. Nobody ever built anything, and it was three deadlocks in a row

`shelters built` was **nought in every arm ever measured**, across the whole
life of the project. It was not a number that wanted tuning.

A tent — the only shelter a stone-age people can raise, and itself a fix for
an earlier deadlock where every other building needed thirty stone nobody
could quarry — wants eight wood and four hides. Wood is a walk. Hides come off
an animal and nothing else in this world produces one. And hunting sat behind
six other branches of the hunger chain and then behind being *desperate* on
top of that, so it was never reached. Three things, each one waiting on the
last.

Unblocking hunting (#31) moved shelters from 2.25 a world to 3.56, which is
not significant and not enough. What was missing is the shelter that depends
on none of it: **a hole in the ground with turf over it.** It costs earth and
a morning. It is worse than a tent in every way except the one that matters.

Two things had to be right about it. It is dug rather than built, so it is its
own verb: `build` is *framing* and wants poles in the hand, which is correct
for a tent and nonsense for a hole. There has been a `burrow` verb sitting
dead in the matrix since the matrix existed, wanting something to dig with,
and it is live now — which finishes the subterranean family. And the decision
checks for the digging tool before choosing it, because an action chosen
without what it needs comes straight back refused and the turn is gone.

**Sixteen worlds a side:**

| | before | after | t |
|---|---|---|---|
| burrows dug | 0.0 | **39.6 ± 2.7** | 14.7 |
| shelters standing, averaged over the run | 0.6 | **30.4 ± 1.7** | 17.4 |
| tents | 2.5 | 2.4 | 0.0 |
| exposures anybody is suffering | 0.07 | 0.04 | -1.7 |
| people at ten thousand ticks | 34.8 | 34.8 | 0.0 |
| burials of people | 13.8 | 13.9 | 0.0 |

A settlement lives under a roof for the first time, and what it is worth is
that people are cold about half as often (t = -1.7, directional rather than
established). Population and burials do not move, which is the honest reading:
nobody was dying of exposure at ten thousand ticks, so the roof buys comfort
rather than lives at this timescale. Tents are unchanged — the burrow does not
replace the better shelter, it fills in underneath it.

### 33. An agent could only ever learn about a situation somebody had named

`Lessons` has recorded what works since it was written, keyed on the thing
attempted: `dry`, `gather:greens`, `fire:claypot`, `hunt`. Every one of those
keys was **written out by hand by somebody who had already thought of it**, and
what stood against it was a single flat number.

So an agent could learn *that* gathering food does not pay, and could never
learn that it does not pay *in the spring*. Everything in this model that
depends on when a thing works therefore had to be a rule somebody wrote down:
the bearing year is a table in `src/world/`, sun-drying is a discovery flag,
the fire that fires clay is a precondition checked in the executor. The agents
were never in a position to find out any of it. That is the ceiling this whole
learning apparatus had been sitting under, and it is why the fish-strips-in-
the-sun generalisation had to be hard-wired last time instead of falling out
of the data.

What is there instead is **the circumstances**: ten coarse facts about the
afternoon — the sky, the season, a fire to hand, a roof overhead, water within
a few paces, anybody else about — gathered by the simulation and written down
against every attempt anybody makes. Nobody names the situation. Nothing in
the arithmetic that reads them knows what a season is or what a fire is for.
What an agent works out is which of them go with a thing working, by comparing
its record under one circumstance against its own overall record of the same
thing — so a man who has only ever dried fish in the sun learns nothing
whatever about the sun, correctly, and it takes one wet afternoon to teach him
anything at all.

There are two touch points and that is the whole of it. Every attempt is
recorded with the afternoon it was made in (`learn_from_this_here`), and every
action the drives choose is judged where it stands rather than in the abstract
(`how_this_agent_answers`). Where an agent has worked nothing out — which is
every agent to begin with — the second is exactly the flat belief it was
before.

**What a settlement actually works out.** Nobody wrote any of these down. From
one ten-thousand-tick world, counting how many of about forty people arrived
at each independently:

| what nobody wrote down | people | effect |
|---|---|---|
| gathering food pays in the autumn | 31 | +0.52 |
| gathering food does not in the summer | 34 | -0.36 |
| gathering food does not in the spring | 33 | -0.36 |
| gathering food does not in the rain | 20 | -0.25 |
| gathering food does not in the winter | 18 | -0.27 |
| gathering food pays under a roof | 15 | +0.58 |

The first five are **the bearing year**, which is a table in the world code
that nothing has ever told an agent about, arrived at from experience by five
sixths of the settlement. The sixth is a confound and worth saying so: a roof
is where the camp is and the camp is where the harvest gets carried, and a
correlational learner cannot tell that apart from a roof helping. That is an
honest failure mode of this kind of learner rather than a bug in it.

**Thirty-two worlds a side:**

| | before | after | t |
|---|---|---|---|
| situation lessons worked out, per settlement | 0.0 | **258.3 ± 10.0** | 25.8 |
| ...by the best-placed person in it | 0.0 | **15.9 ± 0.4** | 42.6 |
| food in the ground through the winter | 154.9 | **203.7 ± 12.8** | 2.6 |
| gathers | 8,480 | **11,382 ± 508** | 3.4 |
| people at ten thousand ticks | 31.2 | 35.3 ± 1.3 | 1.6 |
| hunts | 141.0 | 178.4 ± 20.9 | 1.3 |
| burials of people | 13.4 | 13.7 | 0.3 |
| turns refused | 16,462 | **21,493 ± 718** | 3.6 |
| share of turns refused | 7.7% | **9.2%** | 4.0 |

A settlement that has worked out the harvest **gathers a third more and puts a
third more in the ground** (t = 2.6 and 3.4), which is the first time the
winter store has moved past about a hundred and fifty in any measured arm.
Population is up an eighth, directional rather than established. Burials do not
move.

**And it costs a fifth more refused turns** (t = 3.6), which is established and
worth naming rather than burying. The cause is not mysterious. The asymmetric
belief in `Lessons` — a failure counts for 0.10 and a success for 0.06 —
saturates *any* activity below a 62.5% success rate at the floor, and gathering
has always been well below it. So `gather:food` sits at `NEVER_QUITE_GIVES_UP`
whatever happens, the negative lifts have nowhere to push it, and the whole
effect of the circumstances is the autumn lift pushing it up. The settlement
gathers very hard in the harvest, and a good share of those turns find a patch
somebody has already picked out.

That is two follow-ups rather than one. A `Gather` that knew the node was empty
before spending the turn on it would take most of the refusals out; and the
asymmetry in `Lessons` deserves looking at on its own, because a floor that
everything reaches is a floor that carries no information.

**What this does not reach.** The sun-drying case that prompted it is still not
learnable, and the reason is worth writing down: the signal these lessons are
built on is `ActionResult::success`, which in most executors means *the action
was permitted*, not *the action paid*. Laying food out in the rain succeeds
perfectly well; what fails is the food, three days later, somewhere the agent
is no longer standing. Situation-keyed lessons cannot reach a delayed outcome,
and giving them one is a separate piece of work from giving them a situation.

### 34. Nobody knew a patch was bare, so everybody walked back to it

Refused turns, one world, before this:

| why a turn was refused | count |
|---|---|
| Gather: no food sources nearby | 10,127 |
| Gather: inventory full | 5,255 |
| Gather: no generic sources nearby | 2,209 |

**More than half of everything a settlement ever got refused**, and two
separate faults with the same shape.

The map an agent carries knew *what* was at a place and never whether there
was any of it left. `known_resources` is a position and a resource type, and
nothing anywhere held "I stripped that hedgerow on Tuesday". So somebody would
pick a patch bare, walk home, and walk back to the same bare ground the next
morning, and the morning after, for as long as the drive kept asking.

And several of the paths that produce a `Gather` cannot see the world at all.
`generate_action_for_drive` is a static table that answers Sustenance with
"gather food" and Industry with "gather generic" with no notion of whether
there is any food or any wood in the county, or whether the asker has room in
their pack for it.

Three things. A place goes on the map when somebody strips the last of it —
and it is not a private fact, so everybody standing near watches the ground go
bare. It fades after half a season, because a patch picked out in June is
bearing again by September and a man who writes it off for life is as wrong as
the man who goes back every morning. And a gather that could not come to
anything is refused on the way past, so the drive stands aside and the next one
takes the turn, which is the doctrine the decision already ran on.

The vocabulary `Gather` answers to came out of the executor while this was
being done. It had lived where nothing that had to *decide* whether a gather
was worth asking for could read it, which is the same defect that had clay
spawning in every world for a year with nobody able to pick any of it up.

**Sixteen worlds a side:**

| | before | after | t |
|---|---|---|---|
| turns refused | 20,430 | **8,885 ± 1,096** | -7.9 |
| share of turns refused | 9.0% | **3.9%** | -11.1 |
| gathers attempted | 10,026 | **3,513 ± 302** | -8.1 |
| turns taken | 229,811 | 235,768 | 0.5 |
| food in the ground through the winter | 184.5 | 206.1 ± 19.6 | 0.9 |
| people at ten thousand ticks | 34.4 | 37.8 ± 3.3 | 0.9 |
| hunts | 223.5 | 143.6 ± 20.7 | -2.4 |
| burials of people | 13.7 | 15.3 | 0.9 |

**Refusals more than halved** and the failure rate went from nine per cent to
under four. The settlement takes the same number of turns and wastes far fewer
of them; store and population are up and neither is established. Hunting is
down (t = -2.4), which is the one established cost and is what you would
expect: with fewer turns burned on gathers that were going to fail, hunger gets
answered by gathering more often and by setting off after a deer less often.

One number in the previous entry has to be corrected in the light of this.
**The situation lessons a settlement works out fell from 258 to 59.** Nothing
is wrong with the learning; there is simply far less waste left to learn about.
Most of what agents had been working out was *when gathering fails*, and a good
share of the failing was this. A measurement of emergent knowledge that is
partly a measurement of a defect is worth flagging as such.

### 35. Curiosity could not ask a question whose answer arrives later

Curiosity in this model was always the same shape: pick a working nobody here
has tried, do it, and get the answer back in the same turn. That is right for
"what does this lump of clay do if I press it" and wrong for most of what a
stone-age people has to find out, because most of it does not answer for three
days and does not answer where you are standing.

There was one branch that reached for the later kind — putting food down to see
whether the sun keeps it — and it was **gated on the sky being clear**. Which
is to say the code already knew the answer and only let anybody run the
experiment on the days it comes out well. Finding out that meat left in the
rain is ruined is the same discovery as finding out that meat left in the sun
keeps, and a people that can only make the second one has not found anything
out at all.

So: a question somebody has open. What was done, to what, where, when, what it
was like then, and **what the sky was doing at the time** — that last carried
rather than looked up on the way back, because by the time anybody returns the
rain has stopped. Coming back and finding it changed is the lesson. Coming back
a week later and finding it exactly as it was left is also a lesson, and an
important one: it is what stops a man doing the same pointless thing every week
for the rest of his life. Somebody walking off with it ends the question and
teaches nothing, which is right — the experiment was interfered with, not
concluded.

Two things were wrong with the first cut and both were found by measuring it.

It asked each question **once**: an agent that had got a single answer was
marked as knowing. Sixty-five questions a world were asked and answered and
**exactly nought conclusions were drawn from any of them**, because no agent
ever held more than one instance and the pattern arithmetic wants eight. One
answer is one afternoon, and the thing being reached for here is that it
depends on the afternoon.

And it tipped the whole pack on the grass. `PutDown` drops the entire stack, so
a curious man with six fish left six fish out. Measured, that cost an eighth of
the people and a seventh of the winter store — directional rather than
established, but consistent across four correlated measures. An experiment
costs a portion now.

**Sixteen worlds a side, against the entry above:**

| | before | after | t |
|---|---|---|---|
| questions put to the world and answered | 0 | **187.9 ± 9.0** | — |
| people at ten thousand ticks | 37.8 | 35.6 ± 1.8 | -0.6 |
| food in the ground through the winter | 206.1 | 186.4 ± 13.8 | -0.8 |
| share of turns refused | 3.9% | 3.4% | -1.7 |
| burials of people | 15.3 | 14.4 | -0.5 |

A settlement puts and answers **a hundred and eighty-eight questions to the
world** that nobody arranged, and nothing else moves in either direction. The
mechanism runs and costs nothing measurable, which for a new kind of curiosity
is the result to want.

**What it does not yet reach, and the number is worth stating.** The answers
feed two things. The first is the existing drying discovery, which is immediate
and works: somebody who leaves cut fish out and comes back to find it dried has
found out what the sun does, and so has everybody standing near. The second is
the situation record from #33, and there it is **at the edge of the threshold
rather than past it**. The best-placed person in a world accumulates 13 to 20
answers about one thing, against the eight-per-circumstance the pattern
arithmetic wants for a contrast; situation lessons drawn from wondering appear
in about **one world in three**, held by one person. Widening the look from two
paces and four days to five paces and a week took the best single record from 4
to 20 and the answers from 65 to 188, and that is as far as this goes without
either more curiosity turns or somebody telling somebody else what they found.

### 36. Nothing had ever counted the waste

The point of preserving anything is that the time spent getting it was not
wasted. **If half the meat rots before it is eaten then half the hunt was
wasted** — the hours are gone either way and only one of them fed anybody, and
an hour spent hunting is an hour not spent doing anything else.

Nothing in this project had ever counted that. Every preservation change for
the last dozen entries has been judged on how much was *in* the store, which
is a measure of activity rather than of whether the activity was any use. Food
goes off in three places — in a pack, in a pit, and where it lies — and all
three simply deleted it.

Counted now, in all three. And the number, which nobody had:

| | |
|---|---|
| food eaten, per settlement per ten thousand ticks | 3,382 |
| food that rotted first | 1,135 |
| **share of what was got that was any use to anybody** | **74%** |

**A settlement throws away a quarter of everything it acquires.** That is the
yardstick this whole line of work should have been measured against from the
beginning, and it is the one to hold the next batch to.

### 37. Only leaving a thing out was a question, and nobody could tell anybody

Three things, and they belong together.

**The other verbs.** Burying and salting are questions like leaving a thing on
the grass is a question, and the answer arrives days later in the same way. The
catch is that **the verb has to decide what counts as a good answer.** A thing
left on the grass that is exactly as it was left a week later teaches nothing:
nothing came of leaving it there. A thing *buried* that is exactly as it was
left a week later is the entire point of burying it. Getting that backwards
would have taught a settlement that its own larder was useless — which is the
same efficiency argument as #36 read from the other end: no rot is the win,
because rot is the wasted half of the hunt.

Each question now knows where to go and look, too: burying puts a thing in a
hole, salting leaves it in the pack — so that one travels with its owner and
never wants a walk back — and only leaving it out puts it on the grass.

**Firing.** Working clay is immediate in this model: the pot comes out of the
fire in the turn it went in, so it was already a same-turn experiment and not
this kind of question at all. What was missing is the version that *is* one — a
lump left lying at a lit fire is not a lump of clay in the morning. That was
already in the model as an accident that happens to somebody carrying clay; it
is a thing anybody can deliberately do now, and clay is the one material in
this world worth leaving somewhere to see what becomes of it.

**And telling.** Nothing anywhere let a man who had worked something out *tell*
anybody. Everything in this model is found out first-hand or watched being
done, so a settlement of forty could work the same thing out forty times over
and be no further on than the first man who worked it out.

Somebody carrying a thing you have never seen the like of is worth asking
about — under Curiosity, which is to say only when nothing worse is pressing,
because a man does not stop to ask after somebody's supper while his own
children are hungry. They have to actually understand it: a man holding dried
meat who has never dried anything cannot tell you how. And what passes between
them is the *name of the discovery*, not a belief — which means being told lets
the hearer go and try it, and what happens when they try it is what decides
whether they believe it. That is the whole difference between being told a
thing works and finding out.

**Thirty-two worlds a side:**

| | before | after | t |
|---|---|---|---|
| questions put to the world and answered | 196.4 | **661.3 ± 24.3** | 17.8 |
| discoveries passed from one head to another | 0.0 | **249.5 ± 11.0** | 22.8 |
| people at ten thousand ticks | 37.1 | 39.0 ± 1.5 | 0.9 |
| food in the ground through the winter | 186.9 | 181.0 | -0.4 |
| burials of people | 14.1 | 13.2 | -1.1 |
| share of turns refused | 3.4% | 3.5% | 1.0 |

Questions asked and answered **more than trebled**, and a quarter of a thousand
discoveries a world now pass from the head that made one to a head that needed
it. **Nothing else moves.**

That null is worth understanding rather than shrugging at, and the reason is in
the table above it. The settlement's one load-bearing discovery is that laying
cut food out keeps it, and **it was already reaching essentially everybody** —
36.6 people out of 37.1 alive, before any of this. Watching somebody else's
fish dry has saturated that channel for as long as it has existed. Telling is
redundant for the only thing worth telling.

So the channel is built, it works, and it will be worth something the first
time this people has a discovery that is *hard to witness* — one that does not
announce itself to everybody standing nearby. There is not one yet. Sixteen
worlds a side had population, store and burials all leaning favourably; at
thirty-two all three came back to nothing, which is the honest reading and the
reason for running the second sixteen.

### 38. A deer bigger than a man could carry was silently deleted

`Inventory::add_item` enforces the weight limit and returns `false`.
Butchering called it and **ignored what it returned.** So a kill that came to
more than a hunter could carry did not get left in the field — it stopped
existing, every time, counted nowhere, and invisible to the waste ledger built
one commit earlier. A hunter walked away from part of an animal and the world
behaved as though the animal had been that size.

What will not fit stays where it fell now. It can be come back for, it counts
against the hunt when it rots, and it is there for something else to find.

Which makes carrying capacity the quiet third term in the whole preservation
argument, and the interesting half of this entry. Rot is the wasted half of a
hunt; so is a carcass left in a field because it would not fit in the pack.
**Drying takes the water out, and water is most of what meat weighs** — so
dried meat is a third of the weight of the meat it was, and a hunter who dries
a kill before walking home carries more of the animal home. Preserving buys
carrying capacity as well as time, and they are the same thing seen from
different ends. Salting buys the keeping and not the carrying, because salt
puts back about what it draws out; the two preserving verbs stopped being
interchangeable.

And a leather bag holds rather more than a flax basket. It costs an animal and
a leatherworker, which is the point: carrying capacity is what this people is
shortest of, and being good at something ought to buy you more of what
everybody is short of.

**Thirty-two worlds a side:**

| | before | after | t |
|---|---|---|---|
| food actually eaten | 3,308 | **3,917 ± 152** | 2.56 |
| **share of what was got that fed somebody** | **73%** | **77%** | 2.98 |
| meat left in the field rather than deleted | 0.0 | 46.3 ± 15.8 | 2.9 |
| people at ten thousand ticks | 37.0 | 39.9 ± 1.4 | 1.4 |
| food in the ground through the winter | 186.8 | 206.7 ± 9.8 | 1.2 |
| food lost to rot | 1,144 | 1,175 | 0.5 |
| burials of people | 13.6 | 14.5 | 1.0 |

**A settlement eats an eighth more than it did and wastes a quarter less of
what it gets.** The mechanism is the weight of dried food rather than the
leaving-behind: the deletion was real but rare, at forty-six units a world
against three and a half thousand eaten. Population, store and burials all lean
favourably and none of them is established.

**A measurement mistake, recorded because it nearly went into this document.**
The first run of this comparison reported meat left behind at *645 units a
world*, which would have made it the largest waste in the model. It was
nonsense twice over. A `cd` into the baseline worktree persisted across
commands, so `cargo build --example` rebuilt the baseline and left the
project's harness binary stale — **the entire "after" arm measured the commit
before the change.** And the stale binary was still printing an older column,
so the number in that slot was the count of questions asked, which happens to
sit around 660. Two independent harnesses disagreeing by fifty-fold is what
caught it. Neither would have been noticed from one.

### 39. Where the food actually goes, and it is not where anybody looked

Broken down across twelve worlds, food lost per settlement per ten thousand
ticks:

| where it went | units |
|---|---|
| rotted in the pits | **537** |
| rotted where it lay | 438 |
| rotted in somebody's pack | 231 |

**The larder is the biggest single source of waste in the model.** The store
that a dozen batches of work went into filling loses more food than anywhere
else: it gets filled and is not drawn down, and buried food ages out over ten
thousand ticks even at the quarter rate a covered pit gives it.

Which means every "winter store" headline in the entries above — and there are
several — has been measuring a stock that quietly loses about half of itself.
That is not a reason to distrust those measurements, which were all comparative,
but it is a reason to stop treating the size of the store as the goal. The goal
is food in somebody, and the store is only a means to it.

Not fixed here. It is the next thing to look at, and it wants its own batch:
either people draw on the larder far too rarely, or the pit's rate is wrong, and
telling those apart is a measurement rather than a guess.

### 40. Nothing in this world had ever wanted a vessel

`what_i_would_make` asks only after **tools** — something to hunt with, to cut
wood with, to work a hide with. A carved bowl and a fired pot both declare what
they hold, and neither was ever made by anybody on purpose, because nothing
anywhere wanted one.

Which cost three things at once. No agent could carry water, so every drink was
a walk to the river and back. `Boil` was refused for want of something to hold
the sea in **250 times a world**, so salt was effectively out of reach. And the
whole fluid family — built deliberately in an earlier batch *because vessels
existed* — has been inert ever since.

Two things were wrong underneath it, and both were older than this batch.
Carving a bowl wanted **discovering**, where weaving a flax basket is obvious
and hollowing out a block of wood is no greater a leap; a people that carves a
spear can hollow a log. And `WHAT_A_FIRED_POT_HOLDS` was set to **exactly what
a carved wooden bowl holds**, with a doc comment directly above it reading "a
little more than a carved wooden bowl". The comment was right and the number was
wrong, so firing clay bought a people nothing over carving wood and there was no
reason on earth to bother with pottery.

The other half of the entry is **making the trip pay**. "I am going here or
doing this action anyway — is there anything I can do which decreases the time
to satisfy a drive without detracting from the current one?" The trip out is the
expensive part and the load is nearly free, so somebody standing on a salt flat
takes what they can carry rather than what they need today. Three conditions,
each doing work: it has to be **underfoot**, because the premise is that the
walk is already paid for; it has to **keep**, so there is no sense carrying home
a fortnight of berries; and the agent has to hold **less than a working stock**,
or everybody spends their life at a woodpile.

**Sixteen worlds a side:**

| | before | after | t |
|---|---|---|---|
| **burials of people** | 14.6 | **11.4 ± 0.7** | **-2.66** |
| salt held | 0.6 | 5.1 ± 2.6 | 1.7 |
| meat left in the field | 52.6 | 8.5 ± 7.1 | -1.6 |
| food actually eaten | 3,472 | 3,519 | 0.1 |
| share of what was got that fed somebody | 74% | 74% | 0.4 |
| people at ten thousand ticks | 37.9 | 35.6 | -0.6 |
| things that hold water, per settlement | 11.9 | 11.4 | -0.2 |
| `Boil` refused for want of a vessel | 250 | 252 | 0.0 |

**One established result: a fifth fewer burials.** Everything else is a null,
and that includes the thing this entry is named after. **The vessel half does
not reach the field.** Vessels per settlement did not move and boil refusals did
not move, so salt is still mostly out of reach and water is still mostly a walk.

The diagnosis, as far as it goes: `what_i_would_work_on` takes the **first**
thing in the working table it can do and stops, and carving a bowl sits late in
that table, so anything earlier with materials to hand wins the turn. That is
the same trap that once had a whole people's discoveries decided by the order of
a list, and which `what_working_i_would_try_out` fixed for itself by starting
each agent at a different place in the table. The obvious next move is to do the
same here — and it is a *next* move rather than this one, because it was tried
in this batch, made things measurably worse, and was reverted rather than kept
on a hunch.

**Two self-inflicted regressions, both caught by measurement.** The first cut
put the vessel and top-up branches at the **head** of the provisioning branch,
where an agent that wanted a bowl and had nothing to carve with returned a
refused `Work` every turn instead of burying, drying or storing anything: the
winter store halved (t = -5.0), food eaten fell 38% (t = -3.7) and the failure
rate tripled (t = 6.6). **A branch that can refuse must never stand in front of
branches that cannot.** The second was subtler: with the vessel branch in place
but ungated, "nothing in hand that is any use for Crafting" became the single
largest refused action in the model at **1,739 turns a world**, all of them
spent asking to carve a bowl bare-handed.

**And a third instance of the vocabulary defect this project keeps producing.**
`ItemType::Salt`, `Greens` and `Roots` all exist — salt has had a trade value of
twelve since the economy was written — and none of the three was in
`id_to_item_type`, the one table that turns a thing in a pack into a thing the
world can price or store. So the moment agents actually started holding salt,
they were refused when they tried to put any by, **666 times a world**. That is
the third time a table has drifted from the vocabulary beside it, after the
gather words and the executor's own list.

### 41. The order of a list, a hide that wanted cutting, and the thing that was actually wrong

Three things, and the third is the one that matters.

**The order of a list.** `what_i_would_work_on` took the *first* thing in the
working table it could do and stopped, so whatever sits early in that table and
has materials to hand won every turn, for every agent, for ever. This exact trap
had already been found and fixed once, in `what_working_i_would_try_out` —
retting flax sits above fermenting fruit, so over eight worlds nobody ever
fermented anything, because somebody always had flax — and the fix was never
carried across to the function beside it. Each agent starts at its own place in
the table now.

Worth noting how it had to be done: the starting place is worked out *before*
the belief is consulted. `will_try_this_again` is a coin toss, so folding it in
first changes the list's length from turn to turn and a man's trade changes with
it. Where he starts is his own and fixed; whether he can be bothered today is
not.

**A hide that wanted cutting.** Taking a flint to a hide removes the hair and
turns skin into leather; *cutting* a hide gets you two smaller hides. It is
`scrape` now, which is what leatherworking is. And sewing a bag out of the
leather afterwards is crafting rather than leatherworking — the skill sits one
step earlier, on the scraping, and putting it on both steps paid a man twice for
one trade. What gates the bag is the material, not the hand.

**And the thing that was actually wrong.** The previous entry said the block on
vessels was that carving a bowl sits late in the working table. **That diagnosis
was wrong**, and the fix for it measures a clean null. What was wrong is this,
counted directly in one world of thirty-one people:

| | |
|---|---|
| people who wanted a vessel *and could make one* | 26 |
| people holding the two wood it takes | 28 |
| people owning anything to carve with | **4** |

`WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO` — the list of trades a person wants to be
equipped for — held Hunting, Woodcutting and Leatherworking, and **nothing
anywhere else in the model ever wanted a tool.** So no Crafting tool was ever
made on purpose; the four who had one had made a knife for skinning, which
happens to serve both. The same for Mining: "nothing in hand that is any use for
Mining" was six hundred refused turns a world at the digging alone.

**Thirty-two worlds a side, all three changes together:**

| | before | after | t |
|---|---|---|---|
| things that hold water, per settlement | 10.9 | 14.1 ± 1.3 | 1.8 |
| food lost to rot | 1,159 | 1,095 | -0.9 |
| burials of people | 14.8 | 13.8 | -1.1 |
| share of what was got that fed somebody | 72% | 75% | 1.3 |
| `Boil` refused for want of a vessel | 262 | 253 | -0.3 |
| people at ten thousand ticks | 35.7 | 35.8 | 0.0 |

**Nothing is established.** Vessels are up 29% and that is the nearest thing to
a result at t = 1.8; rot, burials and the used-share all lean the right way and
none of them reaches. Two rounds of sixteen were run rather than one, because
the first sixteen had vessels at t = 1.95 and that was worth checking rather
than reporting.

So the honest state after two batches on this: **a settlement still does not
make vessels**, water is still a walk, and salt is still mostly out of reach.
What has been established is why *not*, which is worth having and is not the
same as a fix. The residual is that agents do not make stone knives even now
that a pair of hands wants one for carving — the want reaches the list and does
not reach the pack — and that is a question about `what_i_would_make` and the
step chain under it rather than about any of the three things changed here.

### 42. A want that reached the list and not the pack

Of thirty-two people: **twenty-seven wanted something to carve with, thirty-two
knew how to make one, and five owned one.** The want was there, the knowledge
was there, and the knife was not.

`Action::Craft` was refused almost never — 110 taken and 0 refused in a whole
world. It was simply **never attempted**. `what_i_would_make` (the tool a man
wants) and `what_i_must_find` (the material that tool wants) both sat behind
`what_i_would_work_on`, which is "work any material I happen to be holding into
whatever it makes". That is undirected and nearly always answerable, so it was
the answer every single turn: 1,896 workings against 110 crafts.

**Two things came out of trying to fix it, one kept and one reverted.**

**Kept.** `what_to_do_first_knowing` checks the *materials* and nothing else, so
it will name a step wanting a hammerstone in the hand of a man with none, or one
wanting a fire where there is no fire. With the ordering changed so the branch
actually ran, that became **2,378 refused crafts a world out of 2,719
attempted** — 1,421 "wants a handaxe" and 957 "no fire burning here". A refusal
is worse than a wasted turn, because it goes into the record and teaches a man
that making knives does not work. There is a `what_to_do_first_that_can_be_done`
now that asks the fuller question, and the tool-getting-out machinery has been
taught about a making's `wants_in_hand` — the matrix could not express it,
because it is keyed on the verb rather than on the recipe, so a man who owned
the hammerstone and had not got it out was refused every time.

**Reverted, and this is the useful part.** Putting the directed wants ahead of
the undirected working — "being equipped comes before pottering", which sounds
obviously right — cost a settlement **two thirds of its vessels** (11.4 to 4.3,
t = -4.6) and put its rot up (t = 2.1). Trying it the other way round, with the
vessel branch at the very head, changed nothing: still 4.3.

The reason is worth writing down. **The pottering is where bowls come from.**
Carving a bowl is a *working*, not a *making*, so the undirected branch is the
only route to a vessel anybody actually takes. Demoting it did not redirect
those turns to something better; it deleted the thing they were producing. That
is a general lesson about this decision tree and not a fact about bowls: a
branch that looks like idle behaviour may be the sole producer of something, and
the way to find out is to move it and count what disappears.

**Sixteen worlds a side, with the ordering reverted and the proposals kept:**

| | before | after | t |
|---|---|---|---|
| **crafts refused** | 68.6 | **0.0** | — |
| food lost to rot | 1,126 | 966 | -1.3 |
| things that hold water | 11.4 | 12.2 | 0.3 |
| crafts attempted | 209 | 152 | -1.2 |
| people at ten thousand ticks | 33.6 | 36.3 | 0.6 |

**Every wasted craft turn in the model is gone** — exactly nought in all sixteen
worlds against a mean of 68.6, which is a mechanical result rather than a
statistical one and is why the t is not worth quoting. Nothing else moves, and
nothing is harmed.

What is *not* fixed is the thing the entry is named after. A settlement still
holds three or four knives between thirty-odd people. The chain is clean now and
still shallow: crafting sits fourth of five in the trades a pair of hands wants,
behind hunting, woodcutting and leatherworking, and a stone knife is three steps
deep. Most people never get past a spear.

## Housekeeping

### 43. Committed backup file

`src/analytics/mod.rs.backup` is checked into the repository.

### 44. Build warnings

15 warnings on `cargo build`, all unused variables and imports. `cargo fix`
handles most.

### 45. Placeholder package metadata

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
