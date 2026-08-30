// src/analytics/wanting/quarry.rs
//! Hunting and fishing: going after something that would rather not be
//! caught.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::Action;

impl Simulation {
    /// Whether this agent should be taking on this animal at all.
    ///
    /// Anything that fights back is a job for someone with a weapon in hand.
    /// An unarmed agent that walks up to a bear is not hunting, it is dying.
    pub(in crate::analytics) fn worth_hunting(
        &self,
        agent: &crate::agents::Agent,
        animal: &crate::environment::Animal,
    ) -> bool {
        use crate::environment::AnimalBehavior;

        if !animal.is_alive() || animal.is_domesticated {
            return false;
        }

        let species = match self.world.animals.get_species(&animal.species_id) {
            Some(species) => species,
            None => return false,
        };

        // Nothing this one could not bring down with what it is carrying.
        // The executor has asked this since hunting was written and the
        // decision layer never did, so an agent walked to a deer it had no
        // means of killing and threw the turn away being refused. See
        // `Simulation::could_bring_it_down`.
        if !Self::could_bring_it_down(agent, species) {
            return false;
        }

        let dangerous = matches!(
            species.behavior,
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial
        );

        !dangerous || agent.equipment.get_weapon().is_some()
    }

    /// The nearest animal this agent could reasonably take, and where it is
    pub(in crate::analytics) fn nearest_prey(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(uuid::Uuid, (i32, i32))> {
        self.world
            .get_animals_in_radius(
                (agent_position.0, agent_position.1),
                Self::HUNT_SEARCH_RADIUS,
            )
            .into_iter()
            .filter(|animal| self.worth_hunting(agent, animal))
            .min_by_key(|animal| {
                (animal.position.0 - agent_position.0).abs()
                    + (animal.position.1 - agent_position.1).abs()
            })
            .map(|animal| (animal.id, animal.position))
    }

    /// Whether the agent has a reason to go after an animal.
    ///
    /// Two of them: nothing to eat, or nothing warm to wear and no skins to
    /// make it from. Fur and hides are the warm half of the garment table and
    /// the only way to them is off an animal.
    pub(in crate::analytics) fn wants_to_hunt(agent: &crate::agents::Agent) -> bool {
        // An agent hunts for skins, and the meat is a bonus.
        //
        // Hunting for the meat as such does not pay: berries and fish are
        // there for the taking and an animal has to be found, walked to and
        // hit. Agents that went after every animal because their pack was
        // empty starved for it, and two settlements in forty died out.
        //
        // It also keeps hunting until there are enough skins for the garment,
        // not until there is one skin: a fur coat takes five hides, and an
        // agent that stopped at the first came home with a single pelt over
        // and over and never wore anything warmer than woven flax.
        if !Self::wants_more_clothing(agent) {
            return false;
        }

        let quality = Self::expected_garment_quality(agent);

        let wants = crate::agents::equipment::GARMENT_RECIPES.iter().any(|recipe| {
            matches!(recipe.material_item, "hides" | "leather" | "wool")
                && Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
        });

        if !wants {
            return false;
        }

        // Stop once there is enough of anything to make one. An agent with a
        // pack full of hides has no business going after a sheep for the wool
        // it has never had.
        let can_already_make = crate::agents::equipment::GARMENT_RECIPES.iter().any(|recipe| {
            matches!(recipe.material_item, "hides" | "leather" | "wool")
                && Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
                && Self::can_spare_material(agent, recipe)
        });

        !can_already_make
    }

    /// Going after an animal: strike if it is within reach, close on it if not.
    ///
    /// Nothing in the simulation had ever selected `Action::Hunt` - the one
    /// place it appeared passed a nil animal id that the executor could not
    /// resolve - so no agent had ever hunted, and meat, hides and wool never
    /// reached an inventory at all.
    pub(in crate::analytics) fn hunting_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        if !Self::wants_to_hunt(agent) {
            return None;
        }

        // Somebody who has gone after animals a dozen times and come back
        // empty every time stops going after animals. Nothing tells them to:
        // it is what their own record says, and a hunter with a good record
        // keeps at it on the same evidence.
        if !agent.lessons.worth_trying(Undertaking::Hunting) {
            return None;
        }

        // Nothing to throw: the spear is the job.
        //
        // This is the preparation the model did not have. Wanting to hunt was
        // decided here, the walk to the animal was taken, and only when the
        // agent was standing over it did anything ask whether it had a spear -
        // at which point `make_what_this_wants` went looking for the makings
        // of one in whatever wood or meadow the deer happened to be standing
        // in, and did not find them. Measured over six worlds: 643 hunts
        // reached that question, 633 of them wanted a spear, and in 613 no
        // step in the chain could be taken from where the agent was.
        //
        // Wanting a thing you have not got is a reason to go and get it, and
        // the place to do that from is wherever the agent is when it forms the
        // want - beside the camp, where the stone and the wood are - not
        // beside the animal. So the answer to "I would hunt" from somebody
        // empty-handed is the next step towards a spear, and the errand
        // machinery carries it the rest of the way.
        if let Some(getting_one) = self.what_a_hunt_wants_first(agent) {
            return Some(getting_one);
        }

        let (animal_id, animal_position) = self.nearest_prey(agent, agent_position)?;

        let reach = (animal_position.0 - agent_position.0)
            .abs()
            .max((animal_position.1 - agent_position.1).abs());

        if reach <= Self::HUNT_REACH {
            return Some(Action::Hunt {
                animal_id,
                weapon: agent.equipment.get_weapon().map(|weapon| weapon.name.clone()),
            });
        }

        Some(Action::Move {
            target: (animal_position.0, animal_position.1, agent_position.2),
        })
    }

    /// Getting hold of something to hunt with, for somebody who means to hunt
    /// and has nothing.
    ///
    /// `None` when there is nothing to be done about it here - which is the
    /// honest answer for a man in a meadow with no stone in it, and leaves the
    /// hunt to be attempted with bare hands against whatever a thrown stone
    /// will still bring down.
    pub(in crate::analytics) fn what_a_hunt_wants_first(
        &self,
        agent: &crate::agents::Agent,
    ) -> Option<Action> {
        use crate::agents::skills::SkillType;

        if agent.what_i_have_to_work_with(SkillType::Hunting).is_some() {
            return None;
        }

        // The humblest thing that will do, not the best thing known.
        //
        // `what_i_would_rather_have` answers a different question - it is the
        // *upgrade*, and takes the highest `how_much_better` the agent knows
        // how to make. Asked by somebody with nothing at all it names the bow,
        // and a man who cannot come by a bow this afternoon then does nothing,
        // when a sharpened stick was three turns away. Measured with the best
        // one asked for: 1,881 wants of a hunting tool in six worlds and 340
        // that came to anything.
        //
        // So the ladder is walked from the bottom and the first rung this one
        // can actually get onto is the answer. Getting onto it is what makes
        // the next rung reachable later; `would_a_better_tool_pay` is what
        // climbs.
        let mut ladder: Vec<&'static crate::environment::making::Tool> =
            crate::environment::making::what_helps_with(SkillType::Hunting)
                .filter(|tool| agent.knows_how_to_make(tool.called))
                .collect();
        ladder.sort_by(|a, b| {
            a.how_much_better
                .partial_cmp(&b.how_much_better)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ladder
            .into_iter()
            .find_map(|tool| self.how_i_would_come_by(tool.called, agent))
    }

    /// How far a parent lets a child of its own get before going after it
    pub(in crate::analytics) const CHILD_LEASH: i32 = 8;

    /// How far an agent can work a reach from where it stands
    pub(in crate::analytics) const CAST: i32 = 1;

    /// How far an agent will walk to get to water
    pub(in crate::analytics) const WORTH_WALKING_TO_WATER: i32 = 14;

    /// A reach carrying this many fish is as good as fishing gets
    pub(in crate::analytics) const A_GOOD_REACH: f32 = 60.0;

    /// What comes out of the water on a cast that works
    pub(in crate::analytics) const FISH_PER_CAST: u32 = 2;

    /// How often a thrust tells in an empty reach, for somebody with nothing
    /// in his hands.
    ///
    /// Everything worth having is added to this: the thickness of the run, the
    /// hand, a rod, a spear. On its own it is a man standing in a river
    /// hoping.
    pub(in crate::analytics) const A_THRUST_THAT_TELLS: f32 = 0.15;

    /// What standing in the water costs, whether or not anything takes.
    pub(in crate::analytics) const WHAT_A_THRUST_COSTS: f32 = 8.0;

    /// What share of a fish is guts, heads and bone rather than meat.
    ///
    /// It goes to waste in the pack the moment the fish is caught, which is
    /// what puts a fishing agent in the way of doing a field good without ever
    /// meaning to.
    pub(in crate::analytics) const OFFAL_SHARE: f32 = 0.35;

    /// The reach an agent standing here can work, if there is one.
    pub(in crate::analytics) fn reach_within_cast(
        &self,
        agent_position: (i32, i32, i32),
    ) -> Option<crate::world::Position> {
        self.world
            .resources
            .iter()
            .filter(|resource| resource.resource_type.grows_in_water())
            .filter(|resource| resource.amount > 0)
            .map(|resource| resource.position)
            .find(|position| {
                (position.x - agent_position.0).abs() <= Self::CAST
                    && (position.y - agent_position.1).abs() <= Self::CAST
            })
    }

    /// Standing in a river after fish, and walking to a river worth standing in.
    ///
    /// A fishery is not another way of getting a meal. It is the only food a
    /// settlement can take that the land does not pay for, because a fish is
    /// grown at sea and comes up the river under its own power - so what is
    /// left of it, put on a field, makes the country richer rather than
    /// slower to run down. Everything else a settlement does with the ground
    /// is at best a return of what it already took.
    ///
    /// Nobody is told this. An agent fishes because it is hungry and there is
    /// water; the guts go into its pack as waste like anything else; and if it
    /// has learned that tipping the pack on a field does the ground good, the
    /// two habits meet on their own. What the agent keeps of that meeting is
    /// its own record of whether fishing pays - a person who stood in an empty
    /// winter river a dozen times stops going.
    pub(in crate::analytics) fn fishing_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        // Somebody who has stood in the water a dozen times and come out with
        // nothing stops going to the water.
        if !agent.lessons.worth_trying(Undertaking::Fishing) {
            return None;
        }

        // Fishing is what an agent does when it wants food or wants a store of
        // it. Both, in a settlement beside a river, most of the year.
        let hunger = agent
            .drives
            .get(DriveType::Hunger)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);
        let sustenance = agent
            .drives
            .get(DriveType::Sustenance)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);

        if hunger.max(sustenance) < Self::WORTH_GETTING_WET {
            return None;
        }

        // And nobody stands in a river for more fish than he will eat. This is
        // where it bites hardest: whole fish is the largest single thing that
        // goes off in anybody's pack in this model.
        if Self::more_food_than_he_will_get_through(agent) {
            return None;
        }

        if self.reach_within_cast(agent_position).is_some() {
            return Some(Action::Fish);
        }

        // Otherwise walk to the best water within reason: the thickest reach,
        // discounted by how far it is. A river in the run is worth crossing a
        // settlement for and an empty pool next door is not.
        let (best, _) = self
            .world
            .resources
            .iter()
            .filter(|resource| resource.resource_type.grows_in_water())
            .filter(|resource| resource.amount > 0)
            .filter_map(|resource| {
                let reach = (resource.position.x - agent_position.0)
                    .abs()
                    .max((resource.position.y - agent_position.1).abs());

                if reach > Self::WORTH_WALKING_TO_WATER {
                    return None;
                }

                let worth = resource.amount as f32 / (1.0 + reach as f32);
                Some((resource.position, worth))
            })
            .fold(
                None,
                |best: Option<(crate::world::Position, f32)>, (position, worth)| {
                    match best {
                        Some((_, best_worth)) if best_worth >= worth => best,
                        _ => Some((position, worth)),
                    }
                },
            )?;

        Some(Action::Move {
            target: (best.x, best.y, agent_position.2),
        })
    }

    /// How much an agent has to want food before it will go and stand in a river
    pub(in crate::analytics) const WORTH_GETTING_WET: f32 = 0.35;
}
