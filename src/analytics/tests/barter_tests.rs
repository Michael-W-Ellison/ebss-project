// src/analytics/tests/barter_tests.rs
//! Tests for handing things over.
//!
//! "The agents should also use a barter system if they have an abundance of
//! something another agent wants and that agent has an abundance of something
//! they want."
//!
//! Both halves have to hold, or it is not a trade. What each of them wants is
//! not a preference anybody wrote down: it is the raw stuff every step and
//! every working in the chain asks for, minus what is already in the pack. So
//! a man with forty sticks and no stone wants stone, and a man with forty
//! stones and no wood wants wood, and between them there is a trade.
//!
//! A gift is a trade with one side missing. It costs the giver and it is worth
//! more to the bond than a trade is, which is the whole difference between
//! them: a trade leaves both square and a gift leaves one of them owing.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// Two people standing next to each other with nothing in their packs
fn two_people() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    for who in 0..2 {
        simulation.population.agents[who].state.position = (25, 25, 0);

        let everything: Vec<(String, u32)> = simulation.population.agents[who]
            .inventory
            .get_all_items()
            .values()
            .map(|item| (item.item_id.clone(), item.quantity))
            .collect();

        for (what, how_many) in everything {
            for _ in 0..how_many {
                simulation.population.agents[who]
                    .inventory
                    .remove_item(&what, 1);
            }
        }
    }

    simulation
}

fn give(simulation: &mut Simulation, who: usize, what: &str, how_many: u32) {
    simulation.population.agents[who]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            what.to_string(),
            how_many,
            1.0,
        ));
}

// --------------------------------------------------------------------------
// What somebody wants
// --------------------------------------------------------------------------

/// What an agent is short of is the raw stuff the chain asks for and the pack
/// has not got.
#[test]
fn what_somebody_is_short_of_is_what_the_chain_wants() {
    let mut simulation = two_people();

    let short = simulation.population.agents[0].what_i_am_short_of();
    assert!(
        short.contains(&"wood") && short.contains(&"stone") && short.contains(&"flax"),
        "an empty pack is short of everything the chain uses: {short:?}"
    );

    give(&mut simulation, 0, "wood", 40);

    let short = simulation.population.agents[0].what_i_am_short_of();
    assert!(
        !short.contains(&"wood"),
        "a man with forty sticks does not want sticks: {short:?}"
    );
    assert!(
        short.contains(&"stone"),
        "and still wants stone"
    );
}

// --------------------------------------------------------------------------
// Trading
// --------------------------------------------------------------------------

/// An abundance for an abundance, each of which the other wants.
#[test]
fn two_people_with_opposite_problems_trade() {
    let mut simulation = two_people();

    give(&mut simulation, 0, "wood", 40);
    give(&mut simulation, 1, "stone", 40);

    let them = simulation.population.agents[1].id;
    let position = simulation.population.agents[0].state.position;

    assert_eq!(
        simulation.somebody_to_trade_with(&simulation.population.agents[0], position),
        Some(them),
        "a man with sticks and no stone should find the man with stone and no sticks"
    );

    let result = simulation.execute_action(&Action::Trade { with: them }, 0);
    assert!(result.success, "and trade with him: {:?}", result.message);

    assert!(
        simulation.population.agents[0].how_many_i_have("stone") > 0,
        "the first one comes away with stone"
    );
    assert!(
        simulation.population.agents[1].how_many_i_have("wood") > 0,
        "and the second with wood"
    );
    assert!(
        simulation.population.agents[0].how_many_i_have("wood") < 40,
        "each of them is lighter by what they gave"
    );
    assert!(simulation.population.agents[1].how_many_i_have("stone") < 40);
}

/// Two people with the same problem have nothing to say to each other.
#[test]
fn two_people_with_the_same_surplus_have_no_trade() {
    let mut simulation = two_people();

    give(&mut simulation, 0, "wood", 40);
    give(&mut simulation, 1, "wood", 40);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .somebody_to_trade_with(&simulation.population.agents[0], position)
            .is_none(),
        "nobody swaps sticks for sticks"
    );
}

/// And one abundance is not a trade: both sides have to want something.
#[test]
fn one_sided_abundance_is_not_a_trade() {
    let mut simulation = two_people();

    // One of them has everything spare and the other has nothing at all
    give(&mut simulation, 0, "wood", 40);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .somebody_to_trade_with(&simulation.population.agents[0], position)
            .is_none(),
        "a man with nothing to offer has nothing to trade, however much he wants"
    );
}

/// Nobody trades across the map.
#[test]
fn nobody_trades_with_somebody_out_of_reach() {
    let mut simulation = two_people();

    give(&mut simulation, 0, "wood", 40);
    give(&mut simulation, 1, "stone", 40);
    simulation.population.agents[1].state.position = (60, 25, 0);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .somebody_to_trade_with(&simulation.population.agents[0], position)
            .is_none(),
        "you have to be able to hand it over"
    );
}

/// A trade leaves both of them thinking better of the other.
#[test]
fn a_trade_is_a_good_turn_on_both_sides() {
    let mut simulation = two_people();

    give(&mut simulation, 0, "wood", 40);
    give(&mut simulation, 1, "stone", 40);

    let me = simulation.population.agents[0].id;
    let them = simulation.population.agents[1].id;

    let bond = |simulation: &Simulation, who: usize, about: uuid::Uuid| {
        simulation.population.agents[who]
            .relationships
            .get_relationship(&about)
            .map(|bond| bond.bond_strength)
            .unwrap_or(0.0)
    };

    let before = (bond(&simulation, 0, them), bond(&simulation, 1, me));

    simulation.execute_action(&Action::Trade { with: them }, 0);

    let after = (bond(&simulation, 0, them), bond(&simulation, 1, me));

    assert!(
        after.0 > before.0,
        "the one who proposed it thinks better of the other: {:.2} against {:.2}",
        after.0,
        before.0
    );
    assert!(
        after.1 > before.1,
        "and so does the other: {:.2} against {:.2}",
        after.1,
        before.1
    );
}

// --------------------------------------------------------------------------
// Giving
// --------------------------------------------------------------------------

/// A gift costs the giver and gets nothing back.
#[test]
fn a_gift_costs_the_giver() {
    let mut simulation = two_people();

    give(&mut simulation, 0, "wood", 40);
    give(&mut simulation, 1, "stone", 40);

    let them = simulation.population.agents[1].id;

    let result = simulation.execute_action(&Action::GiveTo { to: them }, 0);
    assert!(result.success, "handing it over works: {:?}", result.message);

    assert!(
        simulation.population.agents[0].how_many_i_have("wood") < 40,
        "the giver is lighter"
    );
    assert!(
        simulation.population.agents[1].how_many_i_have("wood") > 0,
        "and the other has it"
    );
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("stone"),
        0,
        "and nothing came back"
    );
}

/// And is worth more to the bond than a trade, because it leaves somebody
/// owing.
#[test]
fn a_gift_is_worth_more_than_a_trade() {
    fn bond_after(what: &dyn Fn(uuid::Uuid) -> Action) -> f32 {
        let mut simulation = two_people();
        give(&mut simulation, 0, "wood", 40);
        give(&mut simulation, 1, "stone", 40);

        let me = simulation.population.agents[0].id;
        let them = simulation.population.agents[1].id;

        simulation.execute_action(&what(them), 0);

        simulation.population.agents[1]
            .relationships
            .get_relationship(&me)
            .map(|bond| bond.bond_strength)
            .unwrap_or(0.0)
    }

    let traded = bond_after(&|with| Action::Trade { with });
    let given = bond_after(&|to| Action::GiveTo { to });

    assert!(
        given > traded,
        "a gift should count for more than a square deal: {given:.2} against {traded:.2}"
    );
}

/// Nothing they want is nothing to give.
#[test]
fn nobody_gives_away_what_the_other_has_plenty_of() {
    let mut simulation = two_people();

    give(&mut simulation, 0, "wood", 40);
    give(&mut simulation, 1, "wood", 40);

    let them = simulation.population.agents[1].id;
    let result = simulation.execute_action(&Action::GiveTo { to: them }, 0);

    assert!(
        !result.success,
        "he has plenty of sticks: {:?}",
        result.message
    );
}
