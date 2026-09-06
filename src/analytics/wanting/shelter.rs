// src/analytics/wanting/shelter.rs
//! Keeping warm and dry.
//!
//! A roof, a hole in the ground, and something to wear - and the arithmetic
//! of whether what is worn is already enough.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::environment::Action;

impl Simulation {
    /// Whether the agent is bare enough, and pressed enough, to want another
    /// layer.
    ///
    /// **The Shelter drive decides this, not the thermometer.** It used to be
    /// "am I cold at this instant, and am I bare", which made the drive a
    /// router rather than a cause: `DriveType::Shelter` sent the tick here and
    /// this line sent it straight back unless the body was already below its
    /// ideal. Nobody made a coat in autumn, because in autumn nobody is
    /// shivering yet - and the coat that is wanted in December has to be cut,
    /// carried and sewn in October.
    ///
    /// Being cold now still counts, and counts on its own: a body under its
    /// ideal wants a layer whatever a drive says about it. What is new is the
    /// second way in - a Shelter drive over its own threshold, which is what
    /// the drive layer means by "this one is exposed and it matters", and
    /// which rises on the weather and the season rather than on the current
    /// reading of one body.
    ///
    /// Bareness is the same test as before and is the harder half: somebody
    /// already carrying `ENOUGH_INSULATION` wants nothing more however cold it
    /// is or however hard the drive presses, which is what stops a settlement
    /// spending the winter sewing.
    pub(in crate::analytics) fn wants_more_clothing(agent: &crate::agents::Agent) -> bool {
        if agent.body.total_cold_insulation() >= Self::ENOUGH_INSULATION {
            return false;
        }

        let cold_now = agent.body_temperature.current
            < agent.body_temperature.ideal - Self::CHILLY_MARGIN;

        let the_drive_is_pressing = agent
            .drives
            .get(crate::core::DriveType::Shelter)
            .map(|drive| drive.value >= crate::core::DriveType::Shelter.default_threshold())
            .unwrap_or(false);

        cold_now || the_drive_is_pressing
    }

    /// Whether this one of mine is bare enough that I should clothe it.
    ///
    /// A child cannot clothe itself - it cannot gather flax, has no skill to
    /// sew, and until now nobody made it anything. The model already knew
    /// this: `update_agent_exposure` has a paragraph explaining that a child
    /// left to the weather runs two or three degrees colder than the adults
    /// round it and dies of that, and it works around it by counting a carer
    /// standing nearby *as shelter*. That is a plaster over the hole rather
    /// than the thing itself, and it only holds while somebody is stood next
    /// to them.
    ///
    /// So clothing hangs off Protection as well as Shelter, which is what a
    /// parent is for.
    pub(in crate::analytics) fn one_of_mine_who_is_bare(
        &self,
        agent: &crate::agents::Agent,
    ) -> Option<uuid::Uuid> {
        use crate::agents::LifeStage;

        self.population
            .agents
            .iter()
            .filter(|child| child.state.is_alive)
            .filter(|child| child.parent_ids.contains(&agent.id))
            .filter(|child| {
                matches!(child.state.life_stage, LifeStage::Infant | LifeStage::Child)
            })
            .filter(|child| child.body.total_cold_insulation() < Self::ENOUGH_INSULATION)
            // Near enough to hand it over. A gift is a thing you put in
            // somebody's hands, not a thing you post.
            .filter(|child| {
                let (x, y, _) = child.state.position;
                let (mx, my, _) = agent.state.position;
                (x - mx).abs().max((y - my).abs()) <= Self::WITHIN_SIGHT
            })
            .map(|child| child.id)
            .next()
    }

    /// What an agent of this much practice turns a given material into.
    ///
    /// The generic skill quality curve puts every untrained agent at Pathetic,
    /// and skills start ten levels below untrained, so a first cloak was worth
    /// half of nothing and no agent ever cooked or sewed often enough to climb
    /// out. A first attempt here is crude but wearable, and practice tells.
    pub(in crate::analytics) fn expected_garment_quality(agent: &crate::agents::Agent) -> crate::agents::skills::Quality {
        use crate::agents::skills::Quality;

        let practice = agent
            .skills
            .get_skill_if_exists(crate::agents::SkillType::Leatherworking)
            .map(|skill| skill.level)
            .unwrap_or(-10);

        match practice {
            level if level < 0 => Quality::Crude,
            0..=3 => Quality::Basic,
            4..=6 => Quality::Moderate,
            7..=8 => Quality::Advanced,
            _ => Quality::Expert,
        }
    }

    /// How warm a garment of this recipe and quality is
    pub(in crate::analytics) fn garment_warmth(
        recipe: &crate::agents::equipment::GarmentRecipe,
        quality: crate::agents::skills::Quality,
    ) -> f32 {
        recipe.warmth() * quality.modifier()
    }

    /// How warm the agent is already, in that slot
    pub(in crate::analytics) fn warmth_worn(agent: &crate::agents::Agent, slot: crate::agents::equipment::EquipmentSlot) -> f32 {
        agent
            .body
            .equipment
            .get(&slot)
            .map(|worn| worn.cold_insulation())
            .unwrap_or(0.0)
    }

    /// A garment in the pack worth changing into
    pub(in crate::analytics) fn garment_to_put_on(agent: &crate::agents::Agent) -> Option<String> {
        use crate::agents::equipment::garment_recipe;

        agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .filter_map(|item| {
                let recipe = garment_recipe(&item.item_id)?;
                let quality = item.quality.unwrap_or(crate::agents::skills::Quality::Crude);
                let wear = match (item.current_durability, item.max_durability) {
                    (Some(current), Some(max)) if max > 0.0 => (current / max).clamp(0.0, 1.0),
                    _ => 1.0,
                };
                let warmth = Self::garment_warmth(recipe, quality) * wear;

                if warmth > Self::warmth_worn(agent, recipe.slot) + Self::WARMTH_WORTH_CHANGING_FOR
                {
                    Some((recipe.id.to_string(), warmth))
                } else {
                    None
                }
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
    }

    /// The warmest garment the agent could make right now that would be an
    /// improvement on what it is wearing
    pub(in crate::analytics) fn garment_to_make(agent: &crate::agents::Agent) -> Option<String> {
        let quality = Self::expected_garment_quality(agent);

        crate::agents::equipment::GARMENT_RECIPES
            .iter()
            .filter(|recipe| Self::can_spare_material(agent, recipe))
            .filter(|recipe| {
                Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
            })
            .max_by(|a, b| {
                Self::garment_warmth(a, quality)
                    .partial_cmp(&Self::garment_warmth(b, quality))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|recipe| recipe.id.to_string())
    }

    /// Putting a roof up, or going to fetch what it needs.
    ///
    /// Building was chosen without ever looking at what the agent was
    /// carrying, so the Construction drive spent an eighth of a settlement's
    /// life restating that it was short of materials. Measured, `Build` failed
    /// 100.0% of the time and the commonest single reason was being
    /// twenty-six wood and all thirty stone short of a house.
    ///
    /// An agent that has what a tent takes puts one up. An agent that does not
    /// goes and gets the thing it is shortest of, which is the same answer a
    /// person would give and turns a wasted turn into a useful one.
    pub(in crate::analytics) fn raising_a_roof(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::BuildingType;

        // A tent is what a stone-age people can raise. Anything grander needs
        // stone they have no way to quarry.
        let wanted = BuildingType::SkinTent.requirements();

        let short_of = wanted
            .iter()
            .filter_map(|needed| {
                let name = format!("{:?}", needed.resource_type).to_lowercase();
                let have = agent
                    .inventory
                    .get_item(&name)
                    .map(|item| item.quantity)
                    .unwrap_or(0);
                if have >= needed.amount {
                    None
                } else {
                    Some((name, needed.amount - have))
                }
            })
            .max_by_key(|(_, missing)| *missing);

        match short_of {
            None => Some(Action::Build {
                structure_type: "tent".to_string(),
                position: agent_position,
            }),

            // Hides do not grow on bushes. Sending an agent to forage for them
            // is a wild goose chase, and a measured one: eighteen thousand
            // refusals of `No hides sources nearby` in a single world before
            // this told the difference between what the ground gives and what
            // has to be taken off an animal.
            //
            // And if there is nothing to hunt, dig in instead of standing
            // there. A tent wants eight wood and four hides; hides come off
            // animals and nothing else; and hunting was unreachable for the
            // whole life of this project - three deadlocked things in a row,
            // which is why `shelters built` was nought in every arm ever
            // measured. A hole in the ground with turf over it needs none of
            // them.
            Some((what, _)) if what.contains("hide") || what.contains("leather") => self
                .hunting_action(agent, agent_position)
                .or_else(|| self.digging_in(agent, agent_position)),

            Some((what, _)) => Some(Action::Gather { resource_type: what }),
        }
    }

    /// Digging yourself in, for want of anything to build with.
    ///
    /// Worse than a tent in every way except that it can be done. It wants
    /// ground that will take a hole - the same question the larder asks - and
    /// no roof already standing here, because a settlement that digs a second
    /// burrow on top of the first has spent a morning for nothing.
    pub(in crate::analytics) fn digging_in(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::Position;

        // Something to dig with. The matrix enforces it before the action
        // runs, so choosing this without one spends the turn on a refusal -
        // the pattern that cost a settlement half its winter store three
        // batches ago.
        if agent
            .what_i_have_to_work_with(crate::agents::SkillType::Mining)
            .is_none()
        {
            return None;
        }

        let here = Position::new(agent_position.0, agent_position.1);

        if !self.is_ground_a_pit_will_go_in(here) {
            return None;
        }

        // Not on top of somebody else's roof, and not on top of a hole that
        // is already there.
        let already = self
            .world
            .buildings
            .iter()
            .any(|building| {
                (building.position.x - here.x).abs() <= Self::HOW_CLOSE_TWO_ROOFS_GET
                    && (building.position.y - here.y).abs() <= Self::HOW_CLOSE_TWO_ROOFS_GET
            });

        if already {
            return None;
        }

        Some(Action::Build {
            structure_type: "burrow".to_string(),
            position: agent_position,
        })
    }

    /// How near one shelter goes to another.
    ///
    /// Two paces. A settlement is people living beside each other, not people
    /// living on top of each other, and without this a camp digs a burrow
    /// every tick for ever.
    pub(in crate::analytics) const HOW_CLOSE_TWO_ROOFS_GET: i32 = 2;

    /// Going to a child of one's own.
    ///
    /// A parent keeps its children near it, and goes to one that has strayed
    /// or that something is stalking. This is the whole of the Protection
    /// drive: it is answered by being where the children are, not by
    /// acquiring anything.
    ///
    /// It matters more than it looks. The young are kept warm by whoever is
    /// beside them, so a parent that wanders off leaves its child to the
    /// weather - and children freezing is what emptied settlements before.
    pub(in crate::analytics) fn protective_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::LifeStage;

        // Somebody of your own, lying ill within reach, and something in the
        // pack for it: that is what looking after your own comes to before
        // it comes to standing between them and a wolf.
        if let Some(sick) = self.one_of_mine_who_is_ill(agent, agent_position) {
            return Some(Action::Treat { who: Some(sick) });
        }

        // And one of your own going bare in the cold.
        //
        // Clothing hangs off two drives now, which is what was asked for and
        // what the model always implied: Shelter is a coat for yourself,
        // Protection is a coat for your child. A garment in hand goes to the
        // child that has none; with none in hand, making one is the errand,
        // and `clothing_action` already knows how to cut flax and sew it.
        //
        // Both of those are behind the ill and in front of the wolf, on the
        // same reasoning as the line above: a thing that is happening to a
        // child now beats a thing that is wearing it down, and a wolf beats
        // both.
        if let Some(bare) = self.one_of_mine_who_is_bare(agent) {
            if Self::garment_to_put_on(agent).is_some() {
                return Some(Action::GiveTo { to: bare });
            }

            if let Some(garment) = Self::garment_to_make(agent) {
                return Some(Action::MakeClothing { garment });
            }
        }

        // Only the small ones. An adolescent can look after itself.
        let mine: Vec<(i32, i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|child| child.state.is_alive)
            .filter(|child| child.parent_ids.contains(&agent.id))
            .filter(|child| {
                matches!(child.state.life_stage, LifeStage::Infant | LifeStage::Child)
            })
            .map(|child| child.state.position)
            .collect();

        if mine.is_empty() {
            return None;
        }

        // Anything with teeth near one of them brings a parent at a run
        let hunted = mine.iter().find(|child| {
            self.world
                .get_animals_in_radius((child.0, child.1), Self::DANGER_TO_A_CHILD as f32)
                .into_iter()
                .any(|animal| {
                    animal.is_alive()
                        && !animal.is_domesticated
                        && self
                            .world
                            .animals
                            .get_species(&animal.species_id)
                            .map(|species| !species.prey_species.is_empty())
                            .unwrap_or(false)
                })
        });

        if let Some(child) = hunted {
            return Some(Action::Move {
                target: (child.0, child.1, agent_position.2),
            });
        }

        // Otherwise, the one that has wandered furthest off
        let strayed = mine
            .iter()
            .map(|child| {
                let distance = (child.0 - agent_position.0)
                    .abs()
                    .max((child.1 - agent_position.1).abs());
                (child, distance)
            })
            .max_by_key(|(_, distance)| *distance)
            .filter(|(_, distance)| *distance > Self::CHILD_LEASH);

        strayed.map(|(child, _)| Action::Move {
            target: (child.0, child.1, agent_position.2),
        })
    }

    /// How far an agent will walk to break new ground
    pub(in crate::analytics) const FIELD_WALK_RADIUS: u32 = 12;

    /// How many fields a settlement wants within reach of where it is standing
    pub(in crate::analytics) const FIELDS_WANTED: usize = 6;

    /// Getting dressed, in whatever order the situation needs: put on what is
    /// already made, make what there is material for, or go and gather it.
    ///
    /// Only a cold agent bothers. Insulation was always zero before this,
    /// because nothing ever drove an agent to make or wear anything, so cold
    /// was a thing agents endured for their whole lives rather than solved.
    ///
    /// With `immediate_only` this reports only what can be done on the spot,
    /// which is what outranks walking to shelter: pulling on a coat you are
    /// already carrying beats crossing a field to get out of the wind, and
    /// going off to cut flax does not.
    pub(in crate::analytics) fn clothing_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        immediate_only: bool,
    ) -> Option<Action> {
        if !Self::wants_more_clothing(agent) {
            return None;
        }

        if let Some(garment) = Self::garment_to_put_on(agent) {
            return Some(Action::WearClothing { garment });
        }

        if let Some(garment) = Self::garment_to_make(agent) {
            return Some(Action::MakeClothing { garment });
        }

        if immediate_only {
            return None;
        }

        // Gathering reaches only as far as foraging does, so a patch further
        // off than that is somewhere to walk to first
        let (material, patch) = self.material_to_gather(agent, agent_position)?;

        let from = crate::world::Position::new(agent_position.0, agent_position.1);
        if from.distance_to(&patch) > Self::FORAGE_RADIUS {
            return Some(Action::Move {
                target: (patch.x, patch.y, agent_position.2),
            });
        }

        Some(Action::Gather {
            resource_type: material,
        })
    }

    /// Whether standing on this tile counts as being under cover
    pub(in crate::analytics) fn is_shelter_tile(&self, position: &crate::world::Position) -> bool {
        use crate::world::TerrainType;

        let in_building = self
            .world
            .get_building_at(position)
            .map(|building| building.is_completed())
            .unwrap_or(false);

        let in_woodland = self
            .world
            .grid
            .get_tile(position)
            .map(|tile| matches!(tile.terrain.terrain_type, TerrainType::Forest))
            .unwrap_or(false);

        in_building || in_woodland
    }

    /// Closest cover the agent can actually walk to, by walking distance.
    ///
    /// Reachability rather than raw proximity: a hut across a lake is no use,
    /// and heading for one leaves the agent stepping back and forth in the
    /// weather instead of sheltering. `None` means there is nowhere to go, and
    /// the agent is better off getting on with something it can accomplish.
    pub(in crate::analytics) fn nearest_shelter_from(&self, position: (i32, i32, i32)) -> Option<crate::world::Position> {
        use crate::world::Position;
        use std::collections::{BTreeSet, VecDeque};

        const MAX_VISITED: usize = 4096;

        let start = (position.0, position.1);

        let mut queue = VecDeque::new();
        let mut seen = BTreeSet::new();

        queue.push_back(start);
        seen.insert(start);

        let mut visited = 0usize;

        while let Some(current) = queue.pop_front() {
            visited += 1;
            if visited > MAX_VISITED {
                break;
            }

            let candidate = Position::new(current.0, current.1);

            if self.is_shelter_tile(&candidate) {
                return Some(candidate);
            }

            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let next = (current.0 + dx, current.1 + dy);

                if !seen.insert(next) {
                    continue;
                }

                if self.is_passable_tile(next.0, next.1) {
                    queue.push_back(next);
                }
            }
        }

        None
    }

    /// Whether the agent is currently standing in a completed building
    pub(in crate::analytics) fn agent_has_shelter(&self, agent_index: usize) -> bool {
        use crate::world::Position;

        let agent = &self.population.agents[agent_index];
        let pos = Position::new(agent.state.position.0, agent.state.position.1);

        self.world
            .get_building_at(&pos)
            .map(|building| building.is_completed())
            .unwrap_or(false)
    }

    /// One of this agent's own, ill, close enough to hand something to.
    ///
    /// Kin rather than anybody, because `Protection` is the drive for one's
    /// own and a settlement where everybody doses everybody is a settlement
    /// with no herbs left by Tuesday. Nearest first.
    fn one_of_mine_who_is_ill(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        // Nothing to give is nothing to offer.
        agent.what_i_have_for_it_for_somebody_else()?;

        let mut nearest: Option<(i32, uuid::Uuid)> = None;
        for other in self.population.agents.iter() {
            if !other.state.is_alive || other.id == agent.id {
                continue;
            }
            if !other.wants_something_for_it() {
                continue;
            }
            let theirs = other.parent_ids.contains(&agent.id)
                || agent.parent_ids.contains(&other.id)
                || other.parent_ids.iter().any(|p| agent.parent_ids.contains(p));
            if !theirs {
                continue;
            }

            let apart = (agent_position.0 - other.state.position.0)
                .abs()
                .max((agent_position.1 - other.state.position.1).abs());
            if apart > Self::HOW_FAR_YOU_WILL_GO_TO_DOSE_YOUR_OWN {
                continue;
            }
            if nearest.map(|(so_far, _)| apart < so_far).unwrap_or(true) {
                nearest = Some((apart, other.id));
            }
        }

        nearest.map(|(_, who)| who)
    }

    /// How far somebody will carry a remedy to one of their own.
    ///
    /// Across the camp, and no further. Anything past that is a journey and
    /// wants an errand rather than a drive.
    const HOW_FAR_YOU_WILL_GO_TO_DOSE_YOUR_OWN: i32 = 12;
}
