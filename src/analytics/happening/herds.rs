// src/analytics/happening/herds.rs
//! What kept beasts do about their keepers, each turn (#354).

use super::super::Simulation;
use std::collections::BTreeMap;

impl Simulation {
    /// How near its keeper a kept beast grazes: it is called in from
    /// further, and comes to hand from here.
    pub(in crate::analytics) const A_KEPT_BEAST_GRAZES_WITHIN: i32 = 3;

    /// What a day at its keeper's side does to a young one's tameness:
    /// tame in about a month untended.
    pub(in crate::analytics) const WHAT_A_DAY_BESIDE_ITS_KEEPER_TAMES: f32 = 0.03;

    /// How long a young one being raised waits for its keeper before it
    /// begins to go back to the wild: two days.
    pub(in crate::analytics) const A_YOUNG_ONE_WAITS: u32 = 2 * crate::environment::seasons::TICKS_PER_DAY;

    /// How much tameness a young one being raised loses in a day nobody
    /// feeds it, once its two days' grace is up. Taken at a fifth, it is
    /// gone four days after that.
    pub(in crate::analytics) const WHAT_A_DAY_UNFED_TAKES_OFF: f32 = 0.05;

    /// How often a tame beast left out of a fold strays in the night: about
    /// one night in thirty.
    pub(in crate::analytics) const A_NIGHT_OUT_STRAYS: f64 = 1.0 / 30.0;

    /// How much dung lies in a fold before its keeper sees what it is good
    /// for (`do:MuckOut`).
    pub(in crate::analytics) const DUNG_WORTH_NOTICING: f32 = 3.0;

    /// Each kept beast keeps by whoever keeps it (#354).
    ///
    /// - By day it grazes within a few paces of its keeper, and is walked
    ///   along wherever its keeper goes, at a walk: herds go with their
    ///   people, not the people with the herds.
    /// - At night it is shut in a fold if its keeper sleeps by one - out of
    ///   sight of a wolf, and leaving its dung - and otherwise grazes where it
    ///   is, and a tame one now and then strays and is not seen again.
    /// - A young one being raised that nobody feeds goes back to the wild.
    /// - A beast whose keeper is dead is nobody's, and goes wild.
    ///
    /// And the keeper sees a beast of theirs in milk, or shedding its wool,
    /// and dung lying in the fold, which is how those are found out.
    pub(in crate::analytics) fn the_kept_stay_by_their_keepers(&mut self) {
        use rand::Rng;

        let keepers: BTreeMap<uuid::Uuid, ((i32, i32), bool)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.id, ((agent.state.position.0, agent.state.position.1), agent.fatigue.is_sleeping)))
            .collect();
        let edge = (self.world.grid.width as i32 - 1, self.world.grid.height as i32 - 1);
        let now = self.current_turn;
        let a_turn_beside_its_keeper = Self::WHAT_A_DAY_BESIDE_ITS_KEEPER_TAMES
            / crate::environment::seasons::PLANNING_PERIODS_PER_DAY as f32;
        let a_turn_unfed = Self::WHAT_A_DAY_UNFED_TAKES_OFF
            / crate::environment::seasons::PLANNING_PERIODS_PER_DAY as f32;
        let a_night_out = Self::A_NIGHT_OUT_STRAYS
            / (crate::environment::seasons::PLANNING_PERIODS_PER_DAY as f64 / 3.0);
        let a_walk = crate::world::pace::CELLS_IN_A_HALF_HOUR_OF_WALKING as i32;
        let near = Self::A_KEPT_BEAST_GRAZES_WITHIN;

        let mut seen: Vec<(uuid::Uuid, &'static str)> = Vec::new();
        // Drawn only when a tame beast is out for the night, so that a world
        // with nothing kept rolls exactly as it did
        let mut rng: Option<rand::rngs::StdRng> = None;
        let world = &mut self.world;
        let what_it_gives: BTreeMap<String, Vec<String>> = world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_kept())
            .filter_map(|animal| {
                let species = world.animals.get_species(&animal.species_id)?;
                Some((
                    animal.species_id.clone(),
                    species.living_products.iter().map(|product| product.material_id.clone()).collect(),
                ))
            })
            .collect();

        // The fold each sleeping keeper lies by, if any
        let folded_by: BTreeMap<uuid::Uuid, usize> = keepers
            .iter()
            .filter(|(_, (_, asleep))| *asleep)
            .filter_map(|(keeper, (at, _))| {
                world.fold_near(&crate::world::Position::new(at.0, at.1)).map(|which| (*keeper, which))
            })
            .collect();
        let folds = &mut world.folds;

        for animal in world.animals.get_all_mut().iter_mut() {
            if !animal.is_alive() {
                continue;
            }
            animal.in_the_fold = false;
            let Some(keeper) = animal.owner_id else {
                continue;
            };
            let Some(&(at, asleep)) = keepers.get(&keeper) else {
                Self::turned_loose(animal);
                continue;
            };

            // A young one being raised takes to whoever it stays beside, a
            // little each day, whether or not they have anything to give it;
            // tending it speeds that (`taming`). Apart from its keeper it
            // waits two days and then begins to drift back to the wild.
            //
            // **Feeding alone was not enough.** Its keepers were mostly
            // hunters, at home of an evening one time in three: tended or
            // not, fifteen of sixteen young taken in forty worlds ran off.
            if !animal.is_domesticated {
                let by_its_keeper =
                    (animal.position.0 - at.0).abs().max((animal.position.1 - at.1).abs()) <= near;
                let seen_to = animal.fed_at.unwrap_or(0).max(animal.taken_at);
                if by_its_keeper {
                    animal.tame(a_turn_beside_its_keeper);
                    if animal.is_domesticated {
                        seen.push((keeper, "do:Tame"));
                    }
                } else if now.saturating_sub(seen_to) > Self::A_YOUNG_ONE_WAITS {
                    animal.tame_level -= a_turn_unfed;
                    if animal.tame_level <= 0.0 {
                        Self::turned_loose(animal);
                        continue;
                    }
                }
            } else {
                let gives = what_it_gives.get(&animal.species_id);
                let gives = |what: &str| gives.is_some_and(|all| all.iter().any(|given| given == what));
                if gives("milk") && animal.is_mature() && animal.nursing_for > 0 {
                    seen.push((keeper, "milk"));
                }
                if gives("wool") && animal.has_ready("wool") {
                    seen.push((keeper, "wool"));
                }
            }

            if asleep {
                if let Some(&which) = folded_by.get(&keeper) {
                    let fold = &mut folds[which];
                    animal.position = (fold.at.x, fold.at.y);
                    animal.in_the_fold = true;
                    fold.dung += crate::world::Fold::WHAT_A_BEAST_LEAVES_IN_A_HALF_HOUR;
                } else if animal.is_domesticated
                    && rng.get_or_insert_with(crate::core::dice::roll).gen_bool(a_night_out)
                {
                    Self::turned_loose(animal);
                }
                continue;
            }

            // Called in, or walked along
            let (dx, dy) = (animal.position.0 - at.0, animal.position.1 - at.1);
            if dx.abs().max(dy.abs()) > near {
                let toward = |from: i32, to: i32| from + (to - from).clamp(-a_walk, a_walk);
                let wanted = (at.0 + dx.clamp(-near, near), at.1 + dy.clamp(-near, near));
                animal.position = (
                    toward(animal.position.0, wanted.0).clamp(0, edge.0),
                    toward(animal.position.1, wanted.1).clamp(0, edge.1),
                );
            }
        }

        // And the dung, seen by whoever is awake beside a fold with some in it
        for (&keeper, &(at, asleep)) in &keepers {
            if asleep {
                continue;
            }
            if let Some(which) = self.world.fold_near(&crate::world::Position::new(at.0, at.1)) {
                if self.world.folds[which].dung >= Self::DUNG_WORTH_NOTICING {
                    seen.push((keeper, "do:MuckOut"));
                }
            }
        }

        for (keeper, what) in seen {
            if let Some(agent) = self.population.agents.iter_mut().find(|agent| agent.id == keeper) {
                if agent.found_out_how_to(what) && matches!(what, "milk" | "wool") {
                    agent.found_out_how_to("do:CollectAnimalProduct");
                }
            }
        }
    }

    /// A kept beast nobody keeps any more goes back to the wild.
    fn turned_loose(animal: &mut crate::environment::fauna::Animal) {
        animal.owner_id = None;
        animal.is_domesticated = false;
        animal.tame_level = 0.0;
        animal.in_the_fold = false;
    }
}
