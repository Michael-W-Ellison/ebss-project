# Standing suite failures

What is red, why, and who is looking at it. One file, in the repository,
because a register that lives anywhere else does not survive the week: this
list was kept in a session task list and was lost with the container that held
it, while `ISSUES_FOUND.md` came through untouched because it is committed.

A test that is red and tracked is a known cost. A test that is red and
untracked makes every future full run read as normal. Keep the count here
honest and close nothing without a measurement.

**Last full run**: `cargo test --lib` - 2,636 tests, 53 minutes, **9 failed**,
2 ignored. Down one from the ten this file opened with: salt water came off
(the fixture, not the model), and a defect that was not on the list at all was
found and fixed on the way to it - ISSUES_FOUND #219, a body aged thirty times
too slowly, so nobody ever grew up and nobody ever died of old age. Two tests
that pinned that defect rather than the behaviour were corrected with it. Every one of the ten predates the clock work of ISSUES_FOUND #217
and #218; those two commits took six others green.

## Open

| test | reports | what is known |
|---|---|---|
| `cooking_tests::an_agent_lights_a_fire_and_cooks_on_it` | no fire is ever lit in 400 turns | **Two real faults found and fixed, and what is left is a design question.** The fixture gave no blade, and a whole fish cannot be cooked without one; and `ENOUGH_TO_HAND` was six against a fire costing ten, so no fire could be built in any world ever (ISSUES_FOUND #221). With both put right a fire is lit in **5 of 24** seeded worlds, because cooking is deliberately suppressed while an agent is putting food by - Dry 486 against Cook 119 across those worlds. The test's claim and the `!putting_by` gate disagree. **Needs a decision, not a fix**: either the test should assert what the model intends, or drying-beats-cooking wants revisiting. Do not seed it to one of the five. |
| `ecology_tests::most_of_what_lived_here_still_lives_here` | 8 worlds open with 468 head and hold 107 | Species all survive; the head count is ten short of the quarter it wants. Probably downstream of the predator layer - re-run after that is settled rather than treating it as its own finding. |
| `longevity_tests::a_settlement_still_raises_children_late_on` | nobody born into the settlement at 9,000 turns | **#167's question, measured again.** Nobody is ever born at all: 63,456 refusals in 6,000 steps and every one of them "could not feed a child". The gate wants 129,600 units and the best-placed agent holds 8,500 - a factor of fifteen, against the fifty-three #167 measured. Not a test problem and not a gate problem; the store has to fill first (#240, #241, #213). Leave red. |
| `predator_prey_tests::the_land_will_only_carry_so_many` | 0 against 0 | Both fixture herds go extinct. At #217 it read 2 against 6. The predator layer below. |
| `situation_tests::a_settlement_works_things_out_that_nobody_wrote_down` | nobody notices one afternoon goes better than another | Passed at ISSUES_FOUND #177 and is red again. Probably downstream of the predator layer - less happening in the world to notice. |
| `survival_pressure_tests::the_children_of_a_settlement_live_past_infancy` | 0 born here at 6,000 turns | Same as the row above - #167's gate, now fifteen times out of reach rather than fifty-three. Its bound is sound: it counts by parentage, which is the right predicate. Leave red. |

## The predator layer

Three of the ten above point at one thing, and it has its own finding rather
than a row here. ISSUES_FOUND #218 corrected `what_a_grazer_is_worth_to`, which
converted days of keep into hunger units with `TICKS_PER_DAY` where
`hunger_rate` is charged once a *pass*: one deer fed a wolf for four hundred
and fifty days. The conversion is not in doubt. What it revealed is that the
predator layer was living on that thirtyfold subsidy and cannot make a living
without it - `taken` is 2 at simulated month 6 and still 2 at month 60.

Do not fix it by putting `TICKS_PER_DAY` back. That is the subsidy, and paying
it again is how it stayed hidden for a month.

## Closed, with the measurement

Six remain open. Three of them - `most_of_what_lived_here_still_lives_here`,
`the_land_will_only_carry_so_many` and
`a_settlement_works_things_out_that_nobody_wrote_down` - point at the predator
layer below rather than at anything of their own, and two -
`a_settlement_still_raises_children_late_on` and
`the_children_of_a_settlement_live_past_infancy` - are ISSUES_FOUND #167's
central open question and should stay red until the store fills. That leaves
one that is nobody else's: the fire, and what is left of it is a question
rather than a defect.

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
