// src/analytics/tests/coupling_tests.rs
//! What a coupling that did not take does to the wish for a child.
//!
//! A carrier has one fertile day a month, and people who sleep side by side
//! couple on many more days than that. What those other days cost the wish
//! decides whether it is still there on the one that counts. See
//! ISSUES_FOUND #299 and #311.

use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// Two grown people side by side, fed, with a winter put by, each wishing
/// for a child as much as `wishes` says. Returned with which of them carries.
fn a_couple(wishes: [f32; 2]) -> (Simulation, usize) {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let mut population = Population::new();
    for _ in 0..2 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);
    for (agent, wish) in simulation.population.agents.iter_mut().zip(wishes) {
        agent.state.now_this_many_years_old(30);
        agent.state.position = (20, 20, 0);
        agent.state.health = 100.0;
        agent.traits.traits.retain(|t| *t != crate::core::traits::Trait::Infertile);
        agent.reproduction_drive_modifier = 1.0;
        for drive in [DriveType::Hunger, DriveType::Thirst] {
            if let Some(it) = agent.drives.get_mut(drive) {
                it.value = 0.0;
            }
        }
        if let Some(it) = agent.drives.get_mut(DriveType::Reproduction) {
            it.value = wish;
        }
        let a_day = agent.state.physiology.what_i_burn_in_a_day;
        let gap = crate::agents::provision::how_long_the_land_gives_nothing() as f32;
        agent.state.what_the_larder_says = Some(
            crate::agents::provision::WhatIsPutBy::reckon(a_day * 3.0 * gap, a_day, 90.0, 0),
        );
    }

    let ids: Vec<_> = simulation.population.agents.iter().map(|a| a.id).collect();
    let carrier = if ids[0] <= ids[1] { 0 } else { 1 };
    (simulation, carrier)
}

fn wish(simulation: &Simulation, index: usize) -> f32 {
    simulation.population.agents[index]
        .drives
        .get(DriveType::Reproduction)
        .unwrap()
        .value
}

fn threshold(simulation: &Simulation, index: usize) -> f32 {
    simulation.population.agents[index]
        .drives
        .get(DriveType::Reproduction)
        .unwrap()
        .threshold
}

/// Person 0 asks person 1, on the turn given, until a coupling happens that
/// does not take; the result is fed back as a turn of the person asking
/// feeds it back. `None` if it took, or nobody ever agreed.
fn a_try_that_did_not_take(wishes: [f32; 2], fertile: bool) -> Option<(Simulation, usize)> {
    for _ in 0..200 {
        let (mut simulation, carrier) = a_couple(wishes);
        let fertile_turn = simulation.population.agents[carrier].my_next_fertile_turn(0);
        simulation.current_turn = if fertile {
            fertile_turn
        } else {
            neither_ones_day(&simulation)
        };

        let them = simulation.population.agents[1].id;
        let result = simulation.execute_action(&Action::Mate { target_agent_id: them }, 0);
        if !result.success || simulation.population.agents[carrier].pregnancy.is_some() {
            continue;
        }
        simulation.population.agents[0].apply_feedback(&result, DriveType::Reproduction);
        return Some((simulation, carrier));
    }
    None
}

/// A turn that is neither partner's fertile day.
fn neither_ones_day(simulation: &Simulation) -> u32 {
    let a_day = crate::environment::seasons::TICKS_PER_DAY;
    (1..)
        .map(|day| day * a_day)
        .find(|&turn| {
            simulation
                .population
                .agents
                .iter()
                .all(|agent| !agent.could_conceive_now(turn))
        })
        .unwrap()
}

/// A turn that is `index`'s fertile day and not the other's, if they do not
/// share one.
fn only_their_day(simulation: &Simulation, index: usize) -> Option<u32> {
    let a_day = crate::environment::seasons::TICKS_PER_DAY;
    (0..Agent::DAYS_IN_A_CYCLE)
        .map(|day| day * a_day)
        .find(|&turn| {
            simulation.population.agents[index].could_conceive_now(turn)
                && !simulation.population.agents[1 - index].could_conceive_now(turn)
        })
}

/// A couple whose fertile days are different days.
fn a_couple_on_different_days(wishes: [f32; 2]) -> (Simulation, usize) {
    loop {
        let (simulation, lower) = a_couple(wishes);
        if only_their_day(&simulation, lower).is_some() {
            return (simulation, lower);
        }
    }
}

fn a_days_wish() -> f32 {
    DriveType::Reproduction.base_accumulation_rate()
        * crate::environment::seasons::PLANNING_PERIODS_PER_DAY as f32
}

/// Off the fertile day, a coupling answers the wish for the day and no more:
/// both are left a day's regrowth short of asking again.
///
/// It was 0.05 off each, and twice that off the one who asked, so somebody
/// whose wish stood at 1.0 tried seven times inside one day and came out far
/// below wanting anything (#311).
#[test]
fn a_try_off_the_fertile_day_leaves_both_a_day_short() {
    let (simulation, _) = a_try_that_did_not_take([1.0, 1.0], false).expect("a coupling");

    for index in 0..2 {
        let expected = threshold(&simulation, index) - a_days_wish();
        assert!(
            (wish(&simulation, index) - expected).abs() < 1e-4,
            "person {index} left at {:.3}, a day short is {expected:.3}",
            wish(&simulation, index)
        );
    }
}

/// And it does not touch somebody who had no wish for a child to begin with.
///
/// Being asked took 0.05 off whoever was asked. Eight grown people in thirteen
/// on one settlement, most of them young, spent most of 90 days below 0.1,
/// held there by the tries of others (#311).
#[test]
fn being_asked_does_not_spend_a_wish_you_did_not_have() {
    let (simulation, _) = a_try_that_did_not_take([1.0, 0.1], false).expect("a coupling");

    assert!(
        (wish(&simulation, 1) - 0.1).abs() < 1e-6,
        "the one asked went from 0.1 to {:.3}",
        wish(&simulation, 1)
    );
}

/// Nor somebody nearly there: a wish that was not yet asking is not answered.
///
/// Answering it as well kept people who were climbing towards the threshold
/// a day short of it for as long as somebody else kept asking: most grown
/// people on one settlement sat between 0.5 and 0.6 (#311).
#[test]
fn being_asked_does_not_set_back_a_wish_that_was_nearly_there() {
    let (simulation, _) = a_try_that_did_not_take([1.0, 0.58], false).expect("a coupling");

    assert!(
        (wish(&simulation, 1) - 0.58).abs() < 1e-6,
        "the one asked went from 0.58 to {:.3}",
        wish(&simulation, 1)
    );
}

/// On the fertile day a coupling that does not take costs the one who asked
/// what it costs the other, once. It cost them twice.
#[test]
fn a_failed_try_on_the_fertile_day_costs_the_one_who_asked_once() {
    let (simulation, _) = a_try_that_did_not_take([0.9, 0.9], true).expect("a coupling that did not take");

    for index in 0..2 {
        assert!(
            (wish(&simulation, index) - 0.6).abs() < 1e-4,
            "person {index} left at {:.3}, against 0.6",
            wish(&simulation, index)
        );
    }
}

/// Whichever partner is on their own fertile day carries, whatever their ids.
///
/// It was always the lower id, so a person's fertile day counted only with a
/// partner of a higher id, and the highest id in a settlement never carried
/// (#312).
#[test]
fn whoever_is_on_their_fertile_day_carries() {
    use crate::agents::reproduction::does_this_one_carry;

    let (simulation, lower) = a_couple_on_different_days([1.0, 1.0]);
    let higher = 1 - lower;
    let agents = &simulation.population.agents;

    let theirs = only_their_day(&simulation, higher).unwrap();
    assert!(does_this_one_carry(&agents[higher], &agents[lower], theirs));
    assert!(!does_this_one_carry(&agents[lower], &agents[higher], theirs));

    let ours = only_their_day(&simulation, lower).unwrap();
    assert!(does_this_one_carry(&agents[lower], &agents[higher], ours));

    // On a day that is neither's, the lower id, so it does not matter who asks
    let neither = neither_ones_day(&simulation);
    assert!(does_this_one_carry(&agents[lower], &agents[higher], neither));
    assert!(!does_this_one_carry(&agents[higher], &agents[lower], neither));
}

/// And so a coupling on the higher id's fertile day is their chance for the
/// cycle, and can make them the one carrying.
#[test]
fn the_higher_ids_fertile_day_is_a_chance_too() {
    let mut carried_by_the_higher = 0;
    for _ in 0..400 {
        let (mut simulation, lower) = a_couple_on_different_days([1.0, 1.0]);
        let higher = 1 - lower;
        simulation.current_turn = only_their_day(&simulation, higher).unwrap();

        let them = simulation.population.agents[1].id;
        let result = simulation.execute_action(&Action::Mate { target_agent_id: them }, 0);
        if !result.success {
            continue;
        }

        assert!(
            simulation.population.agents[higher].last_cycle_tried.is_some(),
            "the higher id's fertile day went by without counting as their chance"
        );
        assert!(
            simulation.population.agents[lower].pregnancy.is_none(),
            "the one not on their fertile day conceived"
        );
        if simulation.population.agents[higher].pregnancy.is_some() {
            carried_by_the_higher += 1;
        }
    }
    assert!(carried_by_the_higher > 0, "nobody of the higher id ever conceived");
}
