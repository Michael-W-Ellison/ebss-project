# Standing suite failures

What is red, why, and who is looking at it. One file, in the repository,
because a register that lives anywhere else does not survive the week: this
list was kept in a session task list and was lost with the container that held
it, while `ISSUES_FOUND.md` came through untouched because it is committed.

A test that is red and tracked is a known cost. A test that is red and
untracked makes every future full run read as normal. Keep the count here
honest and close nothing without a measurement.

**Last full run**: `cargo test --lib` - 2,642 tests, 36 minutes, **3 failed**,
2 ignored. #228, #229 and #230 took none off and broke none; what they moved
is recorded below. #230 is the one that moved the model: births across six
seeded settlement-years went from two to nine. Two of the three are one question - the store that fills and is
never eaten out of - and the third is a binary threshold that has now been on
both sides of its line four times.

Down from the ten this file opened with, by way of twelve: salt water came off
(the fixture, not the model), then the production chain, the practised hand
and the carried waterskin, then the fire after three separate faults in it,
then the ecology and the predator layer together, then the settlement that was
asked what it knew after everybody in it had died. ISSUES_FOUND #220 to #227.
Three defects that were on nobody's list were found on the way: a body that
aged thirty times too slowly (#219), every animal in the world drifting two
cells south-west a turn (#225), and three people in twelve standing in the sea
(#227).

## Open

| test | reports | what is known |
|---|---|---|
| `longevity_tests::a_settlement_still_raises_children_late_on` | nobody born into the settlement at 9,000 turns | **#167's question, and it is moving.** It was 63,456 refusals of "could not feed a child" and no conception ever. After #227 the same fixture conceives and bears one child; after #228 the turns anybody is actually *in condition* to breed went 368 to **1,117**, hunger deaths 9 to 7, and the last founder lives out the year instead of dying on day 342. Leave red: a settlement of twelve still collapses over its first winter with six thousand units in its pits, and until that stops there is nothing to raise a child on. |
| `survival_pressure_tests::the_children_of_a_settlement_live_past_infancy` | 0 born here at 6,000 turns | Same question as the row above. Its bound is sound: it counts by parentage, which is the right predicate. Leave red. |

## The predator layer, and what it turned out to be

Three of the failures pointed here and the reading was wrong. ISSUES_FOUND
#218 corrected `what_a_grazer_is_worth_to`, which converted days of keep into
hunger units with `TICKS_PER_DAY` where `hunger_rate` is charged once a
*pass*: one deer fed a wolf for four hundred and fifty days. The conversion is
not in doubt and must not be put back - that is the subsidy, and paying it
again is how it stayed hidden for a month.

What was read off it - that the predator layer had been living on the subsidy
and could not make a living without it - does not survive measurement.
**#300's `taken` at 2 in month 6 and still 2 in month 60 was taken in a world
where the deer had all walked off the map.** Every animal in the model drifted
two cells south-west a turn, because a wander was built out of a signed
remainder; see ISSUES_FOUND #225. A herd put down in the middle of a fifty by
fifty map was pressed into the corner by the thirtieth day, partly outside it,
and starved there with forty thousand units of forage standing behind it.

With the wander put right, `predator_prey_tests` and `ecology_tests` are green
end to end - 53 tests, the hunting ones among them.

## Closed, with the measurement

Two remain open, and they are one question:
`a_settlement_still_raises_children_late_on` and
`the_children_of_a_settlement_live_past_infancy` are ISSUES_FOUND #167's
central open problem - the store that never fills - and should stay red until
it does. A settlement of twelve dies out on day 327, which is what both of
them are really reporting.

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


**`predator_prey_tests::the_land_will_only_carry_so_many`** and
**`ecology_tests::most_of_what_lived_here_still_lives_here`** - the model, and
now fixed. Both had been read as the predator layer; neither was. The first
fixture has no predators in it at all.

Every animal in the world walked two cells south-west a turn, because five
places built a wander out of `any::<i32>() % 5 - 2` and a signed remainder
carries the sign of what it divides: -6 to 2, mean -2. Nothing clamped it
back, though the migration pass and the hunt both clamp. So a herd put down in
the middle of a map was in the corner by the thirtieth day and starved there.

On the same fixture and seed: grazers that find anything within reach went
from **21% to 81%**, a mouth's take at day 100 from 4.4 of the 8.6 it wanted
to 6.4, mean hunger at day 83 from 135 of 180 to 68, and the herd from dead at
day 130 to twenty-four head and growing. ISSUES_FOUND #225.


**`situation_tests::a_settlement_works_things_out_that_nobody_wrote_down`** -
the fixture, not the model, and it was reading a number that could only ever
have been nought. It ran the year out, broke when nobody was left alive, and
then summed `how_much_i_have_worked_out` over `population.agents` - which
`Population::turn` has already emptied, because it takes the dead off the
roll.

Probed turn by turn, the settlement works things out from the tenth day and
peaks at **thirty-one** between twelve people on day 195. It then dies out on
day 327 and the reading was taken from its graves. It takes the high-water
mark while the settlement is alive now, and says how many turns it lasted when
it fails, so a future failure distinguishes "learned nothing" from "nobody
lived". ISSUES_FOUND #226.

The die-off is real and is deliberately left to the two tests above. A
measurement taken after a die-off reads the survivors, and a total die-off
leaves none - worth remembering before reading any other run-it-out-and-sum
test.


**What is under the two that are left, so far.** Neither has come off, but
both moved, and what moved them was not the store.

A settlement of twelve was spending the last month of its life boxed in:
**2,975 refusals of "No passable route toward destination (standing on Sea,
which is walkable, with 0 ways out)"**. `Terrain::is_walkable` has said
`Water | Sea => false` since the sea was split off from fresh water, and
`Simulation::is_passable_tile` - a second answer to the same question - named
`Water` alone. So the pathfinder walked people into salt water and left them
there: three of twelve at once, one on the same tile from day 120 to day 190.

Asked of the terrain now. Same fixture and seed: the pits hold **8,178** at
their fullest against 4,476, nobody stands in the sea, and the settlement
**conceives and bears a child** where the whole history of that fixture was
178,913 refusals of "could not feed a child" and no conception at all.
ISSUES_FOUND #227.

It still dies out, around day 335 instead of 327, with 6,700 units in its
pits. Getting people out of the sea doubled the store and did not get the
store into them.


**What is under the two that are left, continued.** Still neither has come
off. #227 got people out of the sea and doubled the store; #228 went after why
they starve in front of it.

Two things were wrong and one was not. **Every infant in the model read as
living on its own reserve**, because `is_the_body_eating_itself` divided by a
grown body's capacity and an infant's is a fifth of one - a completely full
infant came out at 0.20 against a line of 0.25, for the whole of its infancy.
And **a starving man gave up on his larder at the first empty hole**: the
branch took the nearest pit he remembered and stopped, so a hole he had
already emptied answered for all the others. Over a winter's samples of a body
under a quarter of its own reserve carrying nothing, the branch came back
empty 13 times in 51 - and in **all thirteen** the man remembered a pit with
food in it.

The third was the hedgerow gate asking an acute predicate (`is_starving`:
thirty hours since the last bite) of a chronic question. That line is widened
to the one this module already draws, and it **moves nothing** - by the time
these people are in trouble the hedgerows are bare anyway.

And an ablation that is not kept: narrowing the shelter override, whose
recorded rejection rested on a premise that had since changed twice. It is
still worse - births 1 to 0, `ready to breed` 368 to 6. The note now carries
both measurements.

Same fixture and seed: hunger deaths 9 to 7, turns anybody was ready to breed
368 to **1,117**, and the last founder lives out the year rather than dying on
day 342. ISSUES_FOUND #228.


**And a third look under them, which found a real fault and moved nothing.**
The turn that kept coming up for a man a third through his own reserve was
`GiveTo` - seven to eight thousand of them in a settlement-year. One father,
four consecutive days, was handing his child **nine whole fish**, harmful by
the third day and spoiled as well by the fourth, and choosing it again each
morning because the child still had nothing to eat. `a_child_of_mine_to_feed`
asked `what_food_i_can_spare`, which asks `is_food`; the store branch had
already named what is wrong with that and the giving branch went on asking it.

Fixed, and **measurably neutral**: six seeded settlement-years give
person-turns 1,016,653 to 1,016,901, births 2 either way, three settlements in
six emptied either way. A single seed read much louder in both directions and
was noise. ISSUES_FOUND #229.

The standing problem is unchanged and is now measured across six seeds rather
than one: **two births in six settlement-years of twelve founders, and half
the settlements empty completely.**


**And the fourth look, which found the largest refusal in the model.**
`GiveTo` was chosen 7,156 times in a settlement-year of at most twelve people
and **6,230 of them came to nothing** - 87 per cent - and the decision that
chose them sits above every drive there is.

Three faults, each hiding the next. The executor had **no arm for one's own
child**: its only food path was the band rule, which keeps a day's food back
because the man beside you is not your child, while the decision fires on any
meal past two units. Then nobody asked whether the gift would go in the
taker's pack, though the store branch has asked exactly that since #215. Then
- #215 again, word for word - the two sides asked about different *amounts*:
the giver checks room for **one unit** and `giving_to` hands over **half the
stack**, and `hand_over` was all or nothing, so a man with room for three was
offered twenty and neither of them got anything.

Six seeded settlement-years, twelve founders each: **births 2 to 9**,
person-turns 1,016,901 to 1,037,206, wasted giving turns from about 37,000 to
468. One more settlement in six emptied, which is more children born and more
of them dying, and a binary count over six samples moving by one either way.
ISSUES_FOUND #230.

The two red tests above are unchanged. Their fixtures are longer and larger
than the six-seed probe, and a settlement that now has children in it still
does not hold them at six and nine thousand turns.


**And the fifth look, which is mostly a measurement.** With food moving
between people (#230), the same question was asked of the ground. Over three
seeded settlements across the hungry gap - 92,249 agent-turns - **27.0% went
on SeekShelter and 2.0% on the store**, and what came out of the pits was
**4.2 items a person-day against the 11.5 a grown body burns**.

The store was reachable from the hunger drive only through
`the_larder_or_this_walk`, which needs somewhere to walk to; in deep winter
there is nowhere, so a body burned the first sixteen days of a seventy-five
day gap on its own reserve with the store in the ground behind it. There is a
rung for it now. It is small: person-turns 1,037,206 to 1,040,633, one
settlement in six back off the floor, births and hunger deaths unchanged.
ISSUES_FOUND #231.

**The arithmetic that entry records is the useful part**, and it is what the
next work belongs to - read from `actions_taken` rather than from a probe,
because a probe that asks the decision layer a second time in the same turn
draws from the same seeded stream and moves the world it is measuring. The
first cut of those numbers was taken that way and is corrected in #231.

What the model actually did, over 99,046 person-turns across the gap: **Move
57.5%, SeekShelter 28.5%, Gather 11.4%, Eat 3.6%, PickUp 1.6%.** Six turns in
seven walking or sheltering, and 1.75 sittings a person-day - some two hundred
energy against the 1,440 a grown body burns, which matches what the physiology
reports directly (761 a person-day at day 300, 440 at day 330). A meal is the
size of what is carried, so the shortfall is not that eating is capped: there
is nothing in the pack, and the turns that would fetch some go on the walk and
the roof.


**And the sixth, which is the one the fixture noticed.** `Pit::something_to_eat`
answered with the **first** meal in the pit rather than the largest stack, and
a trip to the larder takes eight *capped at that one stack*. So a man standing
over a pit holding two legumes and four hundred roots took the two legumes and
came back tomorrow for two more. Over three seeded settlements through the
hungry gap: 1,635 trips yielding 5.5 items each against the eight asked for,
and **eighteen** refused for want of room - so it was never the pack.

Six seeded settlement-years: **settlements that emptied went from 3 of 6 to
0 of 6.** Every settlement now has somebody alive at the end of its first
year, which is the first time that has been true. Person-turns and births are
flat. ISSUES_FOUND #232.

They still get 9,359 items out of the ground against the 23,730 they burn
across the gap, and live on the difference out of their own reserve. The
binding term is how often a body reaches the store at all - 0.75 trips a
person-day against the two it would take - and #231 has that arithmetic.


**And the seventh, which kept nothing.** #231's budget put `SeekShelter` at
28.5% of every gap person-turn against `Eat` at 3.6%, so #233 went after it.

The walk is not the cost: a roof is within **half a pace** on 97.6% of those
turns and the agent is already under one on 24.1%, so almost every
`SeekShelter` is a huddle in place. And it cannot end the spell -
`needs_shelter` reads the exposure list, which hypothermia occupies for as
long as the body is cold, while `update_temperature_with_shelter` already
warms a sheltered body wherever it stands. The action does nothing the body
was not getting anyway and cannot resolve the state that chose it.

Both fixes measure badly and are reverted. Letting a roof mend exposure
without the turn is **inert** - it reproduced the baseline to the digit on
every seed, because `exposure_damage` feeds only `is_critical()` and that
never adds a turn. Skipping the override for the already-sheltered costs a
settlement: emptied 0 of 6 back to 1 of 6, births 8 to 7. The oddity is
recorded rather than fixed. ISSUES_FOUND #233.

**And #228's note about this override is corrected.** It called a one-seed
ablation decisive (births 1 to 0) for a change that is now measured as hitting
the same 28.3% of turns either way. The note in `wanting/mod.rs` now carries
the six-seed figures and says not to cite the old ones.


**And the eighth: the walk is not walking.** #231 put `Move` at 57.5% of every
gap person-turn. Read against agent positions, only **19,186** of those 56,982
turns changed anybody's position, and barely a thousand were refused.

A `Move` whose target is the tile the agent is standing on returns **success**
- "Already at destination" - so it is booked as a `Move`, costs the whole
turn, and appears in no refusal tally. Every instrument this project has for
finding wasted turns reads refusals, and this is not one. Counted from inside
that branch: **15,296 of 99,046 gap person-turns, 15.4%** - one turn in six -
against `Eat` at 3.6% and the store at 1.6%. The walking that does happen
nets 1.9 paces a person-day out of ten covered.

The counter is committed and stays. The obvious guard - a drive whose answer
is to stand still has not answered - removes 86% of them and is **not kept**:
gap person-turns fell 99,046 to 69,641, so thirty per cent fewer people were
alive to take them. Something downstream is living on those turns, and which
caller proposes the walk wants finding out rather than guessing at.
ISSUES_FOUND #234.

## Two thresholds that flap, and are not re-baselined

Two of the behavioural thresholds #298 was filed for. Each has crossed its
line in both directions more than once since #218, and on the last two full
runs they crossed in opposite directions. Neither is re-baselined, for the
reason that task records: moving a threshold to fit a measurement is what hid
the last defect for a month.

| test | now | threshold |
|---|---|---|
| `errand_tests::a_walk_is_finished_rather_than_re_decided_at_every_step` | green after #227, red after #224 | 50% |
| `relationship_graph_tests::a_settlement_ends_up_with_enemies_in_it` | red after #227, green after #225 | somebody falls out |

**Both have now been on both sides of their lines, and the two swapped over.**
The full run after #225 had the errand test red and the enemies test green;
the full run after #227 has it the other way about. The first is a ratio with
a spread of 0.23 to 0.50 across eight seeds; the second is a single yes-or-no
read off one world. Neither is measuring what it claims to, and neither is
re-baselined.

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
