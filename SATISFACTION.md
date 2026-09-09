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

**In the model: `core::goals`, and it is the wrong shape.** `InternalGoal` is
about emotions - `IncreaseEmotion`, `ReduceStress`, `SeekEntertainment` - and
`ExternalGoal` is about property - `OwnHouse`, `StockHouseFood`. Neither says
"obtain potable water". #187 measured the branch that carries them and found it
nearly dead: un-gating it cost five per cent of person-days, because what it
carried was not worth carrying.

So the layer exists, is occupied by something else, and the something else does
not work. It wants rewriting into the shape above rather than extending.

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

### Horizon

Immediate, short-term, long-term. **A man dying of thirst does not dig a well,
however good a well is** - and that is not the well being worth less, since
over a season it is worth far more than a mouthful. So the horizon is a *gate*
applied before utility decides within it, not another term subtracted from the
score. A cost that is subtracted can always be outweighed by a big enough
number; this must not be.

### Reach

Seven of the twenty-four ways are declared and cannot fire: no rain catchment,
no water table to sink a well into, no settlement object to ask for a share, no
owner on a shelter, no condition to mend. Each says so. A named gap is one
somebody can count and go and fill; an unnamed one is a gap nobody knows is
there.

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
   goal is worth little until there are strategies underneath it.
