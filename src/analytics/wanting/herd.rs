// src/analytics/wanting/herd.rs
//! Seeing to the kept beasts (#354): what a keeper does for them in the
//! evening, which one is killed when there is meat wanted, and moving camp
//! when the grazing is eaten down.

use super::super::Simulation;
use crate::environment::fauna::Animal;
use crate::environment::Action;

impl Simulation {
    /// A tame beast is fed by hand once it is this hungry, as a share of
    /// what it can hold: the grazing has not filled it.
    pub(in crate::analytics) const HUNGRY_ENOUGH_TO_FEED: f32 = 0.5;

    /// How much dung is worth going in for.
    pub(in crate::analytics) const DUNG_WORTH_CARRYING_OUT: f32 = 3.0;

    /// The grown beasts a keeper kills none of when merely hungry: the
    /// herd that breeds the next one.
    pub(in crate::analytics) const A_HERD_KEPT_BACK: usize = 4;

    /// And the fewest a keeper keeps back however hungry: a pair.
    pub(in crate::analytics) const A_PAIR_KEPT_BACK: usize = 2;

    /// How hungry, as a share of what they can hold, a keeper's beasts are
    /// before the camp is moved to fresh grazing.
    pub(in crate::analytics) const A_HERD_GOING_HUNGRY: f32 = 0.6;

    /// This one's kept beasts, alive.
    pub(in crate::analytics) fn my_beasts<'a>(
        &'a self,
        agent: &'a crate::agents::Agent,
    ) -> impl Iterator<Item = &'a Animal> + 'a {
        self.world
            .animals
            .get_all()
            .iter()
            .filter(move |animal| animal.is_alive() && animal.owner_id == Some(agent.id))
    }

    /// How many beasts a keeper goes on catching young ones until they keep.
    pub(in crate::analytics) const A_HERD_WORTH_BUILDING: usize = 8;

    /// A young one to catch, by somebody who has raised one and keeps fewer
    /// than a herd (#360): one of a kind that can be kept, in sight and not
    /// anybody's, the kinds already kept first so that they breed. Caught
    /// when it is to hand, closed on when it is not.
    ///
    /// **One beast is not a herd.** A young one left by a kill came now and
    /// then, a dozen in forty worlds over two years; a people does not wait
    /// for the next. Somebody who has raised one goes out after more, and
    /// two of a kind kept together breed, and what a kept mother bears is
    /// kept.
    pub(in crate::analytics) fn a_young_one_to_catch(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<Action> {
        if !agent.knows_how_to_do("Tame") {
            return None;
        }
        let kept: Vec<&str> = self.my_beasts(agent).map(|animal| animal.species_id.as_str()).collect();
        if kept.len() >= Self::A_HERD_WORTH_BUILDING {
            return None;
        }
        let sight = Self::how_far_a_hunter_looks(agent);
        let (id, at, _) = self
            .world
            .get_animals_in_radius((here.0, here.1), sight)
            .into_iter()
            .filter(|animal| animal.is_alive() && !animal.is_mature() && !animal.is_kept())
            .filter(|animal| {
                self.world
                    .animals
                    .get_species(&animal.species_id)
                    .is_some_and(|species| species.can_domesticate)
            })
            .filter(|animal| self.world.can_walk_between((here.0, here.1), animal.position))
            .map(|animal| {
                let far = (animal.position.0 - here.0).abs().max((animal.position.1 - here.1).abs());
                let a_kind_not_kept = !kept.contains(&animal.species_id.as_str());
                (animal.id, animal.position, (a_kind_not_kept, far))
            })
            .min_by_key(|(id, _, rank)| (*rank, *id))?;
        let far = (at.0 - here.0).abs().max((at.1 - here.1).abs());
        if far <= Self::A_KEPT_BEAST_COMES_TO_HAND {
            Some(Action::Tame { animal_id: id, food_type: None })
        } else {
            Some(Action::Move { target: (at.0, at.1, here.2) })
        }
    }

    /// What there is to do for the beasts in the evening, by their keeper
    /// (#354): a young one being raised fed, a hungry tame one fed, milk or
    /// wool taken that is ready, a fold put up where they will sleep, and
    /// the dung carried out of it.
    pub(in crate::analytics) fn the_beasts_to_see_to(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<Action> {
        let now = self.current_turn;
        let to_hand = |animal: &&Animal| {
            (animal.position.0 - here.0).abs().max((animal.position.1 - here.1).abs())
                <= Self::A_KEPT_BEAST_COMES_TO_HAND
        };
        let mine: Vec<&Animal> = self.my_beasts(agent).filter(to_hand).collect();
        if mine.is_empty() && self.world.folds.is_empty() && !agent.knows_how_to_do("Tame") {
            return None;
        }

        // Fed or tended, the young before the hungry: a young one being
        // raised is seen to whether or not there is fodder in the pack, a
        // grown one fed only from what there is
        let unfed = |animal: &&&Animal| {
            animal.fed_at.is_none_or(|fed| now.saturating_sub(fed) >= Self::FED_TODAY)
        };
        let has_fodder = Self::fodder_in(agent).is_some();
        let wants_feeding = mine
            .iter()
            .filter(unfed)
            .find(|animal| !animal.is_domesticated)
            .or_else(|| {
                mine.iter().filter(unfed).filter(|_| has_fodder).find(|animal| {
                    animal.hunger >= animal.max_hunger * Self::HUNGRY_ENOUGH_TO_FEED
                })
            });
        if let Some(animal) = wants_feeding {
            return Some(Action::Tame { animal_id: animal.id, food_type: None });
        }

        // Milked or plucked, of what this one knows to take
        let known = agent.what_i_found_out();
        let ready = mine.iter().filter(|animal| animal.is_domesticated).find(|animal| {
            animal
                .product_timers
                .keys()
                .any(|product| known.contains(product) && animal.has_ready(product))
        });
        if let Some(animal) = ready {
            return Some(Action::CollectAnimalProduct { animal_id: animal.id });
        }

        // Folded for the night
        let at = crate::world::Position::new(here.0, here.1);
        let fold = self.world.fold_near(&at);
        if fold.is_none()
            && mine.iter().any(|animal| animal.is_domesticated)
            && agent.knows_how_to_do("PutUpFold")
        {
            if agent.how_many_i_have("wood") >= Self::WHAT_A_FOLD_TAKES {
                return Some(Action::PutUpFold);
            }
            if self
                .nearest_resource_within(agent, here, Self::FORAGE_RADIUS, |resource| {
                    resource.resource_type == crate::world::ResourceType::Wood
                })
                .is_some()
            {
                return Some(Action::Gather { resource_type: "wood".to_string() });
            }
        }

        // And mucked out
        if let Some(which) = fold {
            if self.world.folds[which].dung >= Self::DUNG_WORTH_CARRYING_OUT
                && agent.knows_how_to_do("MuckOut")
                && agent.how_many_i_have("dung") < Self::AN_ARMFUL_OF_DUNG
            {
                return Some(Action::MuckOut);
            }
        }

        // And another young one to the herd, if one is in sight
        self.a_young_one_to_catch(agent, here)
    }

    /// One of this one's own beasts to kill for its meat (#354), if the herd
    /// can spare one: the herd kept back for breeding is not touched, or
    /// only down to a pair by somebody starving. The oldest of the grown
    /// ones that is not in milk goes first.
    pub(in crate::analytics) fn a_beast_to_spare(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
        desperate: bool,
    ) -> Option<Action> {
        if !agent.knows_how_to_do("Slaughter") {
            return None;
        }
        let grown: Vec<&Animal> = self
            .my_beasts(agent)
            .filter(|animal| animal.is_domesticated && animal.is_mature())
            .collect();
        let kept_back = if desperate { Self::A_PAIR_KEPT_BACK } else { Self::A_HERD_KEPT_BACK };
        if grown.len() <= kept_back {
            return None;
        }
        grown
            .iter()
            .filter(|animal| animal.nursing_for == 0)
            .filter(|animal| {
                (animal.position.0 - here.0).abs().max((animal.position.1 - here.1).abs())
                    <= Self::A_KEPT_BEAST_COMES_TO_HAND
            })
            .max_by_key(|animal| (animal.age, animal.id))
            .map(|animal| Action::Slaughter { animal_id: animal.id })
    }

    /// Whether this one's beasts have eaten the grazing down where they are
    /// (#354): most of what they can hold, on average, and none of it fed.
    pub(in crate::analytics) fn my_herd_is_going_hungry(&self, agent: &crate::agents::Agent) -> bool {
        let (mut beasts, mut hunger) = (0, 0.0);
        for animal in self.my_beasts(agent).filter(|animal| animal.is_domesticated) {
            beasts += 1;
            hunger += animal.hunger / animal.max_hunger.max(1.0);
        }
        beasts > 0 && hunger / beasts as f32 >= Self::A_HERD_GOING_HUNGRY
    }

    /// What open ground a beast grazes, by the ground.
    fn what_this_ground_grazes(terrain: crate::world::TerrainType) -> f32 {
        use crate::world::TerrainType;
        match terrain {
            TerrainType::Meadow => 1.0,
            TerrainType::Plains => 0.9,
            TerrainType::Hills | TerrainType::Riverbank => 0.6,
            TerrainType::Wetland | TerrainType::SaltMarsh => 0.3,
            TerrainType::Forest | TerrainType::Mountain => 0.2,
            _ => 0.0,
        }
    }

    /// Fresh grazing a move away (#354): of the ground a day's move off in
    /// each of the eight directions, whichever grazes best and can be walked
    /// to. Herders follow the grass, and a herd that has eaten a place down
    /// is moved on to ground it has not.
    pub(in crate::analytics) fn fresh_grazing_from(&self, here: (i32, i32, i32)) -> Option<(i32, i32, i32)> {
        let far = Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK * 6;
        let (width, height) = (self.world.grid.width as i32, self.world.grid.height as i32);
        let mut best: Option<(f32, (i32, i32))> = None;
        for (dx, dy) in [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)] {
            let there = (here.0 + dx * far, here.1 + dy * far);
            if there.0 < 0 || there.1 < 0 || there.0 >= width || there.1 >= height {
                continue;
            }
            let mut grazes = 0.0;
            for sx in (-8..=8).step_by(4) {
                for sy in (-8..=8).step_by(4) {
                    if let Some(tile) = self.world.grid.get_tile(&crate::world::Position::new(there.0 + sx, there.1 + sy)) {
                        grazes += Self::what_this_ground_grazes(tile.terrain.terrain_type);
                    }
                }
            }
            if grazes > best.map_or(0.0, |(worth, _)| worth)
                && self.world.can_walk_between((here.0, here.1), there)
            {
                best = Some((grazes, there));
            }
        }
        best.map(|(_, there)| (there.0, there.1, here.2))
    }
}
