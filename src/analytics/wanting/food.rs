// src/analytics/wanting/food.rs
//! Hunger and thirst, which are the two that kill.
//!
//! What a body wants when it is empty, where it remembers water being, which
//! patch is worth the walk, and whether a thing is worth putting on a fire
//! first.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::agents::physiology;
use crate::core::DriveType;
use crate::environment::Action;

impl Simulation {
    /// Action an agent needs to take to stay alive, if any.
    ///
    /// Nothing else in the decision pipeline satisfies hunger or exhaustion:
    /// drives sit below goals and percepts, so a long-running goal (stocking a
    /// house with food, say) or a steady stream of resource percepts keeps
    /// winning the tie until the agent starves holding a full inventory.
    ///
    /// With `critical_only` this reports only what an agent already dying of
    /// hunger must do, which is the one thing urgent enough to outrank fleeing
    /// a threat. Fear can stay pinned for hundreds of ticks with no attacker
    /// left to run from, and an agent that flees until it starves has not
    /// survived either.
    pub(in crate::analytics) fn survival_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        critical_only: bool,
    ) -> Option<Action> {
        let thirsty = agent
            .drives
            .get(DriveType::Thirst)
            .map(|thirst| thirst.is_active())
            .unwrap_or(false);
        let dehydrated = agent.state.is_dehydrated();

        let hungry = agent
            .drives
            .get(DriveType::Hunger)
            .map(|hunger| hunger.is_active())
            .unwrap_or(false);
        let starving = agent.state.is_starving() || agent.nutrition.is_starving();

        if critical_only && !(starving || dehydrated) {
            return None;
        }

        // Water before food: thirst kills in about three days here where
        // hunger takes seven, so a parched agent drinks first.
        if thirsty || dehydrated {
            if let Some(action) = self.water_action(agent, agent_position, dehydrated) {
                return Some(action);
            }
        }

        if hungry || starving {
            if let Some(action) = self.food_action(agent, agent_position, starving) {
                return Some(action);
            }
        }

        // Collapse-level fatigue takes precedence over everything but hunger
        if agent.fatigue.desperately_needs_sleep() && !agent.fatigue.is_sleeping {
            return Some(Action::Sleep { duration: 10 });
        }

        None
    }

    /// How a thirsty agent gets a drink, if it can.
    ///
    /// `desperate` marks an agent far enough gone that finding water is worth
    /// abandoning whatever else it was doing for.
    pub(in crate::analytics) fn water_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        desperate: bool,
    ) -> Option<Action> {
        use crate::agents::senses::ScentType;
        use crate::core::memory::SpatialMemoryType;
        use crate::world::ResourceType;

        // A drink from the waterskin, or from a spring within reach. Both go
        // through the same action: it prefers open water and falls back to
        // whatever the agent is carrying.
        // Enough for a swallow, not a dribble. `Gather` drinks from a
        // waterskin when there is no source about, but only in whole units -
        // so an agent with half a mouthful left kept choosing to drink and
        // being told there was no water anywhere, which was the largest single
        // failure in the simulation.
        let carrying_water = agent.inventory.available_water() >= 1.0;

        // Not the sea. Everybody knows better than to drink out of the sea -
        // this is not a discovery, a mouthful tells you what it is - and
        // everybody stops knowing better once they are dying of thirst, which
        // is exactly how people have always come to do it.
        let would_drink_the_sea = agent.would_i_drink_the_sea();
        let drinkable = |resource: &crate::world::ResourceNode| {
            if resource.resource_type != ResourceType::Water {
                return false;
            }

            // A spring that has given what it has this hour is not somewhere
            // to go for a drink. It is still a spring and it will be running
            // again shortly - see `ResourceNode::what_can_be_taken` - but
            // walking to it now buys a refusal, and walking to the next one
            // over buys water. Without this the flow model put the failure
            // rate up by a fifth on its own.
            if resource.what_can_be_taken() == 0 {
                return false;
            }

            if would_drink_the_sea {
                return true;
            }
            !self
                .world
                .grid
                .get_tile(&resource.position)
                .is_some_and(|tile| tile.terrain.is_the_water_salt())
        };

        let water_in_reach = self
            .nearest_resource_within(agent_position, Self::FORAGE_RADIUS, drinkable)
            .is_some();

        if carrying_water || water_in_reach {
            return Some(Action::Gather { resource_type: "water".to_string() });
        }

        // Otherwise head for water the agent can smell or remembers
        if let Some(target) = self.known_source_position(
            agent,
            agent_position,
            ScentType::Water,
            SpatialMemoryType::Water,
        ) {
            let distance =
                (target.0 - agent_position.0).abs() + (target.1 - agent_position.1).abs();

            if distance > 1 {
                return Some(Action::Move { target });
            }

            return Some(Action::Gather { resource_type: "water".to_string() });
        }

        // Nowhere known to drink by sight or smell, but somewhere that has
        // answered this before.
        //
        // This was aimed at `Gather: No water sources nearby`, the largest
        // single failure in the simulation, on the theory that nothing joined
        // the drink an agent had yesterday to the bank it drank from. Measured
        // over eight worlds a side it did not move that failure at all - the
        // rate is 3.7% of all actions without this and 4.7% with it, which is
        // noise - so whatever is producing those refusals is not an agent
        // being unable to remember where water is. See ISSUES_FOUND #2. It is
        // kept because it is the answer of last resort before striking out
        // blind, and because it costs nothing.
        if let Some(there) =
            agent.somewhere_that_answered(DriveType::Thirst, agent_position, self.current_tick)
        {
            return Some(Action::Move { target: there });
        }

        // Nowhere known to drink: go looking, if it has come to that
        if desperate {
            return Some(Self::search_leg(agent, agent_position, self.current_tick));
        }

        None
    }

    /// How a hungry agent gets a meal, if it can.
    ///
    /// `desperate` marks an agent starving badly enough that finding food is
    /// worth abandoning whatever else it was doing for.
    pub(in crate::analytics) fn food_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        desperate: bool,
    ) -> Option<Action> {
        use crate::agents::senses::ScentType;
        use crate::core::memory::SpatialMemoryType;

        let carrying_food = agent.has_edible_food();

        // What is in the pack in autumn is not necessarily supper.
        //
        // This is the provisioning gap, and it is why forty pits a world got
        // dug and none of them ever had anything in it. Nothing in this model
        // gathered *for the winter*: it gathered because it was hungry, ate
        // what it picked in the same breath, and put away whatever happened
        // to be left over. Probed directly in autumn, only 108 agent-samples
        // in 3,254 were carrying any food at all - three in a hundred - so
        // there was never a load to carry home.
        let putting_by = self.is_this_lot_for_the_store(agent, agent_position);

        // A carcass has to come apart before any of the rest of this means
        // anything. This sits above cooking and above eating because it is
        // the step that makes either possible: a man with a deer over his
        // shoulder and nothing cut is a man with no food at all, and before
        // this he simply ate the deer.
        if let Some((verb, to)) = agent.what_flesh_i_should_cut_up() {
            return Some(Action::Work { verb, to });
        }

        // A fire right here turns a third of what is in raw meat into nearly
        // all of it, so one tick spent cooking buys back several meals' worth.
        // Not when starving: then the difference between a poor meal now and a
        // good one next tick is the difference between eating and dying. And
        // not on a harvest, because cooking a thing stops it being dried, and
        // drying is worth twenty times what cooking is.
        if !desperate
            && !putting_by
            && agent.state.years_old() >= Self::OLD_ENOUGH_TO_COOK
            && Self::has_food_worth_cooking(agent)
            && self
                .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
                .is_some()
        {
            return Some(Action::Cook {
                food_type: "generic".to_string(),
            });
        }

        // Eat what we carry as soon as we are hungry; an agent that walks
        // around starving with a full pack is the bug this guards against.
        // Unless this lot is for the store, in which case being a bit hungry
        // is the price of eating in February.
        if carrying_food && !putting_by {
            return Some(Action::Eat { food_type: "generic".to_string() });
        }

        // Anything edible within foraging reach can simply be eaten
        if self
            .nearest_edible_within(agent_position, Self::FORAGE_RADIUS)
            .is_some()
        {
            return Some(Action::Eat { food_type: "generic".to_string() });
        }

        // An animal standing right here, when the nearest food anybody knows
        // of is a walk away.
        //
        // Hunting had been put behind everything: behind eating what you
        // carry, behind foraging, behind walking to a known patch, behind
        // moving the whole camp, behind walking back to ground that fed you
        // once - and then behind being *desperate* on top of all that. It was
        // never reached. Measured, forty agents in forty-seven still believed
        // hunting paid and none of them had done any, which is what a belief
        // with nothing to update it looks like.
        //
        // The rule that makes sense of it: a deer at your feet beats a berry
        // patch twelve tiles off. Not a deer across the valley - that is the
        // expedition that does not pay and never did - and not while there is
        // something to pick up where you stand, which is the branch above.
        //
        // The learning gate is the existing one: somebody who has thrown at
        // six animals and hit none stops throwing.
        // And nobody throws at a deer with a pack already fuller of food than
        // he will get through, which is what a hunt that pays actually means.
        if !putting_by
            && !Self::more_food_than_he_will_get_through(agent)
            && agent.lessons.will_try_this_again("hunt")
        {
            if let Some((animal_id, animal_position)) = self.nearest_prey(agent, agent_position) {
                let reach = (animal_position.0 - agent_position.0)
                    .abs()
                    .max((animal_position.1 - agent_position.1).abs());

                if reach <= Self::AS_NEAR_AS_PREY_HAS_TO_BE_TO_BOTHER {
                    if reach <= Self::HUNT_REACH {
                        return Some(Action::Hunt {
                            animal_id,
                            weapon: agent
                                .equipment
                                .get_weapon()
                                .map(|weapon| weapon.name.clone()),
                        });
                    }

                    return Some(Action::Move {
                        target: (animal_position.0, animal_position.1, agent_position.2),
                    });
                }
            }
        }

        // Otherwise head for the closest source the agent knows of - so long
        // as it is one that could still come to something.
        //
        // This branch is the reason ISSUES #229 never fired. Measured over six
        // worlds, an agent that had been hungry long enough to give up on the
        // country it was standing in took this branch in **69% of those
        // ticks**, and the place it was sent to had nothing standing on it in
        // **99.6%** of them - a patch a pace and a half away, picked bare, that
        // it walked back to every tick until it died. The gather that came out
        // of it was refused by `could_this_gather_come_to_anything` every
        // single time, so the turn bought nothing and the branches below -
        // moving camp, and leaving altogether - were reached five times in
        // eleven hundred.
        //
        // The check is the settlement's own, and asks nothing the agent does
        // not know: is there anything within foraging reach that this one has
        // not already picked out. When the answer is no, somewhere within that
        // reach is not somewhere to go, and the tick falls through to the
        // question of whether to live here at all. A source further off than
        // foraging reach is outside what that check looked at, so it still
        // stands.
        let paces = |target: &(i32, i32, i32)| {
            (target.0 - agent_position.0).abs() + (target.1 - agent_position.1).abs()
        };

        let known = self
            .known_source_position(agent, agent_position, ScentType::Food, SpatialMemoryType::Food)
            .filter(|target| {
                paces(target) as u32 > Self::FORAGE_RADIUS
                    || self.could_this_gather_come_to_anything(agent, agent_position, "food")
            });

        if let Some(target) = known {
            let distance = paces(&target);

            // Walk to food we know about before trying to pick anything up -
            // and not at all with more about him than he will get through.
            // A man with a pack of berries going over does not want another
            // hedge, he wants to eat what he has or put it somewhere it keeps.
            if Self::more_food_than_he_will_get_through(agent) {
                return None;
            }

            if distance > 1 {
                return Some(Action::Move { target });
            }

            return Some(Action::Gather { resource_type: "food".to_string() });
        }

        // Hungry for long enough, with the country round about picked bare:
        // go somewhere else. This is above the local search below because
        // walking twelve tiles and back is what an agent does when it has
        // mislaid its dinner, not when the ground has stopped producing one.
        // A need that keeps going unanswered is a reason to live somewhere
        // else, not a reason to walk further today
        if let Some(action) = self.go_and_live_where_it_is(agent, agent_position) {
            return Some(action);
        }

        if let Some(action) = self.migration_action(agent, agent_position) {
            return Some(action);
        }

        // Ground that has fed this agent before, when nothing nearer will.
        if let Some(there) =
            agent.somewhere_that_answered(DriveType::Hunger, agent_position, self.current_tick)
        {
            return Some(Action::Move { target: there });
        }

        // Starving with nowhere known to go: search rather than stand still
        // and wait to die. Agents that are merely hungry let the tick go to
        // whatever comes next - sheltering from the cold, a plan, a goal -
        // because gathering thin air on the spot accomplishes nothing and
        // blocks everything they could usefully be doing.
        if desperate {
            // An animal is food. Hunting does not pay against berries and
            // fish, which is why an agent does not do it for the meat as a
            // rule - but an agent with nothing else left is a different case.
            if let Some((animal_id, animal_position)) =
                self.nearest_prey(agent, agent_position)
            {
                let reach = (animal_position.0 - agent_position.0)
                    .abs()
                    .max((animal_position.1 - agent_position.1).abs());

                if reach <= Self::HUNT_REACH {
                    return Some(Action::Hunt {
                        animal_id,
                        weapon: agent
                            .equipment
                            .get_weapon()
                            .map(|weapon| weapon.name.clone()),
                    });
                }

                return Some(Action::Move {
                    target: (animal_position.0, animal_position.1, agent_position.2),
                });
            }

            return Some(Self::search_leg(agent, agent_position, self.current_tick));
        }

        None
    }

    /// A place the agent knows to look: what it can smell right now, falling
    /// back to the nearest place it remembers.
    ///
    /// Scent wins because it is current, where a memory may be of a patch
    /// already eaten bare. Scent also carries by straight-line distance while
    /// walking is counted in steps, so what an agent smells can still be a
    /// journey away - which is why this reports somewhere to go rather than
    /// somewhere to reach for.
    pub(in crate::analytics) fn known_source_position(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        scent_type: crate::agents::senses::ScentType,
        memory_type: crate::core::memory::SpatialMemoryType,
    ) -> Option<(i32, i32, i32)> {
        let walking_distance = |candidate: &(i32, i32, i32)| {
            (candidate.0 - agent_position.0).abs() + (candidate.1 - agent_position.1).abs()
        };

        let smelled = agent
            .senses
            .smell
            .get_scents_by_type(scent_type)
            .into_iter()
            .map(|scent| scent.source_position)
            .min_by_key(walking_distance);

        smelled.or_else(|| {
            agent
                .memory
                .recall_locations(memory_type)
                .into_iter()
                .map(|memory| memory.position)
                .min_by_key(walking_distance)
        })
    }

    /// One leg of a search for something the agent cannot find nearby.
    ///
    /// The heading holds for a stretch of ticks: re-rolling it every tick
    /// produces a random walk that barely leaves the spot it started from, so
    /// an agent would jitter in place while what it needed sat just outside
    /// its range. It varies per agent and per leg, so agents setting out from
    /// the same place fan out instead of marching together.
    /// How long hunger has to go unanswered before an agent gives up on the
    /// country it is standing in.
    ///
    /// Ten days of the world's calendar of being hungry and not being fed.
    /// Short enough that a settlement whose ground has stopped producing does
    /// something about it; long enough that a bad afternoon, a hard winter or
    /// one picked-over hedgerow does not empty a village.
    pub(in crate::analytics) const HUNGRY_ENOUGH_TO_LEAVE: u32 = 120;

    /// What a man expects of the country he is standing in, in the order that
    /// running out of it kills him.
    ///
    /// Both of these are reasons to walk away from a place, and only the
    /// second was ever treated as one.
    pub(in crate::analytics) const WHAT_A_COUNTRY_HAS_TO_PROVIDE: [DriveType; 2] = [DriveType::Thirst, DriveType::Hunger];

    /// How far off counts as somewhere else rather than the next field over.
    pub(in crate::analytics) const FAR_ENOUGH_TO_BE_WORTH_THE_WALK: i32 = 20;

    /// Leaving: what an agent does when the ground where it lives has stopped
    /// feeding it.
    ///
    /// Nobody decides this on the settlement's behalf. It falls out of the
    /// drive: hunger that keeps being denied presses harder every tick it
    /// waits, and past a certain point the agent stops working the fields it
    /// has and walks. Where it walks to is the best thing it can remember
    /// that is far enough away to be different country; failing any memory,
    /// it strikes out on a bearing of its own and keeps going, which is what
    /// turns a starving settlement into a scattering one.

    /// How long a need has to keep going unanswered before an agent stops
    /// walking back and forth to it and goes to live beside it.
    pub(in crate::analytics) const ASKED_FOR_IT_ONCE_TOO_OFTEN: u32 = 96;

    /// How near counts as camped on a thing.
    ///
    /// Wide enough that a people spread out along a river rather than piling
    /// onto the one tile of it. At four tiles they concentrated hard enough to
    /// work the ground out under themselves: the nutrient-loop regression,
    /// which asks that farmed ground not lose half its fertility in ten
    /// thousand ticks, started failing about one run in three.
    pub(in crate::analytics) const CAMPED_ON_IT: i32 = 4;

    /// How much of what is in the pack is something to eat.
    pub(in crate::analytics) fn how_much_food_is_in_the_pack(agent: &crate::agents::Agent) -> u32 {
        agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.is_food())
            .map(|item| item.quantity)
            .sum()
    }

    /// How much somebody picks before they stop picking and carry it home.
    ///
    /// Small enough that a trip is a few turns rather than a season, and
    /// comfortably above what `Cover` keeps back, so that every trip actually
    /// puts something in the ground.
    pub(in crate::analytics) const WHAT_A_HARVEST_TRIP_IS: u32 = 10;

    /// How hard hunger has to be pressing before somebody stops filling the
    /// store and eats what is in their hand.
    ///
    /// Well short of desperation. A person puts food by while they are
    /// comfortable, not while they are going short - and a settlement that
    /// provisions right up to the edge of starving buries more people than it
    /// saves.
    pub(in crate::analytics) const WHAT_HUNGER_STOPS_A_HARVEST: f32 = 0.5;

    /// How hard hunger is pressing on this one, on the scale a drive is
    /// weighed on.
    pub(in crate::analytics) fn how_hungry_is_this_one(agent: &crate::agents::Agent) -> f32 {
        agent
            .drives
            .get(DriveType::Hunger)
            .map(|hunger| hunger.urgency())
            .unwrap_or(0.0)
    }

    /// The nearest fire within reach, and where it is.
    ///
    /// With `lit_only` this reports only a fire that is actually burning; with
    /// it false, a cold hearth counts too, which is what relighting looks for.
    pub(in crate::analytics) fn nearest_fire_from(
        &self,
        position: (i32, i32, i32),
        reach: i32,
        lit_only: bool,
    ) -> Option<(uuid::Uuid, (i32, i32, i32))> {
        self.world
            .heat_sources
            .all()
            .into_iter()
            .filter(|fire| !lit_only || fire.is_lit)
            .filter(|fire| {
                (fire.position.0 - position.0).abs() <= reach
                    && (fire.position.1 - position.1).abs() <= reach
            })
            .min_by_key(|fire| {
                (fire.position.0 - position.0).abs() + (fire.position.1 - position.1).abs()
            })
            .map(|fire| (fire.id, fire.position))
    }

    /// What the agent would put on a fire, if anything.
    ///
    /// A named food is taken at its word - cook a berry if you insist, and
    /// lose it. Asked for anything, an agent picks something a fire actually
    /// improves, because nobody sets out to burn their dinner.
    pub(in crate::analytics) fn cookable_item(agent: &crate::agents::Agent, food_type: &str) -> Option<String> {
        use crate::world::nutrition::CookingOutcome;

        let named = !food_type.is_empty() && food_type != "generic";
        if named {
            return agent
                .inventory
                .get_item(food_type)
                .filter(|item| item.quantity > 0)
                .filter(|item| {
                    crate::world::nutrition::Piece::of(&item.item_id).can_it_be_cooked()
                })
                .map(|item| item.item_id.clone());
        }

        agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .filter(|item| {
                item.food_data
                    .as_ref()
                    .map(|food| food.preparation == crate::world::nutrition::PreparationState::Raw)
                    .unwrap_or(true)
            })
            .filter(|item| {
                crate::agents::storage_integration::id_to_item_type(&item.item_id)
                    .map(|item_type| item_type.cooking_outcome() == CookingOutcome::Improves)
                    .unwrap_or(false)
            })
            // A whole beast does not go over a fire. What happens if you try
            // is that the outside chars and the inside stays raw, which is
            // the same thing as not having cooked it.
            .filter(|item| {
                crate::world::nutrition::Piece::of(&item.item_id).can_it_be_cooked()
            })
            .map(|item| item.item_id.clone())
            .min()
    }

    /// Whether the agent is carrying something a fire would improve
    pub(in crate::analytics) fn has_food_worth_cooking(agent: &crate::agents::Agent) -> bool {
        Self::cookable_item(agent, "generic").is_some()
    }

    /// How far an agent will walk to reach a fire that is already burning
    pub(in crate::analytics) const FIRE_WALK_RADIUS: i32 = 20;

    /// How much warmer a garment has to be before it is worth changing into.
    ///
    /// Without a margin an agent swaps between two near-identical coats every
    /// tick forever: whatever it is wearing wears down a little each tick, so
    /// the one folded in its pack is always fractionally better.
    pub(in crate::analytics) const WARMTH_WORTH_CHANGING_FOR: f32 = 0.05;

    /// How much better a new garment has to be before it is worth the material
    /// and the work of making one.
    ///
    /// Whatever is on an agent's back wears a little thinner every tick, so
    /// against a bare comparison there is always a fresh coat worth making:
    /// agents replaced their clothes every few hundred ticks and ended up
    /// carrying dozens of cast-offs. A quarter better means a real
    /// improvement - a better material, or a hand that has learned something -
    /// rather than ordinary wear.
    pub(in crate::analytics) const WORTH_MAKING_ANEW: f32 = 1.25;

    /// Getting raw food onto a fire, in whatever order the situation needs:
    /// cook here, walk to the fire, light one, or go and cut the wood for it.
    ///
    /// Cooking is worth the trouble - raw meat gives up about a third of what
    /// is in it, cooked meat nearly all of it - but only for food a fire
    /// improves, so an agent carrying nothing but berries never lights one.
    pub(in crate::analytics) fn cooking_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        if !Self::has_food_worth_cooking(agent) {
            return None;
        }

        // Standing at a fire that is burning: put the food on it, if this one
        // is old enough to be trusted with it
        if agent.state.years_old() >= Self::OLD_ENOUGH_TO_COOK
            && self
                .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
                .is_some()
        {
            return Some(Action::Cook {
                food_type: "generic".to_string(),
            });
        }

        // A fire burning within walking distance is worth the walk
        if let Some((_, position)) =
            self.nearest_fire_from(agent_position, Self::FIRE_WALK_RADIUS, true)
        {
            return Some(Action::Move { target: position });
        }

        // A cold hearth in reach costs only the fuel to bring back to life
        let relightable = self
            .nearest_fire_from(agent_position, Self::FIRE_REACH, false)
            .is_some();
        let wood_needed = if relightable {
            Self::FIRE_FUEL_WOOD
        } else {
            Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD
        };

        if agent.inventory.has_item("wood", wood_needed) {
            return Some(Action::LightFire);
        }

        // No wood, but trees within reach: fetch some
        if self
            .nearest_resource_within(agent_position, Self::FORAGE_RADIUS, |resource| {
                resource.resource_type == crate::world::ResourceType::Wood
            })
            .is_some()
        {
            return Some(Action::Gather {
                resource_type: "wood".to_string(),
            });
        }

        None
    }

    /// Position of the closest edible resource within `radius` walking steps
    pub(in crate::analytics) fn nearest_edible_within(
        &self,
        position: (i32, i32, i32),
        radius: u32,
    ) -> Option<crate::world::Position> {
        self.nearest_resource_within(position, radius, |resource| {
            Self::edible_item_for(resource.resource_type).is_some()
        })
    }

    /// The nearest thing to eat that this particular agent would actually
    /// walk to.
    ///
    /// The same question as `nearest_edible_within` asked of somebody with a
    /// memory. A patch in a wood where this one saw wolves last month is
    /// further away than it looks, and a patch it has no bad history with is
    /// nearer - which is the whole use of a map that has danger on it.
    ///
    /// Falls back to the plain answer when everything within reach is bad
    /// ground: hunger outlasts a fright, and a settlement that starves rather
    /// than walk past a wood is not being careful.
    pub(in crate::analytics) fn nearest_edible_this_one_would_go_to(
        &self,
        agent: &crate::agents::Agent,
        position: (i32, i32, i32),
        radius: u32,
    ) -> Option<crate::world::Position> {
        use crate::world::Position;

        let here = Position::new(position.0, position.1);
        let now = self.current_tick;

        let best = self
            .world
            .resources
            .iter()
            .filter(|resource| resource.amount > 0)
            .filter(|resource| Self::edible_item_for(resource.resource_type).is_some())
            .filter(|resource| here.distance_to(&resource.position) <= radius)
            // Nor ground this one stripped itself and has no reason to think
            // has grown back. A patch picked bare in June is bearing again by
            // September, so this fades - but until it does, walking back every
            // morning is the single commonest wasted turn in the model.
            .filter(|resource| {
                !agent
                    .exploration_knowledge
                    .is_it_picked_out(resource.position, now)
            })
            .min_by_key(|resource| {
                let apart = here.distance_to(&resource.position) as f32;
                let bad = agent
                    .exploration_knowledge
                    .how_bad_is_it_there(resource.position, now);

                // What the walk feels like, rather than what it measures.
                ((apart + bad * Self::WHAT_A_BAD_PLACE_ADDS_TO_A_WALK) * 100.0) as u32
            })
            .map(|resource| resource.position);

        best.or_else(|| self.nearest_edible_within(position, radius))
    }

    /// How much further away a place feels for having gone badly.
    ///
    /// A remembered mauling puts twelve paces on a walk, at full strength and
    /// fading with the memory. Enough to send somebody to the next patch when
    /// there is one, not enough to keep them out of the only wood there is.
    pub(in crate::analytics) const WHAT_A_BAD_PLACE_ADDS_TO_A_WALK: f32 = 12.0;

    /// Resource types an agent can eat straight from the land, paired with the
    /// inventory item they correspond to.
    ///
    /// Foraging accepts everything that smells of food, so an agent does not
    /// starve standing in a grain field because only berries counted as edible.
    pub(in crate::analytics) fn edible_resources() -> [(crate::world::ResourceType, crate::world::ItemType); 8] {
        use crate::world::{ItemType, ResourceType};

        [
            (ResourceType::Food, ItemType::Food),
            (ResourceType::Grain, ItemType::Grain),
            // What there is before anything ripens, which for half the year
            // is the whole of what there is
            (ResourceType::Greens, ItemType::Greens),
            (ResourceType::Roots, ItemType::Roots),
            // The mast, and the best thing in the wood while it is down
            (ResourceType::Nuts, ItemType::Nuts),
            // And the pod crop, which is food and rent at once
            (ResourceType::Legumes, ItemType::Legumes),
            (ResourceType::Fish, ItemType::Fish),
            (ResourceType::Meat, ItemType::Meat),
        ]
    }

    /// The most one pair of hands takes off a patch in one trip.
    ///
    /// Owned by `provision`, because what a pack has to have room for is
    /// exactly this and the pack is not part of the decision layer.
    pub(in crate::analytics) const AS_MUCH_AS_ONE_TRIP_TAKES: f32 =
        crate::agents::provision::AS_MUCH_AS_ONE_TRIP_TAKES;

    /// How much of a turn one pace of walking comes to.
    ///
    /// A `Move` action is one tile, so a patch twenty paces off is twenty
    /// turns of walking each way - most of two days - however cheap the
    /// picking is at the end of it. Counted both ways.
    pub(in crate::analytics) const TURNS_A_PACE_TAKES: f32 = 2.0;

    /// What a trip to this patch is worth, per unit of effort spent on it.
    ///
    /// Three things, where there used to be one.
    ///
    /// **What is standing there.** "Why would they go to a berry bush with a
    /// single berry if there is another berry bush with 100 berries?" They
    /// would not, and they were: both food-choosers looked only at what kind of
    /// food it was and how far off it stood, so a patch stripped to its last
    /// berry read exactly as well as a full one at the same distance and the
    /// nearer of the two won. Capped at what one trip can carry off, because
    /// past that a bigger patch is not a better morning.
    ///
    /// **What it is worth to eat.** A unit of fish is twenty-five energy and a
    /// unit of leaf is six, so a fish patch is worth four trips to a leaf one -
    /// which is what makes a people walk past the hedge at the door to get to
    /// the river.
    ///
    /// **What the trip costs.** The walk both ways plus the work of getting
    /// that particular food out of the ground, which
    /// `provision::what_foraging_costs` already prices.
    pub(in crate::analytics) fn what_this_patch_is_worth(energy: f32, standing: u32, paces: u32, costs: f32) -> f32 {
        let carried_off = (standing as f32).min(Self::AS_MUCH_AS_ONE_TRIP_TAKES);
        let brings_back = carried_off
            * physiology::UNITS_IN_ONE_ITEM
            * physiology::what_a_unit_of_this_is_worth(energy);

        // What is left after paying for the trip, over the time the trip takes.
        //
        // "The agents should be optimizing their actions by heading to food
        // sources which will provide the energy needed in the time they need
        // it." Both halves of that. A ratio of worth to *effort* answers the
        // wrong question - it picks the cheapest trip, so a handful of leaf
        // underfoot beats a river full of fish twenty paces off and starves
        // the man who takes it. Net energy alone answers the other wrong
        // question - it ignores that a walk is paid for in turns as well as in
        // sweat, and a step is a turn, so a patch twenty paces off is most of
        // two days there and back.
        //
        // So: what the trip is worth, less what it costs, per turn it takes.
        let turns = 1.0 + paces as f32 * Self::TURNS_A_PACE_TAKES;
        (brings_back - costs) / turns
    }

    /// The inventory item a resource yields when eaten, if it is edible at all
    ///
    /// `ResourceType::is_edible` is the authority on whether something counts
    /// as food; this only says what it turns into in a pack.
    pub(in crate::analytics) fn edible_item_for(resource: crate::world::ResourceType) -> Option<crate::world::ItemType> {
        if !resource.is_edible() {
            return None;
        }

        Self::edible_resources()
            .into_iter()
            .find(|(resource_type, _)| *resource_type == resource)
            .map(|(_, item_type)| item_type)
    }
}
