# Patterns which can satisfy drive demand

Five layers between a body wanting something and a pair of hands doing
something about it. This is the specification; what follows each layer is where
it lives in this model, and where it does not live anywhere yet.

---

## Layer 1: Drives

What a body wants. Hydration, hunger, warmth, shelter, safety, rest, social
belonging, status.

**In the model: `core::drives::DriveType`, sixteen of them** — Thirst, Hunger,
Rest, Shelter, Safety, Aggression, Preparedness, Industry, Sustenance,
Curiosity, Social, Reproduction, Luxury, Utility, Construction, Protection.
Ranked by how fast each would kill (#75), and a drive does not build until the
one before it is answered (#76).

Near enough one-to-one with the list above. Nothing to do here.

---

## Layer 2: Goals

What answering the drive would look like. Obtain potable water; obtain edible
calories; secure a sleeping place; acquire a cutting tool; preserve food;
improve local shelter.

**In the model: `wanting::goal`, and the finding is that the layer was never
absent - it was distributed.** Four spellings, none of them called a goal:

| What | Where it lives | What it answers |
|---|---|---|
| The thresholds | `Preparedness` and `Sustenance` - `ENOUGH_FOOD`, `ENOUGH_MATERIALS`, `ENOUGH_TOOLS` | how much is enough |
| The commitment | `Errand` and `stick_to_the_errand` | holding to it until it is met |
| The food reckoning | `provision::WhatIsPutBy` | days in hand against a winter |
| The name `Goal` | `core::goals` | emotions and property, and neither of the above |

So "enough put by to see a winter out" is not a goal under Hunger here. It was
**promoted to a drive of its own**, with the threshold as a constant inside it.
That is a design decision rather than a mistake - it is why a full man with an
empty pit still goes to work - but it is why the layer reads as missing.

`wanting::goal::Goal` names the six goals and, for each, says what enough means
and **which of the above already asks it** (`who_already_asks_it`). Where a
threshold already exists it is read from where it lives rather than restated,
and `the_goal_table_and_the_drives_cannot_drift` holds it to that, so naming the
layer did not fork the model into two opinions about what enough is.

Exactly one goal is asked by nobody: **obtain potable water**. A container is
filled as a side effect of drinking at a source, and thirst only rises once the
body is already dry, so nobody ever fills a skin against tomorrow. That one is
now taken, last of all, on a turn that would otherwise have been spent standing
still - a goal that can outrank a pressing drive is a drive, and this layer is
not for making more of those.

`core::goals` is left where it stands. It answers a different question badly,
#187 has the measurement, and folding it in here would be two changes at once.

---

## Layer 3: Strategies

The distinct ways of reaching a goal. For hydration:

1. drink carried water
2. drink from a nearby source directly
3. fetch water from a source
4. dig or repair a local well
5. trade for water
6. follow an agent who knows where water is

**In the model: `analytics::wanting::strategy`.** Built. Thirst, Hunger and
Shelter have their ways declared - twenty-four of them - each with a name, a
horizon, a reachability, preconditions, and a cost.

Thirst and Shelter choose through it. **Hunger declares its ways and its arm is
deliberately not wired to them**: that arm carries the plan reader, the
composition reader and the search that #188 to #190 measured into it, and
putting a fresh ranker in front of all that would throw them away to buy an
ordering. Moving hunger over means moving those over with it.

What was there before, and what it cost:

The ranking exists - but as **seventeen hand-written `.or_else()` chains** in
`analytics::wanting`, where the order is source order and the source keeps
apologising for it:

> *"This took the first thing in the table it could do and stopped, so the
> order of a hand-written list decided what a whole people ever made."*

> *"retting flax sits above fermenting fruit, so over eight worlds nobody ever
> fermented anything, because somebody always had flax."*

Two separate places work around it with the same trick - `self.id.as_u128() %
could.len()`, "where a man starts in the list is his own business" - which is a
per-agent rotation standing in for a ranking nobody could learn.

**And the rungs are not even one strategy each.** The hydration arm's first
test is `carrying_water || water_in_reach`, so *drink what you carry* and
*drink from the water in front of you* are one rung producing one action. They
are not the same strategy: one runs out and has to be refilled, the other does
not and cannot be carried. Nothing in the model can tell them apart, so nothing
can learn the difference between them.

---

### Utility

```text
utility = relief - time - effort - danger - wear - uncertainty
```

**All six terms in one currency: drive demand.** A formula that subtracts turns
from demand and energy from both is arithmetic on three different things, so
every cost is converted before it is taken off. That the currency is drive
demand is not arbitrary - it is what the pattern layer is already denominated
in, and #188 is a long note about what happens when one part of this model
quietly starts keeping a second set of books.

Uncertainty discounts the relief rather than being subtracted beside it: a
half-believed mouthful is worth half a mouthful. Priced flat, a big enough
relief would swamp any doubt at all.

Each term comes from the thing in the model that already knows it - dread from
the trails, confidence from the lessons, turns from the distance actually being
walked - rather than from a table of guesses.

#### What the price is currently worth, which is less than it looks

All three drives now choose through this layer, hunger last and hardest: its
arm was the longest in the model and carried the in-hand pre-empt, the plan
reader and the composition reader, none of which were about hunger. They went
with it, so thirst and shelter have them too.

Ranking on the price was then measured against simply taking the ways in the
order they are written, two blocks of 32 worlds over two years, against the
hand-ordered ladder the layer replaced:

|                   | person-days       | emptied | month nine |
|-------------------|-------------------|---------|------------|
| the old ladder    | 105,429 / 106,431 | 28 / 28 | 8.2 / 8.5  |
| ranked by price   | 105,746 / 103,600 | 25 / 27 | 7.8 / 7.8  |
| in written order  | 108,396 / 102,780 | 24 / 28 | 8.2 / 7.6  |
| price, no doubt   | 107,982 / 102,270 | 26 / 29 | 8.2 / 7.6  |

Written order won the first block and lost the second by as much. Over all 64
worlds the four come to 211,860, 209,346, 211,176 and 210,252 person-days: a
spread of about one per cent on a measure whose block-to-block noise is ten. So
**the price is not yet deciding anything**, and the reason is the scaling
rather than the shape.

Within one need every way relieves the same need by the same amount, so
`relief` is common to all of them and cancels. What is left to tell two ways
apart is the cost spread, and the whole spread from reaching into your own pack
to going hunting is about a fifth of a point. Uncertainty is
`relief * (1 - confidence)` and runs to a full one. **The price of everything
the ways differ in is a fifth of the price of the one thing they do not**, so
the sort is very nearly a sort on `Lessons`' confidence with the walk and the
work as rounding error.

Denominating a walk and a doubt in one currency is therefore the next piece of
work on this layer, and it is the same complaint #193 makes about errands. The
sort stays meanwhile, because the specification asks for the ways to be priced
and chosen on the price and because nothing measured argues against it;
`a_doubt_outweighs_every_cost_put_together` holds the arithmetic still until it
is done.

One way is named, reachable and silent: **`TradeForFood` has never once
fired.** Removing the arm entirely gave a byte-identical 32 worlds - same
person-days, same worlds emptied on the same days - so `somebody_to_trade_with`
never answers a hungry man in a live settlement. Compare #226.

### Horizon

Immediate, short-term, long-term. **A man dying of thirst does not dig a well,
however good a well is** - and that is not the well being worth less, since
over a season it is worth far more than a mouthful. So the horizon is a *gate*
applied before utility decides within it, not another term subtracted from the
score. A cost that is subtracted can always be outweighed by a big enough
number; this must not be.

### Access - what may I use?

The fourth of the six questions, and the one that had no answer anywhere in
this model. There were two owner fields. `Territory` has one and nothing
outside its own file has ever read it. `Building` has one, `owns_house` in the
goal world-state reads it, and **nothing has ever written it** - so every agent
in every world has always been told it owns no house. Neither was a claim
anybody could act on.

`world::belonging` holds it now, and it is deliberately two things rather than
one:

- **`Belongs`** is a fact about a thing, and it lives on the thing.
  `ToNobody`, `To(somebody)`, `ToUsAll`. A pit belongs to whoever dug it; a pit
  dug under the common roof is the settlement's; a hut to whoever put it up if
  it is a dwelling and to the settlement if it is a workshop; a berry bush to
  nobody, which is the honest answer for most of what a stone-age settlement
  uses.
- **`Access`** is what a *particular person* may do with it, which is not a
  fact about the thing at all - the same hut is a man's own, his brother's, or
  a stranger's, depending entirely on who is asking. `Agent::may_i_use`
  answers it, and answers it out of the `RelationshipMap` the model has kept
  since the relationship graph was built. **There is no household object and
  there does not need to be one**: a household is who you are kin to - parent,
  child, sibling, partner - and that has been written down all along without
  anything ever asking it a question. A friend is not kin, on purpose:
  friendship is who you would help, kinship is whose store you would open
  without asking, and widening it to friends would make the distinction a
  formality in a camp of twelve.

**A claim orders what a man reaches for. It never refuses him.** A stranger's
pit is still the answer when it is the only one he remembers, and every tile
that was cover before is cover to somebody now. Hunger and the weather take
four deaths in five here; a rule that let a man starve beside a full larder, or
freeze outside a hut, over whose it was would cost far more than it bought.
Two tests hold that in both directions and they are the two worth keeping if
the rest went.

What it buys today:

- `UseHouseholdShelter` and `ShareCommunalShelter` were declared
  `NotYet("shelter has no owner, so somebody else's is not a different thing
  from one's own")`. That sentence is now untrue, and shelter is four ways -
  his own, a kinsman's, the settlement's, and a wood - which between them
  accept exactly the tiles the single arm accepted. They all come out as
  `SeekShelter`, so which fires does not change where he goes; it changes what
  `Element::By` records, and the pattern layer can now find out that one of
  them keeps working and another stops when a brother dies.
- A man walks to his own store, his kin's, or the settlement's before he walks
  to one somebody else sank. This is the first place in the model where access
  decides anything.
- `owns_house` starts being true for somebody.

What is still missing is the thing `RequestCommunalAllocation` wants: there is
no settlement to *ask*, so `ToUsAll` means "anybody here may use it" rather
than naming a body that owns it. Multi-agent
coordination sits on top of this and is much the larger piece.

### Reach

Ten of the twenty-five ways are declared and cannot fire: no rain catchment, no
water table to sink a well into, no settlement object to ask for a share, no
condition to mend, and building answered by Construction rather than by
Shelter. Each says so. A named gap is one somebody can count and go and fill;
an unnamed one is a gap nobody knows is there.

Two of them have already been filled by being counted. "No owner on a shelter"
was the reason `UseHouseholdShelter` and `ShareCommunalShelter` could not fire,
and it is a sentence somebody could read and act on - which is the whole
argument for declaring a way you cannot yet take.

---

## Layer 4: Tasks and actions

walk, gather, dig, fill, craft, trade, hunt, fish, build, sew, knap.

**In the model: `environment::Action`, and the verb matrix in
`environment::verbs`.** Actions are already operators with preconditions as
data - `Wants::{BareHands, AFreeHand, AToolFor(trade), ThisInHand(name),
AVessel}` - and effects and costs as data, through
`ActionResult::with_drive_change` and `with_energy_cost`.

Preconditions, inputs, effects and costs are there. Risks and skill
requirements are scattered rather than declared.

---

## Layer 5: Capabilities and resources

cutting tool, digging tool, carrying container, water source access, fibre
source, fuel, labour help.

**In the model: `environment::making::Tool`, on a different axis.** A tool is
`{ called, helps: SkillType, how_much_better: f32, how_long_it_lasts }`, one
row per tool-and-trade pair. That is already a capability table with
coefficients:

| | | |
|---|---|---|
| shovel | Mining | **1.9** |
| handaxe | Mining | **1.5** |
| diggingstick | Mining | **1.2** |

which is `digging_tool 1.0 / 0.7 / 0.3` in a different normalisation. The axis
is the **trade** rather than the **capability**, and for digging, mining and
fishing the two coincide.

They come apart for the rest:

- **cutting_tool** is smeared across three trades - handaxe/Woodcutting 1.8,
  stoneknife/Leatherworking 1.8, stoneknife/Crafting 1.3 - so a verb that wants
  "something that cuts" has to name a trade and gets the wrong tool.
- **hunting_weapon** lumps a bow with a spear, which are not the same thing in
  the hand.
- **carrying_container** and **water_container** are not in the table at all.
  Containers run through a parallel mechanism: `Wants::AVessel` and
  `Working.holds`.

Three of the seven tags already have an axis. Four do not, and containers are a
second mechanism answering the same question - which is this project's
recurring defect, already in place.

### What a tool is worth: two questions, not one

A tool answers two questions and they have different inputs.

| | Reads | Sets |
|---|---|---|
| `how_fast_my_tools_make_this_go` | technology, workmanship, **wear**, in hand or in pack | the energy a trip costs, the odds a cast or throw tells, the work a turn of making gets through |
| `how_much_my_tools_bring_back` | technology, workmanship | what comes back off a bush, off a carcass, off a core |

**Durability lives on the first and nowhere else.** A blunt flake takes longer
over a carcass; it does not leave more on the bone. Speed has no single
spelling here because a turn is a fixed slice of a day with no clock inside
it - the three currencies above are all the same quantity, how much of the job
one turn finishes.

The wear curve is a straight line rather than a set of bands, so every stroke
of use tells a little and none of them tells suddenly. See ISSUES_FOUND.md
#200, including what it cost to anchor a gathering trip's cost wrongly.

### What workmanship is worth, and what decides it

Quality is the third input, beside technology and wear, and it is the one that
reaches furthest: it tells on how fast the work goes, on how much comes back,
on how long the thing lasts, and - on a garment - on how much weather it keeps
off. Two agents in the same coat cut to the same pattern are not equally warm.

The ladder is the specification's own: Crude, Poor, Common, Good, Fine,
Masterwork, with `Common` the neutral rung that everything else is priced
against.

What decides the quality of what comes out is **the hand and the tool
together**: `min(Quality::from_hand, tool.material_quality_limit())`. Skill
decides whether the attempt comes off at all; the tool caps how good the
result can be. A master with nothing but a crude flake turns out good work and
not fine work, and a beginner with a fine knife still turns out a beginner's
work.

A spoiled attempt is not a refusal - `ActionResult::attempted` holds the two
apart - because a beginner who learns from spoiling a hide that *making does
not work* never practises into a master, and the whole point of a skill
deciding the odds is that practice pays.

See ISSUES_FOUND.md #201, including three things added there that the
specification did not ask for, each of which made the measurement look like a
regression caused by the specification.

### Stage 0: what the layer starts holding

A capability table says what a people *can* use. It says nothing about what a
people *has* on the first day, and until now nothing did: that was spelled
thirty-two times over, once per recipe, in `Making::obvious`.

`environment::stage` names it. Thirty-five development paths, each with a
Stage 0 - what is in a people's hands, what that lets it do, what it cannot do
until the path advances - and, honestly, whether this world carries it:

| | Paths |
|---|---|
| Carried whole | 19 |
| Carried in part | 12 |
| Not carried at all | 4 |

The twelve half-carried paths are the layer's real gap list, each naming its
missing half: no ember is ever carried, no shellfish is ever gathered, nothing
follows a camp for its scraps, nobody can tell anybody what to do, and the
carrying half of the water path is one carved bowl that almost nobody makes.

One path the table and the model disagree about, and the disagreement was
measured rather than argued: the specification starts a people able to
hand-shape a clay vessel, and making that so cost **4.9% of person-days and
eight of twenty-one first winters**. Not because of clay - because nothing in
the model asks whether what a working makes is worth making, and an unfired
shape is the first obvious product that is worth nothing.

The stages above zero are the user's to write. What is here is the shape that
takes them - `Stage { number, .. }` - and two rules that stop the table and the
recipe tables drifting into two opinions about what a people knows. See
ISSUES_FOUND.md #199.

---

## Satisfiers and enablers

**Not represented, and its absence has cost.**

Hydration is satisfied by water. A gourd is a transport and storage enabler.
Nothing in the model marks the difference: both come off Thirst demand through
one undifferentiated path. #179 (*a spring is a flow, not a barrel*) and #180
(*thirst is a reason to move camp, and nothing treats it as one*) are both that
distinction going missing.

For hydration:

- **Satisfier**: potable water consumed.
- **Enablers**: a container; access to a source; a path to it; time; safety.

---

## The order of work

1. **Layer 3**, because it is the only layer genuinely absent, and because the
   pattern layer already knows how to learn a ranking - it does it for verbs,
   for places and for runs. A strategy is one more kind of element.
2. **Layer 5's missing four tags**, folding `Wants::AVessel` in rather than
   leaving containers as a second mechanism.
3. **Satisfier and enabler**, which is small and which Layer 3 will make
   obvious.
4. **Layer 2**, last, because it is the largest blast radius and because a
   goal is worth little until there are strategies underneath it. Done: and
   the blast radius turned out to be small, because three quarters of the
   layer was already standing under other names. See Layer 2 above.
