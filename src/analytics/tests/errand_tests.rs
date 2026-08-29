//! An errand held across turns, rather than re-decided at every step.
//!
//! "Once an agent plans an action, it would not change its mind unless its
//! situation changed in some manner... In most cases, the agents should not
//! need to change their decisions once they are made."

use crate::agents::{AgentConfig, Errand, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

fn a_settlement(founders: usize, turns: usize) -> Simulation {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..founders {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for _ in 0..turns {
        simulation.tick();
    }
    simulation
}

fn how_often(simulation: &Simulation, what: &str) -> u64 {
    simulation
        .what_a_threat_came_to
        .get(what)
        .copied()
        .unwrap_or(0)
}

/// Agents set out on errands and finish them.
#[test]
fn a_walk_is_finished_rather_than_re_decided_at_every_step() {
    let simulation = a_settlement(12, 600);

    let set_out = how_often(&simulation, "errand: set out");
    let kept_to = how_often(&simulation, "errand: kept to it");
    let got_there = how_often(&simulation, "errand: got there");

    assert!(set_out > 0, "nobody ever set out anywhere");
    assert!(
        kept_to > 0,
        "every errand was abandoned on the turn it was set out on"
    );
    assert!(
        got_there > 0,
        "{set_out} errands set out and not one of them arrived"
    );

    // A walk of more than a single step, on average. Before this, every tile
    // was a fresh decision and a trip of any length rarely finished.
    assert!(
        kept_to as f64 / set_out.max(1) as f64 > 0.5,
        "errands are being dropped as fast as they are made: \
         {set_out} set out, {kept_to} kept to"
    );
}

/// What ends an errand is a change in what the agent needs, not the clock.
#[test]
fn an_errand_ends_when_the_need_changes_and_not_before() {
    let simulation = a_settlement(12, 600);

    let got_there = how_often(&simulation, "errand: got there");
    let something_else = how_often(&simulation, "errand: something else came first");
    let gave_up = how_often(&simulation, "errand: gave up on it");

    // Giving up on an unreachable place is a backstop, not the usual way an
    // errand ends: if it is the commonest ending, agents are setting out for
    // places they cannot get to.
    assert!(
        gave_up < got_there,
        "more errands were given up on ({gave_up}) than finished ({got_there})"
    );
    assert!(
        gave_up < something_else.max(1),
        "the backstop is doing the work the drives should be doing"
    );
}

/// A short walk is not abandoned on its first step.
#[test]
fn even_the_shortest_errand_gets_a_few_turns() {
    let errand = Errand {
        going_to: (0, 0, 0),
        for_drive: crate::core::DriveType::Thirst,
        pressed_this_hard: 1.0,
        turns_on_it: 0,
    };

    // Standing on the spot it is going to, the walk is nothing at all, and
    // three times nothing is still nothing - so there is a floor under it
    assert_eq!(errand.how_far_it_was((0, 0, 0)), 0);
    assert!(
        Errand::AT_LEAST_THIS_MANY_TURNS > 0,
        "an errand with no turns in it is abandoned before it starts"
    );

    // And it knows when it has arrived, ignoring height
    assert!(errand.arrived((0, 0, 4)));
    assert!(!errand.arrived((1, 0, 0)));
}

/// The nearer the end of an errand, the more it takes to turn somebody off it.
///
/// "If an agent is a few steps away from getting a meal and hydration drive
/// suddenly kicks in, then the agent abandoning its current task to get a drink
/// could waste the invested energy the agent spent to get a meal."
#[test]
fn a_walk_nearly_finished_is_harder_to_abandon_than_one_just_begun() {
    // Twenty paces off, one turn in: almost all of the trip is still ahead
    let just_set_out = Errand {
        going_to: (20, 0, 0),
        for_drive: crate::core::DriveType::Hunger,
        pressed_this_hard: 1.0,
        turns_on_it: 1,
    };
    // The same errand, nineteen turns later and one pace short of the patch
    let nearly_there = Errand {
        going_to: (20, 0, 0),
        for_drive: crate::core::DriveType::Hunger,
        pressed_this_hard: 1.0,
        turns_on_it: 19,
    };

    let at_the_start = Simulation::what_it_takes_to_turn_me_round(&just_set_out, (0, 0, 0));
    let at_the_end = Simulation::what_it_takes_to_turn_me_round(&nearly_there, (19, 0, 0));

    assert!(
        at_the_end > at_the_start,
        "a walk nineteen twentieths done ({at_the_end:.2}) should be harder to \
         abandon than one just begun ({at_the_start:.2})"
    );

    // And neither is a veto: a drive that presses hard enough still wins, which
    // is what keeps a body from dying of thirst two paces from its supper
    assert!(
        at_the_end < 10.0,
        "sunk cost is a thumb on the scale, not a lock: {at_the_end:.2}"
    );
}
