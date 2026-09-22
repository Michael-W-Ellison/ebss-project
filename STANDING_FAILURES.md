# Standing suite failures

What is red, why, and who is looking at it. One file, in the repository,
because a register that lives anywhere else does not survive the week: this
list was kept in a session task list and was lost with the container that held
it, while `ISSUES_FOUND.md` came through untouched because it is committed.

A test that is red and tracked is a known cost. A test that is red and
untracked makes every future full run read as normal. Keep the count here
honest and close nothing without a measurement.

**Last full run**: `cargo test --lib` - 2,636 tests, 36 minutes, **12 failed**,
2 ignored, of which four were collateral of a first cut at the firewood
keep-back and are fixed. What remains is the five below plus the errand
threshold, which now has eight seeds behind it and is noise (see the table at
the end).

The count has come down from the ten this file opened with: salt water came
off (the fixture, not the model), the production chain, the practised hand and
the carried waterskin came off, and the fire came off after three separate
faults in it were found - ISSUES_FOUND #220 to #224. A defect that was not on
the list at all was found on the way - #219, a body aged thirty times too
slowly, so nobody ever grew up and nobody ever died of old age.

## Open

| test | reports | what is known |
|---|---|---|
| `ecology_tests::most_of_what_lived_here_still_lives_here` | 8 worlds open with 468 head and hold 107 | Species all survive; the head count is ten short of the quarter it wants. Probably downstream of the predator layer - re-run after that is settled rather than treating it as its own finding. |
| `longevity_tests::a_settlement_still_raises_children_late_on` | nobody born into the settlement at 9,000 turns | **#167's question, measured again.** Nobody is ever born at all: 63,456 refusals in 6,000 steps and every one of them "could not feed a child". The gate wants 129,600 units and the best-placed agent holds 8,500 - a factor of fifteen, against the fifty-three #167 measured. Not a test problem and not a gate problem; the store has to fill first (#240, #241, #213). Leave red. |
| `predator_prey_tests::the_land_will_only_carry_so_many` | 0 against 0 | Both fixture herds go extinct. At #217 it read 2 against 6. The predator layer below. |
| `situation_tests::a_settlement_works_things_out_that_nobody_wrote_down` | nobody notices one afternoon goes better than another | Passed at ISSUES_FOUND #177 and is red again. Probably downstream of the predator layer - less happening in the world to notice. |
| `survival_pressure_tests::the_children_of_a_settlement_live_past_infancy` | 0 born here at 6,000 turns | Same as the row above - #167's gate, now fifteen times out of reach rather than fifty-three. Its bound is sound: it counts by parentage, which is the right predicate. Leave red. |

## The predator layer

Three of those above point at one thing, and it has its own finding rather
than a row here. ISSUES_FOUND #218 corrected `what_a_grazer_is_worth_to`, which
converted days of keep into hunger units with `TICKS_PER_DAY` where
`hunger_rate` is charged once a *pass*: one deer fed a wolf for four hundred
and fifty days. The conversion is not in doubt. What it revealed is that the
predator layer was living on that thirtyfold subsidy and cannot make a living
without it - `taken` is 2 at simulated month 6 and still 2 at month 60.

Do not fix it by putting `TICKS_PER_DAY` back. That is the subsidy, and paying
it again is how it stayed hidden for a month.

## Closed, with the measurement

Five remain open, and none of them is nobody else's. Three -
`most_of_what_lived_here_still_lives_here`, `the_land_will_only_carry_so_many`
and `a_settlement_works_things_out_that_nobody_wrote_down` - point at the
predator layer below rather than at anything of their own, and two -
`a_settlement_still_raises_children_late_on` and
`the_children_of_a_settlement_live_past_infancy` - are ISSUES_FOUND #167's
central open question and should stay red until the store fills.

**`salt_tests::the_sea_costs_more_than_it_gives`** - the fixture, not the
model. `water_left_after_three_days` ran `TICKS_PER_DAY * 3` passes, which is
ninety days rather than three, and a body dries out in exactly
`MINUTES_TO_DIE_OF_THIRST` = three days - so both men sat at nought for
eighty-seven days and the comparison had nothing left to measure. Run over the
three days the model actually lives, the sea costs seventy per cent of what is
left:

    the man who drank the sea   hydration 0.150
    the man who drank nothing   hydration 0.500

Nothing in the model needed changing; #155 built it correctly. Same defect
class as ISSUES_FOUND #218, in a fixture rather than in the model.


**`agent_building_integration_tests::test_production_chain_buildings_cluster`**
- the model, and now fixed. Every criteria score in the placement code is
`weight / (1 + distance)` and saturates; the walk to the site was
`distance * 2.0` and did not. Only the four tiles orthogonally touching a
prerequisite could beat the builder standing still, so a production chain
clustered on a terrain roll. The walk is bounded now. Over twenty-four seeded
worlds the mill's distance from its farm went from 22 of 24 within eight with
a worst case of 41, to **24 of 24 with a worst case of 2**. ISSUES_FOUND #220.

**`specialisation_tests::a_dedicated_farmer_brings_back_more_than_a_casual_one`**
- the fixture. A pack holds forty-two and a founder starts with food in it, so
within a few trips both hands sat at 42.0/42.0 and every later trip took
nothing; what the patch lost measured how fast each filled a bag, and the
better hand fills it sooner. Carrying the load home between trips gives
**367 for the casual hand against 913 for the practised one, a ratio of
2.49** - which is what the code always claimed. ISSUES_FOUND #222.

**`thirst_tests::agents_drink_from_a_carried_container`** - the bound.
`turns_without_water` counts ticks, thirty to a step, and the test compared it
against forty *steps* - asking for a drink in the last two minutes of a
twenty-hour run. Its own failure message held the answer: 1,200 less 1,020 is
180, so the agent drank at step six. ISSUES_FOUND #222.



**`cooking_tests::an_agent_lights_a_fire_and_cooks_on_it`** - the model, in
three separate places, and now fixed. Red since it was written.

Two of the three were already known: the fixture gave no blade for a fish that
has to be cut before it will go over a fire, and `ENOUGH_TO_HAND` was six
against a fire costing ten (#221). Neither was enough, and the season was a
red herring - midsummer lit *fewer* fires than autumn.

What was left, from #224:

- `an_action_for` built a `Cook` without asking whether there was a fire, so
  eighty-three of them across twenty-four worlds were turns spent on nothing
  and lessons learned about the wrong thing.
- shedding tipped the makings of a fire onto the grass: forty wood down to
  four by turn six, and never ten again.
- and the one that decided it - `cooking_action` chains correctly and offered
  `LightFire` on twenty-two of eighty turns, but it answers **Sustenance**,
  which pressed at 0.01 against Preparedness at 11.3 and rising. It was never
  asked. The Hunger arm had a cooking branch of its own that required a fire
  to already be burning.

`food_action` now lights one where it stands when the wood is in the pack.
Over the fixture's four hundred turns: **LightFire 2, Cook 17, none refused.**

Only that one step moved across from the Sustenance ladder. Walking to
somebody else's fire and going out for wood stayed where they were - a branch
that can send a hungry man across the valley must not stand in front of eating
what he is carrying.

## Moved by #220-#222, and not re-baselined

Two of the behavioural thresholds #298 was filed for went green with #218 and
are red again after the placement and firewood work. Neither is re-baselined,
for the reason that task records: moving a threshold to fit a measurement is
what hid the last defect for a month.

| test | now | threshold |
|---|---|---|
| `errand_tests::a_walk_is_finished_rather_than_re_decided_at_every_step` | 466 of 1,413 kept to, **33.0%** | 50% |
| `relationship_graph_tests::a_settlement_ends_up_with_enemies_in_it` | green again at #224 | somebody does |

The second went green again with #224 and is off the list.

The first is **measured now, and it is noise.** It reads a single default-seed
world, and the quantity it reads is chaotic. Run over eight seeds, before and
after the fire chain of #224:

| | mean ratio | range | seeds over the 0.5 line |
|---|---|---|---|
| before | 0.377 | 0.236 - 0.462 | **0 of 8** |
| after | 0.365 | 0.226 - 0.502 | 1 of 8 |

The threshold is outside the spread on both sides of the change, so the test
does not pass on any seed in either arm and the 49.2% it once reported was
luck rather than health. Whatever it is measuring, it is not what it claims to
be measuring: 1,401 of 1,413 errands *arrive*, so `kept to it / set out` is
really the average length of a walk less one, and a settlement that finds what
it wants nearby scores badly for it.

**Do not re-baseline it and do not delete it.** It wants a predicate that
survives a change of seed - arrivals against abandonments would be one, and
the counters for it are already kept. That is its own piece of work.

For contrast, the same eight seeds on what #224 was actually for:

| | fires standing | LightFire | Cook |
|---|---|---|---|
| before | 0 to 2 | 0 to 2, **two worlds with none** | 0 to 100, two with none |
| after | 1 to 8 | 4 to 15, all eight | 35 to 195, all eight |
