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

### 1. Nineteen tests fail intermittently

**Three of the twenty that used to be on this list were not flakes at all.**
`water_is_not_used_up` failed 12 times out of 12 and
`honest_agents_do_not_end_up_accused` 8 out of 8, on the commit that found them
and on the one before it: standing, reproducible failures filed as flakes and
left alone, each with a real defect under it (#46, #47, #48).
`test_resource_clustering` really was intermittent, at 28%, and had a real
defect under it too (#49). All three are fixed and none of them belongs here.

The moral is worth keeping: **a test on this list is a claim that nobody has
reproduced it, not a claim that it cannot be.** Three of twenty had never been
run more than a handful of times on their own.

    world::tdd_tests::spatial_planning_tests::test_minimize_travel_time_from_agent_position
    analytics::tests::agent_building_integration_tests::test_production_building_placed_near_resources
    analytics::tests::agent_building_integration_tests::test_production_chain_buildings_cluster
    analytics::tests::agent_building_integration_tests::test_different_building_types_use_appropriate_strategies
    analytics::tests::clothing_tests::a_cold_agent_ends_up_dressed
    analytics::tests::working_tests::nobody_works_more_than_they_have_a_use_for
    analytics::tests::situation_tests::a_settlement_works_things_out_that_nobody_wrote_down
    analytics::tests::keeping_it_tests::a_deer_at_your_feet_beats_a_berry_patch_a_walk_away
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
    analytics::tests::table_order_tests::the_same_man_reaches_for_the_same_thing

`a_deer_at_your_feet_beats_a_berry_patch_a_walk_away` was found the same way
and characterised the same way: **0 failures in 20 runs here and 1 in 20 on the
commit before**. It is about hunting, and the batch it turned up in touched
nothing to do with hunting, which is exactly why it was worth twenty runs a
side rather than a shrug.

`a_settlement_works_things_out_that_nobody_wrote_down` was found in a
full-suite run while the scarcity work was going on, and characterised against
it before being believed: **2 failures in 40 runs here and 1 in 40 on the
commit before**. Indistinguishable, so it is a pre-existing flake that had
never been caught rather than anything the food changes did — which was worth
forty runs to establish, because thinner food is exactly the sort of thing that
would leave a settlement with less to work out.

`test_minimize_travel_time_from_agent_position` was re-characterised while
checking whether the spring work had broken it, and its recorded rate is a
considerable underestimate: **10 failures in 20 runs here and 7 in 20 on the
commit before**, which is a coin toss rather than a one-in-fifteen. Both arms
are equally bad, so it is not a regression, but it is much the worst offender
on this list and it is the one to fix first.

`being_told_lets_you_try_it_rather_than_making_you_believe_it` was the single
failure in the suite run for the flee fix, and characterised the same way:
**3 failures in 20 runs here and 1 in 20 on the commit before**. Not
distinguishable at that sample, and the change it was checked against touches
nothing an agent does with somebody else's word.

`the_same_man_reaches_for_the_same_thing` turned up in the suite run for the
deposit-reporting work, which changes what a man keeps in his head and could
plausibly change what he reaches for - so it was worth the forty runs.
**3 failures in 20 here and 3 in 20 on the commit before**: identical, and an
undocumented flake rather than a regression.

`a_settlement_works_things_out_that_nobody_wrote_down` was re-characterised
during the material-fetching work, which changes what a settlement spends its
turns on and could plausibly change what it works out: **6 failures in 20 here
and 4 in 20 on the commit before**. Indistinguishable, and its recorded rate
above - 2 in 40 - is a considerable underestimate. It belongs beside
`test_minimize_travel_time_from_agent_position` among the bad ones.

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

### 43. The larder was four years deep and everything past the first winter rotted

Entry #39 measured where a settlement's food actually goes and named the store
as the biggest single leak: **537 units a world rotted in the pits**, against
438 on the grass and 231 in somebody's pack. It closed by saying the question
was open — "either people draw on the larder far too rarely, or the pit's rate
is wrong, and telling those apart is a measurement rather than a guess."

This is that measurement. It is the first, and it is not close.

**What is in the pits.** Almost all of it is *dried* food in *lined* pits —
the best this model can do, a twentieth the spoilage rate and a bowl between
the food and the ground. It is not the wrong food and the rate is not wrong.

**How much is in them.** A pit takes 300 and a settlement eats about a hundred
in a winter, so `has_room` — the only thing that ever stopped anybody burying —
was never once the binding question. A people buried until the ground held
**four years' eating** and then went on burying. Everything past the first
winter was going to rot whatever its rate was.

#### The thing that did not work, measured first

The obvious reading of #39 is that nobody draws on the store. So: ask the store
*before* the ordinary food branch instead of after it, so that a hole full of
supper underfoot beats a walk to a berry bush. At thirty-two worlds a side:

| | before | after | t |
|---|---|---|---|
| rotted in the pits | 520 | **254** | −6.65 |
| food taken out (actions) | 114 | **590** | 9.14 |
| **food eaten** | 3750 | **2961** | **−3.07** |
| **people alive** | 39.1 | **33.3** | **−2.47** |
| efficiency (eaten ÷ acquired) | 0.758 | 0.766 | — |

The waste halves and it costs a fifth of everything anybody eats and six of the
people. A meal out of a hole costs two turns where a berry costs one — you pick
it up and *then* you eat it — and nearly everything taken out had been put in by
somebody a day earlier. The settlement spent its afternoons moving food between
a pit and a pack. Efficiency, which is the whole point, did not move at all.

Reverted. `larder_tests::going_out_for_food_comes_before_digging_up_the_store`
stands on it so nobody tries it again.

#### The thing that did work

Stop filling a store that is already full. Not "is there room in the hole" but
"is there already a lean season's eating in the ground for the people about" —
which is a thing somebody standing in their own camp can see. Thirty-two worlds
a side:

| | before | after | t |
|---|---|---|---|
| **food eaten** | 3750 | **9831** | **8.49** |
| **people alive** | 39.1 | **47.2** | **2.97** |
| peak population | 42.9 | 49.6 | 2.77 |
| rotted in the pits | 520 | **367** | −3.39 |
| times anybody buried anything | 775 | **338** | −8.87 |
| efficiency (pooled) | 0.758 | **0.815** | — |
| winter store | 194 | 349 | 5.73 |

A settlement eats **two and a half times as much food** and carries eight more
people. Burying halves. The store gets *bigger*, not smaller, because there are
more mouths for it to be sized against — the cap is per-person, and the people
it freed went and lived.

Absolute waste rose, 1200 to 2238 units, because two and a half times as much
food is moving through the world; as a fraction of what was acquired it fell.
That is the number the exercise is about: half the meat rotting means half the
hunting was wasted, and less of it is wasted now.

#### Three defects underneath it

**A circle, of the family this project keeps finding.** `Cover` hands a person
one meal back when they bury the rest — the store is right there — and drawing
on the store asked for a person with **no food at all**. That one meal was
exactly enough to lock somebody out of the pit they had just filled. Fixed;
worth nothing on its own, because the branch behind `food_action` rarely got the
turn either way, but it is a precondition for anything else working.

**A count that lied.** The gate counted `is_food`, which answers yes to an uncut
haunch, a stack that has gone over, and raw flesh this one has been ill off.
A man carrying a rotten carcass read as provisioned. `how_many_meals_i_have`
counts what somebody would actually eat, on the same terms `find_best_food_to_eat`
picks by.

**A starvation loop.** `Pit::something_to_eat` returned whatever was nearest the
top that was not a basket. The moment anything actually drew on the store, a pit
holding an uncut haunch offered it over and over to somebody who could not eat
it: they picked it up, were no better fed, and picked it up again. **One
settlement in sixteen starved to death standing on its own larder**, twenty-three
thousand turns and every one of them a success. `something_to_eat` and `has_food`
now mean a meal.

### 44. An agent in a corner refuses to run, forever

Found while measuring the above, not fixed here.

`Action::FleeFrom` tries three directions — straight away, and the two square to
it. If all three land somewhere impassable it returns `"Nowhere to run"`, and
nothing about the next turn is different, so it returns it again. In one world
of the store-cap arm this was **76,644 refusals**, three quarters of every turn
in the settlement and by a distance the largest single refusal the model has
produced.

It is rare — four other worlds in the same arm ran at the ordinary 2-3% failure
rate, and four baseline worlds never produced one — and it is the recurring
defect this project has now hit five times: *a branch that can refuse must not
stand in front of branches that cannot.* It wants either more directions tried
before giving up, or standing your ground treated as an answer rather than a
failure. Left alone deliberately: it is a combat change, it wants its own
measurement, and folding it into the larder batch would have confounded the
result above.

**Fixed and measured. See #66 below** — and the cause was neither of the two
things guessed at here.

### 45. Stacking a thing onto a thing keeps the wrong clock

`Pit::put_in` and `Inventory::add_item` both merge by name and keep the
*existing* entry's `food_data`. Bury a dried strip into a pit that already holds
a raw stack of the same name and the dried strip takes on the raw clock; bury a
fresh one onto an old stack and it inherits the old freshness. Pits were also
observed holding several hundred units of food with no `food_data` at all, which
therefore never rots and never counts as a meal — dead weight that a store can
never work off.

Not investigated to a root cause and not fixed. It is a data-modelling wart
rather than a behaviour, and every preservation measurement in this file has
been taken over the top of it.

### 46. A settlement drank its own springs dry, and the comment said it would not

Three tests in this file's own intermittent list were failing in the suite.
Two of them turned out not to be intermittent at all: they failed **twelve
times out of twelve** and **eight times out of eight**, on this commit and on
the one before it. They were real, standing, reproducible failures that had
been filed as flakes and left. This is the first of them.

`water_is_not_used_up` says a river should not be drunk dry. Run with **nobody
in the world**, the total holds at 100%. With twelve founders it fell to 55% in
six thousand ticks, and the per-source breakdown says where it went:

| terrain | left | of |
|---|---|---|
| Hills | 2 | 423 |
| Forest | 2 | 444 |
| Hills | 2 | 289 |
| Meadow | 1 | 312 |
| *(five more the same)* | | |
| Sea, SaltMarsh | full | — |

**Eight of twenty-one sources drawn to two units and left there.** The sea and
the salt marshes read full because nobody drinks them, which is also why the
test's whole-world total was a poor measure of anything.

The numbers beside the terrains were:

```rust
TerrainType::Water | TerrainType::Riverbank => 3.0,   // "whatever is drawn is replaced from upstream"
TerrainType::Mountain | TerrainType::Hills => 1.5,
TerrainType::Wetland | TerrainType::Forest => 1.2,
_ => 0.8,
```

These are per pass of the resource tick, which comes round once every ten
ticks — so a spring gave back **0.15 a tick** against a camp of forty drinking
something like three. A twentieth of what it needed. The comment on the first
line is the correct design and the number under it never implemented it.

It also explains a figure that has been sitting in every refusal table in this
file without being read: **"Gather: No water sources nearby" was the single
largest refusal in the model**, up to 6,769 in a world. That is not a map with
too little water on it. That is a settlement standing in the middle of its own
dry springs, walking further every year for a drink.

Fixed by making a stream a flow rather than a stock: running water replaces
whatever was drawn, a spring gives 20 a pass and will carry a camp, a seep 12,
and standing water 6 plus what falls on it. A pond can still be drunk down,
which is right — it is why a village sits on a spring.

The test now asserts source by source rather than on a total, because the total
cannot tell a river drawn to nothing from a puddle that was always small. Over
thirty-two sampled worlds the worst case is three sources of twenty-one drawn
below a tenth, against eight before; the thresholds are set at a third and a
half, which leaves real margin either way.

### 47. Being honest has nothing to do with keeping your hands off other people's things

The second standing failure. `honest_agents_do_not_end_up_accused` fills a
settlement with twenty-five people who all have `Trait::Honest` and asserts
that none of them ends up on anybody's books as having been caught out in a
claim. Between six and a hundred and thirteen of them were, every run.

Two separate causes, and this is the smaller one. `they_took_something_of_mine`
recorded a **theft** through `update_on_verification(false)` — the same column
`wrong_count` that a proven lie goes in. An honest man can still help himself
to a neighbour's spear; `Trait::Honest` governs what he says, not what he
takes. So the settlement of people who would not dream of lying was full of
men on the record as liars, for thefts.

`TrustRating` now has a `took_from_me` column of its own. The weight on the
trust is unchanged — a thief is no more to be relied on than a liar — so
nothing about behaviour moves; only the charge is filed correctly.

### 48. Worked ground looked exactly like ground somebody made up

The larger half of the same failure, and a nastier shape.

The model already had the right idea in it. A liar names a place a good walk
from the real one and claims he passed it this morning; an honest man names
the real place and says when he actually saw it. Walk to the spot, find
nothing, and `was_he_answerable_for_it` decides: a fresh claim is a lie and an
old one is only news that kept badly.

What it could not tell was *somebody else got here first*. A renewable node —
a berry patch, a fish run — stays on the map when it is emptied, so a picked
patch still reads as "there is something here" and nobody is convicted. But a
**mined-out mineral seam is deleted**, and deleted ground is indistinguishable
from the invented spot a liar names. Report a clay seam you honestly passed
yesterday, have somebody mine it out this morning, and you are a proven liar
to everybody who walks past.

The world now remembers where a seam was worked out, and bare ground that was
worked does not convict anybody.

**And there were two copies of the verification sweep**, one in
`Simulation::tick` and one in `Population::process_exploration_with_world`,
carrying the same comment quoting the same requirement. Fixing the first
took the count from 19 to 10 and no further, which is how the second was
found. The decision now lives in one place, `Hearsay::does_bare_ground_convict_him`,
and both call it. That is the fourth instance of this project's duplicated
vocabulary defect.

### 49. A cluster of three was usually one

The third failing test, and the only one of the three that really was
intermittent: 28% over twenty-five runs.

`spawn_resource_clusters` picks a centre on terrain the resource likes, then
places the rest of the cluster at a random offset within the radius — and
**dropped any that landed on the wrong terrain**, silently, with one throw of
the dice each. Clay wants wetland or riverbank, which is ribbon terrain a
couple of tiles wide, so an offset of five in each direction usually lands on
dry ground.

Asked for five clusters of three, a world produced **5.8 nodes**, and a quarter
of worlds had no two clay nodes within twenty paces of each other — which is
not clustering, and is what the test was quite correctly complaining about.
Every clustered resource in the world went through this: clay, sand, coal,
grain, flax, herbs, cotton and fish.

A cluster now takes up to twenty-four throws before giving up on a node, and
still gives up, because a centre at the tip of a spit may genuinely have
nothing near it. Five clusters of three now produce **13.5 nodes** and forty
worlds out of forty cluster. The test passes sixty times in sixty.

#### And what correcting the world exposed

The three fixes were measured together against the commit before them, and
then the clustering fix was backed out and the other two measured on their own,
because the first result had something in it worth separating. Thirty-two
worlds a side for the first, sixteen for the second:

| | before | all three | water and trust only |
|---|---|---|---|
| failure rate | 0.034 | **0.030** (t = −4.1) | **0.031** (t = −2.8) |
| people alive | 50.8 | 56.9 (t = 1.8) | 42.8 (t = −2.0) |
| food eaten | 10,365 | 10,256 | 8,691 |
| **efficiency** | **0.82** | **0.74 (t = −8.5)** | 0.79 (t = −1.1) |
| rotted in packs | 731 | **1,467** (t = 9.2) | 622 |
| rotted on the ground | 1,087 | **1,412** (t = 4.8) | 931 |

The **failure rate falls in both arms**, which is the water fix doing exactly
what it was meant to: a settlement that is not walking half a mile for a drink
does not spend a twentieth of its turns being told there is no water nearby.
Population moves in neither arm at these sample sizes.

But putting the world's resources back to the number the config always asked
for costs **eight points of efficiency**, and it is not a subtle effect.
Doubling what there is to gather does not double what anybody eats — food eaten
is flat — it doubles what rots in a pack and on the grass. The agents gather to
the limit of what is in front of them rather than to the limit of what they
will eat.

That is the same defect as #43, one step upstream: a people that buried four
years of food into a hole also picks four years of berries off a bush. The
larder was capped by asking what the camp would eat before winter; gathering
has no such question in it at all. Not fixed here — it is a change to what
`is_this_lot_for_the_store` and the ordinary gather branch ask, it wants its
own measurement, and folding it in would have confounded three fixes that are
each about something else. It is the obvious next thing.

The clustering fix stands. The world was misgenerating and now is not; what it
exposed is a fault in the agents, and hiding it behind a broken world was never
a fix.

### 53. A spring is a flow, not a barrel

Raising the rate in #46 was only half the fix, and the half that was left is
the one that matters. Water was still a `ResourceNode` with an `amount` and a
`max_amount` — a stock that drinking decrements and inflow refills. It is not
a stock. **A spring does not have a set amount of water in it**: it recharges,
steadily, out of a catchment that is not in this model, and what limits what
you can draw from it in an afternoon is its rate. Twelve people cannot drain a
decent spring, and there was no sentence anywhere in this code that said so.

Three things went in.

**A source cannot be drawn below what it puts out.** The rate is worked out
from terrain in `regenerate_resources`, which knows what tile a spring sits on;
it is now recorded on the node and spent in `harvest`, which does not.
Everything that is a stock still is one — a berry patch stripped bare is bare,
a seam mined out is mined out — and water alone has a springline under it.
Measured over six thousand ticks: at twelve founders a world keeps 99% of its
water and the emptiest source holds 214 of 400; at eighty founders, a hundred
and forty-one people alive, it keeps 67% and **the emptiest source still holds
twelve**. Nothing can be drunk to nothing at any population.

**Springs know their rate before anybody drinks.** `regenerate_resources` does
not run until the tenth tick, which was ten ticks in which the founders could
drink one dry — the whole failure arriving early.

**Thirst is a reason to leave a country.** `migration_action` read the Hunger
drive and nothing else, and `moving_on` counts what is *edible* standing within
reach. So a settlement whose springs had gone dry and whose hedgerows were full
had no reason anywhere in this model to pick up and move, and did not — which
is the answer to "why did the agents not migrate?" There is even a constant
named `HOW_FAR_A_PEOPLE_WILL_MOVE`, documented as "how far a people will pick
up and move for water they can count on", which was used only by a food-seeking
branch. A man leaving for want of water now walks towards water he remembers
rather than towards a berry bush.

#### Two things measured wrong on the way, both worth keeping

**A river's flow is bigger than its bed.** The springline was first set to
`inflow.min(max_amount)`, and running water's inflow is deliberately larger
than any bed — so a river's springline became its entire capacity and **rivers
were undrinkable**. A source whose flow refills its whole bed between one pass
and the next keeps back nothing, because there is nothing to protect.

**Exempting springs from the picked-out memory made things worse.** A place
found empty is remembered as empty for half a season, which looks obviously
wrong for a spring that will be running again in ten ticks. Backing it out for
water put the failure rate **up** rather than down (t = 3.75 against 2.56):
a man who does not remember that the spring was low walks back to it and is
refused again, and remembering is what sends him to the next one. Left alone.

#### What actually fixed the cost

The flow model cost half a point of failure rate on its own, and "Gather:
Resource source was empty" became the fourth largest refusal in the model —
a strange thing to be able to say about a running spring. The pool is what has
gathered and the springline is what is *arriving*; a man kneeling at a spring
that is down to its springline is not looking at a dry hole, he is looking at
water coming out of the ground, and what he does is drink it as it comes. So a
source at its springline gives a mouthful taken from the flow, and the pool
does not move. A queue at a spring all get a drink and none of it comes out of
the pool.

Measured at thirty-two worlds a side against the commit before: **the failure
rate regression goes to nothing, t = 0.01**, and every other column is null —
population, food eaten, waste, the store. Which is the right result for a batch
whose whole purpose is that the world should stop doing something impossible.

#### Still not modelled

Water is conserved by fiat here rather than by circulation. Waste and the dead
return litter and fouling to the soil — `return_what_the_living_and_the_dead_leave`
— and no water term goes with them, so a body's water is not returned to the
ground it falls on; it simply never left, because a source cannot be drawn
below its flow. That is the computationally cheap version of the right answer
and it is worth writing down as such.

### 57. Making food scarcer does not make a people careful

Two halves of one problem, done together at the user's direction and measured
separately so they would stay readable: thin what there is to gather (#184),
and stop a people taking more of it than it will eat (#178). The batch is
**mostly a negative result**, and the negatives are worth more than the change.

#### The vocabulary that was in two places again

The first attempt thinned `TerrainResourceMapper::amount_range` — berries from
(20, 60) to (8, 24) — and measured **nothing**. A world still held 994 units of
berries against 1,000 before.

There are **two resource spawners in this project**. The naturalistic one
places clustered minerals and crops and reads that table. The basic one in
`World::generate_resources` places wood, stone, food, greens and roots, and had
its own hard-coded `gen_range(20..60)` sitting inside it. Berries come out of
the second one. Thinning the first and measuring the result was measuring
nothing at all.

**Fourth instance of this project's duplicated-vocabulary defect**, and the
second in two batches. Both spawners now read one table and both route through
one `what_this_ground_carries`, so a patch is the size the *kind* of thing
carries, on the fertility of the tile it is rooted in — which `regenerate_in_ground`
has always capped regrowth by and which the crop a world *started* with ignored
entirely.

With that fixed the thinning is real: a world's edible standing crop goes from
**7,413 to 3,944**, berries from 987 to 218.

#### And it still changed nothing

Thirty-two worlds a side, against thinner hedgerows, shy animals and a cap on
what one person takes: **not one column reaches significance.** Not population,
not food eaten, not waste, not efficiency, not the store.

Three reasons, and each is worth keeping:

**The standing crop is a buffer; the flow is what matters.** A patch regrows at
`base_rate` per tick until it hits its cap. Halving the cap does not halve the
flow — the patch simply tops out sooner and goes on producing at exactly the
same pace. This is the springs lesson from #53 in reverse, arrived at from the
other side.

**Hunting is 250 actions in 270,000.** Wild animals now get out of a person's
way — nothing in the fauna module knew agents existed except the predator pass,
so a deer stood where it stood while a settlement walked up to it — and it
cannot possibly matter at that volume. The change is right and its effect is
unmeasurable, which is the honest thing to say about it.

**The demand cap has almost nothing to bite on.** A hungry man with food in his
pack eats it rather than gathering; the branch that fills a pack past what
anybody will eat is a *kill* or an autumn store trip, and the store trip was
already capped in #43.

#### The thing that was tried next, and reverted

If the flow is what matters, halve the flow: berries from 0.025 a tick to 0.012,
wild grain from 0.015 to 0.008. Thirty-two worlds a side:

| | before | scarcer | |
|---|---|---|---|
| people alive | 54.9 | 52.2 | — |
| food eaten | 9,271 | 8,310 | — |
| **efficiency** | **0.74** | **0.70** | **t = −3.0** |
| rotted in packs | 1,355 | **1,655** | t = 2.4 |
| left where it fell | 43 | **105** | t = 2.1 |

Scarcer food did not make a winter bite. The population did not move and
**efficiency got significantly worse**: people ranged further, carried more when
they found anything, and lost more of it in transit. Reverted.

That is the finding. **The waste in this model is a behaviour, not a supply
artefact, and starving people does not fix a behaviour.** Every previous entry
that reached for a resource number to change how a settlement behaves should be
read against this one.

#### What was kept

All of it except the rate, on the grounds that each piece is *correct*
independent of whether it is measurable: one vocabulary across both spawners, a
crop that follows the ground it stands in, a wild hedge that is not an orchard,
animals that get out of the way, and a person who does not go back to the river
for more fish than he will eat. Measured together: **null on every column**, at
no cost anywhere.

#### Two measurement mistakes, recorded

**Two copies of the same pipeline ran at once** and clobbered each other's
output files, producing a 45-row CSV for a 32-world run with 13 malformed rows.
This is the second time in this project's history that racing output paths have
produced a false reading, and the first was already written down. Results from
the clobbered files were discarded rather than filtered.

And a `pkill -f` pattern **matched the command line of the shell running it**,
so the guard against a runaway run killed the run.

### 61. A thing rebuilt from its name is not the thing

Entry #45 noted in passing that stacking kept the wrong clock and left it as a
data-modelling wart. It is not a wart. It is one mistake made in five places,
and in a simulation whose whole subject is what an agent *decides*, it matters
for a reason that has nothing to do with realism: **an agent chooses whether to
eat now, dry it, bury it or leave it on the strength of what its pack says it
is holding.** A pack that lies about its own food produces perfectly sensible
decisions about a world that is not there.

#### The stacking rule

`Inventory::add_item` and `Pit::put_in` both merged by name with a bare
`quantity += other.quantity`, keeping whichever stack happened to be there
first. So the same act went either way by accident: this morning's berries
tipped onto a week-old basket inherited the week-old timer, and a week-old
handful tipped onto a fresh stack was **silently made new again**.

Freshness is derived from `created_tick`, so the timer is the thing that has to
move. `FoodData::the_older_clock` now takes the older tick, the faster-spoiling
preparation and the shorter spoilage span, and `InventoryItem::absorb` is the
single merge both call sites use — mould spreads, and a stack is as old as its
oldest part and as perishable as its worst. Half of this the user asked for and
half of it is the same bug seen from the other side.

#### And four places that rebuilt an item out of its name

The stack merge is one instance of a wider mistake: taking an item's *name* and
count and constructing a fresh item, discarding everything else about it.

- **Giving.** `Action::Give` removed the item from one pack and built a new one
  for the other: same id, same count, and nothing else — no food data, no
  freshness, no preparation, and a flat weight of 2.0 whatever it was. Giving
  somebody a week-old fish handed them a fish that would never go off; giving
  away a dried strip threw the drying away.
- **Theft.** The same, exactly. Stealing a week-old fish got you a fish that
  keeps for ever.
- **Harvesting a plant.** The flora harvest path attached no food data at all,
  so anything picked off a cultivated plant was inert from the moment it
  existed.
- **The merge itself**, when one side had no food data: the dataless side could
  win, and an item with no `food_data` **never rots**. One such stack in a pit
  swallowed every honest stack merged into it afterwards, which is where the
  several hundred units of immortal food in #45 came from.

Measured directly, tracking food-named items with no clock: inert food first
appeared in a pack around **tick 1,800** and in a pit by **6,800**. After the
fixes, pits are clean and packs show one residual case in three ten-thousand
tick runs, from a path not yet identified — recorded rather than claimed to be
finished.

#### The clock rule is written, tested, and **not shipped**

This is the uncomfortable part. Everything above ships. The clock rule — the
thing the entry is named after and the thing that was actually asked for — does
not, and here is why.

Measured at thirty-two worlds a side, with the rule on:

| | before | with the clock rule |
|---|---|---|
| **food eaten** | 9,703 | **4,638** (t = −8.4) |
| people alive | 55.5 | 48.0 (t = −2.2) |
| winter store | 320 | 105 (t = −10.7) |
| rotted in packs | 1,346 | 390 |
| rotted on the ground | 1,340 | 697 |
| rotted in pits | 485 | 967 |

A settlement ate **less than half as much**. And the loss does not turn up
anywhere: eaten plus waste falls from **12,874 to 6,692**, so something like
six thousand units leave the ledger without being eaten, without rotting in a
pack, on the ground or in a pit, and without still being held at the end.

It was attributed properly rather than guessed at. Three arms, sixteen worlds
each:

- Without the harvest clock: eaten 4,184. Still halved, so that is not it.
- Without the clock merge, everything else on: eaten **9,389** against a
  baseline 9,703, t = −0.32, and every other column null.

So the clock merge is responsible on its own.

> **This entry was wrong, and #65 corrects it.** It concluded that six thousand
> units were leaving the ledger and held the rule back on that basis. There is
> no sink: measured directly, food is conserved to within a hundred units over
> six thousand ticks. The settlement does not lose food, it **acquires less**.
> The rule ships. See ISSUES_FOUND #65.

The reason to come back to it is the first paragraph of this entry: this is a
behavioural model, and an agent's decisions are only as good as what its pack
tells it. A settlement that has been told its winter store is fresh has no
reason to dry anything.

### 65. There was no sink. The settlement was living on food that never aged

#61 held the food-clock rule back because eaten plus waste fell from 12,874 to
6,692 with it on, and concluded that six thousand units were leaving the ledger.
**That conclusion was wrong.** This entry corrects it and ships the rule.

#### Food is conserved

The obvious instrument, built last instead of first: sum every unit of food
anywhere a person could get at it — packs and pits — once a tick, and compare
what leaves the stock against what is booked as eaten or rotted. Over six
thousand ticks, twice:

    in 2923  out 1927  booked 3005  unexplained -96
    in 2978  out 1794  booked 3021  unexplained -92

**Unexplained: under a hundred units, and negative** (double-counting on ticks
where the stock rose and something was eaten in the same pass). Nothing is
leaking. The settlement does not lose food with the rule on; it **acquires far
less of it** — `Gather` falls from 45,380 actions to 30,622, a third — and
spends the difference preserving: `Dry` goes from nowhere in the top sixteen
actions to 1,678, and burying rises 76%.

That is a coherent response to honest information rather than a fault. Before,
a stack's clock was whatever the first thing in it had, and a pack topped up
all day was a pack that never aged; a settlement was living on food that could
not go off. Now it can, so a people preserves what it has instead of picking
more, and settles smaller.

#### Four hypotheses, each measured and each wrong

Recorded because the hunting was most of the work, and because a list of things
that are *not* the cause is worth as much as the thing that is.

1. **The harvest clock.** Arm without it: eaten 4,184. Not it.
2. **Weight.** `absorb` can change a stack's preparation, and preparation
   decides weight — a dried stack weighs a third of the same thing raw — while
   `add_item` only ever added the *incoming* item's weight, so `current_weight`
   read low until the next `recalculate_weight` corrected it in one jump. If
   that jump put a pack over its limit, every later `add_item` returned false
   and the food was destroyed, because almost every caller ignores the bool.
   A real bug, **fixed**, and worth about **nine units a world**. Not it.
3. **The gate counted rot.** `more_food_than_he_will_get_through` asked
   `is_food`, which is true of mould, so a man with eight units of spoiled
   berries declined to pick anything. Exactly the mistake #43 fixed once
   already, made again one entry later. A real bug, **fixed** with
   `how_much_good_food_i_have`. Measured: no material change. Not it.
4. **Pits merging a season's loads into one.** A pit is a list, not a pack, and
   has no need to merge; a fresh load inheriting last autumn's clock rots at
   once. Now this autumn's load goes in **beside** last autumn's unless there
   is something of an age to join. Right, and **fixed**. Measured: no material
   change. Not it.

#### What shipping it costs

Thirty-two worlds a side, everything in:

| | before | after | |
|---|---|---|---|
| food eaten | 10,375 | 4,283 | t = −8.3 |
| people alive | 52.1 | 43.6 | t = −2.1 |
| rotted in packs | 1,532 | **388** | t = −11.3 |
| rotted on the ground | 1,438 | **640** | t = −10.5 |
| rotted in pits | 490 | 915 | t = 6.6 |
| left where it fell | 99 | **27** | t = −2.8 |
| winter store | 363 | 94 | t = −12.1 |

A settlement is about a sixth smaller and eats less than half as much, and
**wastes a quarter of what it used to** in packs and on the ground. The rot
moves into the pits, which is where a people that preserves rather than picks
would put it.

This is a large change and it is stated plainly rather than buried: the model's
central quantity halves. It ships because the alternative is a pack that lies
to the agent reading it, and every decision this simulation exists to make —
eat now, dry it, bury it, leave it — is made off that reading.

### 66. Two words for one question, and a man who could not get off the beach

#44 recorded 76,644 refusals in one world — three quarters of every turn taken
in the settlement, and by a distance the largest single refusal this model has
produced — and guessed at two possible causes: too few directions tried, or
standing your ground not counting as an answer. Both guesses were about the
running. Neither was the cause.

#### The cause

Two things asked whether there was anywhere to run, and they asked it in
different words.

`how_this_one_answers_a_threat` asked `is_there_anywhere_to_run`, which tried
three directions at **three paces**. Having got a yes, it returned
`Action::FleeFrom`. The executor for `Action::FleeFrom` then tried the same
three directions at **nineteen paces** — `HOW_FAR_A_FRIGHTENED_PERSON_GETS`,
which is deliberately far enough that a man does not run one pace, look round,
and run one pace again.

Between those two numbers sits a shoreline. A man three paces from open water
with the thing inland has somewhere to go at three paces and nothing but water
at nineteen. The decision said run; the running said `"Nowhere to run"`. Nothing
about the next turn was different, so it said it again, and again, for the rest
of that agent's life.

This is the project's duplicated-vocabulary defect for the **fifth** time, and
the third in four commits: two verification sweeps, two resource spawners,
`is_food` against meals twice, and now two answers to "is there anywhere to
run". The pattern each time is the same — a question asked in two places, in
words that agree on the day they are written and drift apart afterwards.

#### The fix

One function answers it. `where_this_one_would_run` returns the tile, the
decision asks it whether there is one, and the executor uses the one it
returns. There is no second opinion to drift from.

While it was open, both of #44's guesses were taken as well, because both are
right about the running even though neither was the cause:

- **Eight ways out rather than three**, each tried at the full bolt and then at
  every shorter distance down to a single pace. A narrow gap is a gap. Behind
  is in the list too: running past the thing is a poor answer and the scoring
  says so, but it beats being caught standing.
- **Standing your ground is an answer, not a refusal.** Where there genuinely is
  nowhere — a tile with water on all four sides — the action costs the turn and
  the agent stays put, which is exactly what `Action::Freeze` does. A branch
  that can refuse must not stand in front of branches that cannot, and this one
  no longer refuses at all.
- Ranking now weighs getting clear against what the agent remembers. It used to
  rank the three landings by remembered danger alone, which was fine with three
  and would have been wrong with eight, since running towards the thing scores
  as well as running away from it on danger alone.

#### Measured, 32 worlds a side, 10,000 ticks, 12 founders

| | baseline | with the fix | t |
|---|---|---|---|
| refusals ("nowhere to run") | 613.3 ± 613.3 | **0.0 ± 0.0** | — |
| worst world's refusals | **19,626** | **0** | — |
| worst world's failure rate | **0.0984** | **0.0262** | — |
| runs actually taken | 0.25 ± 0.11 | **2.22 ± 0.79** | **2.48** |
| failure rate | 0.0245 ± 0.003 | 0.0228 ± 0.000 | -0.69 |
| alive | 48.7 ± 2.5 | 52.2 ± 2.8 | 0.93 |
| eaten | 4,557 ± 273 | 5,098 ± 287 | 1.37 |
| deaths | 21.3 ± 1.2 | 21.8 ± 0.9 | 0.31 |
| fights | 2.4 ± 1.3 | 2.8 ± 1.1 | 0.20 |

The mean failure rate barely moves and the standard error tells you why: **the
defect is a tail, not a level.** One world in thirty-two had it, and that world
alone ran at a 9.8% failure rate against 2.2% for the other thirty-one. Its
whole standard error is that one world. Afterwards no world in thirty-two
produces a single refusal and the worst failure rate in the arm is 2.6%.

Everything else is null, which is what a fix to a rare pathology should look
like. The one non-null column is the point: **running happens nine times as
often**, because the decision's yes is now a yes the running can act on.

#### And a thing worth knowing that came out of it

`Freeze` was taken **zero times in sixty-four worlds**, and `FleeFrom` a
quarter of a time per world before the fix. The whole threat-response tree
built for #133-#135 — fight, flee, cornered, helpless, freeze — fires a
handful of times in ten thousand ticks. The likely reason is the change two
commits ago that made animals shy away from people, which is in both arms
here: an animal that keeps its distance is never appraised as a threat. That
is realistic and it may also have quietly retired a subsystem. Not
investigated; recorded so somebody does.

> **Both numbers in that paragraph were wrong, and so is the "runs actually
> taken" row of the table above. They were read off a counter that was
> booking two different things under two different names.** `actions_taken`
> booked everything chosen in the fear branch as "Flee"; `actions_failed`
> booked by the action's own name. So a run that happened went under "Flee",
> a run that was *refused* went under `FleeFrom`, and `Freeze` — also chosen
> in that branch — went under "Flee" as well and therefore read as never once
> taken. The 19,626 refusals are real, but "against no attempts at all" was an
> artefact of the same defect this entry is about, one level up. #67 corrects
> it, retracts the flee row, and reports what #176 actually did. The
> shy-animal hypothesis was wrong too: `shy_away_from` already exempts every
> Aggressive and Territorial species, which is every predator in the table.

### 67. Two names for one action, and the two claims it cost

#66 ended with a paragraph about the threat tree looking retired, and #187
asked whether shy animals had done it. Neither the answer nor the question
survived contact with an instrument.

#### First, the instrument

The lesson from #65 — build the measuring thing before the hypotheses — taken
this time. `Simulation::what_a_threat_came_to` counts where the threat
decision comes out, one hash lookup on a path that runs once per agent-turn.
It has to exist because every way of declining used to look identical from
outside: `None`. An action tally can count the answers a decision reached; it
cannot tell a tree that is working from a tree that is never asked.

The first run of it, four worlds and 1.1 million agent-turns, answered the
question in one line and raised a worse one:

| | count | share of turns |
|---|---|---|
| turns decided | 1,197,947 | 100% |
| a creature on the mind: resented | 146,394 | 12.2% |
| a creature on the mind: feared | 8,057 | 0.67% |
| on the mind, but under the gate | 149,303 | 12.5% |
| felt: afraid enough to act | 9,497 | 0.79% |
| felt: angry enough to act | 3,630 | 0.30% |
| — tree declined: nothing named | 7,316 | 0.61% |
| — tree declined: named, but not about | 3,343 | 0.28% |
| — tree declined: not worth crossing to | 204 | 0.02% |
| — **tree answered: runs** | **1,558** | 0.13% |
| — tree answered: stands its ground | 42 | 0.004% |
| **`actions_taken["FleeFrom"]`** | **0** | **0%** |

The decision reached `Action::FleeFrom` 1,558 times and the action tally
recorded it none.

#### The defect

```rust
let did = if running_away { "Flee".to_string() } else { Self::name_of(&action) };
*self.actions_taken.entry(did).or_insert(0) += 1;
```

Everything chosen in the fear branch was booked as "Flee". The failure path
four lines below books by `Self::name_of(&action)`, which is `"FleeFrom"`. So
a run that happened and a run that was refused went into two different
buckets, and `Freeze` — also chosen in that branch — went into the "Flee"
bucket and read as never once taken.

The note was correct when it was written: running away used to come out as an
ordinary `Move` that nothing could tell from a stroll. `FleeFrom` and `Freeze`
became verbs of their own afterwards and name themselves. The relabel was
never narrowed. **Sixth appearance of the duplicated-vocabulary defect**, and
the first one that corrupted a published measurement rather than a behaviour.

It is now `Simulation::what_to_book`, which relabels only a `Move`, and the
invariant it broke is a test: *nothing can fail at a thing it was never
recorded doing.*

#### What it cost, stated plainly

Two claims in #66 are withdrawn.

1. **"`Freeze` was taken zero times in sixty-four worlds."** Wrong. With the
   labels fixed, twelve baseline worlds take it **10,971 times**, and at
   n = 32 the distribution is 28 zeros and then 16, 31, 161, **934** — the
   same rare-catastrophic shape as the refusal itself.
2. **"Running happens nine times as often, 0.25 to 2.22 a world, t = 2.48."**
   Withdrawn as unresolved. Re-measured with the label fix applied to *both*
   arms, 32 worlds a side: **1,256 to 363 a world, t = -2.79** — the opposite
   direction. A second draw of twelve worlds a side gives 3,151 against 5,118,
   the original direction again. Per-world flee counts run from 1 to 7,365;
   the quantity is too skewed for a mean of thirty-two to mean anything, and
   no claim is made about it either way.

#### What #176 actually did, measured on something that is not skewed

Ground put between the agent and the thing, per run, twelve worlds a side and
3,151 and 5,118 runs:

| | baseline | with the fix |
|---|---|---|
| mean paces moved per run | **7.4** | **17.1** |
| mean paces *gained* on the thing | **7.2** | **16.3** |

The intended bolt is nineteen paces. The old three-way-plus-clamp delivered
under 40% of it: a landing off the map edge was clamped back onto the edge, so
a nineteen-pace bolt could become a one-pace shuffle and the man was still
inside the wolf's radius next turn. The eight-way search takes the furthest
*passable* tile along each ray and scores on ground gained, and delivers 86%.

And freezing — the branch for a body that can neither run nor raise a hand —
falls from 10,971 to 1,092 over the same twelve worlds, and from a worst world
of 934 to a worst world of 5 at n = 32. That is the cornered case being fixed
rather than the branch being dead: after #176 there is nearly always somewhere
to go.

#### The threat tree is not retired, and shy animals were never the cause

`Animals::shy_away_from` filters to `Passive | Neutral | Defensive`. Every
Aggressive and Territorial species — every predator in the table — was already
exempt. The hypothesis in #187 was checkable in ten lines of the function it
accused, and I raised it without reading them.

What does keep the predators away is a different pass, and it is not a defect
either. Put a wolf one pace from a healthy adult and it leaves at **six paces
a tick, straight away, and does not come back**:

```
t0: wolf at (37, 30)   agent at (30, 30)
t1: wolf at (43, 30)   agent at (30, 30)
t2: wolf at (49, 30)   agent at (30, 30)
```

That is `what_the_beasts_make_of_us` from #142 — run from what you cannot
beat — reading the odds against an armed adult and deciding against it. It is
the fauna model working, and it explains the whole shape of the numbers above:

- **Predators leave before they are close enough to frighten anybody**, so the
  fear side of the tree is quiet: creature fear on 0.15% of turns.
- **What stays near a person is what a person can beat**, so it is appraised
  as anger rather than fear: creature resentment on 9.8% of turns.
- And that anger cannot pass its own gate, which is the next section.

Two consequences worth someone's attention rather than mine: a wolf that will
not approach a lone adult cannot hunt one, and the model has predator attacks
on people as a deliberate feature (#29). And this made the wiring test for the
instrument hard to write — the wolf has to be pinned at the agent's elbow each
tick — which is itself a fact about the world worth knowing.

What the tree actually does, with the labels fixed (four worlds, 1.1M turns):
it is asked about **1.45% of turns** and reaches an action about 1,262 times.
Its dominant fallout is **"nothing named" at 80% of everything that reaches
it** — an agent frightened or angry enough to act, at a *person*. That is the
tree correctly declining: the branches below it in the chain handle people,
and it hands over to them. Not a defect.

#### The one thing that is a defect, and is left open

`should_attack()` is `anger > 0.5 && fear < 0.3`. `ThreatAssessment::
emotion_amount()` returns `threat_level * 0.5` for anger, and `threat_level`
is bounded at 1.0.

**Anger at a single creature can therefore never exceed 0.5, and the gate
wants strictly more than 0.5.** A man at maximum rage at one animal sits
exactly on the gate and does not pass it. The branch fires only when two or
more separate grudges *sum* past a half — `update_totals` adds the sources
while the tree reads the strongest one, which is the same two-vocabularies
shape again — so an agent turns on a wolf because it also resents a boar.

Measured: 12.5% of every turn, an agent has a creature on its mind and nothing
comes of it, and 12.1 of those 12.5 points are "neither strongly enough".
Creature *resentment* runs at 9.8% of turns and creature *fear* at 0.15%: this
model's animals are overwhelmingly resented rather than feared, which is #82's
finding recurring one layer down — and, per the section above, is what you
would expect when everything that could frighten a person walks away from one.
So the two halves of the threat tree are quiet for two different reasons, and
only the anger half is quiet because of arithmetic.

Not fixed here. Retuning an emotional threshold is a behaviour change wanting
its own arm and its own measurement, and folding it into a counting fix would
confound both — the same reason #44 was held back from the larder batch.
Filed as #188.

### 68. A place, a date, and how much was on it — and being right, which nothing recorded

Everything one agent could tell another was a position, a resource type and a
date. A listener already weighed the age of a claim - "a seam I passed last
week" against "one I passed this morning" - and had no way at all to weigh
either against **"the last handful of a worked-out one"**. The two sound
identical and are worth walking to on completely different terms.

#### What now travels

`ExplorationKnowledge::how_much_was_there` holds what was standing at each
place this agent last laid eyes on, written wherever a sighting is recorded.
`Hearsay::how_much_they_said` carries it between people. `SpatialMemory::value`
- a field that has existed since the model had memories and was set to `1.0`
for everything, so a spring and a puddle were remembered alike - now holds it
too.

An honest man reports what he remembers, which may be the last handful. A liar
claims `WHAT_A_LIAR_SAYS_IS_THERE`, twenty, because a lie is *for* something:
it buys him a hearing, and nobody invents a seam with nothing in it. The lie in
this model is about where, never about how much.

#### Being right was unrecordable

The measurement that mattered was not the one I set out to take.
`TrustRating::correct_count` was **zero across thirty-two worlds**, against
1,646 wrong ones. Nobody in a running settlement had ever been recorded as
having told the truth.

The cause is structural. Both copies of the verification sweep call
`hearsay_in_view`, which filters to claims where the ground is *bare* - it is
incapable of returning one that held up. `found_out_they_were_right` exists, is
tested, and had one caller in a function nothing in a live settlement reaches.
The recurring defect: a complete subsystem with no live caller, and this time
it meant **a man's standing could only ever fall**.

`hearsay_borne_out` is the other half, and both sweeps now call it. A confirmed
place also stops being hearsay - he has walked to it and looked at it, so he
can pass it on as his own, and whoever told him is credited once rather than
every tick he stands there.

This is also half of what #185 is for. "They might dip temporarily, but they
should recover over time as true statements strengthen trust" needs true
statements to be recorded, and there were none.

#### And an honest report of a poor place is safe to make

`Hearsay::does_bare_ground_convict_him` had two excuses - his news is stale, or
somebody stripped the place first. It has a third: **he did say it was nearly
gone**. A man who reports the last handful of a seam this morning and is found
to have told the truth about the last handful of a seam is plainly not lying;
somebody took the handful. Holding him to it makes honesty about a poor place
more dangerous than silence, which is the opposite of what reporting is for.

It cannot shelter a liar, because a liar claims twenty and the excuse stops at
three.

#### Measured, 32 baseline worlds against 31 in the arm

One arm world was lost to a container restart mid-run.

| | baseline | with the change | t |
|---|---|---|---|
| **vouched for (`correct_count`)** | **0.0 ± 0.0** | **19,494 ± 924** | **21.09** |
| people on record as liars | 33.9 ± 5.8 | 26.5 ± 5.6 | -0.93 |
| times caught out | 51.4 ± 8.7 | 35.8 ± 8.0 | -1.33 |
| gathers | 36,004 ± 1,140 | 39,128 ± 965 | **2.09** |
| alive | 47.1 ± 2.0 | 50.5 ± 1.9 | 1.20 |
| eaten | 4,632 ± 219 | 4,818 ± 192 | 0.64 |
| deaths | 22.3 ± 1.0 | 21.3 ± 0.8 | -0.71 |
| failure rate | 0.0225 | 0.0227 | 0.36 |
| worst world's failure rate | 0.0268 | **0.0268** | — |

Accusations fall about a quarter and it is **not significant at this sample**;
the direction is right and no claim is made beyond that. Two mechanisms could
produce it and they are not separated here: the new excuse, and the fact that a
confirmed claim is now retired from the hearsay book rather than left standing
to be falsified later.

#### The half that is not shipping, and why

The obvious use for a remembered amount is the one the task asked for: walk to
the place you remember most of, rather than to whichever is furthest off, which
is what `migration_action` did on no better ground than that distance was the
one thing a memory could be sorted by.

**Three arms of thirty-two worlds each produced one world that refused for want
of water 3,092, 851 and 13,004 times, against a baseline worst case of seven.**
Weighing the remembered amount by how stale the memory was - the obvious guess,
and the one the task's "the same way staleness does" points at - made the worst
of them worse rather than better, which means the guess was wrong about the
mechanism.

Reverting that one branch and re-running put the worst world's failure rate
back to 0.0268, exactly the baseline's, and left the failure-rate distributions
indistinguishable end to end. So it is that branch and not the rest.

It is held back for the same reason #44 was held back from the larder batch: it
is a real behaviour change, it wants its own investigation and its own arm, and
folding it into a reporting change would confound both. Filed as #189. The
memory carries the amount now, so whoever picks it up has something to work
with.

One thing left unexplained and stated rather than buried: "Gather: No water
sources nearby" still runs at 42.8 a world against 0.5 (t = 1.91), on a fat
tail of four or five worlds. Those worlds are otherwise ordinary - failure
rates of 0.022 to 0.026, and in every one of them the *largest* refusal is a
tool refusal two to twenty times bigger. It is not the pathology above, and it
is not explained.

#### And a dead subsystem noticed in passing

`PlanningContext::from_exploration_knowledge`, `find_nearest_resource` and
`Planner::generate_best_plan` have **no callers anywhere in the project**.
`find_nearest_resource` picks purely by distance and is the other place the
remembered amount would belong, if anything reached it. Recorded, not fixed.

### 69. Every man knew how to make an axe and one in thirty-five owned one

"They struggle to complete simple tasks." So the first thing built was an
instrument for where a settlement's day actually goes, and the first thing it
did was kill my own first guess.

#### Not the walking

`Move` is **42.7% of every turn** taken in a settlement, which looks like the
whole answer. It is not. Counting the length of every unbroken run of walking
between two things actually done: **79% of things done need no walk at all**,
11% need one pace, and the mean is **0.71 paces per thing done**. Fewer than
2% of actions follow a walk of eight paces or more. The walking is a thousand
one-step adjustments interleaved with work, not fruitless trekking. Good thing
it was measured.

#### It is the tools

Eight worlds, ten thousand ticks, 2.3 million turns:

| | attempted | refused |
|---|---|---|
| `Work` | 18,756 | **88.2%** |
| `Excavate` | 6,348 | **99.4%** |
| `TrySwapping` | 6,489 | **100.0%** |
| `Examine` | 9,542 | **92.0%** |
| `Boil` | 3,521 | **86.8%** |

And the reasons, which are nearly all one reason:

```
Work: Nothing in hand that is any use for Leatherworking     6707
Excavate: Nothing in hand that is any use for Mining         6309
Work: Nothing in hand that is any use for Crafting           5710
Work: Nothing in hand that is any use for Mining             3628
Boil: Nothing to hold water in                               3056
Hunt: No spear in hand for that                              2227
```

Half of every refusal in the model was **an agent choosing work it had nothing
to do it with**. Then the second instrument, on who owns what:

> 181 people alive across four worlds. **All 181 knew how to make a handaxe, a
> stone knife and a spear. Five owned an axe. Nineteen owned a knife.**

And crafting was not broken: `Craft` succeeded **every time it was attempted**
and was attempted 270 times a world across forty-five people over ten thousand
ticks.

#### Why

`Craft` lives in the `Utility` branch, behind `what_i_would_work_on` and the
vessel branch, and Utility is a drive that rarely wins a contest against
Hunger. So the tool got made only when nothing more pressing was happening -
while Hunger and the store kept proposing `Work` and `Excavate` that were
refused for want of exactly that tool, over and over, for the whole run.

This file already contains the answer, written for the *other* half of the same
problem:

> Reaching for a tool is not what somebody does with a spare moment, it is what
> they do just before using it.

That is the comment on `get_the_tool_out_for`, which hoists **equipping** a
tool to the moment of use. Nobody had done the same for **making** one.

#### The fix

`make_what_this_wants`, beside `get_the_tool_out_for` in the same
post-processing chain. When the verb matrix is about to refuse an action for
want of a tool, and this one knows a step towards that tool it could take right
now, it takes the step. The turn was lost either way.

Three things keep it honest:

- It asks the **same function** the executor asks. `what_this_wants_that_is_
  missing` returns the structured `Wants`; the refusal message is built from
  it. Two ways of asking whether a man can do a job is how this project lost
  the measurements in #66 and #67.
- It only names a step that **can actually be carried out** - materials,
  knowledge, a fire if the step wants one, a hammerstone in the hand if the
  step wants one. A refusal is worse than a wasted turn, because it goes into
  the record and teaches a man that making knives does not work.
- The substitute is **checked for short-handedness itself** before it is taken,
  or this trades one refusal for another and calls it progress.

`Craft`, `Equip` and `Unequip` are never substituted, because making a thing is
the answer to this and the two must not fight over the turn. `Work` is
deliberately *not* on that list: a working refused for want of a knife is
exactly the case.

#### Measured, 32 worlds a side

| | baseline | with the fix | t |
|---|---|---|---|
| people carrying a knife | 3.9 ± 0.4 | **8.3 ± 0.5** | **7.51** |
| vessels held | 14.2 ± 1.2 | **22.1 ± 1.6** | **3.89** |
| pits dug | 5.5 ± 0.3 | **7.8 ± 0.3** | **5.91** |
| crafts | 281 ± 9 | **349 ± 10** | **5.12** |
| short-handed refusals | 2,695 ± 97 | **1,690 ± 49** | **-9.21** |
| refused workings | 1,919 ± 70 | **1,288 ± 48** | **-7.39** |
| digs attempted | 658 ± 47 | **323 ± 21** | **-6.48** |
| failure rate | 0.0230 | **0.0212** | **-3.77** |
| deaths | 22.7 ± 0.9 | 20.7 ± 0.7 | -1.70 |
| alive | 49.0 ± 1.8 | 51.6 ± 2.0 | 1.01 |
| eaten | 4,780 ± 181 | 5,063 ± 175 | 1.13 |
| people carrying a spear | 7.2 ± 0.7 | 6.2 ± 0.5 | -1.24 |

The digging row is the shape of the whole thing: **half as many attempts and
43% more pits.** They stopped trying to dig holes they had nothing to dig with,
and dug more holes.

Survival moves the right way and is not significant on its own - alive +2.7
(t = 1.01), eaten +283 (t = 1.13), deaths -1.9 (t = -1.70). Spears are slightly
down and null. What *is* significant is that the settlement is equipped, and
being equipped is upstream of everything the model is for.

#### What is still refused, and left open

- **1,690 short-handed refusals a world remain.** These are men who know how to
  make the tool and have not got the makings. The substitution stops at "no
  step can be taken"; going and fetching the material is the next link and
  wants its own arm. Filed as #190.
- **`TrySwapping` is refused 100% of the time** - 6,489 attempts, not one
  success, in eight worlds. A whole verb that has never once worked in a live
  settlement. Filed as #191.
- **`Examine` is refused 92%**, almost all of it "Turned the food over, none
  the wiser" and the same for clay, iron and grain. An agent re-examines what
  it has already learned nothing from. Filed as #192.

### 70. The effort economy is decorative: nobody in this model is ever tired

A specification arrived describing an efficiency architecture - tools that make
work faster, an agent weighing "eight hours with this axe, or two hours making
a better one and six with that", preparation cascades, tool ownership, and
specialisation into trades. The first piece of it looked small and obvious, so
it was built first. It measured **null**, and finding out why was worth more
than the change.

#### What was built

`Tool::how_much_better` multiplies what comes *off* a job - more wood a swing,
more fish a cast - and touches nothing else. So a stone axe and a bronze axe
felled a tree at the same price, and `Excavate` cost a flat
`WHAT_DIGGING_A_PIT_COSTS = 22.0` whether the hole was dug with an axe or with
bare hands. An agent weighing an upgrade had one number and needed two.

So: `Agent::what_this_job_costs_me(trade)`, the other side of
`how_much_my_tools_help` - bare hands pay 1.6x, a tool divides that by how good
it is, floored so no tool makes work free - applied in one place, where the
action result comes back, rather than across forty `with_energy_cost` arms.
Seven tests, all passing.

#### It measured nothing

32 worlds a side: alive 47.1 -> 48.8 (t = 0.67), eaten 4,999 -> 4,950
(t = -0.17), deaths 21.3 -> 22.2 (t = 0.65), pits 7.6 -> 7.4 (t = -0.50),
knives 9.3 -> 8.6 (t = -0.88). Not one column significant, and two of them
drifting the wrong way: short-handed refusals 1,536 -> 1,726 (t = 1.86) and the
failure rate 0.019 -> 0.021 (t = 1.50).

#### And here is why

One probe, 45,000 samples of a living agent's energy across three worlds:

| energy | share of samples |
|---|---|
| 80-100 | **97.2%** |
| 60-80 | 2.7% |
| 40-60 | 0.10% |
| 20-40 | 0.00% |
| 0-20 | **0%** |

**Mean energy 96.6 out of 100. Nothing in a settlement ever drops below forty.**

The mechanism is two lines. Eating restores `amount * 20.0` energy, capped at
100 - so a meal of five units refills the entire pool - and `Eat` is **9.85% of
every turn taken**. One meal pays for four pit-diggings, and everybody eats
constantly.

So every `with_energy_cost` in this codebase - forty-odd call sites, with
carefully tuned constants and comments like *"the most expensive single act in
the model, because it is the one that buys a settlement a February"* - is
charging against a pool that is always full. **The effort economy is
decorative.** Making work cheaper in a currency with no scarcity cannot do
anything, and it did not.

#### Reverted, and what it means for the specification

The change is correct modelling and it is not shipping, because shipping an
inert balance change is how a model becomes something nobody can reason about.
Filed as #200 with the probe.

It also relocates the specification's central idea. The user talks in hours -
"eight hours of work", "reduces hours of sleep needed to ten", "fifteen minutes
to walk". This model's currency for time is the **turn**, and almost every
action takes exactly one turn whatever it is done with. So a tool's yield
multiplier *is* the time economy already: gathering four wood a turn instead of
two is eight turns of work instead of sixteen, and that part works today.

What does not exist is the reckoning - nothing compares the turns an upgrade
costs against the turns it saves - and that is #194, which is the piece of the
specification with no equivalent anywhere in the model. `Excavate` is the one
place where a tool genuinely buys nothing at all, in either currency, and wants
either a yield or a duration.

#### The rest of the specification, filed rather than guessed at

#193 the trip as part of the errand's cost; #194 the upgrade reckoning;
#195 the tool ladder the spec asks for (sling, bow, pole, net, shovel, wheel,
and a flint tier between stone and metal, plus butchering as a hard
requirement); #196 barter for a tool as the cascade's third arm; #197 tool
ownership and lending; #198 anticipating a loss of drive satisfaction rather
than reacting to it; #199 specialisation into trades. #190 - fetching the
material a tool wants - was already open and is the next link after #69.

### 71. A man in a meadow with no stone, who knows how to knap a knife

The second link of the preparation cascade the specification asks for, and the
residue #69 left behind.

#69 turns a turn about to be refused for want of a tool into a step towards
making the tool - but only when a step **can be taken right now**. Past that
the chain is short of something that has to be *found*, and a man who knows
how to knap a knife and is standing in a meadow with no stone gets no further.
Measured: **1,690 short-handed refusals a world remained**, down from 2,695,
and they are all this case.

#### The machinery existed and was in the wrong place

`Agent::what_i_must_find` - the raw thing a tool's chain is waiting on - has
been in this codebase since #175 and sits at the **bottom of the Utility
chain**, behind working, vessels, crafting, trading, stooping, and taking from
somebody. Seven branches, on a drive that rarely wins an argument with Hunger.

That is the same defect `Craft` had, one link along, and it wants the same
answer, which #69 already wrote down:

> Reaching for a tool is not what somebody does with a spare moment, it is what
> they do just before using it.

Neither is fetching the stone. `fetch_what_the_making_of_it_wants` sits where
`make_what_this_wants` gives up, and asks `everything_wanting_knowing` what
this particular tool's chain is short of.

Two guards, both learned from #69:

- **Only something he has laid eyes on, and near enough that the fetching comes
  to anything.** Naming a thing this ground has not got trades a refusal for
  want of a tool for a refusal for want of a source, and a refusal is worse
  than a wasted turn because it goes into the record.
- **The substitute is checked for short-handedness itself**, as before.

#### Measured, 32 worlds a side

| | baseline | with the fix | t |
|---|---|---|---|
| short-handed refusals | 1,536 ± 62 | **822 ± 64** | **-8.03** |
| vessels held | 23.9 ± 2.3 | **40.7 ± 3.7** | **3.86** |
| pits dug | 7.6 ± 0.3 | **9.3 ± 0.3** | **4.20** |
| people carrying a knife | 9.3 ± 0.6 | **11.8 ± 0.9** | **2.26** |
| crafts | 320 ± 8 | **367 ± 13** | **3.08** |
| refused workings | 1,175 ± 48 | **669 ± 52** | **-7.16** |
| digs attempted | 306 ± 23 | **113 ± 18** | **-6.63** |
| failure rate | 0.0190 | **0.0163** | **-4.72** |
| deaths | 21.3 ± 0.8 | 20.1 ± 1.0 | -0.89 |
| alive | 47.1 ± 1.8 | 47.0 ± 2.0 | -0.02 |
| eaten | 4,999 ± 191 | 4,875 ± 213 | -0.43 |

The digging row is the same shape as #69's and more so: **a third as many
attempts and 22% more pits.**

#### The two links together

Over the two commits, against where the session started:

| | before #69 | now |
|---|---|---|
| short-handed refusals | 2,695 | **822** (-70%) |
| vessels | 14.2 | **40.7** (x2.9) |
| pits | 5.5 | **9.3** (+69%) |
| people carrying a knife | 3.9 | **11.8** (x3.0) |

#### And survival is flat, twice

This has to be said plainly because it is the second time: alive, eaten and
deaths are **null in both arms**, and in this one alive is dead flat
(t = -0.02) and eaten slightly down (t = -0.43). A settlement three times
better equipped, with 70% fewer wasted turns, does not feed more people.

That is worth taking seriously rather than explaining away. The likeliest
reading is #70's finding from the other direction: the things a tool buys -
more yield a turn, less effort a turn - are not what is limiting this
population. Something else is, and none of the equipment work touches it. The
honest next question is what actually caps a settlement at fifty people, and
it should be asked with an instrument before anything else is built.

### 72. Nobody starves, nobody freezes, and nine deaths in ten are illness

Two capability changes in a row moved no survival column, so #201 asked what
actually caps a settlement at fifty people. **The premise was wrong and the
answer is not what anybody would have guessed.**

#### Nothing caps it

Sampled once a season, one world:

```
t0        12  ######
t2304     17  ########
t4608     23  ###########
t6912     33  ################
t9216     44  ######################
```

The population is **still climbing when the run ends**. Births outnumber deaths
455 to 170 across eight worlds. "Fifty" was the mean *peak*, which is to say
where ten thousand ticks happens to stop. There is no cap; there is a growth
rate, and every measurement in this file that reports "alive" has been
reporting how far a slow climb got.

#### The instrument had to come first, and it was broken

Causes of death were worked out **after the fact**, by asking a corpse whether
it was hungry. By then the hunger has been eaten away and the cold has worn
off, so the honest answer to every question was no: **70% of every death came
out as `unknown cause`**. A settlement could not say what killed its people.

So each thing that takes health now says what it was as it takes it -
`AgentState::lose_health` - and the reckoning reads the record instead of
interrogating a body. Old age is checked first, because it is a fact about the
man and not about the last scratch he took.

Measured at 32 worlds a side: **null on every column** (deaths t = -0.14, alive
t = 0.69, eaten t = 1.48), which is what an instrument should cost.

#### And then the answer, which is one word

| cause | share |
|---|---|
| **illness** | **91.2%** |
| a blow | 5.9% |
| a fall | 2.9% |
| hunger, thirst, cold, exhaustion, old age | **0%** |

**Nine deaths in ten are disease.** Nobody in this model starves, dies of
thirst, freezes, or works themselves to death - not rarely, but *never*. And
illness is not common: **one person in 174 is ailing at any moment.** It is
rare and it is nearly always fatal.

Deaths by season are the other surprise: **Spring 41.2%, Summer 26.5%, Autumn
20.6%, Winter 11.8%.** Winter is the *safest* season in a model that has spent
several commits making the winter bite.

#### Why nobody can starve

The starvation and dehydration thresholds were never rebased when the calendar
was:

```rust
if self.ticks_without_food as f32 > 1440.0 * reserve { ... }   // energy
if self.ticks_without_food as f32 > 4320.0 * reserve { ... }   // health
if self.ticks_without_food as f32 > 10080.0 * reserve { ... }  // death
```

`TICKS_PER_DAY` is **12**. Those numbers were written for a tick of a minute,
so the first of them is **120 days without food** and the last is over two
years. Agents eat every few ticks. **Not one of these branches has ever fired
in a running settlement**, and the dehydration ones beside them are the same.

This is ISSUES #24 again - *"Food was on a clock a hundred and twenty times too
slow"* - in the survival clock rather than the spoilage clock. Filed as #203
rather than fixed here, because making starvation possible for the first time
is a balance change of the first order and wants its own arm.

#### What this means for the equipment work

It exonerates it. #69 and #71 made a settlement three times better equipped and
moved no survival column, and the reason is now plain: **the columns they would
have moved are not connected to anything.** Food cannot save a life in this
model because nobody dies for want of it. Tools, effort, stores and preparation
all feed a survival system whose only live wire is disease.

That reframes the whole efficiency specification. It is worth building for what
a people can *do* - which is what it is really about - but nobody should expect
it to show up in how many of them there are until #203 is answered.

### 73. Four spellings of one clock, and rebasing it kills every settlement

`AgentState::is_starving` read `ticks_without_food > 1440`. The comment beside
it said "after a day". `TICKS_PER_DAY` is twelve. The clock was a hundred and
twenty days, not one, which is why #72 found that in thirty-two worlds nobody
had ever starved, and why the energy clause `|| self.energy < 20.0` was the
only half of that test that could ever fire - and energy, per #70, never falls.

The numbers were `1440`, `4320`, `10080`, `720`, `2160`: true when a tick was a
minute, left behind when #42 put the calendar on a twelve-tick day.

#### They were written out four times, and no two of them agreed

That is the part worth recording, because the arithmetic was the easy half.

1. `age_tick_with_modifier` - the body. Six bare thresholds deciding when
   hunger and thirst take health.
2. `ticks_before_this_kills_me` - the mind. The same five numbers spelled a
   second time, as `2_160.0`, `4_320.0`, `10_080.0`. This is the function the
   whole drive hierarchy ranks needs by: "the drive which will result in death
   the fastest has the highest priority" is reckoned here.
3. `DriveType::base_accumulation_rate` - the schedule. `Thirst => 0.012` against
   a threshold of `0.75`, which is sixty-two ticks from nothing to a drive the
   agent will act on. On no calendar is that a figure anybody derived.
4. `world::nutrition` - `PROTEIN_DEFICIT_ONSET = 1440`,
   `MICRONUTRIENT_DEFICIT_ONSET = 4320`, and a comment that admits the scale is
   stale. Left alone here so as not to confound the measurement; still wrong.

Rebase (1) alone and every agent is dead in five days: the body starts taking
thirst damage at eighteen ticks while (2) still tells the planner death is two
thousand one hundred and sixty ticks away, so thirst never becomes a live drive
and nobody walks the ten paces to the water. Rebase (1) and (2) and (3) and the
thirst half comes right completely - the driest agent in four hundred ticks
gets to six, the median gap between drinks falls from thirty-four ticks to
three, and thirst deaths stay at zero.

#### And then the hunger half kills everybody

Measured at thirty-two worlds a side, ten thousand ticks, twelve founders,
against `1d859ca`:

|                | baseline | rebased | t |
|---|---|---|---|
| alive at end   | 50.9 +- 2.5 | 1.4 +- 0.2 | -19.5 |
| peak population| 53.1 +- 2.4 | 12.0 +- 0.0 | -16.8 |
| births         | 59.1 +- 2.9 | 0.0 +- 0.0 | -20.0 |
| deaths, hunger | 0.0 +- 0.0 | 8.8 +- 0.4 | +20.8 |
| deaths, thirst | 0.0 +- 0.0 | 0.0 +- 0.0 | 0.0 |
| food gathered  | 273487 +- 8796 | 33099 +- 4130 | -24.7 |

No settlement ever grows past the twelve people it started with. Peak equals
the founder count in all thirty-two.

#### What it is not

Four hypotheses were put up and knocked down, which is worth writing down so
nobody spends the afternoon on them again:

- **Not distance.** Water sits ten paces from the founders and there are
  twenty-one sources; food sits five paces and there are a hundred and
  forty-seven.
- **Not the ranking.** `how_hard_it_presses` is live and multiplies by
  `DriveRank::precedence` - a hundred for a primary drive against one for a
  tertiary - so Hunger does outrank Industry. (A first probe said otherwise by
  comparing how far each drive stood over its own threshold, which is not a
  quantity that means anything across drives with different thresholds. The
  probe was wrong, not the engine.)
- **Not a phantom meal.** `Eat` is the most-chosen action, is chosen on about
  forty-two per cent of turns, and never once fails. Both of its paths reset
  the body clock: the carried path through `eat_food_item`, the foraging path
  through `AgentState::eat` -> `took_a_meal`. (`food_i_ate` counts only carried
  meals, which is why the harness column read zero; that was the instrument
  mismeasuring, not a bug in the model.)
- **Not the size of a meal.** `-0.3` against the hunger drive was a mouthful
  from when a tick was a minute; a tick is two hours now. Raising it to a meal's
  worth, and a drink from `0.5` to `0.8`, moved almost nothing: ten to twelve
  of twelve still died across six worlds.

So agents choose to eat, more often than subsistence needs, succeed every time,
reset the clock every time, and still reach three days empty and die. The ones
who eat and the ones who starve are different agents, and what the second group
is doing instead - `Gather` takes as many turns as `Eat` and puts no food in
anyone's inventory - is where this goes next.

#### Not shipped

The change is reverted. The clock is provably wrong and the fix for it is four
lines of arithmetic, but a settlement that dies in its first month is strictly
worse than one that cannot starve, and making the world survivable at the true
clock is a recalibration of the action economy rather than a rebase of a
constant. It should not ride along inside one.

The instrument is real and the numbers above are the baseline for whoever takes
#204 and #205.

### 74. The body on a clock of minutes, and the three things that were hiding behind the broken one

#73 established that the survival clocks were a hundred and twenty times too
slow and that rebasing them killed every settlement. The specification that
followed settles the units: a tick is a minute, so a day is 1440 of them, an
adult dies of thirst in three days (4,320) and starves in three weeks (30,240).

The decision loop does not have to run at that resolution and should not - a
turn is a decision, and twelve decisions a day is the calendar this model has.
So the body keeps its own clock in minutes and `MINUTES_PER_TURN` of it passes
every turn, derived from `TICKS_PER_DAY` so it follows the calendar rather than
being told about it. `agents::physiology` holds it: hydration with the four
capability bands, a stomach of 600 units emptying into the gut on the stated
eight-stage six-hour schedule, a day in the gut before anything is worth
anything, a reserve of three weeks' burn, caloric density off the food, and
exertion taken from what each action cost. Twenty tests assert the
specification's own numbers rather than the implementation's.

Wiring it in exposed three defects that the broken clock had been hiding.

#### Every world began with twelve newborns

`LifeStage::from_age` calls anything under five hundred an infant, and founders
were spawned at age nought. So every settlement started as twelve babies with
nobody to feed them, and none of them reached `Adult` until tick 2,501 - a
quarter of a ten-thousand-tick run. Each carried an infant's reserve while
foraging for itself. On a real clock that killed every world in six days.
Founders are grown people now; newborns come through `give_birth` and are
untouched.

A small body also burned an adult's 1,440 a day, which is what made the infant
case fatal rather than merely hard. It now burns the three-quarter power of its
size, so a child eats a third of what its father eats rather than a quarter -
and a famine still takes the young first, for the right reason.

#### A satisfied need read as a mortal emergency

`A_LONG_WAY_OFF` is divided by the time left to live to decide whether a need
is shouting. Its doc comment reads: *"Half a day... a satisfied need scores
about a seventh, a need a day out scores a half, and one twelve hours off
starts taking the agent over."* Every figure in that sentence is exactly right.
Only the unit was stale - it was written when a tick was a minute, so half a day
was 720 of them.

Once `ticks_before_this_kills_me` answered off a real body, thirst at a full
skin came back as thirty-six turns rather than four thousand, and against 720
that scored twenty. A perfectly watered agent was permanently dying of thirst,
Thirst maps to `Gather{water}`, and settlements spent **ninety-two per cent of
every turn at the water**: 2,149 Gathers against 181 Eats, while starving.
Derived from the calendar now.

#### An empty stomach could not make an agent hungry

Hunger was weighted evenly between the stomach and the gut. A body eating once
a day keeps five hundred units in the gut at all times, which pinned that term
at nothing and capped hunger at six tenths against a threshold of seven. And
ordinary forage was priced against the middle of the food database - forty -
when berries and roots sit at twenty to thirty, so everything actually being
eaten was worth half a unit.

#### Where it stands

Fixed, the model works: three meals a day, one every 3.9 turns, exactly as
specified; hydration holding between 0.89 and 0.95; the reserve near full;
eleven of twelve alive at 1,200 turns with a full spread of work - eating,
gathering, sleeping, sheltering, storing, talking, fishing.

Then winter arrives. At about turn 1,000 the gut goes from 2,300 units to
forty-four and hunger jumps to 0.90, and over a ten-thousand-turn run every
settlement still dies. That is a food-supply and storage failure rather than a
physiological one - #148 and #198 - and it is the first time anything in this
model has been able to show it, because until now nobody could starve.

**Not finished.** Twenty-six tests in the suite still encode the old model and
need rewriting against the body; the ones that set `ticks_without_food` in
minutes were rewritten mechanically, and what is left needs judgement. See
#206 and #207.

### 75. Enough for the day, the week, the month, the winter

Four horizons, each further out than the last and each less frightening to
fail: no food for tonight is extreme, no week is high, no month is medium-high,
no winter is medium. `agents::provision` holds it. It comes out as one number
and that number is the Preparedness drive, which already knew how to put food
by; what it does to the drives above it, it does through the chain gate, the
same as any other unanswered need.

Three things an agent works out for itself rather than being told. What it eats
in a day, kept as a running average of what its own body actually burned, so a
big agent working hard lays in more than a small one resting. How long a winter
is, counted by living through them, with the calendar's answer used only until
it has seen one out. And what a forage costs: the walk both ways, plus the work
of getting that particular food - a carcass wants butchering where greens are
picked off the hedge - where it had been a flat five whatever the agent did, so
a patch across the valley cost the same as the bush at the door.

#### The calendar has no months in it

A season is twenty-four days and a year is four of them, so four actual weeks is
*longer* than the winter the month rung is supposed to sit inside. The ladder
inverted: an agent with more than a month put by already had more than a
winter, and the winter rung could not be reached at all. The rung is half a
season now - one day, seven days, twelve days, a winter - and a test asserts the
ordering so it cannot invert again.

#### A secondary need was outranking a primary one

Preparedness on a settlement that can never quite lay a week by goes unanswered
for thousands of turns, and `pressure()` grows without bound on a denied drive.
Its urgency passed ten, at which point the ten-against-a-hundred band gap that
exists precisely so that "no amount of wanting a fine coat outweighs being
thirsty" stopped working. Agents walked away from the water to go on gathering.
Urgency is capped just under the band ratio now.

#### And bodies drink before they go short, not after

Thirst reached its drive threshold at four fifths of a full body - by which time
an agent had spent half a day on something else and might be a long walk from
the water. Seven founders in twelve died of thirst in spring, in a world with
twenty-one springs in it that never once ran dry. A body wants a drink at
ninety-two hundredths now, which is what people actually do. Thirst deaths
across six worlds went from nine to thirteen a world down to nought to two.

#### Measured, and what is still wrong

Hunger is now the only thing killing settlements, and it still kills them: ten
to twelve of twelve founders over a ten-thousand-turn run. The store reaches
five to nineteen items where it reached one or two, so the stockpiling is doing
something, but nothing like a winter's worth.

#### And then: a trip to a berry patch brought back one berry

The reason a settlement could not lay anything by was not the deciding. It was
the yield. In the gather table every edible thing was `=> 1`:

```
ResourceType::Wood => rng.gen_range(1..=3),
// An armful at a time, like wood: a garment's worth of
// flax one stem per trip is a week's work
ResourceType::Flax | ResourceType::Cotton => rng.gen_range(1..=3),
ResourceType::Stone => rng.gen_range(1..=2),
ResourceType::Iron => 1,
ResourceType::Food => 1,
```

The comment against flax is the argument, written out, for exactly the thing
food was not getting. One item is one portion, and a body eats three portions a
day - so a trip brought back a third of a day's food for a day's walking. A
settlement gathering like that has no surplus, ever. The Preparedness drive
knew it wanted a winter store, `putting_food_by` knew how to bury one, and
there was never anything in a pack to bury.

Food, greens, roots, grain and herbs come back by the armful now, three to six,
and what actually limits a trip is what a person can carry - `add_item` already
refuses what will not fit, so this is a ceiling on the picking rather than on
the carrying. The winter store went from two-to-ten items a world to
eighteen-to-thirty-four, and settlements started producing births.

Two things had to be right first, and one of them was another duplicated
vocabulary - the eighth. `count_food_in_inventory` counted a hand-written list
of seven item ids, and a forager's pack holds none of them: greens, roots,
grain and herbs go in under their own names and were invisible to it, so a
settlement with twenty-six items of food in hand counted fourteen. It asks
`is_food` now, which is the same question `what_food_i_can_spare` asks, so the
counting and the deciding agree.

#### The gap was measured wrong, and the store works

The twenty-five-fold gap reported against the previous commit was arithmetic on
a bad number. Eighteen-to-thirty-four items was a *winter average over a dying
population*, not the store at its height. Sampled across the year, the pit is
plainly doing its job: nought through spring, a little in summer that gets
eaten, then a hundred and eighty in autumn, held at two hundred and eighteen
through winter and drawn down to a hundred and seventy. The right shape
exactly. The real shortfall is three or four to one, not twenty-five.

Two hypotheses about where the food went were tested and disproved rather than
acted on. It is not going into `world.storehouse_inventory`, the global bag
`Action::Store` deposits into: measured across a year that holds **zero** food,
because `what_i_can_spare` excludes anything anybody eats, exactly as its
comment says. And the settlement is not idling - `Gather` and `Eat` together
are seventy-three per cent of every turn, so locking the discretionary drives
behind Preparedness would free perhaps eight per cent of turns and could not
close three to one.

#### What you cannot carry stays where it fell, in the one place it did not

`node.harvest()` takes the food out of the world before anything asks whether
the person picking it has room. When `add_item` refused, the branch returned a
failure and **what had been picked was destroyed** - not returned to the node,
not left on the ground. Gathering by the armful with full packs was quietly
deleting food. ISSUES #165 states this principle and never reached this branch.
It goes back on the node now.

#### A basket in the season it bears

`when_it_bears` says "autumn is when everything else comes on at once", and a
day spent on a hedge in full fruit is not the same day's work as one spent on a
picked-over one. Gathering paid an armful either way. In the season a thing
bears it is a basket now - eight to fourteen - and out of it a handful.

That is the whole margin a settlement has. At an armful a trip, a band already
spending three quarters of its turns on food could feed itself and never bank a
winter, however much it wanted to. The store went to sixteen-to-forty-six items
a world and settlements started producing births.

#### Berries were already autumn-only

Asked for, and already true: `is_it_bearing` puts Food, Grain and Honey in
autumn alone, and out of season what is on the plant falls off rather than
merely failing to regrow. Measured over a year: berries nought in spring after
the first fortnight, nought all summer, a hundred and sixty to four hundred and
fifty through autumn, nought again by midwinter. The one exception is that a
world is seeded with fruit on the bushes whatever season it starts in - two
hundred and thirty berries in spring, which then fall off. Filed as #208.

#### What is still wrong, and one thing tried twice and reverted twice

Settlements still die, of hunger, over a full run. The store is real now but a
winter for twelve people is four hundred thousand food units and eighteen items
is sixteen thousand.

Letting Preparedness go and *gather* food when there is none in the pack to
store was tried twice and measured clearly negative both times - the second
time guarded so it could only fire on a full stomach. Deaths came sooner and
the store fell, because a drive that always has something to do takes the turn
that would have been a meal. It reads as the obvious missing link and it is
not one. Filed rather than shipped.

### 76. Nobody could breed, because a well-fed body gets hungry three times a day

Peak population was the twelve founders in every world ever measured. The cause
is one clause of one gate.

`expects_to_be_able_to_feed_a_child` passes if there is food in the pack for
two **or** if feeding itself has simply not been a problem for a long stretch.
The second was `how_long_food_has_been_easy() >= SETTLED_ENOUGH_TO_GROW`, and
`how_long_food_has_been_easy` was the Hunger drive's `answered_ticks` - how many
turns in a row hunger had stayed below its threshold. Twenty days of it.

That was a fair reading of "never once going short" when hunger accumulated at a
rate somebody had chosen. It is not one now. Hunger is read off the stomach, and
a well-fed body crosses that threshold three times a day, because **that is what
three meals a day is**. The counter reset every few hours and could never reach
twenty days for anybody, ever.

Counted over three thousand turns:

| | adult-turns |
|---|---|
| examined | 24,260 |
| "food not easy for 240" failed | **24,229** |
| `food_put_by < 4` failed | 22,054 |
| both food clauses failed | 22,023 |
| passed the gate | 1,655 |

Ninety-nine point nine per cent. The clause was not selective, it was
impossible, and it reduced the whole gate to "is there a full pack", which
fails ninety-one per cent of the time in a settlement that eats what it picks.

The body already keeps the record the clause was reaching for. A reserve is
three weeks of food; one that has been eating enough to stay topped up carries
it nearly full, and one that has been scraping has drawn it down. So
`food_has_been_easy` asks the reserve instead, and nothing has to remember
anything or survive a bad afternoon.

The gate now passes forty-two per cent of adult-turns rather than six point
eight, and peak population passed twelve for the first time - thirteen in three
worlds of six. It is not growth yet: births run nought to four over a full run
against twelve to sixteen deaths, and `food_put_by < 4` still refuses ninety-
three per cent of the time, which is the same surplus problem as #75.

### 77. A body with nineteen days of food inside it read as starving

`Physiology::is_starving` was "nothing in the stomach, nothing in the gut, and
any of the reserve at all drawn on". The last clause was there only to stop a
newly made body - which starts with both empty - reading as starving, and it
excluded nothing else: every body that has lived a day has drawn on its reserve.

So the test reduced to the gut being empty, which is about thirty hours since
the last meal, and that is what a missed meal looks like. Measured over five
runs it fired on two to six per cent of adult-turns; **thirteen to nineteen per
cent of those were bodies carrying more than three quarters of a three-week
reserve, and three to four per cent more than nine tenths of it.** Sixteen and
nineteen days of food inside them, called starving because the gut happened to
be empty - and `immediate_needs_met` reads `is_starving`, so it was one of the
things telling a well-fed settlement not to have children.

It now asks how far into its reserve a body has actually eaten, in days rather
than in a share, so the same question means the same thing for a child as for
its father. Three days: the gut is only thirty hours empty by the time a body
is a day and a quarter in, so it cannot fire on a missed meal, and three days
without food is starving on anybody's reading. It fires on 0.3 to 1.8 per cent
of adult-turns now, and never on a body with more than nine tenths of its
reserve.

A correction to what was said against the previous commit: the twenty-one per
cent figure reported there was a single collapsing world, not the general case.
Across five runs it was two to six. And the other half of `AgentState::is_starving`,
`energy < 20.0`, was measured firing **zero** times in twenty thousand
adult-turns - energy is still never scarce, as #70 found.

### 78. Milk went into a field that nothing reads, so every child ever born died as an infant

Asked what is preventing a live settlement from forming, and traced the whole
cohort - who was born, how old they got, what stage they died at.

```
t=    0 alive=12  Adult 12
t= 1000 alive=12  Adult 12
t= 1500 alive=10  Adult 8, Infant 2      (3 born)
t= 2000 alive= 2  Adult 2                (4 born)
t= 2260 alive= 0                         (4 born)

  4  born here died as Infant at age 0
  5  founder died as Adult at age 4500
  3  founder died as Adult at age 5500
  ...
```

**Four born, four dead as infants.** Not most of them: all of them, in every
world measured. Nothing that has ever been born in this model has reached its
first birthday.

#### The infants were eating, and it was not enough

They forage from the hour they are born - thirty and thirty-nine meals in a
fortnight - and their reserve falls anyway, seven and a half thousand units down
to three thousand. The arithmetic is against them: an infant's stomach holds a
quarter of an adult's, a hundred and fifty units, while it burns thirty-five per
cent of what an adult burns. So it needs **three and a half meals a day against a
grown woman's two and a half**, and has to fetch every one of them itself.

That is right, as physiology. Babies do feed more often than adults. What is
missing is that somebody feeds them.

#### And there is a nursing system, and it feeds them nothing

`process_nursing` is live and complete: it finds infants, checks whether a
caregiver is within reach, calls `nurse()`, tracks `ticks_since_nursed`, and
takes health off one that goes unnursed. For the fed case, the whole of what it
delivers is:

```rust
agent.state.energy = (agent.state.energy + NURSING_ENERGY_GAIN).min(100.0);
```

Five points of `energy` - the field #70 measured as never scarce, and which
fires in `is_starving` exactly nought times in twenty thousand adult-turns.
The stomach, the gut and the reserve, which are what starvation is now reckoned
on, never saw a drop. A nursed infant was fed nothing at all.

Nursing puts milk in the stomach now, a mouthful at a time so a full child stops
and a hungry one takes another, at twice the richness of ordinary forage -
which is the whole reason a child can live on it with a stomach a quarter the
size. And it is charged to the woman: a nursing mother eats for two, and where
there is not enough for two she is the one who goes short, so a hungry season
tells in the next generation rather than only in this one.

Children born in a world now reach Child, Adolescent and **Adult**. Peak
population passed twelve.

#### What is still preventing it

The second half of the cohort trace, which is not fixed. Founders die as Adults
at ages four and a half to six and a half thousand against a `max_age` of nine
to eleven thousand - **about half their span** - and the causes are hunger and
starvation. A settlement whose grown people die at half their age needs a birth
rate it has not got. Deaths still run twelve to thirteen a world against one
birth. That is #75's surplus problem, and it is now the only thing left in the
way.

### 79. The real calendar, and what it does to a year that was tuned for a short one

The calendar is now the one that was specified: a tick is a minute, 1440 to the
day, thirty days to a month, twelve months to a year, three months to a season.
Weeks alternate seven days and eight, which is the only way four of them make
thirty, and it is why a season is exactly twelve weeks with a fortnight at each
end - **early**, **deep** and **late** - and eight weeks between. 518,400
minutes to a year and 36,288,000 to a seventy-year life. Six tests assert every
one of those figures.

The decision turn stays separable. A turn is when somebody stops to think, not a
minute of living: `MINUTES_PER_TURN` is derived from the calendar, so the body
runs on the specified clock whatever the decision loop does. At one turn a
minute a life is thirty-six million decisions, which is the calendar as written
and is not a thing anybody can run.

Three tables replaced three guesses:

- **What a body eats**, year by year: a fifth of an adult's food to four, then
  rising to a full share at sixteen. The reserve used to be sized in five crude
  bands with the burn as its three-quarter power - the right shape for real
  animals, and a stand-in for a figure nobody had given. It made a child need
  more meals a day than its father while carrying a quarter of the stomach to
  take them in.
- **What a body can do**, from one at two years to ten at sixteen, holding to
  thirty-nine and falling away after. Not yet wired to anything - see #212.
- **How fast hunger rises**, as the product of three step tables on the reserve,
  the stomach and the gut.

Life stages are in years now rather than turns, on the specified bands, and
death comes at seventy exactly rather than from a range.

#### The hunger tables are a rate, and reading them as a level kills everybody

They are headed "Hunger Drive Increase". Read as a level - as the drive's value
rather than its climb - the gut table says a body with a day's food behind it is
never hungry at all, so it stops eating until the gut runs dry and then it is
too late. Measured that way, every settlement died twice as fast as before.

#### And the food year is now wrong

Seasons went from twenty-four days to ninety. `when_it_bears` gives greens to
spring, roots to summer, everything else to autumn and nothing to winter, which
is a lean stretch at twenty-four days and three months of a single thin food at
ninety.

Measured: founders now die in **spring**, from day forty-two of ninety, of
hunger - not in winter. Spring gives only greens, whose caloric density is six
against ordinary forage's twenty-five, so a body eating nothing else wants four
times the volume. Twelve of twelve, in every world.

That is not a reason to shorten the calendar back. It is the world's food supply
being wrong against a year that is now right, and it is filed as #209. The test
suite went from fifteen failures to thirty-three, and most of them are the same
thing: tests written against a year of ninety-six days.

### 80. Spring was never short of food; the only path food had out of the ground ate what it took

"Why are the greens running out?" They are not. Measured over a thousand turns
with `examples/_debug_spring.rs`, greens held at two hundred and sixty-nine to
three hundred and eighty-one nodes' worth while the settlement went from twelve
alive to one. Fish was seventy-four per cent of the world's standing edible
stock - three thousand and seventy-two units across forty-two nodes - and the
`Fish` action was taken **seventy times out of seven thousand seven hundred**.
Fifty-two animals were never hunted at all. Nothing ran out.

Three separate things were wrong, and only the third was the settlement-killer.

#### Leaf is a quarter of a food, and both food-choosers picked the nearest thing

Greens are energy six against ordinary forage's twenty-five, so
`how_rich_this_food_is` puts a unit of leaf at 0.24. A stomach holds six hundred
units and empties in six hours, so the most a body can physically take in is
about two thousand four hundred units a day - **five hundred and seventy-six
energy against the fourteen hundred and forty it burns**. A body living on
greens starves however many greens there are.

Both loops that pick food - the one behind `Action::Eat` and the one behind
`Action::Gather` - took the nearest edible node and never asked what was at the
end of the walk. With leaf the commonest thing growing, the nearest edible thing
was almost always leaf. Both now weigh richness against
`provision::what_foraging_costs`, so a root patch across the meadow beats a leaf
underfoot. Measured over thirty-two worlds at the checkpoint where the two
differ, the Gather half is worth about one more person alive at a thousand turns
and nothing either way elsewhere; the Eat half is most of the gain.

Spring also now gives roots as well as greens - cattail and dandelion are dug
when the top growth is young and the root still holds last year's store, which
is exactly what makes them worth digging before anything has ripened - and there
are three times the greens nodes and twice the roots nodes there were.

#### The Eat action picked one berry and ate it standing there

`Action::Eat` harvested `1` from the node it had walked to, ate it, and went home
empty-handed. Instrumented directly with `food_items_into_packs`: over four
hundred turns, **two thousand one hundred and ninety-nine gather trips put wood,
cotton, clay and iron into packs and not one item of food**, because the only
path food ever took out of the ground was the Eat branch and the Eat branch ate
what it took.

So nothing was ever carried, so nothing could ever be stored, so no pit ever
held a winter, and every single meal cost a walk. The Gather branch had been
taught the armful - "a forager strips a bush, they do not pick a single fruit
and walk home" - and the Eat branch had not. The same lesson written down twice
and applied once, which is defect #3 in this document's list for the ninth time.

A meal now strips the patch: one portion goes down on the spot and the rest of
the armful goes home in the pack, where `find_best_food_to_eat` finds it next
time at a cost of one instead of a walk and `what_food_i_can_spare` finally has
something to bury. What will not fit stays on the bush, per #165.

On its own this **made things worse** - thirty-two worlds, mean last-alive fell
from 1551 turns to 878 - while raising the peak larder sevenfold, from 29 to
204. A settlement that carries food and does not eat it dies with a full pit,
which is what the next section is about.

#### And the real killer: a body that ate 2.26 times a day and needed three

`UNITS_IN_A_PORTION` is a third of a day exactly, so three portions is a day's
food. Measured with `examples/_debug_body.rs`: two hundred and seven meals per
agent over ninety-one and three-quarter days - **2.26 meals a day** - and an
intake of one thousand and seventy-three units a day against the fourteen
hundred and forty burned. The reserve fell from 30,180 to 5,023 in a straight
line and everybody was dead by about a hundred days.

Nothing was stopping them. Turn budget was not short: gathering took fifty-two
per cent of turns and eating twenty. Food was not short: they died in a spring
holding six thousand two hundred and fifty-three nodes' worth of edible stock.
Eat never failed once in a thousand turns. **The body simply never asked for the
third meal.**

`base_accumulation_rate` gave Hunger `0.01`, a number picked against a calendar
that no longer exists. At an ordinary product of the three hunger tables that is
about seventeen turns - a day and a half - to climb from nothing to the
threshold. A body that wants a meal every day and a half eats two-thirds of what
it burns and starves in slow motion with its larder full.

The rate is derived now, from the clock the stomach already keeps: a meal holds
for as long as the stomach takes to empty, which is the last entry in
`HOW_THE_STOMACH_EMPTIES` (six hours, three turns), so that is how long the
drive takes to climb its threshold at the ordinary product of the tables. A body
behind on its reserve or empty in the gut climbs faster, which is what the
tables are for.

Measured, thirty-two worlds, two thousand turns, against the commit before:

| | mean last alive | alive at 1000 | alive at 1500 | peak larder |
|---|---|---|---|---|
| before | 1551 | 4.5 | 0.9 | 29 |
| armful only | 878 | 0.4 | 0.0 | 204 |
| **armful and derived rate** | **1878** | **6.3** | **3.6** | **260** |

Intake went from 970 units a day to 1,281-1,503 - the fourteen hundred and forty
the specification asks for, landed on from the arithmetic rather than tuned to.

Founders also now walk in with two days of food, counted off
`UNITS_BURNED_IN_AN_ORDINARY_DAY` rather than picked, so that a people arriving
in a valley is looking for good ground rather than for tonight's supper. Two
runs of thirty-two worlds put it at a wash for survival (1799 and 1922 against
1878 without), which is what two days should be worth.

#### What is still in the way

Nobody survives a year yet. Sixteen worlds run to six thousand turns leave a
mean of one person alive at four thousand, and `dropped` - the armful that would
not fit in a full pack and went back on the bush - reaches eight thousand eight
hundred items by turn two thousand four hundred while the pit sits at a hundred
and fifteen and never grows. The larder is filled and eaten at the same rate.
That is the next thing to measure and it is #213.

### 81. The store is laid down and eaten at the same rate, so it is never a winter store

Over twenty-five hundred turns the pit holds between a hundred and six and two
hundred and one items the whole way through and never accumulates, while nearly
nine thousand items of harvested food are thrown back on the bush because packs
are full. Somebody who cannot carry an armful home should be emptying their pack
into the pit and going back out; instead `Store` takes two hundred and fifty-two
turns out of seven thousand four hundred and eighty-four. Not investigated yet.

### 82. A unit of leaf was worth a quarter of a unit of food, not six units of energy

"If a green supplies 6 energy per unit, and the agents can eat a maximum of
2,400 units of food per day, how come they are not reaching the 14,400 energy
units that they are capable of?"

Because `how_rich_this_food_is` divided the food's energy by
`ENERGY_OF_ORDINARY_FOOD` before anything else touched it. A unit of leaf came
out at **0.24** rather than **6** - a twenty-five-fold understatement - and
every food in the database was scaled the same way, so nothing thin could ever
reach maintenance however much of it there was. The previous entry's conclusion
that "a body living on greens starves however many greens there are" was a
description of that error, not of the food.

#### What pins the scale

The fishery, which was specified with numbers that close on themselves. A fish
every two hours is one a turn on a twelve-turn day; a fish is four to six units;
a unit of fish is twenty-five energy. That is 1,200 to 1,800 energy a day
against the 1,440 a body burns, so **a people can live by fishing and it is a
full day's work** - which is what was asked for. Measured, the fishery already
delivers that rate: two fish a successful cast at about even odds is one fish a
turn.

Everything else follows from it. An item in a pack is a handful -
`UNITS_IN_ONE_ITEM`, five units - rather than a third of a day. What a handful
is worth is its own food's energy: a handful of fish is a hundred and
twenty-five, a handful of leaf is thirty. A sitting down to eat aims at a third
of a day in *energy*, so it is four fish or sixteen handfuls of leaf, and a meal
is several mouthfuls rather than one item.

This supersedes one line of the earlier specification - "three full meals would
result in an intake of 1800 food, which would exceed the 1440 needed" - which
reads the stomach's six hundred as the same currency as the day's fourteen
hundred and forty. It cannot be both: under that reading twelve fish a day is
three per cent of a day's food and nobody could live by fishing. The fishery's
numbers are later and more specific, so the stomach's six hundred is a volume
and a ceiling on gorging rather than a daily target.

The two hunger tables that read the stomach and the gut were rebased with it.
They were written in units when a meal was four hundred and eighty units
whatever it was made of; read as volume, a body with a full supper of anything
dense in it now reads as empty. They are shares of a sitting and of a day, in
energy.

#### A low reserve keeps asking, whatever is in front of it

"This is why having an internal energy level which is low should still increase
hunger drive, to help the agents eat enough to regain their lost energy stores."

`how_fast_hunger_rises` returned nought flat whenever either gut table read
nought, which cancelled the reserve term entirely: a body three weeks into its
reserve with a mouthful in its stomach was not hungry at all. The two gut tables
damp the reserve now rather than switching it off, so a body at the bottom of
its reserve goes on asking with something in front of it.

#### The patch with a hundred berries

"Why would they go to a berry bush with a single berry if there is another berry
bush with 100 berries?" They would not, and they were: both food-choosers asked
what kind of food it was and how far off it stood and nothing else, so a patch
stripped to its last berry read exactly as well as a full one beside it. A
patch is now worth what one trip can carry off it, at what that food is worth to
eat, less what the trip costs, over the turns the trip takes.

All three terms were measured against each other over thirty-two worlds. Worth
per unit of *effort* is the wrong question - it picks the cheapest trip, so leaf
underfoot beats a river full of fish. Net energy alone is also wrong - a `Move`
action is one tile, so a patch twenty paces off is most of two days there and
back. Worth-less-cost over turns is the one that measured best of the three.

#### What it costs, measured

Thirty-two worlds, four runs: **mean last-alive 986, 990, 1033, 1159 turns,
against 1799-1922 before.** A settlement lives about half as long.

This is committed anyway, and the reason is that the number it replaces is
simply wrong. The old model survived longer because the wrong constraint was
doing the work: at four hundred and eighty units an item, a six-hundred-unit
stomach held one item and a day's throughput was five of them, so a body needed
to *acquire* five things a day and digestion was the ceiling. At five units an
item it needs to acquire eleven or twelve, and acquisition is the ceiling. The
first is a shorter life on a right number; the second was a longer life on a
number twenty-five times out, and everything built on top of it would have
inherited the error.

#### What is actually in the way now, and why it is the next thing

Acquisition, and specifically the tension between density and distance.
Measured, a settlement eats thirteen handfuls a day - close to the twelve or
thirteen it needs - but they are the wrong handfuls: leaf at thirty rather than
fish at a hundred and twenty-five, so the intake is around a thousand energy
against fourteen hundred burned.

The chooser cannot fix that on its own. Weighting density harder sends agents to
the river and measured **worse**, because a walk is one tile a turn and the
walking is re-decided every tile: `Move` runs at a third of all turns and
`SeekShelter` at another fifth. A twenty-tile trip to good food is twenty
separate decisions, any of which can be overridden by whatever drive is loudest
that minute, so the trip is rarely completed and the agent eats whatever is
underfoot when it gives up.

That is the commitment problem, and it is #214: "once an agent plans an action,
it would not change its mind unless its situation changed in some manner." A
settlement that can walk to the river without re-deciding at every step can eat
fish; one that cannot is stuck with leaf whatever the chooser says.

### 83. Every tile of every walk was a fresh decision, so no walk ever finished

"Once an agent plans an action, it would not change its mind unless its
situation changed in some manner. For example, an agent wants to walk to get a
drink of water and the trip takes an estimated 10 minutes one-way. The agent
begins walking and for the next ten ticks no new decisions need be made... An
agent spending all day digging a pit need only make decisions to eat and drink
as those drives increase, before returning to dig."

A `Move` action is one tile, and the whole decision - which drive, which patch,
which route - was re-derived from scratch at every one of them against a world
that had shifted a step. A walk to a fish run twenty tiles off was twenty
chances for whatever drive was loudest that minute to send the agent somewhere
else. Measured, `Move` ran at **a third of all turns** and most of the trips it
was made of did not finish; agents ate whatever was underfoot when they gave up.

That is why #82's patch-chooser could not be made to prefer good food. Weighting
density harder sends agents toward the river and measured *worse*, because
"toward the river" was never "at the river".

An agent now holds an **errand** - where it is going, and which drive it set out
to answer - and keeps to it. What ends one is a change in what the agent needs,
and there are four: it arrives, something frightens it, a different drive takes
over, or the walk runs so far past what the distance was worth that the place is
plainly not reachable. Not on that list: a nearer patch coming into view, this
turn's dice, or the same drive pressing slightly differently.

A bare "is a different drive at the head of the queue" test abandoned **58% of
errands mid-walk**, because the top two drives trade places almost every turn as
one is nibbled at and the other builds. Turning somebody round now takes a drive
pressing a quarter again as hard as the one they set out on.

Measured over thirty-two worlds, two runs a side:

| | mean last alive | peak larder |
|---|---|---|
| before the errand | 1278, 1279 | 159, 183 |
| errand, bare comparison | 1490, 1529 | 224, 189 |
| **errand, with the margin** | **1633, 1584** | **220, 201** |

`Move` fell from a third of all turns to an eighth, and `Eat` rose from a fifth
to a quarter. Of 392 errands set out on in a 1,200-turn world, 158 arrived, 227
ended because a drive took over, and **4** were given up as unreachable - the
backstop is a backstop rather than the usual ending, which is what
`errand_tests` asserts.

### 84. A low reserve should bring the next meal forward, not eat through a full stomach

The previous entry had a low reserve go on raising hunger *through* a full
stomach, on the reading that the reserve is the term that kills and must not be
cancellable. That is wrong on its face and was corrected:

"Hunger drive should not increase with a full stomach with a low reserve, but
should increase as the stomach empties. Basically, a low reserve should force an
agent to eat sooner instead of eating while full. Instead of waiting for their
stomach to become empty, an agent might get hungry while their stomach is still
half full."

A body cannot answer a hunger it has no room for, so all a drive rising against
a full stomach buys is turns spent on meals that will not go down. What the
reserve changes is **when full arrives**: a body with its reserve intact waits
until its stomach is down to a tenth of a sitting, and one that has eaten its
reserve away is hungry again at three quarters. The gut table moved the same
way - an ordinary body is settled by a day's food behind the stomach, a spent
one wants two.

Measured over thirty-two worlds: mean last-alive **1278 and 1279 against about
1000** for the version it replaces, and it is the more consistent of the two as
well as the better.

### 85. Two hours making a better axe to save six cutting - and there was nothing to make

"The agent should look at the drive, their skills, the availability of tools to
decrease time, if they need to make any tools, and decide the quickest method of
satisfying their most important drive." The arithmetic for that is the
efficiency specification's own: "eight hours with this axe, or two hours making
a better one and six with that."

`make_what_this_wants` already rescues a job that *cannot* be done without a
tool. The case the model had never had is the other one: the job is perfectly
possible, and doing it badly for the rest of the season is the more expensive of
the two. `Tool::how_much_better` has been in the data since the tools were
written, multiplying what came *off* a job and nothing else, so a stone axe and
a bronze axe felled a tree at the same price and nobody ever had a reason to
upgrade.

Every term in the sum is a figure this model already keeps:

- `Tool::how_long_it_lasts` is the horizon, and it is the honest one. A tool has
  to pay for itself inside its **own working life**, so nothing has to be
  assumed about how long the agent will go on wanting the trade.
- `how_much_my_tools_help` is what the work costs now; `how_much_better` what it
  would cost after.
- `how_many_turns_to_make` is new, and prices the chain along the same walk the
  agent will actually take - `step_towards` named the next step and nothing
  about how many followed it.

So: worth stopping when `how_long_it_lasts * (1/now - 1/after)` beats the turns
the making costs. A starving body works with what it has.

#### And it fired zero times, twice, for two different reasons

**First**: it asked `make_what_this_wants` for a step towards the tool, and that
function refuses a `Craft` on sight - it exists to rescue a job that cannot be
done, and a craft that wants a craft is a loop. So the arithmetic said yes a
hundred and sixteen times in a run and got the action straight back every time.
It now walks `what_to_do_first_that_can_be_done` itself.

**Second, and the real one**: there was nothing to upgrade *to*. Every trade
with a tool behind it - wood, stone, hunting, fishing, butchering, crafting -
has one that founders arrive carrying or knowing how to make, and **Herbalism,
which is the trade behind most food gathering and so most turns of most days,
had no tool at all**. `what_i_would_rather_have` returned nothing for every
trade an agent actually practised, and the whole mechanism reached its
arithmetic twenty-one times in fourteen thousand agent-turns.

So the digging stick, which is the oldest tool there is: a stick and an
afternoon, worth half again at getting roots out of the ground, thirty jobs
before it wears out. The point is not the multiplier, it is that the first rung
of the ladder now exists for the trade that fills the day.

With it, the mechanism is live: fourteen turns diverted onto a tool in a
1,200-turn world, with fifty-three more where the sum said yes and the chain was
short of wood.

#### Measured null on survival, and honestly

Thirty-two worlds, three runs: mean last-alive **1612, 1552, 1633** against
**1633, 1584** for the commit before. A wash, and the spread is tight enough
here (±40) to say so rather than to say "within the noise" and hope.

Fourteen diversions a world cannot move a settlement's life and it would be
surprising if they did. What this buys is the mechanism, correct and wired and
measured to fire; what it is waiting on is #195, the rest of the tool ladder -
sling, bow, pole, net, shovel - because a ladder with one rung is a step.

### 86. Sunk cost: a walk nearly finished is worth finishing

"If an agent is a few steps away from getting a meal and hydration drive
suddenly kicks in, then the agent abandoning its current task to get a drink
could waste the invested energy the agent spent to get a meal."

What it takes to turn somebody off an errand now climbs with how much of it is
behind them: a quarter again at the moment of setting out, and up to a
three-quarters again by the time the patch is one pace off.

Deliberately sunk cost, and deliberately not the fallacy of that name - which is
about *money already spent*. The half-made walk is not the sunk part; the sunk
part is the energy, and what the nearness buys is the rest of the trip at a
fraction of what a fresh one would cost. An agent two paces from a meal is two
paces from a meal. One that turns round is twenty paces from the next one and
has paid eighteen for nothing.

It is a multiplier and not a veto, on purpose. `how_hard_it_presses` grows
without bound as a killing drive nears its clock - `1.0 + deadly *
SOONER_IS_WORSE` - so a body that will actually die of thirst still turns round,
however near its supper is. What this stops is a drive merely crossing its
threshold at an awkward moment.

Measured null on survival as well, within the same ±40 band. Both of these are
correctness rather than performance, and the run that shows it is the same run.

### 87. The rest of the tool ladder, and the three things that were stopping anybody climbing it

Sling, bow, rod, net, shovel and the wheel, as the specification asks for them.
Six new steps in the chain, six new tools, and one thing that is not a tool at
all.

The split between what a people arrives knowing and what it has to find out is
drawn at **invention**, not at usefulness. A sling, a line and a hafted blade
are the same three ideas as the handaxe founders already carry, put to other
ends, so they are `obvious`. A bow, a net and a wheel are each a thing somebody
had to think of, so they are not. The whole ladder was found-out at first, and
measured that way **nobody ever climbed a rung of it**: two digging sticks in a
run and nothing else, because a settlement dies at about a hundred days and
discovery is slower than that. A ladder whose first rung is above the ceiling is
not a ladder.

#### The wheel, which is not a tool

Nothing a cart does is a multiplier on a trade. What it does is carry, and
`TransportSystem` has been able to model exactly that since the day it was
written - capacity, speed, durability, twenty-odd kinds of vehicle and pack
animal. `total_additional_capacity` is already summed into `Inventory::max_weight`
and `speed_modifier` is already multiplied into `movement_speed_at_tick`.

**Nothing has ever put a transport into it.** The whole subsystem was tables
with no caller, which is defect #1 in this document's list for the tenth time.
`Agent::take_up_the_cart` is the missing link: a cart in the pack is a cart in
the hand, asked every turn because a cart can arrive by making, by trade or by
inheritance and can leave by wearing out. Seventy-five pounds more and three
tenths off the walking pace, which is the trade a cart has always been - and it
goes at the largest measured waste in the model, which is nearly nine thousand
items of gathered food put back on the bush in a run for want of anywhere to
put them.

#### Three things were stopping anybody making anything

**The shovel had nothing to bite on.** Digging a pit was a flat twenty-two
energy whether the agent dug with a shovel or with its fingers. It divides by
the tool now, and wears the tool out, like every other job.

**A tool is not one turn's work.** With the ladder in and reachable, the
arithmetic diverted turns and produced *nothing*: a diversion buys the next step
in a chain - a length of cordage, a knapped edge - and the turn after that the
whole decision was made again from scratch and went somewhere else, so
settlements collected half-finished tools they never picked up again. Exactly
the defect #83 found in walking, one layer up. `Errand` carries a `to_make` now,
so a making is an errand like a journey: finished when the thing is in the pack,
ended by the same four things that end a walk.

**And the commonest ending was standing still for want of a stick.** A hundred
and three turns in a run where the sum said the tool was worth having and the
chain was short of something that has to be *found*. That is what
`fetch_what_the_making_of_it_wants` is for and the tool path was not calling it.

With all three: fourteen tools finished in a 1,200-turn world against nought, a
settlement carrying rods, shovels, digging sticks and handcarts by day
thirty-three, and an agent actually pulling a cart.

#### It cost turns, and the larder says when that is allowed

Measured with the ladder and no guard: **1630, 1494, 1387** mean last-alive over
thirty-two worlds against **1612, 1552, 1633** before it. About a hundred and
ninety turns a run were going on tools in settlements that needed the turns for
supper.

The guard is the specification's own rule - "once basic survival needs can be
satisfied over the long term, other concerns start coming into play" - and the
question was already being asked every turn. A body on the larder's bottom rung,
`NotTheDay`, works with what it has. `is_starving` alone is too late a test: it
wants three days into the reserve, and a people permanently a third short of
food is hungry long before that and never technically starving.

With the guard: **1613, 1522, 1616**, against 1599 for the commit before - a
wash, with the ladder in and being climbed. Sixty-three turns a run declined on
the grounds that there was nothing in for tonight, which is the rule doing its
job rather than the mechanism failing.

#### One thing found and not fixed

`update_inventory_capacity_from_transport` computes the base as `100.0 *
body.movement_speed_multiplier()`, and calls that multiplier a strength. It is
the leg-health figure. So an agent's carrying capacity is decided by how well it
walks, and taking up a cart recomputes it - a hale agent gets 175 and a
lame one 107. Filed rather than fixed, because carrying capacity wants its own
measurement and this commit has had enough of them.

### 88. The bottom of the ladder: bare hands were a fully competent workman

"Many actions can be completed by the agent, but without tools, these actions
are not very efficient."

`how_much_my_tools_help` returned **one** for every trade with nothing in the
hand, so a man with no tools was as good as a man with the right one and every
tool in the model was a bonus on top of competence. That is why the ladder in
#87 measured null when it was built: there was nothing wrong with the bottom of
it.

Each trade has a bare-hands figure now, and each is the specification's own
reading of the job. Fishing "can be accomplished by hand but is highly
inefficient" - a quarter. Butchering, where "killing any animal without at least
a stone hand axe makes it nearly impossible to eat the dead animal" - fifteen
hundredths. Digging, which "without any tools should take a significant amount
of time" - three tenths. Picking is the exception at 0.85, because hands are
what picking is *for*; what a digging stick adds is roots, not berries.

Every trade with no tool in the world behind it is left at one, or this would
quietly tax half the model for no stated reason.

#### The handaxe does the third thing it was always said to do

"The most basic tool should be a stone hand axe. This tool allows for crude
cutting, digging, and chopping." It was in the tool table for digging and
chopping and not for cutting, so a people with an axe and no flake could fell a
tree and could not butcher what it killed.

#### Nothing in hand, nothing to bring down

"Hunting any larger animal requires at least a spear... Stones can be used to
kill small animals, but slings make stones more efficient." There was no such
line: an agent with empty hands could walk up to an ox. Above twenty health -
which is where the fauna tables put the gap between a hare and a deer - the hunt
now refuses without something in the hand.

Two rungs of spear, as asked: a `sharpenedstick`, which is one length of wood
and an evening at the fire, and the flint-tipped `spear` above it. No arrows for
the bow, which is still the honest gap - a bow that spends ammunition wants a
whole model of ammunition behind it and there is none.

#### The fishing ladder, and a rod that was counted twice

"Fishing can be accomplished by hand but is highly inefficient. Spear fishing is
more efficient, pole fishing is better than spear fishing, and net fishing is
even better." Measured, hands landed 40 fish in sixty casts, a spear 52, a rod
94 - and a net **91**. The ladder inverted at the top.

Two reasons, both duplicated vocabulary. `Action::Fish` had looked for something
with "rod" in its name since the fishery was built and given it a fifth of a
chance of its own; that was written when nothing in the chain made one, so the
branch had never fired, and the moment a fishing rod became a real tool it was
counted twice. And the odds of a cast are capped at nine tenths, so past a point
better tackle cannot land more *often* - which is exactly what a net is for. It
takes several at once, so the catch is in proportion to the tackle now rather
than in two hand-written steps.

### 89. A cart is not the first thing a people builds, and the wheel is the hard part

"A cart should be a rather advanced piece of technology. An initial method of
moving things would likely be more of a travois."

Which is right, and the cart from #87 was standing in the travois's place: four
lengths of wood and a lashing that founders could turn out on their first
afternoon, which put the wheel in the same bracket as a digging stick.

The advanced part is not the cart. A cart is a box on poles and anybody can see
how to build one; what nobody can see how to build is a disc that turns true on
an axle. So `WHEEL` is its own step and is found out, `HANDCART` wants two of
them, and the same wood and lashing without them makes a `TRAVOIS` - dragged
rather than rolled, so it carries less and costs more of the walking. That is
the whole difference between them and the reason one comes first.

A `BASKET` sits below both, at two lashings, and founders arrive wearing one.

### 90. Carrying capacity came from the legs, and the container sweep it wants

`update_inventory_capacity_from_transport` scaled the base by
`body.movement_speed_multiplier()` under a comment calling it a strength. It is
the leg-health figure, so how much somebody could carry was decided by how well
they walked. It reads arms and torso now - `how_much_this_body_can_lift` - which
is what carrying actually is. That was #87's parting find and is fixed here.

The other half is not. "An agent can eat from a berry bush but cannot carry
additional berries unless they are carrying a pack or container. This means that
the act of walking to and from the berry patch each time the agent is hungry
will take additional time, making it less efficient."

That wants a bare-handed figure around a dozen, so a basket is the difference
between an armful and a load. At twelve, **forty tests fall over** - barter,
larder, sprouting, theft, working, portioning, fluids - because every fixture in
the suite was written when a pair of bare hands held a hundredweight, and a
fixture that gives somebody forty stones is testing who wants what rather than
what anybody can lift.

Measured at twelve with a small basket, the settlement measured *better*: 1724,
1625, 1661 against 1613, 1522, 1616. The constraint is worth having and it is
the constraint doing the work, not the tool floors. What is here is a compromise
that keeps the shape and most of the effect - hands at twelve, a basket at
thirty, a travois at seventy, a cart at a hundred and forty - which cost four
fixtures rather than forty. The remaining question is whether a bare hand should
hold twelve or thirty, and it is #216, because answering it properly is a sweep
of the whole suite and would make this commit unattributable.

### 91. What the whole batch measured

Thirty-two worlds, five runs: mean last-alive **1698, 1492, 1729, 1661, 1645**
against **1613, 1522, 1616** for the commit before. Better, and the low outlier
is inside the spread this harness has always had.

Worth recording what did *not* move it. Measured stage by stage: the carrying
change was worth about sixty-five turns, the bare-hands floors about twenty, the
hunting and butchering gates about fifty-five. The floors are the piece that
matters least to survival and most to the model - they are what makes every rung
of #87's ladder worth climbing, and without them the ladder was decoration.

### 92. No run of this model was ever repeatable, and the threat tree decided on a coin

The first half of making the suite trustworthy, and it found a live defect on
the way.

**Nothing was seeded.** Every roll in the simulation came from
`rand::thread_rng()` - eighty call sites across twenty-six files - which draws
from the operating system and cannot be reseeded. Measured over three runs of
the same binary: twenty tests failed every time and **fifteen more came and
went**. A test that fails two runs in three is worse than one that fails always,
because it cannot tell a regression from a coin.

It cost more than the suite. The survival harness reads a mean over thirty-two
worlds with a spread of about a hundred and twenty turns, so a change worth
fifty turns could not be seen without running it repeatedly and squinting -
and at least two judgements in this project's history were made on differences
inside that band and were wrong.

`core::dice` is the answer and it is deliberately small: `roll()` stands exactly
where `thread_rng()` stood, and the stream behind it is thread-local and can be
set. Under test it starts from a fixed number, so every test is the same world
without two thousand tests each having to remember to ask.

#### And then five tests still came and went

With the dice seeded, the same binary on the same seed still gave different
answers - so the residue was not the dice. It was **map iteration order**.
Rust's `HashMap` orders by a hash seeded per *process*, and this model iterates
maps to pick a best: the best food to eat, the tool that helps most, the place
worth walking to.

The worst of them is not a test problem at all. `Emotions::worst_agent` and
`worst_creature` take a `max_by` over `fear_sources` and `anger_sources`, so
**when two things frightened an agent equally, which one it feared - and so
whether it ran, stood or froze - was decided by the process's hash seed.** The
whole threat tree hangs off that answer. Five collections in the decision path
are ordered now: the inventory, the two emotion tables, an agent's skills, and
everything an agent remembers about a place.

Flaky tests: **fifteen to seven**, measured the same way over three runs.

#### What was left, and what it turned out to be

Not finished at this point. A settlement was still not reproducible run to run
- eight worlds gave 1102, 1199, 1104 - and the reading taken here was that the
remainder lay in the **eighty-three choose-operations in `analytics/mod.rs`**,
each a place where an unordered collection could decide something.

That reading was half right and half wrong, and the wrong half is worth
keeping. Right: it is a missing property, not a bug list - *the decision
layer's inputs must have a stable order* - and chasing call sites one at a time
is how three rounds had already gone. Wrong: the remaining faults were not in
those eighty-three at all. Two of the four were randomness taken outside the
stream, one was a `HashMap` in the *fauna registry* four modules away, and the
last was `rand::random()` in the animals. See #94, which finished it - and note
what actually finished it: not more diligence over the same file, but an
instrument that could see across two processes, and then holding the property
with the type system rather than by hand.

## Housekeeping

### 93. Fourteen per cent of the public surface had no caller

A sweep for `pub fn` definitions whose identifier appears nowhere else in
`src`, `plugins`, `examples`, `benches` or `tests` - not "no call site", *no
mention at all* - turned up **326 of them**. Cutting those exposed a second
wave of 24 that only the first wave had called, and a third of private
helpers, statics and one struct that only those had used. Three fixpoint
passes, **357 items and 3,838 lines gone**, and the sweep now reports zero.

The compiler named exactly one false positive in the whole set,
`Action::primary_drive`, used from `tests/environment_plugin_tests.rs` - a
directory the first scan had not been pointed at. Nothing else broke: the four
configurations (default, `gui`, `bevy_gui`, `--workspace --all-targets`) build
clean, and the warning set is byte-identical to what it was before, 55 either
way.

Deleting an uncalled function cannot change what a program does, so the test
suite should have been untouched, and in aggregate it was: 25 deterministic
failures and 7 flaky before, 24 and 7 after. The five names that moved between
the two lists - the winter, thirst, clothing, distrust and survival-pressure
settlements - each flip on their own between runs of the *same* tree
(FAILED/FAILED/ok/FAILED, FAILED/ok/FAILED/ok, and so on). They are #92's
unfinished business, not casualties of this.

#### What was actually in there

Not all of it was clutter, and that is the finding. Some of it was recurring
defect #1 again - a subsystem built, tested, and wired to nothing:

- **Equipment wear.** `tick_all_equipment`, `apply_tool_wear`,
  `apply_combat_wear`, `unequip_broken`, `can_be_repaired`,
  `get_broken_equipment`, `condition_description`, `sharpness_retention`,
  `flexibility`, `mining_speed_with_traits`, `harvesting_speed_with_traits`.
  A complete durability model that nothing ticked. Tools do wear in this
  simulation - through `environment::making`, on a different vocabulary.
  Defect #3 as well as #1.
- **Twenty-one item constructors** in the same file: `iron_dagger`,
  `steel_axe`, `bronze_sickle`, `yew_bow`, `obsidian_dagger` and the rest. A
  second, richer materials ladder than the one the world actually runs on.
- **Precipitation accumulation.** `world/climate.rs` carried a
  `HashMap<(i32,i32), PrecipitationAccumulation>` of snow depth, standing
  water and ground wetness, ticked by weather type, read by `is_flooded`,
  `movement_penalty` and `shelter_quality`. Nothing read the field, so nothing
  ever called any of them. Snow has never lain in this world.
- **Information verification.** `verify_information_from`,
  `receive_information_with_verification`, `prepare_information_to_share`,
  `spread_liar_reputation`, `react_to_trait_info`,
  `process_information_verification`. A second gossip pipeline beside the live
  one from #93-#101.
- **The global plugin registry.** `global_registry`, `has_plugin`,
  `plugin_ids` and the `static mut GLOBAL_REGISTRY` behind them.
- **`physiology::pass_waste`**, which drained a `waste` accumulator nothing
  drained, and `agent::what_a_body_this_age_can_do`, the age capability curve
  that was written and never hung on anything.

The code is in git history where it can be read; what is gone is the
impression that any of it was running. The rest was accessors, `with_*`
builders, and trend-series getters for an analytics UI that reads its numbers
another way.

Two of these are worth wiring rather than rewriting: the age capability curve,
which already has an open task against it, and the equipment durability model,
which should either replace the `making` vocabulary or come out of the design
as well as out of the code.

### 94. The sweep finished: the same seed is now the same world, to the berry

#92 halved the flakiness and named what was left as *eighty-three
choose-operations in `analytics/mod.rs`* - somewhere in there, it assumed, an
unordered table was still deciding something. That guess was wrong in an
instructive way. The instrument was the thing that had been missing, not the
diligence: a harness that runs one seed, prints a fingerprint of the whole
world per tick, and is run **as two separate processes** and diffed. Rust seeds
hash iteration per *process*, so no test inside one process could ever have
seen this.

The first diff put the divergence at tick **-1** - before a single tick had
run. From there it was four faults, each found by the same loop of fingerprint,
diff, narrow.

**One: names came from the operating system.** `Uuid::new_v4()`, 270 call
sites. An id is not a label in this model - it is a map key, a sort key and a
tie-break - so two runs of one seed disagreed about who was who before anything
had happened, and every ordering downstream of an id disagreed with them.
`dice::name()` draws the sixteen bytes from the seeded stream and sets the same
version bits, so nothing that reads a `Uuid` can tell.

**Two: `all_species()` handed back a `HashMap`'s values.** With the ids fixed,
world generation still spent a different number of rolls each time - 480, 464,
544. A draw counter on the stream put it in `spawn_naturalistic`: the herbivore
list came out in a different order, so a different species was picked, so a
different `herd_size` was rolled, so a different number of animals were spawned.
Not a tie being broken by a coin - a *variable amount of randomness consumed*,
which puts every later draw in the run out of step.

**Three, and the reason to stop chasing call sites: every `HashMap` in the
model.** 439 uses across 87 files. Converting them all to `BTreeMap`/`BTreeSet`
took a script, fifteen `#[derive(PartialOrd, Ord)]`s and about an hour, and it
turns the property from a thing that must be remembered at each new
`.iter()` into a thing the type system holds. Two guard tests now keep it that
way. There is a real cost, stated plainly: **the model runs about 20% slower**
(32 worlds x 4,000 ticks, 39.6s to 47.5s). For a project where the binding
constraint has been the trustworthiness of a measurement, that is a good trade.

**Four: `rand::random()`.** Ten sites in the fauna and flora, missed by #92's
sweep because that one looked for `thread_rng()` and this is the same function
under a friendlier name. Every wander an animal takes, and whether it grazes,
rests or hunts. These ten were enough on their own: with everything else fixed,
the beasts still moved differently in every run, and **by tick 49 it had
reached the people** - `Safety` at 0.020 in one run and 0.034 in the other, for
a man who could see one, with every other number about him identical.

#### What it buys

- **One seed, three runs, 4,000 ticks: byte-identical.** Three seeds tried;
  different seeds still give different worlds.
- **Flaky tests 7 to 0.** Same 28 failures in each of three runs of the whole
  suite. Five of the seven that came and went now fail every time and two pass
  every time - which is the point: each of them now has an answer.
- **A measurement is a fact.** The survival harness gave 2,586 and 2,350 on two
  runs of the *same code and the same seeds* before this; it gives 2,418 twice
  now. The old published means, spread ~120 turns, were reading a sample where
  they claimed to read a number. Nothing here changes what the model does - the
  new figure sits inside the old band - but every figure after it can be
  compared with one run instead of thirty-two.

#### What holds it

`analytics/tests/repeatable_tests.rs`. Two behavioural tests - the same seed is
the same world, a different seed is not - and two source-level guards, which are
unusual and deliberate: `every_roll_comes_from_the_one_stream` fails on any
`thread_rng`, `rand::random` or `Uuid::new_v4` in `src/`, and
`nothing_decides_anything_by_walking_an_unordered_table` fails on any `HashMap`
or `HashSet`. A behavioural test catches a stray roll only if the branch
carrying it happens to run in the test's hundred and twenty ticks; the fourth
fault above sat in exactly such a branch through the whole of #92. This class of
defect has now recurred three times, and the guard is cheap.

### 95. The five-thousand-line function, and the first thing it hid

`execute_action` was 5,723 lines - a third of `analytics/mod.rs` - and one
`match` of fifty-two arms. Every arm was reachable only by scrolling, no two
could be read side by side, and a change to one produced a diff nobody could
review against the other fifty-one.

It is a dispatcher now, in `analytics/doing/`, and the arms are methods grouped
by what they are about rather than by the order somebody happened to add them:
**eating** (food into a body, and the keeping of it), **getting** (taking what
the country has), **making**, **ground**, **keeping** (carrying and putting by),
**meeting** (what passes between two people), **fighting**, **moving**,
**looking**. `analytics/mod.rs` goes 16,779 to 11,060; the largest of the nine
new files is 1,129 lines and the smallest is 195.

Two things stayed in the dispatcher on purpose, because they belong to *doing
something* rather than to any one verb: the single roll, drawn once and lent to
whichever verb needs it - so that how many draws a turn takes does not depend
on which arm is chosen - and the one check against the verb matrix for what
these hands are short of.

#### The proof, which is the point

A refactor of five thousand lines is normally an act of faith. This one is not:
**three seeds run six hundred ticks give byte-identical worlds either side of
the move** - every agent, beast, resource and pit, tick by tick - and the suite
gives the same 28 failures it gave before, stable over two runs. That check
exists only because of #94. It is the first return on that work, and it arrived
one commit later.

#### And the first thing the split found

`Action::Fight` never reads its `weapon`. Two hundred lines from the top of
`fighting.rs`, in a signature that now fits on one line, the compiler pointed at
an unused parameter: **a man standing his ground against a wolf fights it the
same with a flint spear in his hand as with nothing.** `hunting`, two modules
away, is careful about exactly this. The specification is explicit - *"Hunting
any larger animal requires at least a spear... A flint spear should reduce the
number of attacks"* - and standing your ground is the same problem from the
other side.

It has presumably been true since `Fight` was added. Nothing found it while it
sat at line 12,003 of a sixteen-thousand-line file, because nothing could see
it. Left as it is here, because this change is behaviour-neutral by contract,
and filed as its own piece of work.

### 96. The tick, and a flag that would have been dropped silently

With `execute_action` gone, `tick` was the largest function left: 852 lines. Its
actual shape - a run of world phases, then everybody taking a turn, then a
second run of world phases - was buried under six hundred and seventy lines of
per-agent decision code sitting in the middle of it. That order is argued over
in the comments, and several of the arguments were bought with a measurement:
the beasts look before they move rather than after, the world is ticked once
rather than twice, what a body has to pass goes back on the ground before
anybody smells it. None of it could be read while the middle of the function was
longer than the whole of anything else in the file.

`analytics/turn/` now holds it. `tick` is 179 lines and reads as the sequence it
is. `turn/each_one.rs` holds one person's turn in the order a person takes it,
with the five stages named:

1. `keep_the_goals_and_the_plan_current` - the standing intentions, refreshed on
   their own clock rather than every turn.
2. `choose_what_to_do` - the priority ladder, from starving down through fear,
   shelter, what can be seen, the plan, the goals and the drives; and the note
   of *why*, which is the only thing that makes the threat tally mean anything.
3. `and_what_it_takes` - a real target rather than a nil id, an errand held
   rather than re-decided, a reachable place, a free hand, the tool out of the
   bag, the parts fetched, and whether a better tool would pay for itself.
4. `execute_action` - the previous entry.
5. `what_came_of_it` - the body's bill, the tally, the lesson, the plan.

Then, on its own clock rather than on a drive, `look_in_at_the_storehouse`.

`analytics/mod.rs` goes 11,060 to 10,213 - **16,779 to 10,213 over the two
entries, a drop of 39 per cent** - and the largest function left in it is 378
lines.

#### The flag

Cutting the choosing block out gave it a parameter, `ran_for_it`, because
fleeing comes out as an ordinary `Move` and both the tally and the errand need
to know it was one. The block already declared `let mut running_away = false;`
of its own, so the parameter was **shadowed**: every assignment inside went to
the local, the caller's flag stayed false, and `stick_to_the_errand` would have
held a fleeing man to whatever errand he was on.

The compiler said "unused variable" and nothing else. It is not an error to
shadow a parameter, and a reader skimming a 225-line body would not have caught
it; a test would only have caught it in a world where somebody had to run. The
fingerprint harness would have caught it on the next run - which is the point of
having one - but the warning caught it first, and only because the block had
been given a signature to shadow. That is the second thing this split has found
in two commits, and both were found by giving code a name.

### 97. The decision layer had no boundary at all, and that was the original argument

#92 ended by naming what was missing as a *property* rather than a bug list:
"the decision layer's inputs must have a stable order... a layer with a boundary
can be made to hold that property once." There was no such layer. What answers
*given a drive, what would answer it?* was 2,900 lines scattered the length of
the file: the ladder in one place, what hunger asks for eight hundred lines
below it, what the errand machinery does with the answer three thousand lines
after that, and the constants each of them turns on wherever they happened to be
written.

`analytics/wanting/` is that layer now. The ladder is in `mod.rs`; below it,
one module per question, named for what it is about rather than for the drive
that happens to ask - because more than one drive asks most of them:

| | |
|---|---|
| `food` | hunger and thirst, the two that kill |
| `quarry` | hunting and fishing |
| `ground` | working the soil before there is anything on it to take |
| `store` | putting by, and taking out again |
| `shelter` | keeping warm and dry |
| `camp` | whether to stay, and where to go instead |
| `errands` | turning a want into a step somebody can actually take |

Eighty-eight functions and fifty-six constants moved; the constants went with
the functions they belong to, which is the first time most of them have been
anywhere near their use. `analytics/mod.rs` goes 10,213 to **5,276**.

#### What the boundary is actually for

**Nothing in `wanting/` does anything.** Every function in it answers a question
and hands the answer back; the doing is in `doing/`, and the order of a turn is
in `turn/`. That is the whole point of the boundary, and it is a rule that can
now be checked by reading a directory listing rather than by reading sixteen
thousand lines. A change to what hunger *asks for* can no longer quietly change
what eating *does*.

It also makes #92's property tractable. "The decision layer's inputs must have a
stable order" is a sentence about `wanting/`. Before this it was a sentence about
a file.

#### Across the three splits

`analytics/mod.rs` was **16,779 lines** four commits ago and is **5,276** now -
down 69 per cent - and the largest function left in it is 170 lines against
5,723. Nothing about the model changed in any of the three: three seeds run six
hundred ticks give byte-identical worlds at every step, and the suite gives the
same 28 failures throughout.

That the same check passed three times running is worth stating plainly. A
refactor of this size is normally argued about rather than verified. This one was
verified, at each step, in about ninety seconds - and the reason is #94, four
commits back, which at the time looked like housekeeping.

### 98. The third layer, and the parts of it that had never been next to each other

What happens whether or not anybody decides anything: the ground coming up, the
weather on a body, the beasts, birth and nursing, what a person finds out by
being somewhere at the time. It was the last big cluster in `analytics/mod.rs`
and the hardest of the three to see, because **its parts were never next to each
other**. The ground coming up in berries sat two thousand lines from the weather
that made it wet. The beasts deciding what to make of us sat a thousand lines
from the beasts acting on it. Nothing in the file's layout said these were one
subject; only the order they are called in did, and that was buried in `tick`.

`analytics/happening/`, eight modules and 36 functions:

| | |
|---|---|
| `soil` | what the ground does, and what goes back into it |
| `weather` | the weather on a body, and what a clear day dries |
| `beasts` | what they make of us, and what they do about it |
| `kin` | carrying, bearing, and feeding what cannot feed itself |
| `noticing` | what a person finds out by being somewhere at the time |
| `senses` | what can be smelled, and what stops being worth remembering |
| `situation` | reading the world, so a drive rises on a condition |
| `buildings` | buildings, and what standing in one does to somebody |

`analytics/mod.rs` goes 5,276 to **2,491**.

#### The three layers, finished

- [`wanting`] decides - given a drive, what would answer it
- [`doing`] acts - fifty-two verbs, grouped by what they are about
- [`happening`] happens - whether or not anybody decided anything
- [`turn`] says what order they run in, and holds the arguments about that order

That is the shape the modularisation was for. It is worth saying what it buys
beyond tidiness: **each of the three has a different rule about what it may
touch**, and those rules are now checkable by looking at a directory rather than
by reading a file. `wanting` may not change the world. `doing` may not decide.
`happening` runs on the world's clock, not on anybody's drive. Before this, all
three rules were true only by the discipline of whoever last edited line 9,412.

#### Across four commits

`analytics/mod.rs`: **16,779 to 2,491 lines, down 85 per cent.** Largest function
5,723 to 121. Fifty-seven functions left in it, against 176.

Nothing about the model changed in any of the four. Three seeds run six hundred
ticks give byte-identical worlds at every step, and the suite gives the same 28
failures throughout, stable across runs. Four configurations build clean at each
step.

#### One orphan left behind

`process_information_verification` stayed in `analytics/mod.rs`, and it is the
only thing in there that looks like a world process. It is not one: the compiler
has it as never used, and it is the last remnant of the second gossip pipeline
that #93 deleted the rest of. It is left where it is rather than given a home in
`happening/`, because giving dead code a good address is how it survives the next
sweep.

### 99. What one agent makes of another, which is not a layer but a seam

The last big cluster, and the one that would not sit in any of the three:
being afraid of somebody, angry at somebody, willing to trade with them,
willing to give to them, worth asking. `analytics/between_us/`, 22 functions:

| | |
|---|---|
| `threat` | fear, anger, and the four answers to a thing in the way |
| `seeing` | what everybody saw, and what they made of it |
| `exchange` | trading, taking, and giving |
| `asking` | putting a question to somebody who might know |

A beast counts as another here. What these have in common is not that the other
party is a person: it is that there *is* another party, and that what this one
does next depends on what it makes of them.

#### Why it is its own directory

It does not fit the three-layer shape, and forcing it would have been worse than
leaving it. `wanting` consults it - a drive that needs somebody else to answer it
asks who. `turn` runs part of it as a phase - what somebody feels has to be
worked out before they can act on it. Splitting it between the two would put
`what_this_threat_comes_to` in one place and `how_this_one_answers_a_threat` in
another, which is the exact fault the last four entries have been undoing.

So it is a seam rather than a layer, and saying so in the module note is worth
more than a tidier diagram would be.

#### The end of it

`analytics/mod.rs` is **1,208 lines** and 35 functions, the largest of them 86.
It was **16,779 lines and 176 functions, the largest 5,723**, five commits ago -
**down 93 per cent**. What is left in it is what belongs in a file called
`analytics/mod.rs`: the configuration, the builders, save and load, the two
tallies, and a handful of gather helpers that could go either way.

Across all five: three seeds run six hundred ticks give byte-identical worlds at
every step, the suite gives the same 28 failures throughout, and four
configurations build clean at each step. Nothing about the model changed in any
of them.

#### The three orphans, and one that looked worse than it is

Three methods are now visible as never used outside the tests, one in each of
three directories:

- `process_information_verification` (`analytics/mod.rs`) - the last remnant of
  the second gossip pipeline #93 deleted the rest of.
- `nearest_edible_this_one_would_go_to` (`wanting/food.rs`).
- `how_this_one_answers_a_threat` (`between_us/threat.rs`).

The third looked alarming for about a minute - the choosing code names it in a
comment as where "the whole tree lives", and nothing calls it - so it was worth
reading before writing down. It is **a one-line wrapper**:
`self.what_this_threat_comes_to(agent, position).1`, dropping the branch name.
`what_this_threat_comes_to` is the tree, and the live path calls it. The tree
runs.

What is actually wrong is smaller and worth fixing anyway: two names for one
question, one of them used only by tests, and a comment pointing at the wrong
one. That is defect #3 in miniature, and it survived precisely because a reader
who wanted to check would have had to hold two places four thousand lines apart
in their head. It took ten seconds once they were forty lines apart.

### 100. A man with a spear fought a wolf exactly as he would have fought it empty-handed

`Action::Fight` carried a `weapon` and never read it. That was the finding the
`execute_action` split turned up (#95), and it was worse than it looked: not one
oversight but **three places reading a vocabulary the model does not stock**.

- The action's own field was filled from `agent.equipment.get_weapon()`.
- `Agent::own_strength`, which decides the odds, adds `0.3` if
  `equipment.get_weapon().is_some()`.
- The fight itself read neither.

Nothing in this model has ever called `equipment.equip`. The only `equip` calls
in `src/` are `body.equip`, which is clothing. So the field was `None` in every
fight this model has ever run, and `own_strength`'s weapon bonus has never once
fired. This is the same dead vocabulary #93 deleted the rest of and #219 is
about; the live one is `environment::making`, reached through
`how_much_my_tools_help`.

Measured before the fix, instrumenting every fight over sixteen worlds of two
thousand ticks: **eight fights, none with the action's weapon flag set, none
with an equipped weapon - and two of the eight fought by somebody carrying a
spear worth 1.87.** It counted for nothing in both.

#### The fix

Read the spear the way `hunting` reads it two modules away, because standing
your ground is that problem from the other side. It tells twice, which is what
the specification asks for in two separate sentences:

- **whether the blow lands** - `(spear - 1.0) * 0.25` onto the agent's side of
  the odds, exactly the term `hunting` uses;
- **how many blows it takes** - the tool's own worth as a multiplier on the
  damage, floored at one. *"A wooden spear is enough, but should take several
  attacks to kill the animal... A flint spear should reduce the number of
  attacks."*

Measured over forty fights a side: **2.17 blows to put a wolf down bare-handed,
1.73 with a spear.** Bare hands are arithmetically unchanged - the floor sees to
that - because refusing to hunt an ox empty-handed sends somebody home hungry,
while refusing to fight a wolf that is already on you sends them home dead.
There is deliberately no size gate of the kind `hunting` has: whether to be here
at all is the threat tree's question and it has already answered it.

The two construction sites now fill the field from the live vocabulary too, so
the action carries the truth about what is in the hand.

#### And the honest part: it changes nothing a settlement can feel

**Survival is unmoved to the tick** - 32 worlds, 4,000 ticks, mean last-alive
2,418 before and 2,418 after. It could not be otherwise: eight fights in
sixteen worlds of two thousand ticks is a path that fires roughly once per four
thousand agent-ticks.

So this is a correctness fix with no measurable consequence, and the reason is
already on the list. **#188 - anger at one animal can never pass the gate that
lets an agent turn on it.** The tool ladder now works properly on a branch
almost nobody reaches. That is the right order to do the two in - a gate opened
onto a broken fight would have been worse - but the second half is what will
show in a number.

### 101. The gate asked for more than the feeling behind it could give

`EmotionState::should_attack` was `anger > 0.5 && fear < 0.3`.
`ThreatAssessment::emotion_amount` returns `threat_level * 0.5` for anger, and
`threat_level` is bounded at one. **A man at the very worst rage one animal can
produce sat exactly on the gate and did not pass it.**

And the gate read the *sum* of every source while the branch behind it acts on
the strongest single one - `what_angers_me_most` for a creature,
`who_angers_me_most` for a person. So the branch fired only when two separate
grudges added past a half: an agent turned on the wolf in front of it partly
because it also resented a boar.

Measured before the change, 32 worlds of 4,000 ticks:

| | |
|---|---|
| a creature on the mind, resented | **29.7%** of every turn |
| on the mind, but under the gate | **28.7%** |
| felt: angry enough to act | **0.12%** |

#### The threshold, taken from the data rather than chosen

Bucketing the worst single thing angering anybody, over twelve worlds: anger
stops dead at `0.50`, with **6.1% of angry moments sitting exactly on the
ceiling** - every one of them excluded by a gate wanting strictly more.

The fear gate was `0.6` against a ceiling of `0.7`: **six sevenths**. Applying
that same fraction to anger's ceiling of `0.5` gives `0.4286`, and gives the
fear gate back exactly the number it already had. So the two gates are now one
demand expressed on two scales, and neither can drift from its ceiling again -
there is a test that fails the moment one does.

Both gates now read the strongest single source, because that is what the
branches behind them act on. `TOO_FRIGHTENED_TO_STAND` stays on the total on
purpose: the other two ask about the thing in front of you, this asks whether
you are in any state to face it.

#### What it did

Mechanism, 32 worlds: **"angry enough to act" 0.12% to 1.52%**, twelve times as
often. "Stands its ground" 0.030% to 0.090%. `Fight` 18 to 27.

Survival, paired on the same seeds, three separate blocks of 32 worlds:

| seeds | before | after | |
|---|---|---|---|
| 1000 | 2,418 | 2,662 | +244 |
| 2000 | 2,306 | 2,399 | +93 |
| 3000 | 2,505 | 2,506 | +1 |

**Positive in all three, mean +113 turns (+4.7%) over 96 worlds** - and the
spread between blocks is larger than the effect, so +244 is not the headline and
the mean is. Splitting the change in half: the anger gate alone gives 2,593 on
the first block, so roughly three quarters of the gain is the half this issue
was actually about.

#### Two things worth stating rather than burying

**`Attack` went from 37 to 0.** Agent-to-agent retaliation stopped entirely, and
that is a *removed false positive*, not a lost behaviour: anger at a person
never exceeded `0.35` in twelve worlds, so those 37 attacks were being enabled by
summing in anger at an *animal*. A man was hitting his neighbour partly because a
wolf had annoyed him. Person-anger has no ceiling of its own - it accumulates
from lies and theft and can in principle reach one - so the branch is reachable
by somebody robbed repeatedly, just not by somebody robbed once. That it now
reads zero over 32 worlds is worth its own look, and is not this issue.

**One test flipped from pass to fail**: `thirst_tests::agents_keep_themselves_watered`,
which asserts a six-person settlement is still standing after 3,000 ticks in one
seeded world. Traced before writing it down: that world has **zero fights and
zero flights** in the whole run, and its people die of hunger (5), illness (1)
and weather (1). Nothing about combat touched it. What changed is the draw
sequence - a gate answering differently on one turn shifts every roll after it -
and this world fell the other side of a line it was already sitting on. It is
left failing rather than weakened, because the assertion it makes is the standing
problem #206 is about, and rewriting a test to suit a change is how a suite stops
meaning anything.

### 102. Nobody retaliates because nobody is wronged - and a settlement's whole social life was conducted at arbitrary range

#101 left `Action::Attack` at zero and asked whether that was right. The answer,
traced end to end, is **yes for now, and the retaliation gate is not the thing to
change.** Retaliation needs anger at a person; anger at a person comes from being
lied to or robbed; and neither happens.

| | over 32 worlds of 4,000 ticks |
|---|---|
| `TakeFrom` chosen | **0** |
| `Trade` chosen | **0** |
| things anybody was told | **29** (over 8 worlds) |

Lowering the gate would have put the false positive #101 removed straight back:
a man hitting his neighbour over a grudge he does not have.

#### Why nobody steals

Instrumented rather than guessed. `somebody_to_take_from` sits at the tail of an
`or_else` chain, and:

- the branch is **reached 142 times in 120,000 agent-turns** - 0.12% - because
  everything ahead of it has to decline first;
- on those 142 occasions there was somebody within arm's reach **3 times**;
- and on all 3 that person had nothing the thief wanted.

The 3-in-142 is the surprise, because people are not spread out at all: the
nearest other person is on the **same tile 31.6% of the time** and within the
three-tile reach **64%** of it. So the branch is not blocked by distance in
general - it is reached almost only on the turns when somebody is off alone,
which is exactly the turn on which every other option has run out. The defect is
the branch's *position*, not its threshold, and moving it is its own piece of
work: the comment above it records that an earlier attempt to put a refusable
branch ahead of unrefusable ones cost a settlement half its winter store.

#### What the tracing did find

`find_nearest_social_target` returned the nearest person **on the map**, with no
distance limit at all - and neither `socialising` nor `sharing_information`
looked at where that person was. So two men twelve tiles apart, each alone in a
different wood, greeted one another, exchanged news and gave one another
presents. A settlement of a dozen people conducted its entire social life at
arbitrary range, and the Social drive was answerable without anybody ever being
in the same place.

Fixed: one named reach, `WITHIN_TALKING_DISTANCE`, tied to
`CLOSE_ENOUGH_TO_SEE_IT_COME_UP` - if you can see a man pick a thing up, you can
call across to him. The choosing no longer names somebody out of earshot, and
both verbs refuse if asked anyway.

Measured, paired on the same seeds against #101:

| seeds | before | after | |
|---|---|---|---|
| 1000 | 2,662 | 2,881 | +219 |
| 2000 | 2,399 | 2,285 | **-114** |
| 3000 | 2,506 | 2,691 | +185 |

**Mean +97 turns, but one block of thirty-two worlds goes the other way**, so
this is weaker evidence than #101's and is not being claimed as a survival win.
It is landed as a correctness fix - people should not befriend across a valley -
with the survival effect recorded as inconclusive. Socialising did not become
rarer (1,051 to 1,093): three quarters of it was already within earshot, and the
limit mostly removed the long-range quarter.

#### What is still true

`TakeFrom` is still 0 and `Attack` is still 0. The reach fix did not open that
path and was not expected to. **#225 is answered rather than fixed**: zero is
correct while nobody is wronged, and the work that would change it is making
theft reachable - filed separately, because it is a change to the decision ladder
and wants its own measured cycle rather than a ride on this one.

### 103. The other thirteen drive rates were never derived either

Hunger's is derived now, off the stomach's own emptying schedule - see #80 -
and Thirst is read straight off the body. The other thirteen are still numbers
somebody chose. They can be derived the same way only if something measurable
sits behind them, and nothing does: none of them kills, so none has a clock to
be sized against, and all of them were picked against a calendar that no longer
exists.

### 104. The clock is spelled out in the interface too

`gui/panels/controls.rs`, `gui/panels/statistics.rs`, `bevy_gui/ui/mod.rs` and
`bevy_gui/ui/panels/statistics.rs` all compute the date as `tick / 1440` and the
hour as `(tick % 1440) / 60`. Display only, but every one of them shows the
wrong day.

### 105. Build warnings

15 warnings on `cargo build`, all unused variables and imports. `cargo fix`
handles most.

### 106. Placeholder package metadata

`Cargo.toml` still declares `authors = ["Your Name <your.email@example.com>"]`
and `repository = "https://github.com/yourusername/ebss-project"`.
### 107. The bearing year was written for a twenty-four-day season

`when_it_bears` returned a *set of seasons*, so a thing came on for the first
day of a season and went over on the last. That was fine when a season was
twenty-four days. On the real calendar a season is ninety, and the same table
made a year of four uniform blocks: three months of leaf, three more of leaf,
three months of harvest, three months of nothing. Its own doc comment already
said what it should have been doing - "it carries nothing at all for most of
the year and then, for a few weeks, everything at once" - and the code under
it did the opposite.

Two things in it were plainly wrong against a ninety-day year, and neither
needed any measurement to see:

- **`Food` - the fruit node, and the world's staple at energy twenty - bore
  in autumn and in no other season.** Three months of high summer with nothing
  ripe on any bush. Wild fruit in a temperate zone runs from midsummer.
- **Greens and roots stopped dead on the last day of summer**, so autumn had
  no leaf and no roots in it at all. Leaf runs to the frosts, and autumn is
  when a root is worth digging.

#### A window instead of a set

The calendar already keeps the vocabulary this wants: `PartOfSeason`, two
weeks at each end of a season and eight in the middle. A `Bearing` is now
written in it - `from((Summer, Deep), (Fall, Late))` - and resolves to two
days of the year. `is_it_bearing` takes a day rather than a season, which is
what all three of its call sites already had to hand.

| | opens | closes | days |
|---|---|---|---|
| Greens | early spring | deep autumn | 255 |
| Roots | early spring | early winter | 285 |
| Food | deep summer | late autumn | 165 |
| Grain | late summer | deep autumn | 90 |
| Honey | deep summer | early autumn | 90 |
| Flax, cotton, herbs | deep spring | late summer | 165 |

Roots run longest and end the year, which is what a root is *for*: last
year's root in the hungry gap, this year's swollen root in autumn, and the
winter dig out of hard ground. Deep winter still gives nothing whatever, and
that does not move - it is the whole point of a store.

#### Measured

Three independent blocks of thirty-two worlds, a full year each:

| seeds | mean last alive | person-days alive | alive at autumn | worlds emptied |
|---|---|---|---|---|
| 1000 before | 3028 | 816 | 0.75 | 18/32 |
| 1000 **after** | **3088** | **879** | **1.12** | **15/32** |
| 2000 before | 2391 | 717 | 0.50 | 22/32 |
| 2000 **after** | **3607** | **995** | **1.88** | **17/32** |
| 3000 before | 2841 | 807 | 0.66 | 18/32 |
| 3000 **after** | **3559** | **1005** | **1.66** | **13/32** |

Every figure improves in every block. Mean last-alive **2,753 to 3,418
(+24%)**, person-days **780 to 960 (+23%)**, alive at autumn **0.64 to 1.55**,
and the share of worlds standing empty at a year **60% to 47%**. Standing
edible stock mid-summer went 2,097 to 3,372 and mid-autumn 3,007 to 4,455,
which is where the gain comes from and is exactly where the table was wrong.

Four failing tests cleared: a suspicious settlement feeding itself, agents not
staying frozen over a long run, two thousand turns leaving a record of having
been lived, and agents keeping themselves watered. 29 failures to 25.

#### What it does *not* fix, and this is the more useful finding

**Spring is untouched, and spring is what kills everybody.** Of 372 deaths in
a full year, 323 are in spring and 223 of those are hunger; winter takes
fifteen. A settlement goes from twelve alive to 5.6 between day thirty and day
forty-five and never recovers. Measured with `examples/_debug_hungrygap.rs`,
what happens in that window is:

| day | alive | reserve | units/day | richness | energy/day | burn/day |
|---|---|---|---|---|---|---|
| 15 | 10.9 | 0.72 | 57 | 14.9 | 848 | 1264 |
| 30 | 10.5 | 0.47 | 57 | 20.6 | 1183 | 1374 |
| 45 | 5.4 | 0.47 | 61 | 22.6 | 1382 | 1427 |
| 60 | 3.4 | 0.62 | 67 | 20.5 | 1373 | 1505 |
| 75 | 2.8 | 0.74 | 70 | 23.6 | 1643 | 1518 |

Nobody is short of food by day sixty - intake passes burn and the survivors'
reserves climb back. The founders arrive with a full reserve and eat it down
over the first month at about four hundred energy a day, because **the mean
richness of what they eat starts at 14.9**. Greens are 30.6% of every unit
eaten and 10.7% of the energy: a stomach of leaf displaces a stomach of
something a body could live on. Half the settlement is dead before richness
reaches the twenty-two the survivors then hold.

That is not a bearing-year fault - the food is standing there, 3,549 units of
it per world in mid-spring - and widening the table does not touch it. It is
what a world is *seeded* with against what an early spring should hold, which
is #208, and how a forager weighs a thin food underfoot against a dense one
across the meadow. Filed there rather than fixed here, because fixing it by
moving numbers in this table would have been tuning rather than a year.

#### And one flaky test made honest

`a_settlement_lives_through_a_winter` ran one world and did not seed it, in a
model where about half of all settlements are empty by the end of a year. It
had been passing on the draw it happened to get and it flipped on this change
- while every count underneath it improved: settlements reaching winter with
somebody alive 18/32 and 12/32 to 20/32 and 25/32, and alive a year on 14 and
10 to 15 and 15. It now runs eight seeded worlds.

It deliberately asserts no *rate*. The share that reach winter and come out of
it measures 40%, 60% and 85% on three different blocks, so at eight worlds any
bar for it would have been fitted to the block rather than to the model.

### 108. Every bush in full fruit, whatever the date

A world was made with everything standing at what its ground would carry,
without ever asking what day of the year it was. The year opens in spring, so
every settlement ever run in this project began with berries, standing grain
and full hives on the hedges around it. Measured over sixteen worlds with no
agents in them, day nought:

| | Fish | Food | Grain | Greens | Honey | Roots |
|---|---|---|---|---|---|---|
| day 0 | 2881 | **216** | **254** | 3202 | **34** | 1576 |
| day 5 | 2881 | 8 | 21 | 3202 | 1 | 1576 |
| day 10 | 2881 | 0 | 0 | 3202 | 0 | 1576 |

**504 units of food that had no business being there**, which the shedding
rule then took off over ten days. Greens, roots and fish are right and do not
move; the three that are wrong are the three that do not bear in spring.

A world is seeded on its opening day now: `what_this_ground_carries` takes the
day of the year, and anything outside its bearing window starts bare. The
check is asked *before* `is_it_grown`, because honey is not a growing thing
and has a season all the same - a hive worth robbing in autumn is not one in
March, and it used to spawn full in March.

#### And a third spawner, found by the test rather than by reading

`what_this_ground_carries` was written to be the one vocabulary after two
spawners were found to have had two. There were three.
`scatter_the_strange_plants` builds its nodes directly and never went near it,
so the fix reached the hedgerows and not the strange plants, and a spring
world still opened with thirty units of something-or-other standing in it.
The end-to-end assertion written for this entry is what caught it. That is
defect number three in this document's list for the eleventh time, and the
useful part is *how* it surfaced: not by grepping for the vocabulary, but by
asserting the property over the whole world and letting the assertion find the
path that had not been touched.

#### Measured, and it costs

Five independent blocks of thirty-two worlds, a full year each, against the
commit before:

| seeds | mean last alive | person-days alive | worlds emptied |
|---|---|---|---|
| 1000 | 3088 → 3617 | 879 → 899 | 15 → 11 |
| 2000 | 3607 → 3227 | 995 → 909 | 17 → 16 |
| 3000 | 3559 → 3281 | 1005 → 960 | 12 → 12 |
| 4000 | 3576 → 3679 | 1015 → 1010 | 15 → 12 |
| 5000 | 3760 → 3179 | 1074 → 948 | 11 → 22 |

**Person-days alive fall about five per cent**, down in four blocks of five;
mean last-alive falls 3.4%, down in three of five; worlds standing empty at a
year go 44% to 46%. This is landed as a correctness fix that **costs**
survival, not as a win, and the honest reading of the block-to-block spread is
that everything except the person-days figure is inside the noise.

What is interesting is the size of it. Those 504 units are about 22,300
energy, or a day and a third of food for twelve people. Against a run whose
mean is some 280 days that is half a per cent, and it measures ten times that
- because the ten days it is handed over are exactly the ten days in which the
founders eat down the reserve they arrive with, and half of them die. **A
day's food at the crisis is worth ten days of it anywhere else.** Anything
that means to move this model's survival has to land in that fortnight; a
change that adds food to the year at large, as #107 did, does not.

#### A measurement that came out the other way from the guess

The doc comment on the seeding was first written saying that a patch seeded
short would be full again within a day or two, so the exact opening amount did
not matter. Measured by stripping a patch bare and waiting: **a fruit node is
back to full in one day, and greens and roots are still short after thirty.**
The claim was true for a third of the foods it was written about. Corrected in
place, and worth carrying forward on its own account - a settlement that
strips its greens in early spring has no greens for a month, and nothing in
this document has yet asked whether that is happening.

#### Three tests that indexed a corpse

Harder springs took the lone agent out of three single-agent tests, and
`population.agents[0]` panics rather than reporting a death, because the dead
are removed from the vector. `an_agent_ages_by_the_calendar` had a comment
saying in as many words that one person alone often does not get through the
year, directly above the line that assumed he had. All three now ask whether
he is there before asking how he is.

### 109. Seven items for a winter, and a larder anybody could open in July

Two things, and the second is the one in the title.

#### The target was a hundred and sixty-five times too small

`WHAT_ONE_MOUTH_WANTS_PUT_BY` was **seven items** - what one person wants put
by to see them through the whole lean season - and behind it sat every branch
of the store: burying what is in the pack, walking to a pit, digging another,
and going out to gather for the store at all. Twelve people wanted eighty-four
items. A settlement reached that in its first autumn and the entire chain shut
down for the rest of the year.

Seven items is **half a day's food**. It was reasoned carefully from "a person
gets through about a hundred units in ten thousand ticks", which was true of
the body this model had before the starvation clock was corrected in #203 -
the entry that found that clock a hundred and twenty times too slow. The store
was sized against the slow body and never resized.

The arithmetic was already written down, in the right place, with the right
answer. `provision::UNITS_IN_ONE_STORED_ITEM` carries the comment **"Eleven and
a half of them is a day"**. Nothing joined it up.

Measured over sixteen worlds and a full year, before:

| | |
|---|---|
| most the pits ever held, at any point in the year | **14 items** |
| which in days of food for one person is | **0.9** |
| food items carried home over the year | 1,472 |
| food items dropped back on the bush, packs full | **7,794** |

Five items thrown away for every one kept, and a settlement's entire larder
under one person-day.

Derived now, from the two things it is actually about:

- `provision::WHAT_A_BODY_EATS_IN_A_DAY` = `UNITS_BURNED_IN_AN_ORDINARY_DAY /
  UNITS_IN_ONE_STORED_ITEM` = **11.52 items**, and measured at 15.4 because a
  settlement lives on food thinner than ordinary forage.
- `how_long_the_hedgerows_give_nothing()`, read off the bearing year of #107
  rather than named, so retuning the year retunes the store with it: the
  longest run of days on which no growing thing a person can eat is carrying
  anything. **Seventy-five days**, from the last root out of the cold ground to
  the first leaf. Fish and meat are deliberately left out - they never stop,
  and sizing a winter store on the assumption that everybody will be fishing is
  the optimism this entry is about, with `Fish` refused ninety-three times in a
  hundred.

Which is **864 items a mouth** where it was seven.

Two more constants in the same cluster had to move with it, and the test suite
is what said so: `the_cap_is_a_load_rather_than_a_meal_or_a_cartload` asserts
that somebody who would not open the store must not also be barred from
foraging, and that ordering broke the moment the store gate was corrected.
`ENOUGH_NOT_TO_OPEN_THE_STORE` said "two days' worth" and was four items, which
is a third of a day. `WHAT_A_PERSON_GETS_THROUGH` said "well above what anybody
needs for a day" and was eight, which is under it - an anti-hoarding cap that
fired on a man with supper in his bag. Both are counted in days now, and the
second is built off the first so the ordering holds by construction rather than
being a relation between two picked numbers that has to be tested for.

#### And nothing ever asked what month it was

The larger fault, and the one the title names. `something_out_of_the_store` had
no season condition at all: a pit within reach was simply the nearest food, so
a settlement drew on its winter store in July. That is what "laid down and
eaten at the same rate" means - the pits held between seven and fourteen items
from one end of a year to the other and never accumulated, because everything
put in came straight back out.

A store is opened when the land gives nothing now, and the same
`are_the_hedgerows_bearing` that sizes it answers that. Somebody genuinely
starving still opens it in any month: a rule that let a man starve beside a
full pit would be a worse fault than the one it fixed, and there is a test for
each half.

The store now has a winter's shape, which it did not have at all:

| pits hold, mean of 16 worlds | before | after |
|---|---|---|
| midsummer | 7 | 24 |
| deep autumn | 14 | 27 |
| end of autumn | 12 | **33** |
| a fortnight into winter | 11 | 9 |
| deep winter | 4 | 4 |

It fills through the autumn and is eaten through the winter. Before, it was
flat.

#### Measured: settlements last six per cent longer and hold the same people

Five independent blocks of thirty-two worlds, a full year each:

| seeds | mean last alive | person-days alive | worlds emptied |
|---|---|---|---|
| 1000 | 3617 → **3804** | 899 → **969** | 11 → **7** |
| 2000 | 3227 → **3380** | 909 → 903 | 16 → 18 |
| 3000 | 3281 → 3250 | 960 → 887 | 12 → 16 |
| 4000 | 3679 → **3737** | 1010 → 947 | 12 → **11** |
| 5000 | 3179 → **3833** | 948 → **1035** | 22 → **17** |

**Mean last-alive 3,397 to 3,601, up six per cent and up in four blocks of
five. Person-days alive flat** (945 to 948), worlds standing empty at a year
46% to 43%.

Settlements last longer and hold the same number of people, which is exactly
what a winter store does and is not what a food supply does: it carries the
survivors across the lean stretch rather than feeding more of them. Two
long-run population tests flipped back to failing on it, which is inside the
noise those two have shown all session.

#### What is now the binding constraint, and it was not before

`Pit::WHAT_A_PIT_TAKES` is three hundred. A winter for one mouth is 864, so a
settlement of twelve wants **thirty-five holes** and digs, measured, under
three. Room in the ground was never once the binding question while the target
was seven items a mouth - the doc comment on
`does_the_store_still_want_filling` says so in as many words - and it is the
binding question now. There is a test that says it out loud rather than a
comment that will drift. Digging thirty-five holes is a different piece of work
from knowing how many you need, and it is filed rather than done here.

Nor does this touch the spring die-off, which remains what kills everybody: a
store fills in autumn and nobody is alive by then. Sixteen worlds average
**1.6 people** through the autumn the store is filled in. A larder for a
settlement that no longer exists is still the right larder.

### 110. A six-year-old who carried what his father carried

Three things the lifecycle described and nothing read, and two defects found
underneath them that were worth more than any of the three.

#### The curve

`what_a_body_this_age_can_do` - the specification's table of what a body of
each age brings to moving, carrying and working - was written, hung on
nothing, and deleted as dead code in the sweep of #93. That was the right call
for the code and it left the model with **age deciding nothing but appetite**:
a six-year-old carried what a grown man carried, walked as fast, worked as
hard and hit as heavily, on a third of his food. A child was a bargain.

It is restored and hung on the four things the sentence names and implies:
what two hands hold (`update_inventory_capacity_from_transport`), how fast a
body walks (`movement_speed_at_tick`), what a trip brings back (the hand term
in `gathering`), and what a blow is worth (`own_strength`).

#### The bands

`LifeStage`'s own doc comment has carried the supervision rules since the
lifecycle was written - "0-5 must be with a parent at all times; 6-10 must
stay within sight of the camp or of some adult; 11-15 must stay within an
hour's walk" - as prose, on a stage nothing consulted for the purpose. A
five-year-old walked to the far side of the map like anybody else. The three
bands are written in reaches this project already keeps rather than in new
numbers, and sit below fear (a frightened child runs first) and above every
want.

#### Feeding a child

There was no way for a parent to hand a child anything short of
`somebody_of_mine_who_needs_it_more`, which waits until a loved one is
*starving* and hands over food the giver needs itself. A child in this model
foraged for itself from the day it could walk or went without. There is an
ordinary branch now: a child of one's own, within reach, hungry, with nothing
of its own, and food to spare in the pack.

#### And underneath: `Agent::new` made newborns

Hanging the curve on carrying broke **eighty-seven tests and hung one**.

`Agent::new` leaves the age at nought, and `LifeStage::from_age` calls
anything under six an infant. Nothing minded while nothing read a body's age
for anything but its appetite; the moment two hands were scaled by it, every
fixture in the project that says `Agent::new` and means "a person" was
carrying a twentieth of a pack, and the tool-wear test spun for ever waiting
for a tool that could not be picked up to wear out.

This is **#74 one layer down**. That entry found founders spawned at nought -
"every world began with twelve newborns and nobody to feed them" - and fixed
it in `spawn_agent`, which overrides the constructor. The constructor
underneath went on making newborns, and every caller that was not
`spawn_agent` got one. A bare `Agent::new` is a grown person now and
`with_parents`, which is the birth path, sets the age back to nought itself.
Eighty-seven failures to four.

#### And a body that burned at a grown man's rate on a child's reserve

`Physiology::now_a_body_of` resized the reserve and the stomach and **left the
burn alone**. `for_a_body_of` sets all four together; this set three. So a body
resized down to a child carried a fifth of the reserve and went on burning
what a grown man burns.

Measured with a probe: a body of nought years, fifteen days without food, read
**14.4 turns from death against a grown body's 72.0** on the same
going-without - five times, which is exactly the ratio of the two bodies.

The model held both answers at once. `Physiology::starved` has a small body
and a grown one going at the same three weeks, which is deliberate and
documented in #74; `minutes_before_hunger_kills_me` had the child dying five
times sooner. Four hundred lines apart, and nothing ever compared them,
because nothing had reason to ask both. That function also returned the
reserve *in energy* and called it minutes, which is the same number only for a
grown body; it divides by what this body actually burns now.

#### Two vocabularies for one age, and what that cost the status report

`life_stage` is a *stored* field and a dozen places set it directly, leaving
`age` where it was. Everything that reads a body's age reads the years, so
such a body is an adult wearing a child's label.

`a_child_and_an_adult_do_not_rank_the_same_needs_the_same_way` set
`life_stage = Child`, asked how long hunger left the body, and got **47
against 47** - the same answer twice, because both bodies were the same age.
`a_hungry_year_takes_the_children_first` passed **900 and 4000 ticks** for "a
child" and "an adult", figures from the calendar where a year was eleven
hundred ticks; a year is 4,320 now, so both fixtures were nought years old and
the test compared an infant with an infant.

The project status report listed **"a child and an adult come out identical"
as one of three blocking failures** on the strength of three tests like these.
One of them was one line in a fixture. There is a single
`now_this_many_years_old` now that sets the years, the stage and the body
together.

#### Measured: nothing, and that is the honest answer

Three blocks of thirty-two worlds, a full year: mean last-alive 3478 to 3564,
person-days alive 920 to 916, worlds emptied 41 to 47 of 96. A wash, inside
the block-to-block spread this session has shown throughout.

**It could not have been anything else, and that is the point.** Founders are
spawned between twenty and forty, where the curve is at its full ten out of
ten; a year is 4,320 ticks, so nobody reaches the forty where it starts
falling; and **two children are born in 308,000 turns of action**. None of the
three rules can fire in a run of this model as it stands. They are correct,
tested, and idle - which is this document's defect number one, entered
deliberately this time and with the gate named: they wait on reproduction,
exactly as the larder of #109 waits on a settlement being alive in autumn to
fill it.

#### What this did to the failing-test count, and why it is not a regression

**25 to 29.** Two cleared (the child-and-adult ranking, and a specialisation
fixture that indexed a corpse) and six appeared, of which four are one
question: **does a child starve sooner than an adult?**

The model now answers *no*, consistently, everywhere - which is what #74
decided deliberately ("everybody still starves in three weeks, whatever size
they are; a small body simply has less to go without") and what the burn fix
above made true in the one place that disagreed. Three tests say *yes*, and
the real-world argument is on their side: a small body's stores scale with its
mass while its burn scales nearer the three-quarter power, so a child
genuinely has fewer days than its father.

That is a specification question and not a bug, it needs its own measurement,
and answering it at the end of a change this size would be exactly the sort of
thing that gets landed and regretted. Filed as its own task rather than
decided here. The four tests are left failing and now fail for the true reason
- `a_hungry_year_takes_the_children_first` ran for a hundred and sixty-seven
days, by which point both bodies are long dead and both read nought, so it
could not have come out either way; it measures at three weeks now and reports
the actual disagreement.

### 111. The lifecycle against the specification, clause by clause

The specification was handed over. Read against the model, the two tables were
already right and five clauses were not.

**Right, and now asserted verbatim rather than by resemblance:** the
capability table (1 at two years, 10 at sixteen, 9 at forty, 5 at sixty-five)
matches exactly; a year is 518,400 of the specification's ticks and a life
36,288,000 of them, which are this model's *minutes* - a turn is a decision
and not a minute, and the calendar has been written in minutes since #73 for
exactly this reason; and the food table matches year for year.

**Wrong, and fixed:**

- **The fifteenth year fell in a gap.** "Age 14-15: 90%" then "Age 16+: 100%",
  and the bands are half-open elsewhere ("Age 0-4: 20%" is ages nought to
  three), so fifteen was unnamed and the model gave it a full grown share. A
  fifteen-year-old was fed as an adult while doing nine tenths of an adult's
  work. The last child band runs to the adult boundary now.

- **A child in arms occupied no hands.** "Age 0-2: ... Parent agent has one
  *hand* occupied with the child, limiting the types of work the parent agent
  can accomplish." Nothing anywhere. A parent carrying somebody under two now
  has half of what two hands hold; what is on their back is untouched, which
  is exactly why somebody carrying a baby wants a basket.

- **The supervision bands had no camp in them.** "Within eyesight of
  camp/tent/town **or** within eyesight of any adult agent", and the same *or*
  for the hour's walk. The first cut of this read only the second half and
  marched a child by the fire across the map after the nearest adult. Read as
  a *building* rather than as `where_the_camp_is`, which answers "the nearest
  roof to wherever you happen to be standing" when there are too few people
  about - its own doc comment says that is the wrong answer for somebody out
  on the moor, and it would have excused a child that had wandered to a cave
  on the far side of the world. Not an alternative under six: a roof is not a
  parent.

- **Small children were fed on demand rather than on what their parent had.**
  The nursing machinery gave an infant a mouthful whenever somebody was
  standing near and there was room in its belly, and charged the mother
  whatever it came to however little she had. The specification is a band
  table on the *parent's internal store*, and it covers water, and it runs to
  five years rather than to the end of a nursing period:

  | parent's store | child receives |
  |---|---|
  | above four fifths | all of it |
  | above three fifths | three quarters |
  | above two fifths | half |
  | above a fifth | a quarter |
  | below a fifth | nothing |

  Which is a settlement's hunger reaching its children a step behind itself,
  and stopping while the parents are still alive - a parent a fifth full is a
  parent whose child gets nothing, and both of them are still standing.

- **A three-year-old could cook.** "Age 5-10: ... eat any wild food found. Age
  10-15: ... **and cook raw food into cooked food**", so under ten they cannot.
  Refused in the executor as well as in the decision, on the same reasoning as
  `is_ground_a_pit_will_go_in`: a rule that lives only in the wanting layer is
  a rule anything reaching the verb another way walks straight past.

**Measured: a wash, and it could not have been otherwise.** Two blocks of
thirty-two worlds: last-alive 3619/3462 to 3578/3413, person-days 929/896 to
926/878. Founders are twenty to forty and two children are born in 308,000
turns, so none of the five clauses can fire in a run of this model as it
stands. Failing tests unchanged at 29.

**And one clause not done here.** "Agents are gender neutral. There are no
male/female agents, merely child and adult agents." The model has a `Gender`
enum, `can_mate` requires one of each, pregnancy lives on the female and
`give_birth` reads the mother's position: forty-four references across ten
files. That is its own change with its own measurement, and it bears directly
on the largest failure in the model - a rule that only pairs opposite genders
throws away about half of all candidate pairs in a settlement that manages two
births in 308,000 turns. Filed and done next rather than tacked on here.

---

### 112. Half of every candidate pairing was refused on a distinction the specification does not have

"Agents are gender neutral. There are no male/female agents, merely child and
adult agents." The model had a `Gender` enum on every agent, and forty-four
references to it across ten files. `can_mate` opened with a match that took
`(Male, Female)` or `(Female, Male)` and returned false for anything else;
pregnancy could only be started on the female and only be read off her; and
`give_birth` placed the newborn at `parent1.state.position` if parent1 was
female and at parent2's otherwise, with a silent default of 0.8 prenatal
nutrition when neither was - which nothing could reach, and which is the shape
a rule takes when it is written around a field rather than around a fact.

**What it cost.** Gender was rolled at spawn on an even coin. A settlement of
twelve therefore has about half of its 66 possible pairs refused before
fertility, distance, trust, or food put by is asked about at all - in a model
whose largest single failure is that nobody is born.

**What replaced it.** `can_mate` now asks three things: two different people,
neither already carrying, both fertile. `attempt_impregnation(male, female)`
became `attempt_impregnation(carrier, other)`, and which of a pair carries is
the *caller's* decision rather than a property of either of them. Both callers
- the population pairing pass and the `Mate` executor - choose the carrier by
the lower of the two ids: a coin that always lands the same way for the same
pair, which keeps the run repeatable without putting the distinction back on
the agent. Prenatal nutrition and the newborn's position are read off whoever
actually holds the pregnancy, so there is no default left to be silently
wrong. `can_become_pregnant` and `can_impregnate` collapsed into
`can_carry_a_child`. `src/agents/gender.rs` is deleted, along with the field,
the four GUI call sites that displayed it, and the map colouring that used it.

**Measured, and it is a genuine trade.** Two blocks of thirty-two worlds,
4,320 ticks each, paired seeds before and after:

| | before | after |
|---|---|---|
| pairings attempted | 40, 23 | 92, 102 |
| births | 13, 10 | 19, 25 |
| alive at the end (96 worlds) | 3534 | 3160 |
| worlds emptied | 49 / 96 | 59 / 96 |

Pairings roughly quadrupled and births roughly doubled, which is the change
doing exactly what the arithmetic said it would. **Survival fell about eleven
per cent**, and that is not a defect in this change: a settlement that could
not feed twelve now cannot feed twelve plus infants, so the extra children are
being born into the food shortage that #109, #208 and #213 have each moved and
none has closed. The model is specification-conformant and harder, and the
births are now real enough to make the shortage the thing that shows.

**A correction to the status report.** It gave `Mate` two firings and read that
as the reproduction path being all but dead. Two was measured before this
session's food-year work; re-measured on the current head the baseline is 13
and 10 births per thirty-two worlds. The path was not dead, it was halved.

Failing tests 29 to 26, taken as a set difference against a worktree at the
previous head rather than by counting. Five cleared:
`being_told_lets_you_try_it_rather_than_making_you_believe_it`,
`a_settlement_of_the_suspicious_still_feeds_itself`,
`agents_do_not_stay_frozen_over_a_long_run`,
`population_feeds_itself_over_a_long_run`, and
`what_agents_do_in_a_run_becomes_something_they_know`. Two new:
`a_cold_agent_ends_up_dressed` and
`the_young_are_kept_warm_by_the_adults_around_them`.

Neither of the two new ones is about clothing or warmth. Read the panics: the
first is `index out of bounds: the len is 0 but the index is 0` reaching for
`agents[0]`, and the second compares 37.0 against `inf`, which is a mean taken
over an empty set. Both worlds emptied. That is the eleven per cent arriving
in the two tests whose settlements were closest to the edge, and it also says
something about the tests - a settlement test that indexes `agents[0]` without
asking whether anybody is left reports a wiped-out world as an index panic, and
one that averages without a count reports it as `inf`. Filed as #228: a run
that ends with nobody alive should fail saying so.

`a_pair_with_nothing_put_by_do_not_have_a_child` fails, and was failing before
this change too - a fresh agent's reserve is full, `food_has_been_easy` reads a
full reserve as food having been easy, and so an agent carrying nothing passes
the gate that the test says it should not. That is #227's question and it is
not this change's to answer.

The four gender tests were rewritten rather than deleted:
`any_two_grown_people_can_pair` (which was `test_cannot_mate_same_gender`, and
asserted the opposite), `nobody_pairs_with_themselves`,
`nobody_already_carrying_starts_another`, and
`there_is_no_gender_left_to_refuse_anybody`, which pairs ten agents and
requires that every grown pair be allowed and every child pair refused.

---

### 113. Why they still starve: a trickle that never fails, and a pack that holds two days

The food year was rebalanced (#107), the world stopped being seeded with
summer fruit in winter (#108), and the store was given a real target (#109).
Settlements still empty. This is what is actually killing them, measured
rather than guessed.

**It is not that there is no food.** Over four worlds of half a year, taking
every living agent every tick and splitting them by how full their reserve is:

| | starving (reserve under 15%) | fed (reserve over 60%) |
|---|---|---|
| food units within 25 paces | **20** | **618** |
| patches within 25 paces | 3.9 | 25.9 |
| biggest patch within 25 paces | **7.4** | 64.6 |
| food in own pack | 0.5 items | - |

A body burns 1,440 units a day. **The best patch within reach of a starving
agent holds seven and a half units** - half a per cent of one day's food - and
there are four such patches. The fed are standing in thirty times as much. The
two groups are in the same world on the same day.

**And `Eat` never fails.** Over eight worlds of a full year: 24,861 `Eat`
actions, **nought failures**. Not one. The executor has three ways to refuse -
too full, the patch was empty, nothing within the forage radius - and in a year
of twelve settlements starving to death none of them fired, because a patch
holding seven units is not an empty patch. An agent in stripped ground
successfully eats a trickle, every turn, until it dies.

That is the mechanism, and it is why none of the food-year work reached it.
Every signal that could tell an agent to leave ground that will not feed it is
driven by something *failing*, and nothing fails. The settlement's own
`Gather` refuses 500 times for "No food sources nearby"; `Eat`, which is the
verb that would notice, refuses nought times. A rule that fires on failure
cannot see a slow death.

**The second half: a pack holds twelve weight.** *(Wrong, and corrected in
#116: twelve is what a pair of bare hands holds. A live agent almost always
carries a basket and holds forty-two. The figure below was measured on a fresh
`Agent::new` in a unit test rather than on anybody in a running world, and the
rest of this paragraph is right about the waste and wrong about the cause.)*
`Inventory::max_weight` is
12.0 and food weighs 0.5, so a pack holds **twenty-four items of food, which is
two days' eating** against the 11.52 a body gets through in a day. Measured
over the same eight worlds, 11,656 items of food went into packs and **56,020
would not fit** - five dropped for every one kept. So nobody can carry a store,
nobody can provision a walk to better ground, and a settlement is obliged to
live where it stands and eat what is underfoot. This is #215 and #216 arriving
from the other direction: they were filed as a wrong constant, and they are
also the reason the larder never fills.

**What the numbers rule out.** Density is not the whole story: a lone agent on
an empty world survives its first year 62% of the time and starves nought
times, while at twelve founders 77% of everybody dies of hunger. But it is not
simple crowding either - the survivors of a collapsed settlement sit at
85-99% of reserve for the rest of the year, comfortable, on the same map. What
the land supports is not twelve people; it is however many happen to be
standing somewhere that bears.

**A correction to my own first reading.** I ran the food ledger, saw 474 items
eaten against the ~50,000 twelve people need in a year, and drafted the
conclusion that nobody eats at all. That was wrong: `food_i_ate` is incremented
only in `eat_food_item`, and the forage branch of the `Eat` executor feeds the
body directly through `physiology::eat` without going near it. The counter
measures eating *out of the pack* and always has. Worth writing down twice
over - once because the number is misleading to anybody who finds it, and once
because it is the same defect as everything else in this entry: a measurement
that only sees one of two paths.

Filed as #229 (leave ground that will not feed you, on a signal that is not a
failure) and #230 (a pack that holds two days cannot provision anything).

---

### 114. Breeding on a full belly, and one derivation spelled two ways

"Agents should not be reproducing until there is surplus food." They were, and
the reason is one `||`.

`expects_to_be_able_to_feed_a_child` read:

```rust
self.food_put_by() >= Self::FOOD_TO_RAISE_A_CHILD || self.food_has_been_easy()
```

with a careful paragraph above it explaining that "am I hungry this minute" is
not the question, that it says nothing about the next meal, and that a person
who ate today but has nothing put by is in no position to raise a child. And
then `food_has_been_easy` is `reserve >= reserve_capacity * 0.85` - which is
"am I full this minute" - and it is behind an `||`, so it is sufficient on its
own. Measured, a fed agent in this model sits between 85% and 99% of reserve.
The clause was true of every healthy adult alive, the pack was **never once**
the binding question, and the reasoning written directly above the line was
contradicted by the line.

`FOOD_TO_RAISE_A_CHILD` was four items - about eight hours' eating for a grown
body - so even when it did bind it was asking "could you feed this child until
Tuesday".

**What it is now.** A surplus is food that is still there tomorrow, so the gate
asks for what is *put by*: the pack and this agent's share of the camp's
stores, with the stomach and the gut taken back off, against what the parent
and a newborn would eat between now and the land bearing again.

- The stretch is `how_long_the_land_gives_nothing()` - derived from the bearing
  windows, 75 days, not picked.
- The newborn is a fifth of a grown appetite, off the specification's own food
  table.
- `WhatIsPutBy` gained `units_in_the_body`, because the reckoning counts what
  is in the stomach (rightly - somebody who has just eaten is not short of
  supper) and a breeding gate has to be able to take it off again.

Which puts the gate at exactly the store's own target and a fifth: **breed when
you have more put by than you need for yourself.**

It also settles what a surplus can and cannot be. A pack holds forty-two
weight with a basket on the back - about seven days' food, and #113 said two
because it measured a fresh agent rather than a live one; see #116 - which is
still nowhere near a lean season. A surplus worth breeding on was never
something a person could be *carrying*. It is the camp's stores or it is
nothing.

**One derivation, one answer.** The hungry gap was derived inside the decision
layer's store code, where nothing outside that layer could reach it, so the
breeding gate had to derive it again - and the two spellings of the same sum
came out 864 and 865 through float ordering alone. Both now come to
`provision::how_long_the_land_gives_nothing`, which is where the file's own doc
comment already said this kind of figure belongs ("so that anything sizing a
store against a stretch of days has one place to get it from and cannot pick
its own"). `ResourceType` gained `is_it_food` and `all`, so "what counts as
food" is a fact about a resource rather than a list private to analytics;
`all()` is guarded by an exhaustive match that fails to compile if a variant is
added and not listed.

**Measured, paired over thirty-two seeds:**

| | before | after |
|---|---|---|
| births | 20 | **0** |
| alive at the end | 18 | 21 |
| person-days | 31,718 | 32,257 |
| worlds emptied | 14 / 32 | 12 / 32 |
| hunger deaths | 299 | 301 |

Births to nought, which is the point: no settlement in this model has a lean
season's eating in the ground, so no settlement can afford a child, and it
should not be having one. Survival is marginally *better* for it. Births come
back when the larder does, and the gate is the thing that will say so.

**And old age, taken off the board.** `PopulationConfig::nobody_dies_of_old_age`
sets `max_age` past reach, honoured at both ways into a population - spawned as
a founder and born - because a switch honoured in one of them works until the
first birth. Default false, and it is worth being plain about why that costs
nothing: over sixteen worlds of a full year **every death in the model was
hunger or thirst and not one was old age**, because founders are twenty to
forty and nobody has ever lived to seventy. It is insurance against a confound
in a multi-year run, not a fix for anything, and turning it on today would
change no number in this document.

**Tests: 26 failing to 24, none new.** Two cleared, and they are the two that
had been asserting this all along - `a_pair_with_nothing_put_by_do_not_have_a_child`
and `a_child_waits_on_a_surplus_and_not_on_a_full_stomach`. The model had been
failing its own stated rule and the tests had been right about it.

Three fixtures were wrong in the same way and are corrected here:
`fed_adult` set `state.age = 4000` - *ticks*, from the calendar where a year
was about eleven hundred of them - so "a fed adult" was a body in its first
year. The same slip as the one `a_hungry_year_takes_the_children_first` found
and wrote down. And `a_settlement_lives_through_a_winter` went from eight
seeded worlds to thirty-two: at eight it flipped to zero survivors on a change
that improved every count underneath it, which is the failure mode its own doc
comment predicted for asserting a *rate* and is just as true of asserting "at
least one". Seeds 0..32 have eleven settlements alive at the end. The suite
costs 45 seconds more for it.

---

### 115. Eight answers to "is this food", and food that stopped being food when it changed hands

Asked how the eating code decides what to eat, when to eat and what counts as
food, and the third question had no single answer.

**The one part that was well built.** `physiology::how_fast_hunger_rises` reads
three tables off the body - share of reserve, energy in the stomach, food in
the gut - and multiplies them, as a *rate* rather than a level, with the
reasoning for that written down and measured. What to eat is scored by the
nutrient most needed, times freshness, times how fast the thing goes off, so a
dried strip scores a twentieth of today's supper and gets kept for February.
Neither needed touching.

**The eight answers.** `InventoryItem::is_food` (has nutrition data attached);
`Agent::LOOKS_EDIBLE` (substring match on six words); `Piece::can_it_be_eaten`
(only asks whether a thing is an uncut carcass); `FoodDatabase::is_food`;
`ResourceType::is_it_food`; `edible_item_for`; `Pit::is_it_food` ("not a bowl
or a basket"); and an inline `is_food() || name.contains("food") ||
name.contains("grain")`. Measured against each other with an untracked stack -
`food_put_by` / `has_edible_food` / `find_best_food_to_eat`:

| item | put by | can eat | search finds |
|---|---|---|---|
| food | 5 | yes | no |
| grain | 5 | no | no |
| fish | 5 | no | no |
| bread | 5 | no | no |
| greens | **0** | no | no |
| roots | **0** | no | no |

So a pack of untracked grain, fish or bread **counted as provisions and could
not be eaten by anything**: `has_edible_food` reached for the literal item id
`"food"` and no other, and `find_best_food_to_eat` skipped every stack without
nutrition data. Untracked greens and roots counted as *nothing at all*, neither
word being among the six - and they are the whole of what a hedgerow gives for
half the year.

**And the verb trusted its callers.** `eat_food_item` guarded only on
`Piece::can_it_be_eaten`. Called with "wood", "stone", "clay", "bowl" or
"flax" it returned Success, credited twenty energy, fed `nutrition.consume` and
dropped the hunger drive. Nothing reached it that way in a live run, because
every caller filtered first - which is the point. The rule lived in the callers
and not in the verb, which is the shape of every defect in this file.

**Where the untracked stacks came from.** Bartering, gifts and going-without
all rebuilt the receiving stack from scratch - `new_with_weight(name, how_many,
1.0)` - so what arrived had the right name and nothing else: no nutrition, no
freshness, no preparation state, and a flat weight of one against food's real
half, so a traded meal weighed double against a pack that holds forty-two (#113
says twelve; see the correction in #116). With no
freshness it then never spoiled, so it sat in the pack for ever as food that
read as food and was not. Animal products - milk, eggs - were built the same
way and had never carried nutrition at all. Measured, 17 of 213 edible-looking
stacks in packs were untracked.

**And a fourth drift in the name table.** `PlantDrop` names sixty-two things a
plant can give - apples, berries, potatoes, wheat, mushrooms - and
`id_to_item_type` knew four of them. Everything else the flora system produced
arrived as a name nothing could resolve: no nutrition, no price, no place in a
store. The edible ones are mapped onto the types that already exist now; petals,
fibre, bark, straw, seeds and poison mushrooms deliberately are not.

**What replaced it.** `ItemType::is_it_food` is the one answer for types and
`nutrition::is_this_food` is that question asked of a name, through
`id_to_item_type`, so a cooked joint and a cut portion resolve to what they
were cut off. `food_put_by`, `find_best_food_to_eat`, `has_edible_food`,
`InventoryItem::is_food`, the pit and the verb all ask it. `eat_food_item`
refuses anything that is not food. `Simulation::hand_over` moves a stack out of
one pack and into another whole - same weight, same food data, same quality -
and returns nought if the receiving pack will not take it, so a one-sided
bargain is refused rather than half-completed.

One distinction is deliberately kept: a whole fish or an uncut haunch **is**
food and is **not** supper. `food_put_by` counts it and `has_edible_food` does
not, which is what `Piece` and `how_many_meals_i_have` have always said.

**Measured.** Untracked edible stacks in packs **17 to nought**. Food eaten out
of the pack over eight worlds of a year **474 to 920**, which is the direct
effect: the search can see what the agent is carrying now. Person-ticks alive
90,272 to 96,956. Wood, stone, clay, a bowl and flax are all refused. Survival
across three blocks of thirty-two seeds is a wash - alive 21/12/18 to 22/12/18,
worlds emptied 12/21/14 to 12/20/15 - which is what to expect when the paths
being repaired are eight per cent of stacks and a rare verb.

**Tests 24, unchanged, none new.** Seven broke on the way and all seven were
fixtures naming food the model has never produced - "meal", "fruit",
"raw_meat", "spoiled_meat" - which a predicate that resolves names refuses as
firmly as it refuses a stone. They say what the model actually makes.

The drift guard `every_food_type_has_a_template` holds the static list to the
runtime database. It earned its keep immediately: this entry was first written
asserting that nothing in the model is ever called "berries", on the strength
of the word appearing only in prose in the decision layer. `PlantDrop` drops
"berries". The guard failed the moment the name table was taught the drops, and
the claim was wrong. A guard that only ever agrees with you is not a guard.

---

### 116. A basket was worth fifty and held thirty, so everybody walked at half speed

Set out to make a pack big enough to provision a journey, on the strength of
#113's "a pack holds twelve weight, which is two days' eating". **That figure
was wrong, and the way it was wrong is worth writing down**: twelve is
`WHAT_TWO_HANDS_HOLD`, what a pair of *bare* hands carries, and it was measured
off a fresh `Agent::new` in a unit test. Measured on agents in a running world,
**87% carry a basket and hold forty-two**. The premise of the task did not
survive its first measurement.

What was actually there is better.

**One basket, two owners, fifty kilos.** `take_up_the_cart` maps the item
`"basket"` onto `TransportType::Backpack`, whose capacity is thirty, and
`total_additional_capacity` puts that into `Inventory::max_weight`. Then
`effective_max_weight` counted **the same basket again**, at twenty, off the
inventory:

```rust
self.max_weight
    + baskets as f32 * Self::WHAT_A_BASKET_HOLDS   // 20
    + bags as f32 * Self::WHAT_A_LEATHER_BAG_HOLDS // 35
```

So one basket was worth thirty as a thing on your back and another twenty as a
thing in your pack. Two subsystems answering "what does a container add", which
is this project's oldest defect in its plainest form.

**What it cost, which is not what it looks like.** `add_item` gates on
`effective_max_weight` and every report reads `max_weight`, so agents loaded
themselves against the loose figure and were measured against the tight one:
over six worlds, **43.5 kg carried against a stated 34.7 - a hundred and
twenty-five per cent full, permanently.** And `movement_speed_at_tick` takes
`1.0 - weight_percentage() * 0.3`, with `is_overweight` halving it on top. A
settlement of people who could never get under their own limit walked at
between a half and five eighths of their speed for the whole of every run. The
double count was not a bookkeeping error; it was a permanent movement penalty
on everybody.

**Fixed by giving it one owner.** `Transport` has the whole table - capacity,
speed, durability, twenty-odd kinds of carrier - and `take_up_the_cart` puts
what is in the pack onto the back every turn from `tick_with_percepts`. So
`effective_max_weight` is now `self.max_weight` and nothing else, and
`WHAT_A_BASKET_HOLDS` and `WHAT_A_LEATHER_BAG_HOLDS` are gone. The leather bag
had reached capacity only through those constants, so it goes into
`take_up_the_cart` as `LargeBackpack` - fifty, ahead of the basket's thirty,
which is what being a leatherworker is worth.

**Measured, paired over three blocks of thirty-two seeds:**

| | before | after |
|---|---|---|
| mean pack | 43.5 kg of 34.7 (**125% full**) | 30.3 kg of 36.4 (**83%**) |
| person-days | 32,885 / 27,095 / 30,320 | **36,826 / 27,330 / 33,674** |
| worlds emptied | 12 / 20 / 14 | 14 / 21 / 16 |

Person-days up twelve, one and eleven per cent - people live longer because
they can walk again. Worlds emptied is up by one or two in each block, which is
the same thin tail as everywhere else in this model and not worth reading much
into at thirty-two.

**And the real reason nothing gets provisioned, which is not capacity at all.**
With the count corrected, the pack is 83% full and **food is six per cent of
it**. What fills a pack, by weight over six worlds:

| | share of everything carried |
|---|---|
| wood | **23.1%** |
| iron | 4.7% |
| handaxes | 4.4% |
| tinder | 3.5% |
| stone | 2.9% |
| all food | **~6%** |

*(The reading of this table was wrong, and #117 corrects it: those counts are
totals over seven hundred agent-samples, not what one person carries. Per
agent it is five logs, one handaxe and one knife - a sensible kit, not a
hoard. The shares are right; the story told about them was not.)* Filed as
#236, and see #117 for what came of it.

---

### 117. Nobody was hoarding anything, and the rule written for it made things worse

#236 said agents carry six hundred and sixty-six handaxes and never put
anything down. **They carry about one each.** The figure was a total over seven
hundred and nineteen agent-samples, and I read it as a per-agent count and
wrote it into #116, into the task, and into a rule built on top of it. Second
misread in two entries, and the same shape both times: a number measured across
a population and reported as a number about a person.

What one agent actually carries, per sample over six worlds:

| | per agent | kg |
|---|---|---|
| wood | 4.98 | **9.96** |
| handaxe | 0.93 | 1.86 |
| fishportions | 1.47 | 1.47 |
| basket | 0.77 | 0.77 |
| stoneknife | 1.00 | 0.50 |

One axe, one knife, a basket, some fish and five logs. That is a kit, not a
hoard. The only thing on it that looks heavy for a forager is the wood, at a
third of the pack.

**What was tried.** A rule that a person keeps one of each tool, one carrier,
all their food and a bounded amount of any material - the bound taken from the
room a trip's load needs, so five logs at two kilos came down to three. Wired
three ways: a decision branch above everything but eating, a refusal in the
`Gather` verb so what was set down could not be fetched straight back, and
`PutDown` setting down the surplus rather than the whole stack.

It worked, in the sense that it did what it said: wood fell from 4.98 to 3.23
per agent, and **food went from eight per cent of what is carried to eleven**.

**And it made survival worse.** Paired over three blocks of thirty-two seeds,
person-days against the same seeds before it:

| | person-days | against baseline |
|---|---|---|
| baseline | 36,826 / 27,330 / 33,674 | - |
| everything | 35,394 / 28,838 / 29,516 | **-4.2%** |
| without the `Gather` refusal | 32,937 / 30,485 / 29,521 | **-5.0%** |
| surplus-only `PutDown` alone | 34,821 / 28,377 / 29,552 | **-5.2%** |

Every arrangement of it costs more than it gains. A turn spent setting
something down is a turn not spent eating, and the extra armful it buys does
not pay for the turn. So it is not shipped. What is left is the part that
cannot be wrong either way:

- `provision::WHAT_A_HANDFUL_OF_FOOD_WEIGHS`, which was the literal `0.5`
  written in the Gather executor's weight table and again in the forage branch
  of `Eat`.
- `provision::AS_MUCH_AS_ONE_TRIP_TAKES`, moved out of the decision layer,
  where nothing outside it could reach the one figure that says how much a trip
  brings back.
- `Agent::WHAT_CARRIES` as an associated constant rather than a local inside
  `take_up_the_cart`, so what a person carries things in is a list somebody
  else can read.

Those three are ownership moves with live callers and no behaviour in them, and
the run bears that out: over the same ninety-six seeded worlds the counts come
back **bit for bit identical** - 36,826 / 27,330 / 33,674, sixteen and fourteen
and twenty-one worlds emptied, the same to the person.

**What this leaves.** The waste is real - 77,514 items of food went back on the
bush against 7,537 carried home - and it is not hoarding and not a rule about
tidiness. A pack holds thirty-five kilos, a kit weighs thirty, and an armful is
seven: the trip brings back more than there is room for, and the shortfall is
about one armful. Whether the answer is a bigger pack, a lighter kit, a smaller
armful or a second trip is a question for measurement rather than for
reasoning, and reasoning is what produced both wrong premises in this entry and
the last. #236 is reopened with the corrected figures.

---

### 118. The waste was two counters in a trenchcoat, and behind it an armful refused for being one lump

"Seventy-seven thousand items of food went back on the bush against seven
thousand carried home" has been in three entries and every answer given about
this model's food economy. **There is no such waste.** Split the counter and
the number that means food actually lost is **nought**.

`what_would_not_fit_in_the_pack` was being added to from two places that mean
opposite things:

- `into_the_pack_or_on_the_ground` calls `somebody_left_this`. A carcass too
  big to carry is **left on the ground**, where it rots and is gone. That is
  the waste #165 is about, and measured over eight world-years it is **zero
  items of food**.
- The forage branch of `Eat` calls `put_it_back`. An armful that will not go in
  the pack goes **back on the bush**, and nothing is lost at all: the patch is
  exactly as it was, and the same berries are counted again on the next trip,
  and the trip after that. That is all 87,667 of them.

One counter, two meanings, and the meaning that would have been alarming is the
one that never fired. They are `what_would_not_fit_in_the_pack` and
`what_went_back_on_the_bush` now.

**But the put-backs are still telling us something, and it is not what I
thought.** Two candidate causes, both checked and both wrong:

- *The slot limit.* Agents use 5.4 of 20 slots and are at the limit **0.0%** of
  the time.
- *A full pack.* The agents putting food back had, on average, **12.55 kg of
  room - twenty-five items' worth** - while refusing an armful of fourteen.

The actual cause: **`Inventory::add_item` is all or nothing.** Offered twenty
items when there is room for ten it takes none of them. That is right for a
tool, which is one thing or no thing, and wrong for an armful of berries, which
is twenty separate berries. So a forager with room for ten and fourteen in his
hands walked home empty.

Butchering had already worked this out - it computes `fits` and takes that much
- and the two paths that bring food home never learned it. Third time in this
file that one path solved something and its siblings went on without it. There
is one `take_what_fits` now and all three go through it.

**Measured.** Food into packs **9,691 to 11,107, up fifteen per cent**; food
put back 87,667 to 80,728. And survival is **unchanged**: paired over five
blocks of thirty-two seeds, person-days go -6.3%, +9.6%, -8.2%, +0.2%, +3.2%,
which is **-0.9% over a hundred and sixty worlds** against block-to-block
swings of ten. Worlds emptied, fifty-one against fifty-one.

That last is worth saying plainly rather than dressing up. **Carrying half as
much food home again makes no difference to whether anybody lives.** The food
economy is not short of food that got home; agents are fed on the spot by the
forage branch and the pack is a store they rarely draw on - 621 items eaten out
of the pack against tens of thousands foraged. What decides a settlement is
where its people are standing, which is #229, and this changes nothing about
that. It is shipped because the model now says a true thing where it said a
false one, not because it made anybody live longer.

**A note on the three wrong numbers.** #113 said the pack held two days of food
(measured on a fresh `Agent::new`, corrected in #116). #116 said agents carried
666 handaxes (a total over 719 samples, corrected in #117). #113, #116 and #117
all said food was being thrown away ten to one (two counters pooled, corrected
here). Every one of the three was a real measurement read the wrong way round,
and every one of them survived several entries and several answers before
anything checked it. The fix that holds is not "be careful" - it is that a
number worth acting on is worth a probe that isolates it first.

---

### 119. Carrying is not what limits this settlement, and a bigger pack makes it worse

#236 has now been wrong three times about the same thing. It began as "a pack
holds two days of food" (#116: it holds seven, and the two-day figure came off
a unit-test fixture). It became "nobody puts anything down, six hundred and
sixty-six handaxes" (#117: about one each, and the rule written for it cost
five per cent of survival). It became "seventy-seven thousand items of food
thrown away" (#118: nought thrown away, two counters pooled). Each time the
premise was replaced rather than the question.

So this time the question was put to the model directly rather than reasoned
about: **does carrying capacity matter at all?**

Swept over three blocks of thirty-two seeded worlds, total person-days:

| what two hands hold | person-days | against twelve |
|---|---|---|
| 6 | 97,217 | +1.9% |
| **12** (as shipped) | **95,371** | - |
| 120 | 75,081 | **-21.3%** |

**Flat from six to twelve** - within the block-to-block noise of ten per cent -
and **a fifth worse at ten times**. Halving the pack costs nothing measurable.
Making it enormous is the single largest survival regression measured in this
model since the starvation clock was corrected.

**Why a bigger pack is worse**, which is worth having rather than guessing at.
Comparing the settlement's action mix at 120 against 12, per person-tick:

| | at 12 | at 120 |
|---|---|---|
| `Gather` | 0.347 | 0.345 |
| `Eat` | 0.289 | **0.276** |
| `Work` | 0.063 | **0.080** |
| `Sleep` | 0.051 | 0.041 |

Gathering is unchanged. `Work` is up twenty-seven per cent and `Eat` is down.
A person with materials in hand has something to make, and making competes with
eating for the same turn. Capacity is not a bottleneck on food; it is a
*licence* for other work, and the other work is what kills them. Food into
packs doubles and food eaten out of packs doubles with it - and it does not
help, because the settlement was never short of food it had carried home.

**That closes #236.** The four candidates it was reopened with were a bigger
pack, a lighter kit, a smaller armful and a second trip. The first is refuted
with the sign reversed. The other three are all ways of moving food from the
patch to the pack, and #118 already measured what that is worth: fifteen per
cent more food carried home changed survival by -0.9% over a hundred and sixty
worlds. There is no version of "carry it better" that this model is waiting
for.

**What it leaves.** Agents are fed on the spot by the forage branch - 505 items
eaten out of packs against tens of thousands foraged - and the pack is not a
larder. A settlement lives or dies on whether the ground its people are
standing on bears anything, which is #229: a starving agent's best patch within
twenty-five paces holds seven units against a daily burn of fourteen hundred,
while the well-fed stand in six hundred, and nothing moves anybody because
`Eat` never fails. Four entries of carrying work have between them ruled out
the pack, the kit, the counters and the verb, and every one of them points at
the same place.

`WHAT_TWO_HANDS_HOLD` now carries this sweep in its doc comment, so the next
person to think twelve looks arbitrary finds out that it is not.

---

### 120. The signal to leave fired, and a bare patch a pace away outbid it

#229 said a starving settlement never picks itself up and moves. The obvious
reading is that the signal never reaches its threshold, and that reading is
wrong. Measured over six worlds and 45,732 agent-ticks, hunger's `denied_ticks`
stands at 120 or more - `HUNGRY_ENOUGH_TO_LEAVE`, ten days of being hungry and
not being fed - in **3.06% of them**, and reaches 254 at its worst against a
threshold of 120. Starving agents average 86.8. The signal fires.

**What happens to the tick instead.** Instrumenting every branch of
`food_action`, counting only the ticks where the agent had already been hungry
long enough to leave:

| what the tick did | ticks | share |
|---|---:|---:|
| walk to a source it knows | 768 | 69.0% |
| forage where it stands | 323 | 29.0% |
| hunt something near | 10 | 0.9% |
| cut up a carcass, eat what is carried, cook | 7 | 0.6% |
| **reached the branches that leave** | **5** | **0.4%** |

Sixty-nine per cent of them went to one line: `known_source_position` names the
nearest food the agent can smell or remember, and the branch returns before the
leaving branches are reached. So the question is what it was naming.

**It was naming nothing.** Of those 768 targets, **765 - 99.6% - had no food
standing on them at all**, and the mean walk to one was 1.3 paces: the agent
was standing on the bare patch it was being sent to. The `Gather` that came out
of it was refused by `could_this_gather_come_to_anything` **every single time**
(0 of 768 would have survived that gate). A settlement was spending two turns
in three walking to ground it was already on, being refused, and never getting
as far as the question of whether to live there.

**Where the phantom sources came from.** Two places, in roughly equal measure -
434 memories and 334 scents.

The scents are a plain defect. `collect_scent_sources` gives everything a
smell that is not water as `ScentType::Food`, and reads the strength off
`ResourceType::raw_scent_strength`, which kept its own hand-written list:

```rust
ResourceType::Food | ResourceType::Grain | ResourceType::Herbs => 0.08,
ResourceType::Meat | ResourceType::Fish => 0.24,
ResourceType::Water => 0.12,
_ => 0.0,
```

That list had drifted off `is_it_food`. **Herbs**, which nobody in this model
can eat, smelled of dinner. **Greens and roots** - which are what a hedgerow
gives for two seasons out of four and most of what anybody ever eats - smelled
of nothing at all. A starving agent smells the herbs, walks to them, gathers
nothing, and does it again next tick, forever.

That is the third time this document has recorded the same shape: a question
with two answers written out by hand, kept true only by the two of them
happening to agree. `is_edible` was a fourth copy of the same six variants,
with a doc comment claiming to be the single answer.

**The fix, in three parts.** `raw_scent_strength` asks `is_it_food` and keeps
only how far a thing carries; `is_edible` calls `is_it_food`; and the branch
that walks to a known source now refuses one the settlement's own gate says is
spent - which is the check the executor was going to apply a moment later
anyway. A source further off than foraging reach is outside what that gate
looked at, so it still stands. Two guard tests hold the scent table to the food
list in both directions.

**What it did.** The same probe, after:

| what the tick did | before | after |
|---|---:|---:|
| ticks spent hungry enough to leave | 1,113 | **439** |
| walk to a source it knows | 768 (69.0%) | 1 (0.2%) |
| eat what is carried | 4 (0.4%) | 24 (5.5%) |
| hunt something near | 10 (0.9%) | 41 (9.3%) |
| **reached the branches that leave** | **5 (0.4%)** | **136 (31.0%)** |

The one remaining walk to a known source is a real one: 39 units standing, 33
paces off, outside foraging reach. And there are 60% fewer ticks in this state
to begin with, because agents in it are now doing things that feed them.

Person-days alive over 160 worlds of a full year, five paired seed blocks:

| seeds | before | after |
|---|---:|---:|
| 7000 | 1074 | 1161 |
| 0 | 930 | 1070 |
| 64 | 961 | 1016 |
| 128 | 1016 | 1009 |
| 192 | 977 | 1044 |
| **total** | **4958** | **5300 (+6.9%)** |

Worlds emptied inside the year: 86 of 160 down to 75. Split three blocks
apart, the scent fix alone is worth +3.9% and the branch gate a further +5.4%,
so both halves pay.

**What it does not fix.** Thirty-one per cent of these ticks now reach the
leaving branches and all of them take `migration_action`;
`go_and_live_where_it_is` still fires zero times, because it asks its question
once a day and wants a resource of the right kind within sixty tiles. Whether
moving house is reachable at all is worth its own measurement.

**Filed in passing.** `is_it_food` counts six resources and excludes **honey**
and **milk**, both of which the world generates and neither of which anybody
can eat. Not touched here - changing what food is, is a change to the food
supply and wants measuring on its own.

---

### 121. Three spellings of "armed", and a planner that looks one job ahead

"The planner should attempt to anticipate drive demand increase so that
actions can be efficiently executed, reducing the odds of tasks being dropped
mid-completion. Each agent should be slightly different due to varying drive
demands and personality traits. It should also allow for the proper
preparation of actions such as hunting requiring a weapon."

Three things. Two of them worked, one did not, and the measurements say which
is which.

**Where hunting was actually dying.** Over six worlds and a year, 599 hunts
were attempted and **589 failed with "No spear in hand for that"** - 98.3%. No
surviving agent anywhere held anything to hunt with. Instrumenting the rescue
path, `make_what_this_wants` was offered 643 hunts, 633 of which wanted a
spear, and in 613 of those **no step in the spear chain could be taken from
where the agent was standing** - which was next to a deer, in whatever wood or
meadow the deer happened to be in, with no stone and no wood in reach. The
preparation was being attempted one tick before the throw.

**And the requirement was written three times.** `worth_hunting` asked
`agent.equipment.get_weapon()`, which is the equipment slot and which nothing
in this model has ever filled. The executor asked
`what_i_have_to_work_with(SkillType::Hunting)`, which is the pack. And the
verb matrix asked `Wants::ThisInHand("spear")` - one item, by name. So a man
carrying a **sharpened stick, a sling or a bow** was refused before the
executor was reached, and so was a man going after a rabbit, which the
specification says a thrown stone will kill.

Changing the `HUNT` verb alone changed **nothing at all**, and the reason was
in a comment on the lookup that had been there since it was written: *"a hunt
is a throwing and a hunting and both want the spear"*.
`what_this_action_cannot_do_without` gathers every verb sharing a `done_by`,
so `THROW` went on asking for the spear after `HUNT` had stopped. A
requirement written twice is still written once you have removed it from one
of the two places.

**The fix, in three parts.**

1. `Simulation::could_bring_it_down` is the one owner of whether a kill is
   possible - the size rule the executor has always had, now asked by
   `worth_hunting` before anybody sets out.
2. The verb matrix stops claiming an unconditional requirement it does not
   have. What a hunt needs depends on the quarry, which that table cannot see.
3. `what_a_hunt_wants_first`: wanting to hunt with nothing in hand is a reason
   to go and get something, taken **where the want is formed** rather than at
   the animal. It walks the hunting ladder from the bottom - `how_much_better`
   ascending - because `what_i_would_rather_have` answers the *upgrade*
   question and names the bow, and a man who cannot come by a bow this
   afternoon then does nothing when a sharpened stick was three turns away.
   Asked for the best: 1,881 wants and 340 that came to anything. Asked from
   the bottom: 1,391 and 311, and the difference goes on.

`how_i_would_come_by` is the making step or, failing that, the raw thing the
chain is short of - the pair of questions that had been written out three
times over. And `make_what_this_wants` now takes the making on as an errand,
which `would_a_better_tool_pay` has done since `Errand::to_make` was written
and this one never did: twenty turns went on the first step of a hunting tool
in six worlds and not one was ever followed up.

**What it did.** "No spear in hand for that": **589 to nought**. The only
hunts that fail now fail for reasons a hunt should fail for - deer escaped 26,
rabbit escaped 16.

**The anticipation, and a negative result inside it.** The first cut asked
`how_long_before_this_asks` - how many turns until a need crosses its
threshold - and deferred any job a higher-ranked need would interrupt. It
fired on nearly everything. Hunger sits a few turns off its threshold most of
the time and outranks every secondary need, so a settlement stopped
provisioning, stopped building and stopped making tools and did nothing but
eat. Measured over 160 worlds against the same five seed blocks: **4,931
person-days against 5,300**, -7.0%, every block down.

Rewritten on the body's own clock - `ticks_before_this_kills_me`, which is
what `how_hard_it_presses` already reckons the primaries by - it fires 149
times in 2,870 multi-turn jobs and costs nothing measurable. A need being
about to *ask* is ordinary. A need that will have killed you before the job is
done is worth turning round for.

**Per agent, and honestly so.** Every term in `how_long_before_this_asks` is a
number the individual already carries: how far its drive is below threshold,
how fast that drive builds, and how much the weight of having been ignored
(`Drive::pressure`) is making it build faster. Two people standing in the same
field get two different answers. Which needs may interrupt which is
`DriveRank::precedence`, and personality reaches it through `weight` and
`lean` on the ranking above.

**What it did not do, which was the point of asking for it.** Errands dropped
for "something else came first" went from 1,818 of 3,445 (52.8%) to 1,661 of
3,059 (54.3%). **It did not reduce the drop rate at all.** The reason is in
the rule: the anticipation only defers to needs of a *higher band*, and what
actually ends errands is `stick_to_the_errand`'s turn-round test, which two
needs of the *same* band trip constantly as one is nibbled at and the other
builds. Fixing that means looking ahead inside a band, which needs a way to
say how hard a drive will press at a future tick rather than only when it will
start asking. Filed.

**Survival, five paired seed blocks, 160 worlds:**

| | person-days | against HEAD |
|---|---:|---:|
| before | 5300 | |
| preparation only | 5298 | -0.04% |
| + anticipation on the threshold | 4931 | **-7.0%** (rejected) |
| + anticipation on the body's clock | 5276 | -0.45% |
| + the verb matrix fixed | **5205** | **-1.8%** |

Worlds emptied inside the year: 73 of 160, against 75 before. The last 1.3%
is hunting actually happening where it used to be refused, and a hunt costs
turns; on a measure whose block-to-block noise is ten per cent, none of this
is a change in survival either way.

**The guard.** `no_verb_asks_for_one_rung_of_a_tool_ladder_by_name` fails if
any verb states its precondition by naming a tool, and
`a_hunt_asks_for_nothing_this_table_cannot_see` pins the hunt in particular.
The defect they catch cost 589 hunts in 599 and was invisible for as long as
the two lists happened to agree.

---

### 122. Going for a drink was a change of mind, and it emptied the larder

#239 was filed on the reading that undertakings are dropped because two needs
of the same standing trade places, which is the case
`what_it_takes_to_turn_me_round` was written for and the case the anticipation
rule deliberately excludes. **That was wrong, and the measurement says so.**

Tallying every drop by the band of the need that took the turn, over six
worlds and a year:

| the need that took the turn | drops | share |
|---|---:|---:|
| a **higher** band | 1,401 | **84.0%** |
| the same band | 264 | 15.8% |
| a lower band | 4 | 0.2% |

And the pairs are not scattered. **1,062 of the 1,669 - 64% - are a
Preparedness errand cut short by thirst or hunger**: 611 by thirst, 451 by
hunger. Curiosity and Social lose another 220 the same way.

So it is not two needs of a kind swapping places. It is a primary need
interrupting a secondary one, which is what primary needs are *for* - a
primary drive outranks a secondary one whatever its clock says, because
`DriveRank::precedence` is 100 against 10. Thirst does not have to be
dangerous to take the turn; it only has to be asking.

**And the anticipation rule could not have helped.** It defers when
`ticks_before_this_kills_me` runs out inside the job, and thirst's clock is a
day and a half where the errand is twenty turns. The clock says there is
plenty of time. The ranking takes the turn anyway. Both are right; they are
answering different questions.

**What was actually wrong.** `stick_to_the_errand` did not interrupt the
errand - it **destroyed** it. `self.population.agents[agent_index].errand =
None`, and the next turn the whole decision was made again from nothing. So
every attempt at putting food by ended the first time somebody got thirsty,
which on this map is every attempt, every time; the settlement started an
errand it never once finished and started it again the next day.

A man who stops for a drink has not changed his mind about the pit he was
digging. The errand is put down and picked up again: `Errand::set_aside`
counts the turns it spends waiting, `set_the_errand_aside` books it, and
resuming resets the count. What still ends an errand is arriving, giving up on
an unreachable place, being frightened off it, or leaving it standing for two
days - `HOW_LONG_AN_ERRAND_KEEPS`, long enough to outlast a drink, a meal and
a night's sleep, short enough that a patch remembered on Tuesday is not still
being walked to in the spring.

**What it did.** Errand outcomes over the same six worlds:

| | before | after |
|---|---:|---:|
| set out on | 3,047 | 1,868 |
| got there | 1,206 (39.6%) | **1,313 (70.3%)** |
| dropped for something else | 1,717 (56.3%) | **0** |
| set aside and picked up again | - | 18,108 turns |
| left standing too long | - | 130 |
| gave up on an unreachable place | 98 | 311 |

Fewer errands are begun because the ones already begun are still going. The
arrival rate goes from two in five to seven in ten.

**Survival, five paired seed blocks, 160 worlds of a full year:**

| seeds | before | after |
|---|---:|---:|
| 7000 | 1109 | 1164 |
| 0 | 1014 | 1079 |
| 64 | 1047 | 1155 |
| 128 | 1033 | 1206 |
| 192 | 1002 | 1109 |
| **total** | **5205** | **5713 (+9.8%)** |

Every block up, and the largest single gain recorded in this document. Alive
at midsummer goes from 12.24 a block to 15.85, **+29.5%**; alive at autumn
+10.1%.

**And what it costs, which is real.** Alive at the end of the year falls from
3.13 a block to 2.59, and worlds emptied inside the year goes from 73 of 160
to 84. A settlement that carries a third more people through the summer has a
third more mouths to feed in February, and the winter takes them. That is not
an argument against the change - the people are alive for most of a year
rather than dead in the spring - but it does say where the next constraint
is, and it is the same one every entry in this document has ended at: what a
settlement has in for the winter.

**A dormant field found in passing.** `Errand::pressed_this_hard` is written
at all three construction sites and read nowhere. Its doc comment describes a
rule - "so that a drive going quiet, because somebody handed this one a meal,
say, ends the errand" - which nothing implements.

---

### 123. Two thirds of the food on a map never grew back, and was deleted when eaten

Asked to work on the winter food problem, and the first measurement said there
was no winter food problem - there was barely a winter. Deaths by season over
twelve worlds of twelve founders:

| season | deaths a world | alive at the end of it |
|---|---:|---:|
| **Spring** | **9.0** | 3.00 |
| Summer | 0.9 | 2.08 |
| Autumn | 0.0 | 2.08 |
| Winter | 1.8 | 0.33 |

**Three quarters of everybody died in the first season.** Of the deaths with a
cause on them, 80% were hunger or starvation. Winter was not killing
settlements; it was finishing off the two or three people a settlement had left
by the time it arrived.

**Where the food went.** Day by day over sixteen worlds, the food standing on
the whole map fell from 7,360 units on day one to **886 by day 101** - an 88%
drawdown - while the population fell from twelve to two and a half. The ground
only recovered once there was almost nobody left on it. So the question is what
the ground produces, and the answer is best asked of a map with nobody on it at
all:

| a map nobody is standing on | |
|---|---:|
| food standing on day one | 7,641 |
| most it ever holds, in autumn | 8,155 |
| grown over a whole year | **514 units** |
| what one person eats in a year | 4,147 |

**The map grows enough food in a year to feed one person for forty-five days.**
Stripped bare and left alone for a year it comes back to 3,038 units - 37% of
what it started with - and stalls there.

**Why.** Tracking each kind separately on ground stripped bare, only **Fish**
ever came back. Greens, Roots, Food and Grain all sat at nought. And the
composition of a map's food at the turn of the year is:

| kind | units | share | grew back? |
|---|---:|---:|---|
| Greens | 3,308 | 43.3% | **no** |
| Fish | 2,784 | 36.4% | yes |
| Roots | 1,550 | 20.3% | **no** |
| Food (berries) | 0 | - | yes, out of season |
| Grain | 0 | - | yes, out of season |

**Two thirds of everything a settlement eats was a stock that never came back**,
and berries and grain - the two that do grow - have nothing standing at the
turn of the year because their bearing windows open in summer and autumn.

**And it was worse than not growing.** Instrumenting the growth pass, greens
were never offered to it at all - not once in twenty days. The reason is at the
end of every world tick: `World::remove_depleted_resources` keeps an emptied
node only if `ResourceNode::is_renewable` says so, and that function kept **its
own hand-written list** of what renews, with the same hole. So a patch of
greens picked bare was **deleted off the map**, permanently, and could not have
grown back if it had wanted to. The comment on that function states the case
against precisely what it was doing:

> A renewable node stays on the map when emptied so it can regrow; deleting it
> would make berry patches and fish runs single-use and drain the world of food
> permanently.

Three hand-written lists asked what a resource is - `how_fast_it_comes_back`
(then a local inside the growth function), `is_renewable`, and `is_it_grown` -
and two of them had never learned about Greens and Roots, which came in with
the rebuilt bearing year. It is the third time this month: `raw_scent_strength`
had the same hole (#120), and so did the three spellings of "armed" (#121).

**The fix.** The rate table is lifted onto the type as
`ResourceType::how_fast_it_comes_back`, and `is_renewable` now *asks* it rather
than answering for itself - water excepted, which is fed by `water_inflow` and
is the one thing that renews without growing. Greens are given 0.04, the rate
of herbs, because leaf is the quickest thing there is and the reason there is
anything to eat in April; roots 0.02, because a root is a season's work.

Two guards: everything that grows in the ground and can be eaten must have a
rate, and nothing with a rate may be deleted off the map when it is emptied.

**What it did.** Ground stripped bare now recovers its greens in eleven days
and its roots in a fortnight. And over five paired seed blocks, 160 worlds of a
full year:

| seeds | before | after |
|---|---:|---:|
| 7000 | 1164 | 2694 |
| 0 | 1079 | 2032 |
| 64 | 1155 | 2098 |
| 128 | 1206 | 2213 |
| 192 | 1109 | 2285 |
| **person-days** | **5713** | **11322 (+98%)** |

Alive at midsummer 15.85 a block to **40.04**; alive at autumn 9.88 to
**32.81**. Every block up, and a settlement of twelve now reaches midsummer
with eight or nine people rather than three.

**And now there is a winter problem.** Alive at the end of the year falls from
2.59 a block to **1.82**, and worlds emptied inside the year goes from 84 of
160 to **112**. A full settlement now arrives at winter with nothing put by -
the pits held between half a unit and seven all year, against the 300 one
holds - and the bearing year gives nothing between Fall-Deep and Spring-Early
by design. Before this, everybody was dead by autumn and the store was never
the question. It is the question now, and it is the first time it has been one.
Filed.

---

### 124. A settlement buried five hundred units a year and ate four of them

#240 asked why a full settlement reaches winter with nothing put by. It does
put things by. Accounting for every unit that goes into the ground over a year,
eight worlds:

| | units a world |
|---|---:|
| buried | 512.2 |
| taken back out and eaten | **4.0** |
| **rotted in the ground** | **503.9 (98.4%)** |

And of what was buried, **438.9 of 512.2 - 86% - went in raw**.

**Why raw does not keep.** Measured directly, burying a stack and waiting for
it to go:

| food | raw, bare earth | raw, lined pit | dried |
|---|---:|---:|---:|
| greens | **6 days** | 12 | 240 |
| fish | 12 | 24 | forever |
| meat | 20 | 40 | forever |
| berries | 24 | 48 | forever |
| roots | 28 | 56 | forever |

**The land gives nothing for seventy-five days running.** Nothing raw survives
that, and greens - 43% of the food on a map - last six days in the ground. A
hole full of leaf is a hole full of rot in a fortnight.

**Where it came from, and it is written down.** `putting_food_by` has:

> If there is a hole right here with room in it, use it. This goes first,
> ahead of every way of preserving a thing, and the ordering is the whole
> lesson of this batch. Burying is one turn and it is what actually gets food
> through to February.

The reasoning is right about the cost and wrong about what is bought. Burying
*is* one turn, and one turn spent burying leaf buys nothing at all. The
measurement behind that comment - that preservation-first cost a settlement two
thousand turns and put a third as much in the ground - was taken on a
settlement that was starving from the first week, where any turn not spent
eating was fatal.

**The fix.** Not "preserve before burying", which was tried and was worse:
**bury what will keep, and preserve what will not.** `Pit::how_long_this_would_keep`
answers how many days a thing would still be food for if it went in - what is
left of its own clock at the pace that hole lets it run - and
`is_it_worth_burying` holds that against the bare stretch. Where the answer is
no, the turn falls through to the drying and salting branches that were always
underneath.

`Pit::how_much_slower_things_age` is the one owner of what a hole is worth: the
same number ages what is in the pit and answers what would keep in it, so the
two cannot drift.

**What it did to the store.**

| | before | after |
|---|---:|---:|
| buried a year | 512.2 | 69.4 |
| of which raw | 438.9 (86%) | **1.1 (1.6%)** |
| of which preserved | 73.4 | **68.2 (98%)** |
| eaten out of it | 4.0 | **23.6** |
| rotted in it | 503.9 (98.4%) | **27.9 (40%)** |
| standing in the pits at year end | 4.4 | **17.9** |

The larder is no longer a rot pit. Six times as much of what goes in is
actually eaten.

**And it did not save anybody.** Five paired seed blocks, 160 worlds:
**11,322 to 11,551 person-days, +2.0%** - inside the noise on a measure whose
block-to-block spread is ten per cent. Alive at the end of the year fell from
1.82 a block to 1.30 and worlds emptied rose from 112 of 160 to 129, both on
numbers small enough (three to sixteen survivors a block) to be scatter; a
variant with one of the three branches removed came back 2670/2113 against
2672/2112, which is what noise at this scale looks like.

**Because the volume is nowhere near a winter.** Sixty-nine units go into the
ground a year. `what_one_mouth_wants_put_by` is 864, and a settlement that
reaches autumn with seven people wants about six thousand. What limits it is
not the decision to bury - that fires 886 times an autumn - but how much a
settlement can *preserve*: drying wants the agent to have watched food dry
before it will do it on purpose, wants a clear sky, wants the food cut rather
than whole, and works one stack at a time out of a pack that holds forty-odd.
Eighty-six per cent of what used to go in the ground went in raw because raw
was all there was.

So this entry fixes the arithmetic and names the constraint rather than
lifting it. The next question is preservation throughput, and it is filed.

---

### 125. Everybody is born knowing what the sun does, and nobody ate what was about to go

Two things the model got wrong about food, both raised in the same breath.

**Drying was a discovery, and it should never have been one.** Laying a thing
out in the sun so it goes hard rather than green is not an invention. It is
what happens to anything left out, and a person who has ever seen a dead
animal in a dry summer has seen it. The model made it a thing an agent had to
*watch happen* before it would do it on purpose: `is_it_worth_drying` gated on
`found_out.contains(THAT_LAYING_IT_OUT_KEEPS_IT)`, and the only two ways to
get that flag were standing over food while the weather dried it, or asking a
neighbour who already had it.

That is the throughput constraint named at the end of #124, and it is a
constraint the world invented rather than one it modelled. Removing it:
`Agent::found_out` is now seeded from `Agent::what_anybody_is_born_knowing()`,
which is the one owner of the short list of things nobody has to be taught.
Everything that used to hand the flag out is gone with it - the discovery
branch in `who_saw_that_dry`, the `PutDown` branch in `store.rs` that could
never fire anyway, and `what_asking_about_this_meal_would_teach`, because a
dried strip has nothing left to tell anybody. What is still worth asking a
neighbour about is a *making* nobody has worked out, which is what that
machinery was built for.

Watching food dry still teaches something - it is still recorded as a lesson,
which is what it was worth all along. It is no longer the difference between
being able to do it and not.

**And the eating rule preferred the food that would keep.** `find_best_food_to_eat`
scored every stack and took the highest, where the score was

> `effective_nutrition(...) * freshness`

`effective_nutrition` already folds freshness in. So freshness was applied
**twice**, and the rule read: eat the freshest thing you own. A settlement
with a week-old fish and a fresh one ate the fresh one and threw the week-old
one away four days later. Anybody who has ever kept a larder does the
opposite.

The fix is a straight reordering, and it needed one owner to make it sayable:
`FoodData::how_long_this_has_left()` answers how many ticks of edible life a
stack has, from its own clock and what has been done to it.
`Pit::how_long_this_would_keep` asks it too, so the pit and the plate cannot
disagree about what is nearly gone. `find_best_food_to_eat` then ranks by
**fewest whole days left first**, with nutrition as the tie-break inside a
day - whole days, because ranking on the raw float makes an agent eat a
mouthful of each of forty stacks in strict order of decay, which is not eating,
it is grazing. Food the model does not track a clock for sorts last.

**What it did.** Five paired seed blocks, 160 worlds, against #124:

| | before | after |
|---|---:|---:|
| person-days | 11,551 | **11,778** |
| worlds emptied of 160 | 129 | 124 |
| alive at year end, a block | 1.30 | 1.34 |

**+2.0%**, four of five blocks up - which is inside the noise, the same as
#124 was. The number that is not noise is what is standing in the pits:

| | spring | summer | autumn | winter |
|---|---:|---:|---:|---:|
| units in the ground | 62.4 | 83.3 | 66.8 | **31.4** |

Against **4.2** in winter at the commit before #124. The larder now carries
something through the bare stretch. It is still nowhere near
`what_one_mouth_wants_put_by` at 864, and #241 is still the open question:
what caps it now is the clear sky drying wants, one stack at a time, out of a
pack that holds forty-odd.

---

### 126. Every pack in the world was half as much again over its own limit, so no food would go in one

#241 asked what caps the winter store, and named drying as the suspect: it was
a discovery an agent had to watch, it wants a clear sky, it wants the food cut
rather than whole, and it works one stack at a time. #125 removed the first of
those. This entry measures the rest and finds that none of them was the
constraint.

**Where an autumn goes.** Eight worlds, 7,568 agent-ticks of autumn a world:

| | a world |
|---|---:|
| Gather | 2,743 |
| Eat | 2,331 |
| Excavate | 206 (of which **205 refused**) |
| **Dry** | **4.5** |
| **Cover** | **2.2** |

So a settlement buries twice an autumn. It is not the drive: Preparedness is
the most urgent thing on 26.6% of autumn agent-ticks, more than any need but
Reproduction. It is not the land: **6,004 units of food stand ripe on the
ground** in autumn across 248 live patches. It is not the decision, which says
yes 886 times an autumn.

**It is that nobody is carrying anything.** Of autumn agent-ticks:

| | |
|---|---:|
| had food to spare | 1.91% |
| had something worth drying | 0.48% |
| **carrying no food at all** | **96.99%** |
| mean food in the pack | **0.10 units** |

Against `WHAT_A_BODY_EATS_IN_A_DAY` of 11.52. There is nothing to dry because
there is nothing in the pack.

**And the pack is over its own limit.**

| | a world |
|---|---:|
| pack capacity | 26.01 |
| weight carried | **38.86** |
| room left | 0.22 |
| **no room for one handful** | **97.29%** |

Half as much again as it can hold. What fills it is **firewood 10.3, iron 3.7,
handaxe 2.0, tinder 1.0, stone 0.8** - a tenth of it is supper and the rest is
ballast.

**How a load gets over the limit, when `add_item` refuses what will not fit.**
It cannot be *put* there. It gets there the other way round: `max_weight` is
worked out fresh every turn by `update_inventory_capacity_from_transport`, off
what the body can lift and what it has to carry things in, and both fall. A man
loads up in his strong summer, goes hungry, weakens, and wakes in November
carrying more than he can hold. And because a pack already over its limit
refuses *everything*, the load is frozen there for the rest of his life. He can
never pick up food again.

Nothing in this model had ever put a load down. `Agent::what_i_would_put_away`
looks only at what is in the hands and fires only when a job wants one free;
the single `PutDown` in the decision layer is a curiosity experiment.

**What it costs.** Counting every unit of food a trip brings back:

| | a world a year |
|---|---:|
| into packs | 2,543 |
| **back on the bush** | **27,968** |

Eleven thrown back for every one kept, with six thousand standing ripe.

**The fix is an invariant, not a decision.** `what_nobody_can_carry_any_more`
runs before each turn: what a person cannot carry is not carried, and it is not
destroyed either - it stays where they were standing, for them or anybody else
to pick up. `Agent::what_i_would_set_down` is the one owner of what goes: the
heaviest thing that is none of food, a tool this one works with, or the pack
itself. `how_much_of_this_i_would_set_down` is the one owner of how much, taken
off `how_much_too_much_i_am_carrying` so the decision and the amount cannot
drift.

**Weight, not count.** The first cut filtered on `ENOUGH_TO_HAND`, and never
fired once: wood weighs two a stick, so the ten units of firewood filling every
pack in the world were five sticks, and five is not more than six. A reserve
counted in things cannot answer a question asked in weight.

**What it did to the pack and the store.** Eight worlds:

| | before | after |
|---|---:|---:|
| weight carried | 38.86 | **30.99** |
| firewood in the pack | 10.30 | **5.30** |
| no room for a handful | 97.3% | **88.7%** |
| meals in the pack | 0.03 | **0.31** |
| `Dry` an autumn | 4.5 | **29.4** |
| `Cover` an autumn | 2.2 | **9.9** |
| units in the ground, winter | 101.0 | **127.9** |
| dried in the ground, year end | 26.5 | **36.8** |

Six times as much drying, four times as much burying, and a store a quarter
deeper through the stretch the land gives nothing.

**And it did not save anybody.** Five paired seed blocks, 160 worlds:
**11,660 to 11,646 person-days**, which is flat. Worlds emptied went the wrong
way, **118 of 160 to 129**, and that one is not scatter: three separate
variants of this change all came back at exactly 129 against the control's 118.
The likely mechanism is the obvious one - firewood is the heaviest thing in
every pack, so it is the first thing set down, and a man with no wood does not
light a fire in February. Filed.

**Two negative results, both worth keeping.**

*A decision to make room, on top of the invariant.* `make_room_for` rewrote a
trip out for food into a `PutDown` when the pack could not take the load, on
the `free_a_hand_for` pattern. It fired 41.6 times an autumn and moved nothing:
**11,652 person-days with it against 11,657 without**. Removed. The invariant
already covers it, and two places deciding to set a load down is the drift this
document keeps naming.

*Shedding down to the limit less a day's food.* A forager loaded to the last
ounce cannot pick anything up, so shedding to exactly the limit looks like it
buys nothing - and shedding further buys a bigger store: winter 235.5 against
127.9, dried at year end 67.4 against 36.8. It also cost **five per cent of the
settlement's person-days**, 11,076 against 11,646. What a person is willing to
walk about carrying is a decision, and dressing one up as a law made it worse.
The invariant sheds to the limit and no further.

**What is still in the way.** `Excavate` is refused **205 times of 206** an
autumn, every one of them "Nothing in hand that is any use for Mining": the
verb matrix says `AToolFor(SkillType::Mining)` and the executor is written to
let a man dig with his fingers at a cost, with a comment saying in as many
words that "a settlement that cannot dig cheaply cannot keep a larder". Two
owners of one precondition, disagreeing, and the matrix wins - so a settlement
has 3.5 pits a year and burns two hundred turns an autumn failing to dig more.
That is the same defect `HUNT` had, and it is filed.

---

### 127. A map with nobody on it starved its own soil and buried its own animals

The ask was that the ecology should stand up on its own: a world with no
people in it should still be there in thirty years. Run empty, it was not.
Two defects, both of the kind this document keeps naming - a thing that left
the world without going anywhere, and a number that meant two things.

**The vegetation was in terminal decline with nobody touching it.** Midsummer
standing crop, eight worlds, no agents at all:

| | y1 | y10 |
|---|---:|---:|
| Greens | 3,516 | **2,260** |
| Roots | 1,681 | 1,277 |
| Flax | 245 | 183 |
| Cotton | 127 | 95 |

Five per cent a year, compounding, for ever. Not a boom settling to a floor -
the ratio between successive years is flat at 0.95, which is a geometric decay
to nothing.

**And it was the ground, not the growth.** Sampled every fifteen days, greens
sit *exactly* at `how_heavy_a_crop_it_carries` on all 75 patches, all year.
The standing crop was tracking a falling capacity, and capacity follows
fertility. On the tiles that grow greens, fertility went **0.60 to 0.35** in
nine years - while the map-wide mean *rose* to 0.38, which is why nothing had
noticed: the tiles that grow nothing were quietly getting richer while the
tiles that grow food were being mined out.

**By their own plants.** `regenerate_in_ground` draws
`NUTRIENT_PER_UNIT_GROWN` per unit and puts half of it straight back as root
and stalk. The other half is in the part somebody carries away - and nobody
carried it away. What nobody picked went over in its own time through
`what_it_carries_falls_off`, which **deleted it**. Every growing tile on the
map was a one-way drain with no one near it.

The fix closes the arithmetic exactly: what falls goes into the ground it fell
on, at `RESIDUE_PER_UNIT_GROWN`, because the two halves are the same plant and
the same number. A patch nobody touches breaks even; a patch that is picked
still loses, which is what picking a patch means.

| | before | after |
|---|---:|---:|
| Greens, y1 → y10 | 3,516 → 2,260 | 3,516 → **3,338** |
| Roots | 1,681 → 1,277 | 1,681 → **1,692** |
| Flax | 245 → 183 | 245 → **221** |
| Herbs | 406 → 375 | 406 → **408** |

Every growing thing settles inside five to eight years and holds.

**Then the animals.** With the hedgerows fixed, an empty world was still empty
of animals inside twenty years: **seventeen of twenty species extinct in every
one of eight worlds**. Rabbits went 1.6 → 233 → 45.8 → 1.2 → 85 → 0. Sheep,
goat, squirrel, goose, reindeer, elk, boar, fox, owl, eagle, snake, wolf,
polar bear: gone.

**Nothing ever took a dead animal out of the list.** There is no `retain` and
no sweep anywhere in `AnimalManager`, and `self.animals.len()` is what all
seven of the "is there room in this world for another animal" checks ask.
Twenty years in, an empty world held:

| | y7 | y12 | y20 |
|---|---:|---:|---:|
| animal records | 889 | 897 | 918 |
| **of them alive** | **374** | **9.8** | **15.9** |

The corpses reach `max_population` by year seven and hold every slot for
ever. Nothing can be born; the immigration pass breaks out on its first line;
the boom cohort ages out together on a one-to-three-year lifespan; and the
world empties. Ninety-nine per cent of the animal table was carrion.

Three things, all one shape:

- `AnimalManager::how_many_are_alive` is the one owner of "how many animals
  this world holds", and every cap check asks it.
- `bury_the_dead` takes the fallen off the map at the end of the animal pass.
  A body is read exactly once, in the tick it falls - a predator feeds off it
  there and then, a hunter butchers it there and then - and nothing wants it
  afterwards.
- `spawn_group` no longer asks the cap on its own account. Whether there is
  room is the caller's question and each of the three callers already asks it,
  meaning something slightly different each time; asking again here quietly
  overrode the one caller with a reason to say yes.

**And what is gone comes back.** `process_immigration` now lets a species that
is *absent* into a full map. The cap is a rough statement of how much life
this country carries, and a country carrying its whole weight in rabbits is
exactly the country a fox should walk into; refusing him for want of room is
the cap deciding which species exist. A merely thin species still waits.

It also records the peak at spawn rather than only for something alive at a
migration moment - anything that died inside its first two thousand ticks was
otherwise forgotten and could never return, which is what happened to the owl.

**Thirty years, eight worlds, nobody in any of them.** Worlds still holding
each species at year thirty, against the same run before:

| | before | after |
|---|---:|---:|
| sheep | 0/8 | **4/8** |
| squirrel | 0/8 | **6/8** |
| rabbit | 0/8 | **3/8** |
| goat | 0/8 | **3/8** |
| elk | 0/8 | **3/8** |
| deer | 1/8 | **6/8** |
| camel | 1/8 | **4/8** |
| boar | 0/8 | **4/8** |
| wolf | 0/8 | 3/8 |
| fox | 0/8 | 1/8 |
| reindeer, goose, cow, polar bear | 0/8 | present |

Nothing is permanently lost. The wolf, fox, eagle and owl still flicker in and
out - a solitary predator in a world that only ever held one or two of them
genuinely can die out, and immigration is deliberately slow - but they return
rather than being gone for good.

**What this did not fix, and it is worth being plain about it.** The total
head of animals still pins at `max_population` - 880 to 995 living, sat on the
ceiling from year seven onwards. It is an array length, not a carrying
capacity. Grazing takes **nothing at all** off the map: `process_grazing`
feeds an animal from thin air, and the comment above `process_breeding` has
said so in as many words since it was written - "grazing feeds every animal
nearly a hundred times what it burns, so hunger never becomes the limit". The
breeding crowding term is a headcount per patch standing in for the food that
should be doing the work. The ecology is now self-sufficient; what sets the
size of its fauna is still a constant rather than the land. Filed.

### 128. A tick that cost what the map was, not what was happening on it

The next thing asked for is a hundred square kilometres of country. A `Tile`
is forty bytes, so a square metre a cell puts a hundred million tiles and four
gigabytes on the heap before anything happens in it; ten metres a cell is a
thousand by a thousand, forty megabytes, and is the right unit anyway - the
model's own distances are in tens of metres, and `FORAGE_RADIUS` of 25 is a
quarter-kilometre walk.

A thousand by a thousand was not affordable. Two of the sweeps in a tick
walked every tile in the world to find the handful with anything on them:
the sprouting pass looking for seed, and the scent pass looking for muck.
Measured, per tick:

| map | world | simulation |
|---|---:|---:|
| 50x50 | 0.012 ms | 0.045 ms |
| 1000x1000, before | 9.270 ms | **47.233 ms** |
| 1000x1000, after | 0.461 ms | **1.259 ms** |

A world-year is 4,320 ticks: three and a half minutes before, five and a half
seconds after.

`Grid` now keeps a register of the ground somebody has left something on, and
those two sweeps walk the register instead of the map. What counts is what
`Soil::has_somebody_left_something_here` says: fouling and dropped seed, which
only ever arrive through one door. Litter deliberately is **not** in it -
`Soil::for_terrain` gives every tile in the world some leaf litter to begin
with, so a register of tiles-with-litter is a register of the whole map, which
costs a million inserts a tick and saves nothing. What rots litter still
sweeps, once in ten ticks, for about half a millisecond.

The interesting part is the guard. A register is a second representation of
something the map already says, and this document has a standing entry about
two representations that drift. The first cut let any caller reach through
`get_tile_mut` and foul a tile directly; three tests went red, all of them
fixtures that built a midden that way, and the failure mode in a live run
would have been a midden that never smells, never comes up in food and never
breaks down - nothing that shows as a crash. So `Grid::somebody_voided_on` is
now the only door onto fouling for anything holding a grid, and
`ecology_tests::the_ground_register_and_the_map_agree` walks the whole map
after thirty days of a live settlement and asks the register about every tile
it finds muck on, in both directions.

Fingerprints over five seeds and 1,200 ticks are identical to before the
change.

---

### 129. A hundred square kilometres, and a country frozen on its first morning

The map is now `Grid::METRES_PER_CELL` (ten) into a thousand cells each way:
a hundred square kilometres, reached by `WorldConfig::big_enough_for_an_ecology`.
`WorldConfig::default` stays at fifty by fifty and says in its own doc why - it
is the map a test builds, and a test that ticks a hundred square kilometres to
find out whether one man ate is a test nobody runs.

Making it that big turned up four things, three of them cost and one of them
plain wrong.

**The counts were counts, not densities.** Every number in `ResourceConfig` was
an absolute. A hundred square kilometres came out with the same 361 nodes a
quarter of a square kilometre had, spread over four hundred times the ground.
`ResourceConfig::spread_over` scales them against the map they were written for
(`THE_MAP_THESE_WERE_WRITTEN_FOR`, fifty by fifty), and the animal and plant
ceilings go through `Grid::at_the_very_outside` for the same reason. 133,246
nodes and 78,078 plants now, which is the same country only more of it -
`land_tests::a_bigger_map_carries_more` holds the density to within a tenth.

**Placement was the square of the map.** Each node placed asked
`is_position_occupied`, which walks the whole resource list. Stocking a map
places about one node per seven tiles, so building a world cost n². A quarter
of a square kilometre took a millisecond, twenty-five square kilometres took
5,320 ms, and a hundred would not finish. The scan is hoisted into a register
carried through the three spawners (`World::what_ground_is_taken`), and
`spawn_naturalistic` no longer looks up the plant it just planted by id -
`Vec::last_mut` is the one on the end.

**The biome cache was keyed by a coordinate and never cleared.** A biome is a
question about what kind of ground this is and what the calendar says; the
position never entered the calculation. Keying it by position meant one entry
per tile anything had ever asked about - 133,000 of them, one BTreeMap lookup
per node per pass - and, because nothing has ever called `clear_biome_cache`,
**the answer was frozen at the hour and the day it was first asked**. A wood in
a world a year old still carried the temperature of the first morning of it.
Only the weather modifier laid over the top moved at all.

**And the season was a float pretending to be one of four things.**
`Biome::season` was an `f32` documented "0.0 to 4.0, representing
spring/summer/fall/winter", read back as `self.season as u32` and matched
against 0..3. Both tests that set it wrote 1.0 and 3.0 and got what they asked
for. The one live caller wrote `day_of_year / DAYS_PER_YEAR`, which is a
fraction under one, which casts to zero, which is spring. **No world has ever
had a winter as far as its biomes were concerned.** It is a `Season` now.

What a tick costs, at a thousand by a thousand:

| | world | simulation | a world-year |
|---|---:|---:|---:|
| before any of this | 9.270 ms | 47.233 ms | 204 s |
| ground register (#128) | 14.489 | 15.843 | 68.4 s |
| biome by ground, not coordinate | 10.021 | 10.737 | 46.4 s |
| canopy flat, not a tree map | 5.047 | **5.693** | **24.6 s** |

(The register's own row is higher than #128's because the map now carries four
hundred times the nodes and plants; per node it is far cheaper.)

The canopy was the last of it: `tick_in_world` gathered what stands over each
tile into a `BTreeMap` keyed by position, five entries per plant, 390,000
tree inserts a pass. Four megabytes of floats indexed flat is an order of
magnitude cheaper, and the map was never sparse.

Building the big world once still costs about six seconds, most of it the
fallback full-map scan in `find_random_terrain_position` when a hundred random
tries fail to hit rare ground. One-off, and left alone.

Thirty-two worlds, 4,320 ticks, against the same code without the climate fix:
mean last-alive tick 3,772 -> 3,876, mean peak store 112.2 -> 122.3, headcount
at tick 1,000 8.31 -> 7.69. Winter costs something now and more is put by
against it. No test failed that was not already failing;
`news_reaches_everybody_within_earshot` started passing.

### 130. The naturalistic spawner puts two clusters on one tile

`NaturalisticSpawner::spawn_all` builds its own list and never asks what is
already standing where it is putting things, so its clusters land on top of
each other and on top of what the basic spawner placed: eighty by eighty gives
71 doubled tiles, all of them clay, sand, coal, grain, flax, herbs, cotton,
honey or fish. Whichever of the two a tile lookup finds first is the one that
exists as far as anything asking about that tile is concerned; the other is
inventory nobody can reach.

Found by `land_tests::stocking_a_map_leaves_no_two_things_on_a_tile`, which
holds those nine kinds out by name rather than lowering its sights, so the day
this is fixed the exclusion list comes out. Filed.

---

### 131. Plants that never grew old, never seeded, and never died

Vegetation was a fixture. A plant had an `age_ticks` and nothing read it, so
a hedgerow put down when the world was made was the same hedgerow three
hundred years later; the only way anything ever left the map was somebody
harvesting it, and nothing new ever came up. Trees in particular had no life
cycle at all, which is the thing that decides what a wood is.

Now:

- **Lifespans, worked out rather than written down.** `lives_for_years` comes
  off `size` and `is_tree` rather than being a fifty-second field on each of
  fifty-one hand-written species - fifty-one numbers is fifty-one numbers to
  drift, which this document has a standing entry about. A grass or a herb is
  two years, a bush thirty, a birch eighty, an oak two hundred and fifty, the
  largest trees eight hundred.
- **A founding wood has an age in it.** `spawn_naturalistic` gave everything
  it planted `age_ticks` of nought, so a wood laid down all at once would come
  down all at once; the founding cohort is scattered across a lifetime now.
- **Aging happens whether or not a plant grows.** It sat below two early
  returns and below the `share <= 0.0` gate, so a plant on ground too poor to
  grow on did not get any older.
- **Seed.** A bearing plant drops seed within four cells - forty metres, which
  is where nearly all seed goes. On ground of a kind its species can live on it
  gets one throw, and takes or fails; on ground its kind cannot live on it lies
  there and rots on its own clock, a season for a tree's seed and two for small
  dry seed.
- **Two ways to die.** Old age, and a plant that could not make a living where
  it stood: `growth_share` below `WHAT_A_PLANT_NEEDS_TO_HOLD_ITS_OWN` takes
  condition off it, and at nothing it goes over. Both put the plant back into
  the ground it was standing in, woody litter for a tree and leaf for a herb.
- **Something bigger comes up through something smaller.** A sapling in a
  sward shades the sward out; no amount of grass seed displaces a sapling.

Four of those came out of measurement rather than design, and each was a case
of the ecology settling somewhere obviously wrong.

**The light gate did nothing.** Germination was a fresh throw every pass and a
seed keeps for hundreds of them, so every seed that ever landed on free ground
took in the end however dark it was. A hundred and twenty by a hundred and
twenty went from a thousand plants to ten thousand in twenty years, still
climbing, with every grass and herb crowded out by year thirteen. One throw
fixed it.

**Seeds per lifetime had to be the same for everything.** At a flat chance per
pass, a plant's seed output over its life is proportional to its lifetime, so
an oak seeded a hundred times over and a grass a fraction of once. `seeds_per_pass`
is now the reciprocal of the lifetime, and the two come out even over a life.

**A flat count is not enough on its own.** Counted over twenty years: two seed
in five landed on country of a kind their species cannot live on, and of the
three that could, about one in twenty got a root down. So a seed is worth
about three hundredths of a plant, and twenty-five seed a lifetime is
two-thirds of a successor - every class on the map on its way out. The bushes
went first, being neither long-lived enough to wait it out like a tree nor
quick enough to flood the ground like a grass: 145 at the start and none at
all by year 150. A hundred a lifetime leaves about three, and what brings
three back to one is ground that is already taken.

**The seed bank was deciding the ecology.** While the bank is full nothing
seeds at all, so which species held the ground came down to who was earliest
in the list. At a quarter of the plant ceiling it was full from year three
onwards.

A hundred and twenty by a hundred and twenty, nobody on it, one hundred and
twenty years:

| year | trees | bushes | small |
|---:|---:|---:|---:|
| 0 | 326 | 145 | 607 |
| 20 | 365 | 340 | 9,601 |
| 60 | 484 | 1,460 | 8,351 |
| 120 | 696 | 3,099 | 6,096 |

Which is succession: the open ground goes to grass, the grass goes to scrub,
and the scrub is going to wood. Nothing is lost.

**What it costs, and what should pay for it.** The map goes from seven per
cent vegetated to about sixty-eight, which is nine times the plants, and a
year at a hundred and twenty by a hundred and twenty went from 0.2 s to 2.4 s.
Sixty-eight per cent vegetated temperate country is right; what holds it back
in the world is something eating it, and nothing in this model eats a plant
yet. That is the next entry.

Thirty-two worlds, 4,320 ticks: mean alive is up a little at every mark after
the first two (7.69 -> 7.91 at tick 1,000, 5.53 -> 5.97 at 3,000), mean peak
store 122.3 -> 125.8, mean last-alive tick 3,876 -> 3,783.

One test moved: `survival_loop_tests::population_feeds_itself_over_a_long_run`.
It builds its world without seeding `dice`, so its outcome shifts with any
change to how much randomness anything upstream draws, and this change draws
two rolls a pass that were not there before. A test that cannot survive a new
`rng.gen` somewhere else in the model is not holding a line; it should take a
seed like the rest. Filed as #132.

### 132. A test that fails when anything else draws a random number

`survival_loop_tests::population_feeds_itself_over_a_long_run` runs 4,000
ticks of a twelve-person settlement on an unseeded world and asserts that
somebody is still alive. Roughly a third of worlds are empty by tick 4,000 on
any version of this code, so what the test actually depends on is which world
the shared `dice` stream happens to hand it - and that moves whenever
anything anywhere else in a tick draws a roll it did not draw before. It went
red on the plant-seeding change for that reason and for no other. It wants a
seed, and an assertion about a block of worlds rather than about one. Filed.

---

### 133. Grazing that took nothing, and a herd size that was a number in a field

`process_grazing` fed an animal out of thin air. The comment above the
breeding pass had said so in as many words since it was written - "grazing
feeds every animal nearly a hundred times what it burns, so hunger never
becomes the limit" - and what stopped a herd growing instead was a headcount
of mouths per six-by-six patch with a ceiling of eight on it. Nothing on the
map got any smaller for being eaten and nothing went back onto it.

Now `AnimalManager::tick_in_world` takes the ground and the growing things.
A mouthful comes off a plant standing within a step, and the plant is that
much less of a plant for it. What the animal cannot use lands behind it as
muck. A plant cropped to nothing dies on the vegetation's own pass. An animal
that feeds by digging - a big omnivore - takes the whole plant rather than
cropping it, so a bear that digs up a root kills it; that is decided by how
the animal feeds and not by a hand-written list of which plants are roots.
`PATCH_CARRYING_CAPACITY` and `GRAZING_PATCH` are gone: `can_breed` already
asks whether an animal is fed, and now that being fed depends on the grass,
carrying capacity comes out of the grass.

Five things had to be found by measuring, and every one of them was the
ecology settling somewhere plainly wrong.

**The first calibration preserved the number the module calls out as wrong.**
Setting the appetites so a mouthful came out worth what the old flat rates
gave kept the hundredfold. Five thousand seven hundred head on a hundred and
forty-four hectares with a mean hunger of 0.30: the grass was still infinite,
by arithmetic instead of by omission. What an animal reaches for is its own
`hunger_rate` and a margin now, and a point of plant condition answers a point
of hunger, so the two ends of the exchange are both real things.

**A hungry animal stood still.** It either did nothing or shuffled a cell or
two at random when its `state_timer` ran out. Twelve sheep cropped their own
few tiles bare by tick 2,800 and then took **not one further mouthful in
three thousand ticks**, with six hundred plants and thirty-eight thousand of
standing growth on the map around them. An animal that finds nothing in reach
walks towards the nearest ground with something on it.

**A grown tree is browse, not nothing.** Excluding standing timber outright is
right about trunks and wrong about a wooded map, where most of what is growing
is timber. A grown tree gives a flat small amount - the shoots something on
four legs can reach - whatever its size, and cropping it does not touch it.

**A fresh map could not feed the fauna it spawned.** `spawn_naturalistic` put
212 plants on 2,500 tiles, from when a `Plant` was a fixture nothing ate. Left
alone the same country settles at about two-fifths covered, so those figures
were not a sparser world, they were the same world before it had filled in - 
and in the meantime a dozen sheep on twenty-five hectares, light stocking for
real ground, starved. A world opens near where it settles now.

**A plant took more out of its tile than it put back.** An appetite that took
no notice of what kind of plant it was, and a leaf fall by size that took no
notice of what the plant had drawn: two unrelated tables for the same physics,
and the third accounting of it in this model. A small plant took two and a
half times what it returned, and a meadow nobody walked on lost a tenth of its
fertility in a year. A plant that has finished growing gives back everything
it takes; one still building itself keeps half, which comes back when it dies.

What a hundred and twenty by a hundred and twenty does with nobody on it, over
a hundred and fifty years:

| year | trees | bushes | small | animals |
|---:|---:|---:|---:|---:|
| 0 | 326 | 145 | 607 | 174 |
| 15 | 318 | 154 | 4,998 | 31 |
| 60 | 338 | 151 | 5,683 | 63 |
| 150 | 365 | 130 | 5,673 | 55 |

The animals overshoot, eat their ground back, fall, and settle into a cycle
between twenty and a hundred head - which is the shape of a forage cycle and
not a number in a field. Nothing goes extinct.

**Speed.** A tick at a thousand by a thousand, over the whole of this work:

| | simulation | a world-year |
|---|---:|---:|
| before any of it | 47.233 ms | 204 s |
| after the ground register and the map (#128, #129) | 5.693 | 24.6 s |
| plants that live and die (#131) | 8.067 | 34.8 s |
| grazing, and a map stocked to feed it | 16.088 | 69.5 s |
| the growing worked out once in twenty ticks | **11.353** | **49.0 s** |

Two things paid for most of it: the grazing lookup was rebuilt over every
plant every tick, which was three-quarters of a tick on its own, and now runs
on the ten-tick cadence with a flat index; and the vegetation pass, which is
the single most expensive thing in a tick and the one that least needs doing
often, runs once in twenty ticks and stands for twenty. Twenty ticks is under
two days and nothing a plant does happens faster than that.

So a living ecology on a hundred square kilometres - a quarter of a million
plants growing, seeding and dying, and the fauna that eats them - costs about
twice what a dead one did, and a quarter of what the same map cost at the
start of this work.

Thirty-two worlds, 4,320 ticks, against the same code without grazing: mean
alive up at five of the seven marks, mean last-alive tick **3,783 -> 4,062**,
mean peak store 125.8 -> 101.1. Settlements last longer and put by less, which
is what you would expect from a country where the game has to eat too.

`predator_prey_tests::predators_hold_a_herd_down` was rewritten rather than
made to pass. It asserted that a herd nothing eats grows without limit and
that wolves are what stops it - true of the model it was written for, where
grazing was free and predation was the only brake. Now the grass is the brake,
a herd nobody hunts overshoots and crashes, and a herd wolves keep under
carrying capacity is *better* fed: over six seeds, twelve sheep ended at
fourteen unmolested against thirty-three with six wolves in with them. What it
holds now is what is still true - wolves eat sheep, and neither herd runs away
to the ceiling - measured over a block of seeds, because one run of a system
that oscillates says nothing.

`bearing_tests::ground_nobody_harvests_is_no_poorer_a_year_later` now measures
from the fifth year rather than the first. That is a change to when the
question is asked and not to the question: a world opens with less standing
growth than it settles at, so a tile genuinely and rightly loses ground in
year one, into the plants standing on it. What has to break even is the steady
state.

Two tests moved and both are of the kind #132 names: `clothing_tests::a_cold_agent_ends_up_dressed`
and `situation_tests::a_settlement_works_things_out_that_nobody_wrote_down`
build unseeded worlds and assert on the outcome of one settlement, so they
turn over whenever anything upstream draws a roll it did not draw before. The
clothing one fails as an index panic on an empty population rather than as an
assertion, which is #228.

---

### 134. A quarter of a million plants, all of them asked every pass

Vegetation was worked out for the whole map at once. It had been every ten
ticks, then every twenty, and at a hundred square kilometres carrying 247,419
plants it was still the most expensive thing in a tick by a wide margin - four
milliseconds of eleven, and every one of those plants asked whether anything
had happened to it, three-quarters of the time to be told no.

The map is cut into twenty-four bands of rows and one band is grown every
sixty ticks, so a plant is worked out once in 1,440 ticks - four months - and
no single tick carries more than a twenty-fourth of the map. Ground something
is standing on does not wait: `AnimalManager` calls `PlantManager::catch_up_one`
on a plant before it takes a bite out of it, which is what "unless there is
something within interaction range" comes to.

| | simulation | a world-year |
|---|---:|---:|
| every twenty ticks, whole map | 11.353 ms | 49.0 s |
| one band in twenty-four, every sixty | **3.705 ms** | **16.0 s** |

The vegetation went from 4.02 ms a tick to 0.518. A living hundred square
kilometres now costs less per tick than the same map cost when nothing on it
was alive (5.693 ms, #129), and a fifteenth of what it cost at the start of
this work.

**The thing that made it safe was giving every plant one clock.** Two paths
can now grow the same plant - its band's turn, and something standing on it -
and neither is told how many ticks to stand for. Each subtracts
`Plant::grown_up_to` from the tick it is asked about and writes the new one
back, so a plant grows exactly once for every tick that has passed however it
is reached. A `Seed` carries `dropped_at` for the same reason: how old it is,
is now less when it fell, rather than a second counter something has to
remember to wind.

**Three things were only ever right because a pass was short.**

- Growth advanced one stage per call and threw the remainder away. Ten ticks
  could never carry a plant through more than one stage; 1,440 is several for
  anything quick, so a grass would have stuck one step short of bearing for
  ever. It loops now.
- Seeding was a coin whose chance was clamped to one. Over 1,440 ticks a grass
  should shed eight and would have shed one. It draws a count now.
- A plant on ground that will not keep it lost `HOW_FAST_A_PLANT_GOES_BACK`
  per tick, from conditions read once at the top of the pass. Over four months
  that is seven-tenths of the plant inferred from a single wet afternoon or
  dry one, so a pass can now take at most a third of it, and three bad passes
  in a row - a year - still kill it. `Soil::draw` is bounded the same way and
  for the same reason: a straight line drawn from the opening rate runs ahead
  of the true curve, which tapers as the ground empties.

**And one thing was plainly wrong the moment plants stopped being immortal.**
`spawn_plant` left the clock at nought, so anything that came up mid-run aged
the whole run the first time its band came round - for a grass, six times its
own lifetime, so it was dead before it had grown. Every grass and herb on a
hundred and twenty by a hundred and twenty was gone by year fifteen and every
bush by year forty-five, with only the trees left standing because a tree can
afford it. `spawn_plant`, `plant_crop` and `spawn_patch` all take the tick
now, so no caller can forget to say which one.

**What it costs in fidelity.** The country settles thinner than it did under
twenty-tick passes. At year 100 on a hundred and twenty by a hundred and
twenty with nobody on it:

| | trees | bushes | small | animals |
|---|---:|---:|---:|---:|
| every twenty ticks | 865 | 136 | 4,382 | 1,229 |
| one band in twenty-four | 749 | 29 | 1,508 | 339 |

Most of that is recruitment. A seed gets one throw, and under the old cadence
it got it within ten ticks of landing; now it waits up to four months for its
band, and by then the tile it fell on is more often taken. That is a real cost
of stepping coarsely and it is the price of the fifteenfold. Thirty-two
worlds, 4,320 ticks: mean last-alive tick 4,062 -> 3,976 and mean peak store
101.1 -> 120.1, so settlements are unmoved either way.

The bush decline is **not** from this change: at the commit before it, the
same run takes 524 bushes to 71 over a hundred and fifty years. It came in
with the denser starting vegetation of #133, where the balance was checked at
the old density and not the new one. Filed as #135.

One more test moved, and it is the family #132 names:
`longevity_tests::the_young_are_kept_warm_by_the_adults_around_them` builds an
unseeded world and fails with a mean of `inf` - a mean over an empty group,
which is a settlement that died out, not a thermal fault.

### 135. Shrubs go out of a country that nobody is managing

Left alone for a hundred and fifty years, a hundred and twenty by a hundred
and twenty takes its bushes from 524 to 71 and keeps going down, while its
trees hold and its grass holds. Woody middling growth - `PlantSize::Medium`
and not a tree, which is the hazel, the bramble, the berry bush - is squeezed
from both sides: it sheds a fiftieth of the seed a grass does because it lives
thirty times as long, and it is the best browse on the map, so what does come
up is eaten. Neither of those is wrong on its own and the two together are too
much.

It arrived with the denser starting vegetation of #133: the seed-per-lifetime
figure was fixed against a map that opened with 145 bushes and the map now
opens with 524, so the share of the seed rain a bush gets is four times
thinner against a grass population that grew by the same factor. What it
probably wants is for browsing pressure to fall off as the browse gets scarce,
which is the one feedback the grazer does not have - it takes what is in reach
and does not care how little is left. Filed.

### 136. Every animal was two thirds of a food chain, and the map had no say in it

Four things decided what a country was stocked with, and none of them was
about the country.

**A ratio where a pyramid belongs.** `prey_to_predator_ratio: 2.0` put a third
of everything on four legs into the business of eating the other two thirds,
and made no distinction at all between a fox and a wolf: one bag of
"predators", drawn from evenly. There are not as many wolves as foxes and
there are not half as many deer as wolves. `TrophicRole` is the shape the
chain actually has - grazers, small predators, mid-level predators, top
predators, seven tenths and eighteen, nine and three hundredths - and it is
worked out from what a species is rather than declared on it. A thirty-fourth
hand-written field on thirty-three species is thirty-three chances to say
something the other fields already contradict.

**What decides it is what it eats, not how big it is.** A wolf and a fox are
both `AnimalSize::Small`; the comment on the enum says so in as many words
("Small: Foxes, wolves"). So size cannot separate the top of the chain from
the middle of it and the prey list has to: a fox takes rabbits and a wolf
takes deer. Own size is a floor and never more, which was the second half of
this and got it wrong on the first attempt - reading own size on the same
scale as prey size filed the boar and the harbour seal with the tigers,
because both are `AnimalSize::Medium` and a medium *prey* animal is what an
apex predator eats. Nothing is apex by being large. A bear is apex because it
takes deer.

**A head count that was an absolute.** `max_initial_population: 200`, whatever
the map. It never bound on a fifty by fifty and bound at once on a hundred
square kilometres, where it held the whole country to two animals a square
kilometre - and worse, it was one pool filled first-come, so the grazers spent
all two hundred of it before anything that eats them was placed at all. A
hundred square kilometres came out with a thousand head on it and not one
wolf. It is `head_per_10000_tiles` now, and each tier draws against its own
share of it.

**And the map may veto the top of the chain.** A quarter of a square kilometre
with a wolf pack on it is not a small ecosystem, it is a pen: the wolves eat
everything in it and then starve. Only the top tier is held to this, which is
both what the specification says ("only where habitat scale supports them")
and the only place the argument holds - a fox on the same ground is a fox
whose range runs off the edge of the map, which is every animal in this model.

Two things had to be fixed before any of it would show:

- **A species that could not live here lost its slot rather than yielding it.**
  The spawner drew a climate out of the species and then asked whether the map
  had any of that climate; when it had not, the herd or pack asked for was
  simply thrown away. On a small map most of the registry's biomes are absent,
  so a fifty by fifty came out with no predators at all - one pack wanted, one
  draw, and the draw was an arctic fox. The draw is now made among the species
  that could actually live on this ground.
- **The herbivores were drawn evenly, so a small map was stocked with
  mammoths.** A quarter of a square kilometre carried cows, elk and mammoths
  and not one rabbit or squirrel. That is odd to look at and fatal to the
  middle of the chain: every predator below a wolf in this registry lives on
  rabbits, squirrels and fish, so a country with no small herbivores in it has
  nothing for a fox to eat. Species now enter the draw as many times as a
  thing of their size is common - sixteen for a tiny one against one for a
  huge one - and the same fifty by fifty carries rabbits, squirrels, geese, a
  few deer and a hawk.

**And the hunt stopped asking about everything.** A predator looked at every
animal in the world to find one within eight tiles of it, which is every
predator against every animal, most of it string comparison against a list of
prey names. On a hundred square kilometres carrying four thousand head that is
millions of comparisons a tick to find the handful of animals actually in
front of it. Animals are bucketed into blocks the size of a hunt and a
predator looks in the nine around it.

What comes out, seeded alike at four sizes:

| | ground | head | grazers | small | mid | top | ms/tick |
|---|---:|---:|---:|---:|---:|---:|---:|
| 50x50 | 0.2 km² | 26 | 25 | 0 | 1 | 0 | 0.057 |
| 200x200 | 4.0 km² | 161 | 143 | 0 | 18 | 0 | 0.279 |
| 500x500 | 25.0 km² | 211 | 180 | 0 | 22 | 9 | 0.813 |
| 1000x1000 | 100.0 km² | 823 | 703 | 0 | 90 | 30 | 4.932 |

A hundred square kilometres is 21.3 seconds to the world-year, against 16.0
for the same map when it carried two hundred head and no wolves. The top of
the chain appears between four square kilometres and twenty-five, which is
where the rule puts it. The empty column is #137.

**Settlements do better for it**, which was not the point and is worth
recording. Thirty-two worlds, 4,320 ticks, world for world:

| alive at tick | 250 | 500 | 1000 | 1500 | 2000 | 3000 | 4000 |
|---|---:|---:|---:|---:|---:|---:|---:|
| before | 10.63 | 9.66 | 8.63 | 7.16 | 6.91 | 5.50 | 0.94 |
| after | 10.81 | 10.31 | 9.19 | 8.38 | 8.00 | 6.88 | 1.03 |

Mean last-alive tick 3,976 → 3,999 and mean peak store 120.1 → 129.1. The gain
is in the middle years and it is about a quarter at tick 3,000. The likeliest
reason is that there is now small game on the map and a man can take small
game: `could_bring_it_down` wants a hunting tool for anything a thrown stone
will not kill, so a country of cattle and mammoths is a country a stone-age
settlement cannot hunt in at all, and a country with rabbits in it is not.

Four tests moved, and all four had been reading a number that came out of
where the random stream happened to be standing - the family #132 names, since
any change at all to world generation moves every draw after it:

- `hunting_tests::an_agent_hunts_for_the_skins_it_needs` spawns a deer three
  tiles from an unarmed agent and asserts he sets out after it. He cannot,
  and never could; what he was actually walking towards was a hawk eleven
  tiles off that the default world happened to have stocked. It now clears the
  world's own animals and gives the man a spear, which is what its name claims
  it is about.
- `fishery_tests::an_agent_at_the_water_catches_something` compares four rungs
  of tackle, each drawn from wherever the stream stood when its arm began. It
  read "hands 25, spear 52, rod 75, net 0" - a net that landed nothing in
  sixty casts, which is not a statement about nets. Each rung is seeded alike
  now.
- `armed_tests::a_spear_tells_when_you_stand_your_ground` seeded *before*
  building its world, which pins the world and leaves the fight wherever the
  world left the stream. It read 1.4 blows bare-handed against 1.3 with a
  spear: near enough every fight over in one blow, with no room for a spear to
  tell. It seeds after the world is built, because the fight is the thing
  being measured.
- `news_tests::news_reaches_everybody_within_earshot` is seeded, because
  whether twelve people who wander at random fall within earshot of each other
  is a draw.

### 137. There is no such thing in this world as a small predator

The chain this model can build runs grass, grazer, fox, wolf, and skips a
rung. Of thirty-three species in `FaunaRegistry`, eighteen eat plants, nine
are mid-level predators and six are at the top; the small-predator tier -
amphibians, reptiles, small birds, the smaller mustelids, everything that
lives on insects, eggs, frogs and mice - is empty, and no arrangement of the
existing species will fill it. Nothing in the registry is both small enough
itself and takes small enough prey: the smallest thing that hunts anything is
`AnimalSize::Small`, which is the fox and the owl and the hawk, and the
specification calls those mid-level.

It shows up as a hole in the middle of every country the model stocks. Eighteen
hundredths of a country's groups belong to this tier; those groups are asked
for, nothing can fill them, and the country comes out that much thinner than
it is meant to be, at every map size measured.

The snake is the near miss and is instructive. It is in the specification's
small-predator list, and it is in this registry, and it comes out mid-level
because it is `AnimalSize::Small` and there is no size below it that a hunting
animal is allowed to be. `AnimalSize::Tiny` exists but every animal in it is a
rabbit, a squirrel or a bird that eats seed.

Filling it wants species rather than a rule change: a frog, a lizard, a
songbird, a stoat, a shrew - things that eat insects and eggs and each other's
young, and that a fox and an owl in turn live on. Two of the specification's
other guilds are missing in the same way and probably belong in the same piece
of work: nothing scavenges (vultures, crabs, scavenging fish - the model
already has carrion falling to the ground and rotting untouched, see #169) and
nothing engineers a habitat (beavers, burrowing animals, oysters).

One neighbouring oddity in the same data, filed here rather than separately:
`fish` is `DietType::Carnivore` with an empty prey list, so it is counted a
primary consumer. That is what the data supports and it is not far wrong for
what the model uses fish for, but the specification asks for predatory fish as
a real guild and there is nowhere for one to go.

### 138. Now that the chain runs, it eats itself out - and it was not that

**Superseded by #139, and left here because the mistake is the instructive
part.** Everything below reasons from head counts to a cause, and names
predation. A tally of what actually kills was built afterwards and says four
animals were taken by predators in ten years, against five hundred and five
starved and three hundred and fifteen dead of old age. The die-off is real;
the reading of it was wrong, and the wrongness came from inferring a mechanism
from a population curve instead of counting.


Putting small herbivores on the map (#136) gave every predator below a wolf
something it can actually eat, and the food chain started running for the
first time. It does not settle. Over eight worlds and five years, with nobody
in them:

| | species held | head, year 0 → 5 |
|---|---:|---:|
| 0.25 km², before #136 | 38 of 60 (63%) | 344 → 182 |
| 0.25 km², after | 15 of 39 (38%) | 295 → 121 |
| 4 km², before #136 | 68 of 88 (77%) | 1,624 → 2,201 |
| 4 km², after | 57 of 113 (50%) | 1,272 → 1,153 |

The head is steadier than it was - four square kilometres used to grow by a
third in five years, unchecked, because the herbivores that were stocked were
cows and elk and mammoths and nothing in the registry could take one. What
moves now is the *roll call*. The species that go are the small herbivores -
rabbit, squirrel, goose - and then, one after them, the things that live on
those: fox, owl, hawk, eagle, arctic fox, snake. It is a cascade, and it runs
the same way at every map size measured, from a quarter of a square kilometre
to twelve.

Half of it was there before and was hidden. Even at four square kilometres
before this change, seven of the species that went were herbivores being eaten
out; what is new is that their predators now follow them down, which is the
truer outcome and the more visible one.

The thing missing is the same one #135 names for browsing, in its other half:
**nothing about hunting slackens as the prey get scarce.** A predator takes
what is within `HOW_FAR_A_HUNT_REACHES` of it and does not care whether that
was the last rabbit in the county; there is no refuge, no search cost that
rises as the quarry thins, and no switch to a commoner prey. A pair of species
with a fixed per-capita take and no brake is the textbook unstable
predator-prey pair, and this is what one looks like from the inside.

What it probably wants is for a hunt's chance of finding anything to fall with
how much of it there is left - the predator's own version of "browsing
pressure falls off as the browse gets scarce" - and for a predator that
consistently finds nothing to move rather than to starve where it stands.
`most_of_what_lived_here_still_lives_here` had to come down from half to a
quarter to state what the model actually does, and it now guards the head as
well as the roll call so that a country reduced to one rabbit of every kind
does not read as a country that kept its species. Filed.

### 139. It was never the wolves: what actually empties a country

#138 said the food chain was eating itself out, and it was wrong. That entry
inferred the cause from head counts, which is how it came to name predation;
so the first thing built here was a tally of what actually kills - `AnimalManager::what_carried_them_off`
- and the answer is not close. Ten years, four square kilometres, nobody on
the map:

| | deaths in ten years |
|---|---:|
| starvation | 505 |
| old age | 315 |
| **taken by something that eats** | **4** |

Four animals in a decade. Predation is not a brake on anything in this model
and never has been. Behind that reading were five separate faults.

**Every animal in a new world was born on the same morning.** `Animal::new`
sets `age: 0`, so a country's whole fauna was one cohort: nothing could breed
until the first maturity age had passed, and then - between years two and
seven, which is what these lifespans come to - the entire founding stock died
of old age within a season or two of each other. 161 head at the start, 395 by
year two, 149 by year three, 34 by year five, with never more than eighteen of
them starving at once. The flora had exactly this and it was fixed there; the
fauna had never been looked at. `spawn_naturalistic` spreads the founding
cohort across each animal's own span now.

**What was born did not depend on how many were breeding.** `process_breeding`
gated the whole world behind one roll in a hundred and then `break`ed after a
single pregnancy *per species*, so three rabbits and three thousand rabbits
recruited at the same absolute rate - about forty litters a species a year
however many there were of it. Predation is per-predator and therefore
proportional to the herd; recruitment was a constant. That is not a balance
that can be tuned: a constant birth rate against a proportional death rate has
one outcome whatever the constants are. Every fit pair with a mate by it now
takes its own chance, and what paces a species is its own cooldown and
gestation, which is where that belongs.

**A mouse ate thirteen times what a mammoth ate.** `what_it_reaches_for` was
`hunger_rate * 3`. `hunger_rate` is a rate on the animal's *own* scale - how
fast its own belly empties against its own `max_hunger` - and it runs the
other way from size, 0.20 for a mouse and 0.015 for a mammoth. Read as forage
off the ground it says the mouse eats more. Two different quantities were
being read off one number: how much grass an animal removes, which is bulk,
and how much good that does it, which is metabolism. They are split now -
`what_it_reaches_for` by size, `what_a_mouthful_is_worth_to` by the animal -
and the net energy balance is unchanged for a middling grazer, which is the
part that had been measured and tuned.

**A hunt was a speed ratio and nothing else.** `(pred_speed / prey_speed) *
0.4`, and nothing about the ground, the herd, or what the quarry could do
about it. So a lone wolf took a cow out of the middle of a herd at the same
rate it took a hare in an open field, a rabbit's burrow was worth nothing, and
no hunter ever came off worse for trying. Four things bear on it now: a way
out this ground offers that the hunter cannot follow (down a hole, up a trunk,
into the air, into the water - and whether the hunter digs, climbs, flies or
swims in its turn); cover, which helps whichever of the two is smaller, so a
wood shelters a hare from a fox and shelters a wolf coming up on a deer; what
it takes to bring the quarry down against what the hunters bring, cubed, so
that a lone wolf against five cattle standing together brings a twelfth of
what it needs and a twelfth cubed is not a hunt; and what the quarry does back,
for anything with the bulk to turn round.

**And the food web was three hand-written names a species.** A country stocked
with thirty-four geese and nine rabbits fed eighteen predators on the nine,
because no list said "goose". A hunter takes what it can bring down now -
anything up to the size of the largest thing it is named as taking - and the
names say how big that is rather than exhausting the menu. It is also the only
way "many bird species hunt for fish as well as rodents" can be true without
writing out every pairing.

### 140. The mice were right and unaffordable, so the small life is assumed

The bottom of the chain was missing (#137) and it was put in: mice, voles,
songbirds and frogs as records, with stoats, kestrels, kingfishers, adders and
herons living on them. The measurement is worth keeping because it settles the
question for good.

Modelled one for one, rodents are **food-limited, not predator-limited**. The
grass on four square kilometres carries sixteen thousand of them - about four
thousand to the square kilometre, which is less than a real vole year - and
they sit there in permanent boom and starve, sixty-five thousand starvation
deaths in the first year. A hundred square kilometres would want four hundred
thousand records against a tick budget that is the constraint this whole line
of work is written under.

So the small life is assumed, the same standing decision the specification
already makes about decomposers and pollinators. A piece of ground has a
small-game yield - `what_the_small_life_gives` - and three things fall out of
it worth having:

- A stoat, a kestrel or an adder can live somewhere without a herd of anything
  being on the map, which is what a small predator does.
- It is worth having only to something small. A wolf cannot live on voles and
  now does not.
- It is **shared**. What a piece of ground grows is what it grows however many
  are working it, so two hunters on one hunting ground each get half of it and
  the second starves off it. That is what a territory is in a model that
  cannot draw a line on a map, and a hunting ground is sixty-four hectares
  rather than the eighty-metre block a hunt is resolved in - sharing the small
  game out over hunt blocks said a hunting ground held two stoats where it
  should hold three over a hundred times the area, and four square kilometres
  came out with seven hundred and twenty-one of them.

A carnivore with an empty prey list is the honest way to say "lives on what
this world does not count", and it is what now puts a species among the small
predators. That also fixes `fish`, which #137 filed as misclassified.

### 141. The country still does not settle, and it is not for want of looking

**Mostly fixed - see #153, which found what it actually was.** The top and
middle tiers now hold; the smallest specialists still do not. What follows is
the original filing, kept because the reasoning in it is where the answer came
from.

**Was filed open.** With everything in #139 and #140 done, four square kilometres
left alone still does not come to rest. What it does instead, over four years
from 196 head: 426, 498, 333, 225 - and by year four the country is a hundred
and ninety-six geese, fifteen goats, and one or two predators of any kind.

Two things are visible in that and neither is fixed:

- **Predation is still not a brake.** Eleven animals taken in four years. The
  hunt model in #139 is a better *description* of a hunt, and it did not make
  hunting frequent enough to hold anything down. A predator hunts on one tick
  in twenty and a rush that comes off takes one bite out of the quarry; the
  arithmetic never reaches the scale of what a herd breeds.
- **Nothing disperses.** Herbivores move only when there is nothing within
  reach underfoot, so they eat a patch out and overshoot on it while ground a
  few hundred metres away is untouched. Predator dispersal was added here and
  is crude: hunters that cannot make a living on their ground step towards a
  less crowded one, a few of them a tick. The first cut of it moved every
  hunter on a ground the same way, so they travelled as a clump, piled into a
  corner, and made predation quadratic - a four-year run went from three
  minutes to over ten. Scattering the step fixed the cost and not much else.

- **Breeding is gated on a stock, not a flow, so a slow animal never stops.**
  `can_breed` asks for `hunger < max_hunger * 0.4`, which is how full the
  animal is rather than whether it is finding anything. The snake has
  `max_hunger: 200` against a `hunger_rate` of 0.02, so it is four thousand
  ticks of eating *nothing* away from being unable to breed - and it lays up
  to twenty eggs with no gestation. Under the old one-pair-per-species rule
  that never showed; with recruitment proportional to the population (#139) a
  quarter of a square kilometre came out with nine hundred and seventy-one
  snakes on it. What ought to gate breeding is whether an animal is actually
  finding food, which is a rate, and nothing in the model measures one.

Two things were fixed in passing because they were making the measurements
impossible rather than merely wrong:

- **Nothing has ever killed a young animal.** Everything born or hatched was a
  full record from its first tick, subject only to starvation, old age and
  being eaten, so a clutch of twenty eggs was twenty snakes. A thing lays
  twenty eggs *because* almost none of them make it, and
  `how_many_of_a_litter_come_through` is a coarse stand-in for the nest
  predation, cold snaps, disease and failure to thrive the model does not
  have. It is applied at birth rather than played out, because playing it out
  means holding records for animals whose whole purpose is to die.
- **A hunt looked over every animal standing in the nine blocks around it**,
  which on ground that had filled up is every predator against every animal
  again - the thing blocking the map was supposed to prevent. A quarter of a
  square kilometre that ran away to five hundred and sixty head took a
  five-year run from three seconds to over two hundred, and hung the test
  suite. A hunter looks over eight animals a block now, which is also the
  truer statement: it goes for what is in front of it, not for the best of a
  full census.

The thinning is applied only to the part of a clutch above four, which leaves
every mammal in the registry as it was and bites on the egg-layers. Flat, it
took the snakes from nine hundred and seventy-one to four hundred and eleven
and took everything else on a quarter of a square kilometre down with them, to
two head in five years; targeted, three of the five guards this work broke
come back - `a_world_with_nobody_in_it_does_not_empty_of_animals`,
`most_of_what_lived_here_still_lives_here` and
`survival_loop_tests::population_feeds_itself_over_a_long_run`. The snake
still runs away on some seeds and that is the fasting-tolerance point above,
not the litter.

**Three guards are still red and this is filed as unfinished work**, against
twenty failures on the commit before it: `the_hedgerows_are_no_thinner_a_few_years_on`
and `a_herd_settles_at_what_the_ground_will_feed`, both real consequences -
there is more grazing on the map than there was, and a herd no longer settles
where the grass alone would put it - and
`clothing_tests::a_cold_agent_ends_up_dressed`, which is the family #132
names, an unseeded settlement test that has flipped both ways twice in this
work alone. The suite also takes 504 seconds against 223, and that is the
snake: the ecology tests spend their time ticking a world with four hundred
of them on it.

The honest summary is that the *mechanisms* are now right and the *rates* are
not, and that six rounds of tuning did not converge. What it wants next, in
order: a hunt that happens often enough to be a brake at all; dispersal for
everything rather than for hunters only; and a breeding gate that reads
whether an animal is finding food rather than how full it happens to be.

---

### 142. What an animal eats was five buckets, and a cow and a mammoth were in the same one

`what_it_reaches_for` read what an animal takes off the ground out of
`AnimalSize`, which has five steps in it. So a cow at six hundred kilograms
and a mammoth at six thousand differed by a factor of two, a rabbit and a
goose were identical, and the whole span from a stoat to a mammoth - four
orders of magnitude of animal - came out as thirty-five to one.

Species carry a `mass_kg` now and what they eat follows it, at mass to the
three quarters. Not mass itself: what an animal burns rises more slowly than
its bulk, which is why a cow eats ten times a sheep and not ten times a
sheep per kilogram. Anchored on a sixty-kilo sheep at the number the plant
balance was already measured against, so nothing that was tuned moves.

The same five buckets were also standing in for how hard an animal is to
bring down, and that is off the same field now, at the root of mass - reach
and footing and how hard a thing can hit back go up a great deal more slowly
than weight does, and a cow is three hundred rabbits by mass and nowhere near
three hundred times the job.

**It settled two of the three guards that #141 left red.** A hundred square
kilometres was already stocked the same and costs slightly less to run
(11.14 ms a tick against 12.25), so this is not a trade. What moved is which
animals a country ends up made of: four square kilometres at year eight used
to come out as a hundred and ninety-six geese and almost nothing else, and now
comes out with thirty-one geese, a hundred and thirty-three squirrels,
thirty-three rabbits, four cattle, eight goats and a pair of owls. Geese ran
away with the map because a goose record ate what a rabbit record ate and
weighed eight times as much; against mass they no longer do.

`ecology_tests::a_herd_settles_at_what_the_ground_will_feed` and
`ecology_tests::the_hedgerows_are_no_thinner_a_few_years_on` are green again.
The third, `drive_emotion_feedback_tests::test_high_hunger_causes_fear`, is
nothing to do with the ecology and is its own piece of work.

What this does not touch is the thing #141 names: nothing but a hunter
disperses, so a country still eats a patch bare and overshoots on it while
ground three hundred metres off is untouched.

---

### 143. Running away and turning round were the same want

`Action::Attack`, `Action::Fight`, `Action::FleeFrom`, `Action::Freeze` and
`Action::SeekShelter` all answered `DriveType::Safety`. So an agent that ran
and an agent that stood had satisfied the same drive, and nothing downstream
could tell the two apart - not the learning, not the drive that was supposed
to be pressing, not the record of what happened. No appraisal, however good,
could change which of them an agent did, because the thing choosing did not
have two options to choose between.

There is now a second drive. `Safety` is the fear drive - which is what it
always was; a drive is a need and fear is the feeling that names it - and
`Aggression` is the anger drive. **Both read one appraisal**, and that is the
whole design: `Surroundings::what_is_on_me` says how much is on the agent and
`could_face_it` says whether it can be met, and the same reading becomes fear
when it cannot and anger when it can. Nothing converts one into the other,
because there is nothing to convert: a change in what the agent makes of the
situation moves the demand across in the tick it changes. `Aggression` has an
accumulation rate of nought for the same reason - it keeps no reservoir, so
when the thing goes the demand goes with it.

**Fear had no answer of its own.** `what_this_drive_offers(Safety)` returned
`SeekShelter` when a roof was within reach and `None` otherwise, so a
frightened agent in open country could have fear as its strongest drive, win
the tick with it, and produce no behaviour at all. The specification says
drives result in actions and this was the drive where it failed. Fear runs
first now, makes for a roof second, and moves off the ground it is standing on
failing both; anger goes at the thing, through the fight-or-flee tree that was
already there rather than a second copy of it.

**A parent stands while there is still time to buy.** The specification is
explicit and it is not the obvious way round: if the young can still get clear
then standing over them buys the time to do it, and that is anger; if the
thing is already on top of them, standing buys nothing and what is left is
fear. This replaces a deliberate older decision - the tree used to have a
parent fight *whatever the odds were*, described in its own comment as "the
one place in the model where an agent knowingly takes the worse of two
options". That is now conditional on there being something to be gained.

Three things had to be fixed on the way, and the second is the interesting
one:

- **The appraisal saturated.** `what_i_stand_to_lose` was folded into the
  drive reading as well as the feeling, which pushed it to its ceiling
  whenever anything at all was about; a settlement of eight starved inside
  four thousand ticks because fear outranked hunger every tick of every day.
  What a man stands to lose belongs to how much he minds, not to how much is
  there.
- **An agent fled from a rabbit.** `predator_near` is true of anything with an
  `attack_damage` above nought, which is a rabbit, and now that fear always
  offers *something* the flag spent the turn instead of falling through to
  eating. Running is worth a turn only against something that cannot be faced.
- Both the threat reading and what to run from now weigh distance. A wolf at
  the edge of sight is not a wolf at your elbow.

**What it costs.** Nothing measurable, and establishing that took more work
than the change did. Eight people, four thousand ticks, thirty-two worlds a
block: this branch gave 25, 44 and 35 survivors on three different seed
blocks, against 35 for the same measurement before it. The spread between
blocks is several times any difference the change could be making, and
seed-for-seed comparison is invalid anyway - a sixteenth drive means
`Agent::new` draws one more random number, so the same seed is no longer the
same world. Four tests moved for exactly that reason and are seeded now; five
more were counting drives and now ask the enum.

---

### 144. A deer with a wolf standing over it went on grazing

`update_animal_behavior_with_hunger` was a set of dice keyed on
`AnimalBehavior` and on nothing else. It could not see what was standing next
to the animal at all, so a deer with a wolf three paces off did exactly what a
deer alone in a meadow did: rolled for whether to graze, drink or stand about.
`AnimalState::Fleeing` and `AnimalState::Attacking` were only ever set by
`shy_away_from`, which is about people - so between beasts, nothing in this
world had ever run from anything.

Animals now carry the same two numbers an agent carries in
`core::Surroundings` - `what_is_on_me` and `could_face_it` - off the same
`ThreatAssessment`, so there is one model of fear and anger in this project
rather than one for people and another for beasts. What counts as a threat is
anything that eats this one: a thing whose prey list names its kind, or which
takes prey of its size. What counts as being able to face it is what it brings
against what the thing brings, **with every one of its own kind standing near
it counted in** - which is the herd, and it is why eight sheep together turn
on a wolf that one sheep runs from.

It also settles the cattle question from the other end. A wolf never reads a
cow as a threat to begin with, because a wolf's prey tops out at
`AnimalSize::Medium` and a cow is Large: that is this model's way of saying a
lone wolf does not take cattle, and it was already true before any of this.

**What it costs is the reacting, not the looking.** A hundred square
kilometres went from 11.14 ms a tick to 13.69, and almost none of that is the
appraisal: driving the pass from the hunters instead of from every animal -
seven times fewer starting points for the same pairs - moved it by one per
cent, and running it every fourth tick instead of every tick moved it by four.
The rest is that animals which run *move*, every tick, and a country whose
beasts scatter when something comes near them costs more to keep track of than
one whose beasts stand still. That is the feature, and 59 seconds to the
world-year is what it comes to.

Two smaller things went in beside it:

- **Dread and urgency were sharing a horizon.** `A_LONG_WAY_OFF` is half a day
  and is the *urgency* clock - how hard a need should press on what an agent
  does this turn, deliberately tight or everything is an emergency. The fear
  calculation read the same number, so a man fifteen days without food and six
  days from dying of it came out **eight per cent frightened**. Dread looks
  three days ahead now, which is what "I do not have enough food raises fear"
  has to mean if it is to mean anything.
- **Being robbed was anger every time, whoever took it.** A man robbed by
  somebody twice his size came away resolved to do something about it. It is
  the same appraisal the wolves get, pointed at a person: what was taken
  decides how much, and who took it decides whether it is anger or fear.

---

### 145. Six hundred kilogrammes of Passive

The specification asks for a per-species behavioural baseline, and one already
existed: `AnimalBehavior::how_readily_it_stands_its_ground` - Passive 0.0,
Neutral 0.6, Defensive 0.9, Aggressive 1.2, Territorial 1.3 - which is what
`beasts.rs` reads to decide how an animal takes a *person*. It was not read
when animals appraised each other, so between beasts every species had the
same nerve, and a rabbit that happened to out-weigh what was in front of it
would turn round and fight it.

It is one number in one place now: `what_each_animal_is_facing` multiplies
what an animal brings to a stand-off by its temperament, the same way
`beasts.rs` does. It matters most at the bottom of the scale, because Passive
is nought: a rabbit never stands its ground however the arithmetic comes out,
which is the point - a rabbit that fights a wolf is not a rabbit.

Making it load-bearing exposed the data underneath it. **Cattle were
`Passive`.** So were sheep, deer, reindeer and pigs, which is right; a cow is
not. With temperament multiplying into the stand-off, a Passive cow is a cow
that stands and takes whatever arrives, which is the exact opposite of what
"cattle and other large herbivores should be capable of defending themselves"
asks for. Cow is `Defensive` now, which puts it with the rest of the large
herbivores - camel, elk, goat and goose are Defensive, bear and mammoth
Territorial - and cattle were the only outlier in the whole list of
thirty-eight.

The agent side of the same requirement - "animals could receive the same
baseline whereas agents could receive their baseline with minor deviations" -
was already there and needed nothing: `DriveState::with_random_weights` gives
every agent a per-drive multiplier drawn around 1.0, and the Brave and Anxious
traits move `own_strength` in the threat appraisal, so two agents meeting the
same wolf do not necessarily reach the same answer. Animals take the species
baseline flat, with no per-individual draw, which is the asymmetry that was
asked for.

---

### 146. Fear about a need had nowhere to go

The specification asks for fear to feed the other drives, and gives the case:
"'I do not have enough food' equals an increased fear drive". Half of it was
in - `calculate_survival_drive_emotion` weighs a need that has gone unanswered
against how fast it would kill, and that is the model's one reading of dread -
but it only ever produced a *feeling*. The fear **drive** could not see it.
`DriveType::Safety` read the threat appraisal and nothing else, so a man six
days from starving in an empty field had a fear drive of nought.

Two things had to be settled to wire it up, and both of them the wrong way
round would be worse than leaving it alone.

**How much it is worth.** `WHAT_DREAD_IS_WORTH` is 0.4 against a Safety
threshold of 0.5, so dread at its absolute worst cannot carry the drive on its
own: it takes a bare larder *and* something else - the dark, a wound, a thing
in the field. That is deliberate and it is the lesson of the cut before this
one, which folded the whole of what an agent stood to lose into the drive:
fear beat hunger every tick and a settlement of eight starved inside four
thousand ticks with full bushes round it. Fear about a need must press in the
same direction as the need, never in front of it.

**What it comes out as.** A drive that rises has to end in an action, and the
obvious action is wrong: if fear of running short comes out as running or
hiding then a hungry settlement spends its days getting behind trees. So
`what_i_dread` now returns *which* need it is about as well as how much, and
when there is nothing in the field to run from, the Safety branch offers
whatever that need offers - the food action when the dread is hunger, the
water action when it is thirst. Delegated, not duplicated: there is still one
place in this project that knows how to look for food. `Safety` has no death
clock of its own, so the fear drive can never end up pointed at its own tail.

**It changes nothing, and that is the result.** Sixteen worlds on one seed
block, four thousand three hundred ticks: 2426 person-days against 2437, the
same twelve worlds emptied, the same 8.81 alive at midsummer. A term that
pushes the same way the need already pushes should not move the outcome, and
it does not. What it buys is that the fear is now *in the drive* rather than
only in the feeling, which is where the rest of the model can read it.

---

### 147. A hunting ground was a queue, not a living

Predators turning on each other was already in - a hunter takes another
hunter it outranks when the ground is `crowded` or it is nearly starving - but
`crowded` was a flat count: more than three hunters on a block of country,
whatever was on that country. So the pressure came from how many hunters
happened to be standing about and never from the game running out. Winter
could not cause it. A good year could not relieve it. A hard year and an easy
one looked identical, which is the opposite of what the specification asks
for: "as prey species decrease in number, this should cause predators to
attack each other for food".

It is game against hunters now, through one function asked from both ends.
`AnimalManager::how_good_a_living(game, hunters)` is what a piece of country
is worth to one more hunter, and `is_the_ground_crowded` is that against
`WHAT_A_HUNTER_WANTS_UNDER_IT` - ten head to a hunter on sixty-four hectares.
Ten wolves over plenty of deer are neighbours; two on ground that has been
eaten out are rivals, which a crowd rule calls quiet.

The same number fixes where a hunter goes when it gives up on its ground.
That used to pick the neighbouring ground with the fewest rivals on it, and
the ground with no rivals is very often the ground with nothing on it at all -
a hunter walking off a crowded field to an empty moor has swapped competition
for famine. It now picks by the living, counting itself in over there, which
is the same question about a different piece of country and is why it is one
function rather than two.

**What could not be measured, and why.** Two worlds over five years, same
seed, before and after: 27 head against 24.5 across the eight species that
survived at all. That is not a result. The whole map carries a couple of dozen
animals on a hundred square kilometres and single species swing further than
that between years on their own - rabbits went 3.5, 60.5, 21, 2, 4 across the
run - so nothing at this stocking can be told from noise. The rule is right
and it is tested directly; whether it *tells* on the ecology cannot be known
until #141 is fixed, because a model where predators are down to one stoat has
no predator-on-predator pressure to model.

---

### 148. A burrow was proof against wolves and not against February

"The burrows would offer shelter from weather, predators, and places to
hibernate in the winter." Only the middle one was in. `what_a_hunt_comes_to`
has had a hole as the whole of a rabbit's answer to a wolf since the refuges
went in. Winter did not know burrows existed: every animal on the map paid the
same upkeep in February as in June, so the season could only reach an animal
through the forage going off the ground, and it reached every animal the same
way.

`what_a_winter_costs(species, ground, season)` is the winter half, and it has
two rates because there are two different things here. A bear is **asleep** -
`WHAT_A_WINTER_ASLEEP_COSTS`, three tenths. A rabbit in a bank is **out of the
wind** - `WHAT_A_WINTER_UNDER_COVER_COSTS`, 0.85 - because it is awake, it is
still eating, and what it saves is the part of its burn that goes on keeping
warm. Everything else pays in full, which is the point: a rabbit and a deer on
the same bare ground are now two different animals in December.

Both conditions are about the world rather than the species, so they are read
fresh each tick rather than kept as a flag. A rabbit on bare rock has no more
hole than a deer does, and nobody hibernates in July.

**The first cut of this was badly wrong and the measurement caught it.**
Giving every burrower the sleeping rate - three tenths of its burn for a
quarter of the year, while it went on feeding normally - is not a burrow, it
is a rabbit the winter cannot reach. A hundred and twenty by a hundred and
twenty went from **682 head at the end of its first year to 2,533**, and the
following year cost **10.94 ms a tick against 1.9** as the country tried to
carry them; the ecology test that walks that world five years stopped
finishing. At 0.85 the same run gives 858 head against 682, collapses on the
same curve afterwards, and costs 1.91 ms against 1.89. That is a shelter.

**The weather third of the specification has nothing to answer yet.** Nothing
in this model kills an animal with cold directly - winter bites through the
forage - which is why lying up is expressed as burning less rather than as
taking less damage. If cold is ever given teeth of its own, this is where the
burrow answers it, and the shape is already right.

The cost is one read-only pass over the animals, in winter only: it wants the
registry and the ground under each beast, and the upkeep loop holds both
mutably. Three seasons in four it is a season comparison and an empty vector.

---

### 149. Twenty-six thousand rabbits, and three wolves to eat them

Two years on a hundred square kilometres, nobody in the world:

```
  28,718 head        of which     26,276 rabbits
                                   1,515 geese
                                     315 goats
                                       3 wolves
                                       3 arctic foxes
  under 10 kg: 27,804 of 28,718  (97%)
```

The tick over those two years went **17.57 ms in the first quarter to 108.90
in the eighth**, and 47.12 ms averaged over the run. Ninety-one per cent of
what the model was spending its time on was rabbits.

And on a small map the same species does the opposite. A hundred and forty-
four hectares over five years runs 165, 858, 67, 27, 19, 7 head; on the big
map rabbits went 3.5, 60.5, 21, 2, 4 between years before the herds took hold.
That is the signature of the representation, not of any parameter in it: a
fast-breeding animal held as discrete records is a random walk with an
absorbing barrier at nought, so it either finds the barrier or it finds the
array.

**The other half of the same mistake was already here.** The small life a
stoat lives on - mice, voles, the things assumed rather than counted - was
`what_the_small_life_gives`: `cover x size-fit / hunters-sharing-it`, a
constant. It could not be drawn down, could not boom, could not crash, and
nothing an agent did could touch it. So the model held records where a record
is least reliable and an abstraction where the abstraction could not respond
to anything.

`SmallLife` is the abstraction with a stock behind it. Each hunting ground -
eighty cells square, which at ten metres a cell is sixty-four hectares -
carries a head of **grazers** (rabbits, voles, squirrels) and a head of
**hunters** (foxes, stoats, weasels), and what it carries comes from the
cover, the climate and the season. On a hundred square kilometres that is a
hundred and sixty-nine grounds: a hundred and sixty-nine float updates and one
terrain sample each, against a tick that already walks every animal and a
share of a quarter of a million plants.

Measured over the same two years, same seed: **45.42 ms a tick against 47.12**.
Free, within noise, and the country now runs a seasonal cycle instead of a
straight line - 11,700 head of grazers in autumn, 5,400 in February, back to
11,700 by the following autumn, with the hunters trailing a season behind at
82, 66, 76. That is what "in general the population is balanced between
predator and prey" looks like when it is a number rather than twenty-six
thousand records.

Two things it is deliberately not:

- **The hunters do not oscillate.** They track a share of the grazers with a
  lag rather than making a proper predator-prey pair with them. A swinging
  model empties a ground of foxes every few years by arithmetic rather than by
  anything that happened, which is the thing taking the small life out of
  records was meant to stop.
- **A ground trapped out is not a dead ground.** A logistic curve through
  nought never leaves it - the rabbit-as-record failure in another form - so
  there is a floor of a head or two, which is what "there are always a few
  about" comes to when the ground next door is not modelled.

`what_the_small_life_gives` now returns what the ground would give at full
stock, multiplied by how thick it actually is, and takes the head off. Until
there was a number behind it there was no such thing as a trapped-out wood.

Still to come, and the reason this went in first: the records for the
under-ten-kilogramme species are still being spawned alongside the stock that
now stands for them. Taking them out is where the 97% goes, and it cannot be
done before agents have a way of getting at the abstracted layer - which is
trapping, and does not exist yet.

---

### 150. A trapline, and three ways of making one that ruins a settlement

The lower tiers are a population now (#149), which leaves a hole: **you
cannot stalk a number.** `Action::Hunt` walks up to a particular animal
record, so abstracting the small species away would take the small meat with
it. What people actually did was set a line and go round it, so that is what
went in - `Action::SetSnare` and `Action::CheckSnares`, `Undertaking::
Trapping`, and `Snare` on the world.

What a snare does is read off the ground it is set on, which is the point of
the exercise:

- **How often it fills** is straight proportion to how thick the small life
  is there - "the rate of success and speed of catch could be based on the
  total population".
- **How long a catch waits** is hunters against grazers. That is not a rule
  written down; it falls out of the hunters tracking the grazers *behind*
  them. Trap a wood out and its foxes are still on it with nothing else to
  eat, so the ratio spikes and a catch is gone in a turn or two. In a settled
  country it comes out at the quiet rate by construction, because at full
  stock the ratio is exactly `WHAT_SHARE_ARE_HUNTERS`. That is the
  specification's "a decrease in rabbit population could decrease the time an
  agent has to recover a trapped rabbit before a fox steals the catch", and
  nothing had to be written twice to get it.

**Four numbers were wrong first, and each one was found by measuring rather
than by reading the code.**

1. **A snare's rate was a snare's rate.** Twelve agents at a dozen snares
   each put a hundred and forty-four on one sixty-four-hectare block, which
   at 0.02 a tick apiece is **thirty head a day off a ground whose whole
   surplus is two**. The camp's ground went to eight thousandths of what it
   carries inside three months, every catch was robbed before anyone reached
   it - a ground with no game on it is a ground of hungry foxes - and a
   settlement of twelve took **one** rabbit in a year. The ground gives what
   it gives however much string is on it, the same rule
   `what_the_small_life_gives` already applies to hunters sharing a range. A
   longer line reaches the ground's yield sooner and never exceeds it.
2. **A catch put ahead of the food at an agent's feet.** It is the only food
   in this world that walks away if you leave it, so it looked like it
   belonged first. It does not: a hungry man then crosses the country for one
   rabbit instead of eating the berry in front of him. Six worlds over a year
   went from **23,733 person-days to 14,920**, thirty alive at the end
   against eleven. Only the free case belongs in front - a snare the agent is
   standing on. The walk sits behind the ground in front of him, and is
   bounded at a hundred and fifty metres.
3. **Setting string ahead of storing food.** A man with a surplus in his pack
   set snares instead of putting the surplus by, and what he does not put by
   he has not got in February: **20,126 person-days, deaths in the winter
   quarter**. Setting a snare is the last thing in the Preparedness chain
   now, which is the honest place for it - a trapline is what you do when
   there is nothing better to do with the turn.
4. **A round was one snare.** `CLOSE_ENOUGH_TO_A_SNARE` was one cell, so a
   man who stopped at a snare ignored the eleven he set beside it, and a
   settlement recovered one catch in six. Forty metres, and it recovers half
   of them in a healthy country and a third across a year that ends badly -
   the difference being the winter, which is the mechanic working.

**Where it ended up: 23,600 person-days against 23,733 at HEAD, and thirty
alive against thirty.** Cost-neutral, which is the right result for a
supplement rather than a staple: one hunting ground's whole surplus is about
two head a day against twelve people's twenty-odd, so a trapline is a tenth
of a living. It is not meant to be more than that, and a version of it that
was would have been a number chosen to flatter the feature.

`WhatTheSnaresDid` counts caught, robbed and taken, and a test asserts they
add up. The first cut of that tally silently counted nothing - a string
replace that matched no text - and the missing 185 catches looked for a while
like a bug in the model rather than in the instrument.

---

### 151. Twenty-six thousand rabbits become a number, and the tick falls fourfold

The stock existed (#149) and the way in for agents existed (#150), so the
records the stock stands for could go. `AnimalSpecies::
is_stood_for_by_the_small_life` names them - rabbit, squirrel, goose, duck,
chicken, fox, arctic fox, stoat, snake, adder - and world-generation and the
migration that refills a depleted country stop dealing them out.

**They stay in the registry.** A rabbit still has a mass, a temperament, a
diet and a place in the food web, and `spawn_animal` will still put one on
the map if something explicitly asks. What stops is the world stocking them,
because there is already a population of them: counting the same animal twice,
once as a number on a hunting ground and once as a thing standing in a field,
would be worse than either.

Two years on a hundred square kilometres, nobody in the world, same seed:

```
                        HEAD          now
  head                29,773        2,347
  ms/tick, mean        47.12        11.77      4.0x
  ms/tick, last qr    108.90        14.57      7.5x
```

And the shape changed as much as the size. At HEAD the head count ran away -
1,614, 2,175, 4,551, 10,850, 15,243, 20,363, 23,850, 29,773 by quarters, with
the tick following it. It now goes 1,169, 1,303, 1,312, 1,485, 1,657, 1,737,
1,898, 2,091: a country filling up rather than a country exploding.

**It costs the agents nothing.** Six worlds over a year: **24,022 person-days
against 23,733 at HEAD**, twenty-eight alive at the end against thirty. The
rabbits agents used to hunt are the rabbits they now trap, and the trapline
(#150) carries it.

Two things had to be fixed alongside it, and one of them mattered a great
deal:

- **The spawn gate asks whether a predator's prey is present.** That gate
  exists for a good reason - drawn independently it put foxes into worlds of
  cattle, where they never found a meal in eight thousand ticks - but it reads
  the *records* on the map. The moment rabbits stopped being records it said
  "your dinner is not here" of a country thick with rabbits, and a hundred
  square kilometres came out with **no hawk, no owl, no eagle and no boar on
  it at all**. The small life is prey: a species whose named prey is one of
  the abstracted ones is fed by the abstracted layer, and boar came back at 95
  head.
- **A country was empty of rabbits on the morning it was made.** The stock was
  only settled by the first tick, so anything that asked what lived on a piece
  of ground before then was told nothing did. `stock_the_small_life` runs at
  generation, through the same pass, so there is one answer to what belongs on
  a ground rather than two.

**What is still missing is missing from before this.** Hawk, owl, crow,
kestrel, heron, kingfisher, parrot, otter, monkey and the fish record do not
appear on a generated map, and did not at HEAD either - the two-year census
before any of this had one eagle and nothing else of that guild. That is
#137, not this.

**And the small life spreads.** A worked ground used to come back only off its
own floor - "there are always a few about" - which is animals from nowhere and
says nothing about what surrounds it. `let_them_spread` moves grazers between
neighbouring hunting grounds down the gradient of *crowding* rather than of
head count, so a rich block does not drain into a barren one and nothing
crosses onto a salt flat. Each unordered pair is visited once and the flow is
subtracted from one side and added to the other, so head is conserved exactly
- an exchange written as "move towards the average of my neighbours" is not
symmetric and quietly invents animals every tick.

Two tests changed their premise rather than their expectation, which is the
honest thing when a fact moves rather than breaks: `a_country_holds_more_small
_things_than_large_ones` counts the population instead of the records, and
`predators_can_live_off_what_the_world_holds` counts the abstracted layer as
food. Two more were the #132 family and were seeded or widened to a block:
`population_feeds_itself_over_a_long_run` had kept seed 4,200's people through
months of changes and lost them here, in a run where six worlds measured
together went *up*.

---

### 152. The small-predator guild was never placed, three times over

#137 said the tier was empty because the registry had no species for it. That
part was fixed - the stoat, the kestrel, the kingfisher, the adder and the
heron went in - and the tier stayed empty anyway. A hundred square kilometres,
two years, no people: **no hawk, no owl, no kestrel, no heron, no otter, no
crow, no parrot, no monkey, no pig and no fish, ever, at any point in the run
or at any map size.** Eighteen hundredths of a country's groups are asked for
and nothing filled them.

Three separate causes, and all three are one shape: two places asking the same
question and disagreeing.

**Which pool a species is drawn from.** It was `diet` plus the length of the
prey list; `where_it_sits` is the model's actual answer to what a species is,
and the two disagree about six species. An omnivore with an empty prey list -
the crow, the parrot, the monkey, the pig - is a primary consumer by
`where_it_sits` and is *not* `DietType::Herbivore`, so it fell between the two
pools and could never be placed by anything. A carnivore with an empty list -
the kestrel, the adder, the fish - is deliberately a **small predator**,
because `where_it_sits`'s own comment says "a carnivore with nothing on its
list still hunts: it hunts the small life the map assumes" - and
`!prey_species.is_empty()` threw all three away. Both pools come off
`where_it_sits` now.

**Whether it has anything to eat.** The gate that stops a fox being put into a
world of cattle read an empty prey list as "no food here", which is the same
disagreement one step later. And it was applied to every tier at once, before
anything was placed: a heron takes fish, fish are placed among the small
predators, and the heron was judged before a single one was down. The pyramid
is built from the bottom now, a tier at a time, so what is put down low counts
as food for what goes above it.

**Where it was put.** Nothing had ever been placed in water - every water tile
was skipped as "water, for land animals" and there was no second list - so a
species that cannot leave the water had nowhere to be put and was never
placed, and everything living on fish had no prey either. And a hunter was put
down anywhere in a climate it belonged to, most of which is open: what the
small life gives is straight proportion to cover, so a hawk on plain got 0.037
against a burn of 0.070 and was dead whatever else was right.

**And the ladder was stale.** `what_the_small_life_gives` pays by the hunter's
size, and that ladder was calibrated when the small life meant mice. It has
not meant only mice since #149: rabbits, squirrels, geese, ducks and crows are
in it, and a rabbit is a meal for a hawk rather than a scrap. At the old
numbers a hawk in the best wood in the country, with the wood to itself, got
**0.073 against a burn of 0.070** - four per cent - and a third of what it
needed if two others worked the same wood. Small goes 0.35 to 0.70 and Medium
0.12 to 0.25. Every small predator can now keep itself in a wood it has to
itself, and none of them can on open plain or three to a wood, which is a fair
statement of where a bird of prey lives.

**What a country holds now**, on a hundred square kilometres at generation:
eagle 15, hawk 10, heron 24, kestrel 59, otter 11, owl 3, fish 121, seal 4,
wolf 14 - against nought of every one of them before.

**Two things this turned up that are not #137.**

- **The crow is a rabbit in feathers.** Half a kilogramme, a primary consumer,
  and it breeds like one. The moment the pools were fixed and it could be
  placed at all, a hundred square kilometres went to **58,682 crows out of
  61,558 head inside a year**, with the tick at 249 ms - worse than the
  rabbits ever were. It is on the abstracted list now, where every other
  criterion already said it belonged, and was only ever off it because it was
  absent from the map.
- **The pig is the farmyard form of the boar**, which its own description
  says, and the boar is stocked. It also carries a sow's `litter_size` of six
  to twelve against a wild ruminant's one or two, which is right for a pig and
  ruinous in a country with no farmer in it: **3,749 pigs out of 5,607 head**,
  and the tick at 18.5 ms against 11.8 without them. A country made before
  anybody arrives is stocked with wild animals;
  `is_the_farm_form_of_something_wild` says so. `spawn_animal` will still put
  a pig down, and `can_domesticate` is untouched. The cow is deliberately not
  on that list - it has no wild form here to be counted twice against, it
  carries a litter of one, and it sits at seventy-odd head over two years
  without help.

**Where it lands: 11.29 ms a tick and 2,133 head**, against 11.77 and 2,347
before the guild existed and 47.12 and 29,773 before any of this. The guild is
free.

**What it costs the agents, which is not nothing.** Six settlements over a
year: 22,470 person-days against 24,022 before the guild existed, and
twenty-three alive at the end against twenty-eight. The hawks and the owls eat
the small life the people trap, which is what a predator guild *is*, and the
first cut of it cost twice that by mistake - see below. It is above the spread
between seed blocks (about three per cent) and is a real cost, honestly come by.

**One number was wrong in a way worth naming**, because it is a trap the same
shape as everything else here. Raising `what_a_head_of_it_is_worth_to` raised
the energy `what_the_small_life_gives` pays out, and the take that comes off
the ground was `got * HEAD_A_UNIT_OF_FORAGE_COMES_TO` - so doubling the ladder
also doubled the *head* every hunter drew. That says a hawk eats twice as many
rabbits, when what the ladder means is that it gets twice as much out of each
one. A country of hawks then stripped the ground the people trap on: 21,527
person-days, and two settlements of six with anybody left in them at four
thousand ticks against three. The take is divided by the same rung that paid
it now, so the ladder moves what a head is worth and never how many are taken.

**And one test is left failing on purpose.**
`a_cold_agent_ends_up_dressed` has been in the standing failures for months.
It gives a lone man, kept permanently freezing, fifty days to make a garment
out of the flax in his pack. He now comes out of that run carrying fish, meat
and roots as well as the flax and still no coat: a world with a trapline and a
fishery in it gives a man more errands, and clothing never wins the turn.
Widening the window does not help - he does not live two hundred days - so
what the test actually measures is how crowded an agent's day is, which is
worth knowing and is not a clothing bug.

**What is still not fixed is #141.** Over two years the small predators thin
from those numbers to eagle 1, hawk 2, owl 1, otter 1 - and so do the wolves
from 14 to nought, the lions from 10 to 4, the bears from 4 to 1. Every
predator tier thins, uniformly, which is what makes it one problem and not
this one: a species that can only exist at low density cannot recruit, because
two of them never meet. That is the country not settling, and it is filed
where it belongs.

---

### 153. Not one animal was ever taken by a predator

#141 said predation was not a brake and guessed at why. It was six things,
and the first of them settles it: over two years on a hundred square
kilometres, with fourteen wolves, ten lions, four bears and better than a
thousand sheep on the map, **the tally of animals taken by a predator was
nought.** Every hunter in the country starved while its dinner grazed past it
- wolf born 23 and starved 32, eagle 13 and 25, heron 26 and 47 - and the
whole predator tier aged out.

Made findable by making `carried_off` a per-species tally rather than three
totals, and by adding `WhatTheHuntingCameTo`, which counts the four places a
hunt can die. Inferring which from a head count is what got #141 wrong: a
country whose predators all starve looks the same whether they never met
their dinner, never rushed it, or rushed it and missed.

**A hunt that came off took a bite.** `what_a_hunt_comes_to` weighs the cover,
the refuge, how many of the quarry's kind stand with it, how many of the
hunter's hunt together, and the force ratio, and answers whether the rush
succeeded. Then `attack_damage` was applied to the quarry as though the answer
had been "they had a scuffle": a wolf's blow is some fifteen of a sheep's
eighty and the sheep heals a tenth a tick, so a wolf had to catch **the same
sheep six times** to eat once. Two answers to one question. The odds are the
answer, and a hunt that comes off takes the animal.

**A flock of sheep defended itself like a herd of cattle.** The herd term
counted heads without asking whether that sort stands its ground - the same
defect `what_each_animal_is_facing` had, in a second place. Herbivores are
dealt out in fours to twelves and stay in blocks, so eight of their own beside
them is the ordinary case, and eight sheep took a lone wolf from **0.3456 to
0.0028**, one rush in three hundred and fifty, tried one tick in twenty. Sheep
are `Passive`, which is nought, and what they do when a wolf comes is scatter.
Cattle are `Defensive` and a mammoth `Territorial`, and those are what "a lone
wolf should not be capable of killing a herd of cattle" was about.

**Nothing ever walked a hungry hunter towards prey it could see.** The hunt
asked "is there something within eighty metres of me, right now", and if there
was not, the tick was over: 176,125 hunts went looking, 4,379 had something in
the nine blocks around them worth trying for, and **thirteen** were close
enough to rush. A wolf ranges tens of kilometres in a day; this one stood in a
field waiting for a deer to walk into it. It stalks now, two cells a tick
towards the nearest thing it would try for.

**A hunter that could not keep itself where it stood took a step every fifty
ticks.** The fiftieth was set to stop an earlier cut from moving every hunter
on a ground in lockstep into a corner; what actually fixed that was scattering
the step and choosing ground by the living it offers, both of which are still
here. A bird of prey blown onto open plain - where the small life pays it half
what it burns - was dead long before it reached a wood.

**The small life was shared by head count.** A bear standing in a wood halved
what a kestrel got out of it, when a bear turning over a log takes six
hundredths of what a kestrel takes. What shares a layer is the demand on it,
and `what_a_head_of_it_is_worth_to` is already the model's statement of how
much of that layer each sort can use.

**Open water was priced as barren, for a fish.** `cover` on a water tile is a
statement about how much a *land* hunter can find to turn over there. Read for
a fish it prices its own element as a desert: 121 at generation, 3,953 born
over a year and 4,031 starved.

**And nothing kept a herd or a pack together.** Animals are dealt out in
groups and from the first tick every one random-walks on its own account - two
cells a move, four thousand three hundred ticks to the year - so a group is
spread over a hundred and thirty cells inside a year, and a mate is looked for
within ten. Fourteen wolves became fourteen lone wolves that never met again.
`they_keep_together` closes up what the registry already calls a group
(`group_size.1 >= 3`: a wolf is (3, 7), a hawk (1, 2)), and it is gated on
that because packing solitary hunters onto shared ground would divide the
small life between them and starve the lot. What a **solitary** animal does
instead is range for a mate - six hundred metres rather than a hundred - which
is most of what a rut is, and is the only thing standing between a bear at
`group_size: (1, 1)` and never breeding at all.

**Where it lands**, two years on a hundred square kilometres, against the
same run before:

```
                     was          now
  wolf            14 -> 0     14 -> 15   (50 born)
  lion            10 -> 4     10 -> 10
  bear             4 -> 1      4 -> 5    (was 0 born, now 4)
  boar            23 -> 58    23 -> 44   (21 taken - held down at last)
  hawk            10 -> 0     10 -> 7
  hunts rushed         13          825
  hunts that came off   5          126
  taken                 0         real
```

13.16 ms a tick against 11.29, and 21,327 person-days over six settlements
against 22,470 with twenty-nine alive against twenty-three. The tick and the
person-days are what a country with predators in it costs: they eat what the
people would have trapped, and they move about doing it.

**What is still not fixed.** The specialists at the bottom - heron, kestrel,
otter, owl, kingfisher and the fish - still go to nought. They breed
(kestrel born 76) and they starve (112), which is a population sitting at a
ceiling of about nought: their only food is the abstracted small life, and a
sixty-four hectare wood pays two of them. Whether that wants a richer layer, a
smaller appetite, or those species abstracted as well is the next question,
and it is a different one from this.

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
