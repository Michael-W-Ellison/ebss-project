# Ownership map

One place per fact, and the guard that holds it there.

This is not a directory plan. The code is laid out by *layer* - what a body is
(`agents/`), what the world is (`world/`, `environment/`), what a turn does
(`analytics/`) - and that layout encodes the order of a tick, which is worth
keeping. What this map does instead is name, for each subject, **the one place
that owns it**, so that a second answer to the same question is a thing you can
see rather than a thing you find out from a settlement that starved.

The failure this guards against is not a big file. It is one truth written down
twice and then drifting. Every defect in `ISSUES_FOUND.md` numbered 107 and
above was that: `id_to_item_type` drifted four separate times; the hungry gap
was derived in two places and came out 864 and 865; "is this food" had eight
answers that disagreed; the food year, the store target and the breeding gate
each picked their own figure for what a body eats.

**How to use it.** Before adding a fact - a table, a threshold, a predicate,
a rate - find its subject below and put it in the owner. If the owner is the
wrong place, move the owner and update this file. Do not add a second one.

**Status** is one of:
- **held** - one owner, and a test that fails if a second appears.
- **named** - one owner, no guard yet. Drift here is possible and would be
  caught only by reading.
- **split** - more than one owner today. The row says which, and what to do.
- **none** - nobody owns it. The row says what happens instead.

---

## Held

| Subject | Owner | Guard |
|---|---|---|
| **Resources** | `world::ResourceType` - what kinds exist, what bears when (`is_it_bearing`), what is grown, what is food (`is_it_food`, which `is_edible` and `raw_scent_strength` now both ask rather than answering for themselves) | `resources::all_resources_tests::every_resource_is_listed` - an exhaustive match that fails to compile if a variant is added and not listed in `all()`; `scent_tests::only_food_smells_of_food` and `nothing_anybody_eats_is_odourless` hold the scent table to the food list in both directions |
| **Sustenance** | `world::ItemType::is_it_food` for types, `world::nutrition::is_this_food` for names, `world::nutrition::FoodDatabase` for what a food is worth | `nutrition::one_answer_to_what_is_food` - holds the static list to the runtime database, and the name-level question to the type-level one |
| **Ticks** | `environment::seasons` - `TICKS_PER_DAY`, `DAYS_PER_SEASON`, `DAYS_PER_YEAR`, `TICKS_PER_YEAR`, and the window vocabulary (`first_day_of`, `last_day_of`) | `analytics::tests::calendar_tests`, and `growing_up_tests::a_life_is_the_length_the_specification_gives`, which holds the calendar to the specification's own 518,400 minutes a year and 36,288,000 in a life |
| **Drives** | `core::drives::DriveType` - which drives exist, their thresholds, their rates, their tiers | `DriveType::all()` plus `core::tests::drive_hierarchy_tests` |
| **Traits** | `core::traits::Trait` - which traits exist, and what each leans a drive towards (`leanings`) | `traits::every_trait_tests` - an exhaustive match, plus a count of how many traits lean on nothing so the number cannot climb unnoticed |
| **Materials** | `world::ItemType` for what a thing *is*, `environment::material::Material` for its state and category, `agents::storage_integration::id_to_item_type` for turning a name back into a type | `inventory::every_item_type_tests` - an exhaustive match; that nothing is two of food, tool and weapon at once; and that every type survives a round trip through its own name |

## Named

| Subject | Owner | What it owns | Drift risk |
|---|---|---|---|
| **Flora** | `environment::flora` | Species, growth stages, what a plant drops, lifecycle, yield | `PlantDrop` names 62 things a plant can give; `id_to_item_type` must know each one that is food. Guarded only by `one_answer_to_what_is_food` naming a handful. |
| **Fauna** | `environment::fauna` | Species, behaviour, diet, what a kill drops | Kill drops are renamed by `storage_integration::butchered_item_id`; a new species dropping a new cut needs that table |
| **Weather** | `environment::weather` | Events, severity, what they do to the ground and to a body | - |
| **Buildings** | `world::buildings` | What can be built, what it costs, what it does | - |
| **Agents** | `agents::agent::Agent` for the whole, `agents::physiology` for the body's arithmetic, `agents::provision` for what a store is worth in days | Three files, three distinct facts - not drift, but the boundary is worth keeping in mind |
| **Processes** | `environment::making` | The steps a people can put together and what comes of them - the specification's "there are actions which can be taken, and there are outcomes of those actions" | `world::production` and `environment::crafting` also describe processes; see **split** below |
| **Decisions** | `analytics::wanting` | Given a drive, what would answer it | - |
| **World_interaction** | `analytics::doing` | One module per family of verbs, and the dispatcher | - |
| **Discussions** | `analytics::between_us` for the act, `agents::gossip` for distortion and truth-tracking, `agents::shared_knowledge` for what a settlement holds in common | - |
| **Containers** | `agents::transport::TransportType` for what a carrier adds - baskets, bags, travois, carts, pack animals - and `agents::agent::Inventory::max_weight` for the one figure that comes out of it. `InventoryItem` (`new_container`, `fill_level`, `max_capacity`) for vessels that hold fluid | **held**: `carrying_tests::a_basket_is_counted_once` and `a_full_pack_is_full_by_the_same_figure_that_fills_it`. Both were written after `effective_max_weight` was found counting a basket a second time on top of the transport system - fifty from one basket, and everybody permanently over their own limit and walking slowly for it (#116) |
| **Map_generation** | `world::WorldConfig` and `world::ResourceConfig` for the knobs, `world::terrain` and `world::resource_spawning` for the generation, `core::dice` for the randomness | The knobs are node counts and a size. There is nothing to turn for terrain, biome, or how clustered a resource is - see #234 |

## Split - more than one owner today

| Subject | The two | What to do |
|---|---|---|
| **Technology** | `world::technology` (`pub struct Technology`, `TechnologyTree`) and `environment::technology` (`pub struct Technology`, `TechnologyRegistry`). **Both re-exported at crate level**, so two different types share one name and a glob import can pick either. Nothing imports `world::technology` by path; four things import `environment::technology` | #233 |
| **Actions** | `world::actions` (1,575 lines, `pub enum Action` - the live one) and `environment::action` (210 lines), both re-exported. `environment::verbs` owns the *matrix* - what each verb targets, requires and changes - which is a different fact and correctly separate | #233 |
| **Crafting** | `world::crafting` (`RecipeRegistry`) and `environment::crafting` (`RecipeBook`), plus `environment::making` above | #233 |
| **Emotions** | `agents::emotions` (1,710 lines, 16 importers) and `core::emotions` (230 lines, re-exported, imported by path nowhere) | #233 |
| **Tools** | No single owner. What a tool *is* lives in `world::ItemType::is_tool`; what it adds to a job is `Agent::how_much_my_tools_help`; durability is unsettled since the second model was deleted (#219); the ladder of which tool follows which is in `environment::making` | #219 decides where durability lives; the rest wants naming |

## None - nobody owns it

| Subject | What happens instead |
|---|---|
| **Pathfinding** | `world::path_planning::PathPlanner` is a full A* with tests and **no live caller** - it appears outside its own file only in `world::tdd_tests`. Only `RoadNetwork` is instantiated. Agents walk in `analytics::doing::moving::walking`; fauna moves by adding a signum to a coordinate in `environment::fauna`. Three implementations of "how a thing crosses the map", one of them dead, none of them shared. See #235 |

---

## Guards worth adding

A guard is cheap and pays immediately - `every_resource_is_listed` and
`one_answer_to_what_is_food` have each already caught a mistake made in the
same hour they were written, and `only_food_smells_of_food` was written for a
defect that had been costing a settlement seven per cent of its life. The pattern that works:

1. **Exhaustive match.** An `all()` for the enum plus a test whose `match` has
   an arm per variant. Adding a variant without listing it fails to compile.
2. **Two representations must agree.** A static list against a runtime table;
   a name-level predicate against a type-level one.
3. **Derived, not picked.** Where a figure follows from two others, write the
   derivation and let both consumers call it - see
   `provision::how_long_the_land_gives_nothing`, which the settlement store and
   the breeding gate now share after coming out 864 and 865 apart.

Both of the guards this map called for were written the same day, and both
found something before the ink was dry:

- **`Trait::all()`** turned up six variants that call themselves aliases and
  are not. `Hottempered` says "Alias for HotHeaded" in its own comment and is a
  separate enum variant, so the two never compare equal and an agent given one
  gets none of the other's leanings. Both spellings are live. Same for
  `Empathic`/`Empathetic` and `Manipulator`/`Manipulative` - and on the
  empathy pair it is the *less* used spelling that carries the leanings, eleven
  call sites to two. See #234. It also counted 38 of 82 traits leaning on no
  drive at all.
- **`ItemType::all()`** turned up eighteen of seventy-four types that did not
  survive a round trip through their own name: every copper, bronze and steel
  thing, and water. Fifth drift of `id_to_item_type`. Fixed, and the guard now
  holds it.

Still missing, in the order it would pay: something that holds
`environment::verbs` to the `Action` enum, so a verb cannot exist in the matrix
and not in the model (#129); and a guard on the flora drop names, so a plant
cannot drop something `id_to_item_type` has never heard of.
