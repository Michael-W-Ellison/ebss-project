// src/analytics/wanting/afforded.rs
//! What could I do here, holding this?
//!
//! The verb matrix has always known the answer and has never been asked.
//! `EVERY_VERB` declares, for each of seventy-four verbs, what it acts on and
//! what it wants in the hands doing it - and outside the tests the only
//! production caller of the module is `errands`, which asks
//! `what_this_action_cannot_do_without(named)`: *what does this
//! already-chosen action want, so I can go and fetch it*. The matrix is
//! consulted about a verb the hand-written gates have already picked. Nobody
//! ever asked it the other way round.
//!
//! This is the other way round. Run the same two tests - is there such a
//! target here, will these hands do - across the whole matrix instead of
//! across one named verb, and what comes back is the set of things this
//! person could do, standing here, carrying what they are carrying.
//!
//! **That makes "holding a thing opens new verbs" true by construction rather
//! than by anybody writing it down.** A man with a free hand and a stone
//! underfoot can `stack`; put the stone in his hand and he can `throw`. Both
//! verbs have been in the matrix all along with their wants declared; what
//! was missing was anything that enumerated them. Nothing here knows what a
//! stone is for, and that is the point - the affordance falls out of the
//! matrix, so a new thing with the right tags inherits the verb surface
//! without a line of code.
//!
//! What this deliberately does **not** do is choose. Ranking these candidates
//! is the novelty question - how often has this verb been tried on this kind
//! of thing, and did it ever come to anything - and it wants the per-verb,
//! per-object lesson key to exist first. That is a separate piece of work and
//! this is the generator it will read.

use super::super::Simulation;
use crate::agents::Agent;
use crate::environment::verbs::{Targets, Verb, EVERY_VERB};
use crate::environment::Action;
use crate::world::Position;

impl Simulation {
    /// How far off a thing can be and still be something you could act on.
    ///
    /// A person or an animal, not a bush: the verbs that target those are
    /// hand-to-hand, and a deer across the valley is an expedition rather than
    /// an affordance. See `AS_NEAR_AS_PREY_HAS_TO_BE_TO_BOTHER`, which is the
    /// same argument made about hunting.
    pub(in crate::analytics) const WITHIN_REACH: f32 = 1.5;

    /// Every verb this agent could perform standing where it is.
    ///
    /// Includes verbs the matrix declares and nothing performs - `done_by` is
    /// `None` for eighteen of them, `stack` and `throw` among them. They are
    /// returned rather than filtered out on purpose: a generator that quietly
    /// dropped them would report a world of possibilities that is smaller than
    /// the matrix says and give no sign of the difference. `what_i_could_do_here_now`
    /// is the filtered form, for a caller that means to act.
    pub fn what_i_could_do_here(&self, agent: &Agent) -> Vec<&'static Verb> {
        EVERY_VERB
            .iter()
            .filter(|verb| self.is_there_such_a_target(verb.targets, agent))
            .filter(|verb| Self::do_these_hands_do(agent, &verb.wants))
            .collect()
    }

    /// The same, less the verbs nothing in the simulation carries out.
    pub fn what_i_could_do_here_now(&self, agent: &Agent) -> Vec<&'static Verb> {
        self.what_i_could_do_here(agent)
            .into_iter()
            .filter(|verb| verb.done_by.is_some())
            .collect()
    }

    /// Whether there is anything here for a verb of this sort to act on.
    ///
    /// The half of the matrix nothing has ever answered. `Wants` came with
    /// `satisfied_by_hands` and so could always be asked; `Targets` was
    /// declared, tested for coverage, and never once put to a world.
    pub fn is_there_such_a_target(&self, targets: Targets, agent: &Agent) -> bool {
        let at = agent.state.position;
        let here = Position::new(at.0, at.1);

        match targets {
            // Nothing outside the actor, so there is always such a target:
            // somebody can always sit down.
            Targets::Nobody => true,

            // Somewhere else on the map. There always is somewhere else.
            Targets::APlace => true,

            Targets::AThingHeld => !agent.inventory.get_all_items().is_empty(),

            // Lying here, growing here, or buried here. A pit counts: what is
            // in a store is as much underfoot as what is on the grass, and the
            // store branch of `picking_up` has always treated it so.
            Targets::AThingUnderfoot => {
                !self.world.what_is_lying_at(&here).is_empty()
                    || self
                        .world
                        .get_resource_at(&here)
                        .is_some_and(|node| node.amount > 0)
                    || self.world.pit_at(here).is_some()
            }

            // The tile itself - which exists unless the agent is off the map,
            // and an agent off the map has worse problems than this.
            Targets::TheGroundUnderfoot => self.world.grid.get_tile(&here).is_some(),

            Targets::APerson => self.population.agents.iter().any(|other| {
                other.state.is_alive
                    && other.id != agent.id
                    && Self::within_reach(other.state.position, at)
            }),

            Targets::AnAnimal => !self
                .world
                .get_animals_in_radius((at.0, at.1), Self::WITHIN_REACH)
                .is_empty(),

            Targets::AFire => self.world.get_heat_source_at(at.0, at.1).is_some(),

            Targets::AStructure => self.world.get_building_at(&here).is_some(),

            // Water to act on: a river or a pool underfoot, or a spring on
            // this tile. Not what is in a skin - a verb wanting that says so
            // through `Wants::AVessel`, which is the hands half of the
            // question and already answered there.
            Targets::Water => {
                self.world
                    .grid
                    .get_tile(&here)
                    .is_some_and(|tile| tile.terrain.terrain_type == crate::world::TerrainType::Water)
                    || self.world.get_resource_at(&here).is_some_and(|node| {
                        node.resource_type == crate::world::ResourceType::Water && node.amount > 0
                    })
            }
        }
    }

    /// Every verb open here, paired with the thing it would be tried on.
    ///
    /// A verb alone is not what an agent learns about. "I have picked things
    /// up" is not a lesson; "I have picked up forty stones and never a strange
    /// fruit" is, and it is the second that decides whether the fruit is worth
    /// a look. So the candidates are verb-and-object pairs, named exactly as
    /// `Agent::what_was_tried` names them, which is what makes them askable of
    /// `Lessons` at all.
    ///
    /// The object comes from whatever the verb's target actually *is* here: a
    /// verb wanting a thing held is offered once for each kind of thing in the
    /// pack, one wanting a thing underfoot once for what is underfoot. Verbs
    /// whose target is not a kind of thing - a place, a person, the ground -
    /// come back once, keyed on the verb alone, because there is no kind for
    /// them to be about.
    pub fn what_i_could_try_here(&self, agent: &Agent) -> Vec<(&'static Verb, String)> {
        let at = agent.state.position;
        let here = Position::new(at.0, at.1);

        let in_the_pack: Vec<String> = agent.inventory.get_all_items().keys().cloned().collect();
        let mut underfoot: Vec<String> = self
            .world
            .what_is_lying_at(&here)
            .iter()
            .map(|dropped| dropped.item.item_id.clone())
            .collect();
        if let Some(node) = self.world.get_resource_at(&here).filter(|node| node.amount > 0) {
            if let Some(called) = Self::gathered_as(node.resource_type) {
                underfoot.push(called.to_string());
            }
        }
        underfoot.sort_unstable();
        underfoot.dedup();

        let mut out = Vec::new();
        for verb in self.what_i_could_do_here_now(agent) {
            // The handful of actions whose payload names what comes *out*
            // are not offered here at all.
            //
            // The object of a key is filled from the pack and the ground,
            // because that is what a verb with a `AThingHeld` or
            // `AThingUnderfoot` target acts on, and most actions agree:
            // `Dry { what }`, `Examine { what }` and `Work { to }` all name
            // the thing in hand. `Craft { item_type }` names the tool that is
            // to exist afterwards and `MakeClothing { garment }` names the
            // coat, so handing either an input reads a material as a product.
            // Measured over a hundred and twenty days of one settlement, that
            // was a failure all by itself: `Craft` went from 339 asked and
            // **none refused** to 1,563 asked and 1,181 refused, on "Unknown
            // recipe: iron", "Unknown recipe: wood" and "Unknown recipe:
            // flax", three raw materials nobody was ever trying to make.
            // `Build` was worse, because nothing refuses: its executor falls
            // through to a skin tent for any name it does not know, so
            // `build:iron` put a tent up and the lesson written afterwards
            // said that building an iron works.
            //
            // Filling them from the recipe book instead was the obvious
            // repair and was the wrong one: 1,913 asked and 1,512 refused,
            // now on "Cannot make fishingrod: short 1 lashing" and "Nobody
            // here knows how to make a handcart". Choosing *what to make* is
            // a question with an owner - `Agent::what_i_would_try_out`, a
            // rung above this one in the curiosity ladder, which asks what a
            // man knows how to make and whether there is a fire to hand.
            // Offering the whole book here duplicated that rung and did it
            // without either check.
            //
            // So this list is what it says it is: verbs applied to what is
            // actually here. What to make is asked one rung up.
            if matches!(
                verb.done_by,
                Some("craft") | Some("makeclothing") | Some("wearclothing") | Some("build")
            ) {
                continue;
            }

            let things: &[String] = match verb.targets {
                Targets::AThingHeld => &in_the_pack,
                Targets::AThingUnderfoot => &underfoot,
                _ => &[],
            };

            if things.is_empty() {
                out.push((verb, verb.called.to_string()));
                continue;
            }
            for what in things {
                out.push((verb, format!("{}:{}", verb.called, what)));
            }
        }
        out
    }

    /// The thing open here that this agent has tried least.
    ///
    /// "It should be the curious agents which try new things to satisfy their
    /// curiosity drive. Trying something new, even if it does not work, helps
    /// satisfy the drive."
    ///
    /// Novelty alone decides, on `Lessons::how_new_is_this` - one over one
    /// plus the number of times this verb has been tried on this kind of
    /// thing. Nothing here asks whether it worked, and that is the point: a
    /// man is not curious about a thing because it pays. Whether it paid is
    /// what the other drives read, off the same record.
    ///
    /// Ties are broken per person rather than by name, and that is not a
    /// detail. Everything untried scores exactly one, so at the start of a
    /// life almost every candidate ties - and breaking those ties on the key
    /// means picking whatever sorts first. Measured that way, over 2,767
    /// agent-days, a whole settlement reached for exactly two things: `ask
    /// about` 64% of the time and `attach` the rest, both of them near the
    /// front of the alphabet. Deterministic, and no kind of curiosity.
    ///
    /// So the tie is broken on the person *and* the thing together, which
    /// spreads a settlement across the things there are to try while keeping
    /// a seeded world repeatable: the same agent in the same state reaches for
    /// the same thing every run, and their neighbour reaches for another.
    pub fn what_i_have_tried_least_here(&self, agent: &Agent) -> Option<(&'static Verb, String)> {
        self.what_i_could_try_here(agent)
            .into_iter()
            // And only what can actually be carried out. `what_i_could_try_here`
            // reports everything the matrix offers, which is right for a
            // census and wrong for a choice: reaching for a verb that cannot
            // be built spends the turn on nothing and teaches nothing. The
            // whole of what this drops is the walking verbs, whose
            // destinations belong to the travel layer - see `an_action_for`.
            .filter(|(verb, key)| self.an_action_for(verb, key, agent).is_some())
            .max_by(|(_, left), (_, right)| {
                let mine = agent.lessons.how_new_is_this(left);
                let theirs = agent.lessons.how_new_is_this(right);
                mine.total_cmp(&theirs).then_with(|| {
                    Self::whose_turn_it_is(agent, left).cmp(&Self::whose_turn_it_is(agent, right))
                })
            })
    }

    /// A number this person would give this candidate, stable across runs.
    ///
    /// Written out rather than taken from `DefaultHasher`, whose values are
    /// explicitly not promised to be stable - and a seeded world that rolls a
    /// different number on a different build is no longer a baseline. FNV-1a,
    /// over the person's id and the thing they might try.
    fn whose_turn_it_is(agent: &Agent, key: &str) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in agent.id.as_bytes().iter().chain(key.as_bytes()) {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// Turn a chosen verb, and the thing it is to be tried on, into an action.
    ///
    /// The inverse of `Agent::what_was_tried`, and it has to be: that function
    /// names an action and this builds one back, so a drift between them would
    /// mean an agent choosing one thing and learning about another. The test
    /// `every_verb_the_matrix_performs_can_be_built` walks every verb and
    /// asserts the round trip - what comes back out of `what_was_tried` is of
    /// the family the matrix said performs it.
    ///
    /// **`Work` is why this is not a second closed list.** `what_was_tried`
    /// spells `Work { verb, to }` as `"{verb}:{to}"` - the verb is *data* in
    /// that action, not a variant - so any verb whose `done_by` names none of
    /// the particular actions below falls through to it and round-trips
    /// anyway. Twenty-odd of the matrix's verbs are worked that way already.
    /// Add a verb to the matrix tomorrow and it is reachable without a line
    /// here.
    ///
    /// What the key does not carry, this fills in from what is actually here:
    /// a hunt wants an animal and the key names none, a trade wants somebody
    /// and the key names nobody. Those are the targets that are not kinds of
    /// thing - see `what_was_tried` on why they stay out of the lesson key -
    /// and here they come from the ground rather than from the name.
    ///
    /// `None` where the verb wants something this place has not got, and for
    /// the handful whose action wants more than a target to be well formed.
    pub fn an_action_for(&self, verb: &Verb, key: &str, agent: &Agent) -> Option<Action> {
        let done_by = verb.done_by?;
        let what = key.split_once(':').map(|(_, thing)| thing.to_string());
        let at = agent.state.position;

        // A thing named in the key, for the verbs that want one. A verb that
        // wants a kind of thing and was handed no kind is not an action.
        let thing = || what.clone();

        // And the same, for the handful of actions whose payload names what
        // comes *out* rather than what is worked on.
        //
        // `what_i_could_try_here` fills the object of a key from the pack and
        // the ground, because that is what a verb with a `AThingHeld` or
        // `AThingUnderfoot` target acts on. Most actions agree: `Dry { what }`,
        // `Examine { what }` and `Work { to }` all name the thing in hand. But
        // `Craft { item_type }` names the tool that is to exist afterwards and
        // `MakeClothing { garment }` names the coat, so handing either of them
        // an input reads a material as a product. Measured over a hundred and
        // twenty days of one settlement, that was the whole of a new failure:
        // `Craft` went from 339 asked and **none refused** to 1,563 asked and
        // 1,181 refused, on "Unknown recipe: iron", "Unknown recipe: wood",
        // "Unknown recipe: flax" - three raw materials nobody was ever trying
        // to make.
        //
        // The predicates here are the executor's own - `every_way_to_make` is
        // what `crafting` looks the recipe up in, `garment_recipe` is what
        // `making_clothing` refuses on - so this cannot drift from what would
        // actually happen. Asking the question here rather than letting the
        // turn be spent is what `what_i_have_tried_least_here` says its filter
        // is for: "reaching for a verb that cannot be built spends the turn on
        // nothing and teaches nothing."
        let a_thing_that_can_be_made = || {
            let named = what.clone()?;
            let any = crate::environment::making::every_way_to_make(&named).next().is_some();
            any.then_some(named)
        };
        let a_garment = || {
            let named = what.clone()?;
            crate::agents::equipment::garment_recipe(&named).map(|_| named)
        };

        // And for the handful whose *target* is somebody but whose action
        // still wants a thing - sharing something, asking about something -
        // the thing comes off the agent's own back, because that is what it
        // would be. The key cannot name it: the verb targets a person, so
        // there is no kind in it. First by name, so a seeded world picks the
        // same one twice.
        let thing_or_carried = || {
            what.clone()
                .or_else(|| agent.inventory.get_all_items().keys().next().cloned())
        };

        Some(match done_by {
            // ---- the verbs that act on a kind of thing --------------------
            "gather" => Action::Gather { resource_type: thing()? },
            "eat" => Action::Eat { food_type: thing()? },
            "craft" => Action::Craft { item_type: a_thing_that_can_be_made()? },
            "cook" => Action::Cook { food_type: thing()? },
            "examine" => Action::Examine { what: thing()? },
            "equip" => Action::Equip { what: thing()? },
            "unequip" => Action::Unequip { what: thing()? },
            "dry" => Action::Dry { what: thing()? },
            "salt" => Action::Salt { what: thing()? },
            "cover" => Action::Cover { what: thing()? },
            "pickup" => Action::PickUp { what: thing()? },
            "putdown" => Action::PutDown { what: thing()? },
            "makeclothing" => Action::MakeClothing { garment: a_garment()? },
            "wearclothing" => Action::WearClothing { garment: a_garment()? },
            // One of them, because what a store wants is somewhere to put a
            // thing and this is about finding out whether it can be done at
            // all. How much is a question for the drive that means it.
            "store" => Action::Store { item_type: thing_or_carried()?, amount: 1 },

            // ---- the verbs that act on the ground here --------------------
            //
            // The same question as `craft` above, and worse, because nothing
            // refuses: `building` matches the name against its list and falls
            // through to a skin tent, so `build:iron` puts up a tent and the
            // lesson written afterwards says building an iron works. The
            // structure names are the ones that executor answers to; anything
            // else here is a material, not a roof, and is not an action.
            "build" => Action::Build {
                structure_type: match what.as_deref() {
                    None => "shelter".to_string(),
                    Some(named) if Self::A_ROOF_BY_NAME.contains(&named) => named.to_string(),
                    Some(_) => return None,
                },
                position: at,
            },
            // The matrix keeps digging yourself in apart from framing a tent,
            // and `what_was_tried` keeps the same distinction: a burrow is a
            // `Build` that is named for what it is.
            "burrow" => Action::Build {
                structure_type: "burrow".to_string(),
                position: at,
            },

            // ---- the verbs that want nothing but a body and a place -------
            "fish" => Action::Fish,
            "boil" => Action::Boil,
            "lightfire" => Action::LightFire,
            "tillsoil" => Action::TillSoil,
            "tendfield" => Action::TendField,
            "excavate" => Action::Excavate,
            "freeze" => Action::Freeze,
            "taste" => Action::Taste,
            "setsnare" => Action::SetSnare,
            "checksnares" => Action::CheckSnares,
            "spreadmuck" => Action::SpreadMuck,
            "takecutting" => Action::TakeCutting,
            "plantcutting" => Action::PlantCutting,
            "seekshelter" => Action::SeekShelter,

            // ---- the verbs that want a creature ---------------------------
            "hunt" => Action::Hunt {
                animal_id: self.an_animal_here(agent)?,
                weapon: agent.equipment.get_weapon().map(|held| held.name.clone()),
            },

            // ---- and the verbs that want somebody -------------------------
            "trade" => Action::Trade { with: self.somebody_here(agent)? },
            "giveto" => Action::GiveTo { to: self.somebody_here(agent)? },
            "takefrom" => Action::TakeFrom { from: self.somebody_here(agent)? },
            "gowithout" => Action::GoWithout { for_them: self.somebody_here(agent)? },
            "socialize" => Action::Socialize {
                target_agent_id: self.somebody_here(agent)?,
            },
            "shareinformation" => Action::ShareInformation {
                target_agent_id: self.somebody_here(agent)?,
            },
            "ask" => Action::AskAbout {
                who: self.somebody_here(agent)?,
                what: thing_or_carried()?,
            },
            "attack" => Action::Attack {
                target_agent_id: self.somebody_here(agent)?,
                weapon: agent.equipment.get_weapon().map(|held| held.name.clone()),
            },

            // ---- and the one kind this does not answer --------------------
            //
            // A verb that targets `APlace` wants a destination, and a
            // destination is not a kind of thing that could be named in the
            // key - it is the same exclusion `what_was_tried` makes for
            // people and places, and for the same reason. Where to walk is a
            // question with its own machinery behind it: the map memory, and
            // `Patterns::what_every_place_is_worth`. Answering it here with
            // whatever tile was nearest would be a second, worse, spelling of
            // it.
            "move" => return None,

            // ---- and everything else is a working -------------------------
            //
            // Not a fallback so much as the general case: `Work` carries its
            // verb as data, so scraping, carving, cutting, drilling and the
            // rest of the Disruption family are already done this way and
            // need no arm of their own. This is the line that keeps the
            // matrix, rather than this function, deciding what can be tried.
            _ => Action::Work {
                verb: done_by.to_string(),
                to: thing()?,
            },
        })
    }

    /// What `Simulation::building` answers to by name.
    ///
    /// Held here rather than derived because that executor's match is over
    /// string literals and has no list to read. It is a short one and it is
    /// checked: `every_roof_this_names_is_one_that_can_be_put_up` walks it
    /// against the executor so the two cannot drift apart quietly.
    /// Digging yourself in is not here. The matrix keeps a burrow apart from
    /// a framed tent and `what_was_tried` keeps the same distinction - it
    /// spells that `Build` as `burrow`, not `build` - so offering `frame` a
    /// burrow builds an action that answers to the wrong verb. `BURROW` has
    /// its own arm below and needs no name in the key.
    pub(crate) const A_ROOF_BY_NAME: [&'static str; 9] = [
        "tent", "skintent", "shelter", "smallhouse",
        "mediumhouse", "largehouse", "workshop", "storehouse", "farm",
    ];

    /// An animal near enough to act on, picked so that two runs of the same
    /// seed pick the same one.
    fn an_animal_here(&self, agent: &Agent) -> Option<uuid::Uuid> {
        let at = agent.state.position;
        self.world
            .get_animals_in_radius((at.0, at.1), Self::WITHIN_REACH)
            .into_iter()
            .map(|animal| animal.id)
            .min()
    }

    /// Somebody else near enough to act on, picked the same way.
    fn somebody_here(&self, agent: &Agent) -> Option<uuid::Uuid> {
        self.population
            .agents
            .iter()
            .filter(|other| {
                other.state.is_alive
                    && other.id != agent.id
                    && Self::within_reach(other.state.position, agent.state.position)
            })
            .map(|other| other.id)
            .min()
    }

    /// Whether two people are near enough for one to act on the other.
    fn within_reach(one: (i32, i32, i32), other: (i32, i32, i32)) -> bool {
        let paces = (one.0 - other.0).abs().max((one.1 - other.1).abs());
        paces as f32 <= Self::WITHIN_REACH
    }
}
