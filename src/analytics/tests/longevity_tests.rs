// src/analytics/tests/longevity_tests.rs
//! Tests for a settlement that has to last.
//!
//! Every population tested was gone by thirty thousand ticks. Two things were
//! killing them, and neither showed up in the eight-thousand-tick runs
//! everything else is measured over:
//!
//! - Nearly half of everyone ever born died before growing up. Children have
//!   no clothing of their own and cannot get any, so they ran two or three
//!   degrees colder than the adults beside them and died of it.
//! - Water was consumed and never came back. Every drink took a unit out of
//!   the world for good, and a lake drunk dry was deleted, so a world lost
//!   more than half its water in fifteen thousand ticks and the people
//!   drinking from it died of thirst and then of hunger.

use crate::agents::{AgentConfig, LifeStage, Population};
use crate::analytics::Simulation;
use crate::world::{ResourceType, World, WorldConfig};

/// Drinking does not empty the rivers.
#[test]
fn water_is_not_used_up() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    let sources = |simulation: &Simulation| -> Vec<(u32, u32)> {
        simulation
            .world
            .resources
            .iter()
            .filter(|resource| resource.resource_type == ResourceType::Water)
            .map(|resource| (resource.amount, resource.max_amount))
            .collect()
    };

    assert!(!sources(&simulation).is_empty(), "a world should have water in it");

    for _ in 0..6000 {
        simulation.tick();
    }

    let after = sources(&simulation);
    let how_many = after.len();

    // What is being asserted is that a settlement does not drink its own
    // country dry, and it is asserted source by source rather than on a
    // total. The total is a bad measure: it counts the sea and the salt
    // marshes, which nobody drinks and which therefore always read full, and
    // it cannot tell a river drawn to nothing from a puddle that was always
    // small.
    //
    // A puddle *can* be drunk down, and should be - that is why a village
    // sits on a spring and not on a pond. What must not happen is what used
    // to: **eight of a world's twenty-one sources drawn to two units out of
    // four hundred and left there**, with "no water sources nearby" the
    // single largest refusal in the model. See ISSUES_FOUND #46.
    let drawn_low = after
        .iter()
        .filter(|(amount, max)| (*amount as f32) < *max as f32 * 0.10)
        .count();

    assert!(
        drawn_low * 3 < how_many,
        "a settlement should not drink its country dry, and {drawn_low} of \
         {how_many} sources are all but empty"
    );

    let still_deep = after
        .iter()
        .filter(|(amount, max)| (*amount as f32) >= *max as f32 * 0.50)
        .count();

    assert!(
        still_deep * 2 >= how_many,
        "most of the water should still be there, and only {still_deep} of \
         {how_many} sources are half full"
    );
}

/// And a river is a flow rather than a stock: whatever is drawn comes down
/// from upstream, which is what the comment beside these numbers has always
/// said and what they did not do.
#[test]
fn a_river_cannot_be_drunk_dry() {
    use crate::world::{Position, ResourceNode, TerrainType};

    let river = ResourceNode::new(ResourceType::Water, Position::new(0, 0), 400);

    let in_a_dry_spell = river.water_inflow(TerrainType::Riverbank, 0.0, false);

    assert!(
        in_a_dry_spell >= river.max_amount as f32,
        "a reach of running water replaces what is taken out of it, and this \
         one gives back {in_a_dry_spell} against a bed that holds {}",
        river.max_amount
    );
}

/// A spring will carry a camp. A pond will not, and that is the difference
/// between them.
#[test]
fn a_spring_gives_more_than_a_pond() {
    use crate::world::{Position, ResourceNode, TerrainType};

    let source = ResourceNode::new(ResourceType::Water, Position::new(0, 0), 400);
    let dry = 0.0;

    let spring = source.water_inflow(TerrainType::Hills, dry, false);
    let seep = source.water_inflow(TerrainType::Wetland, dry, false);
    let pond = source.water_inflow(TerrainType::Meadow, dry, false);

    assert!(spring > seep, "a spring runs harder than a seep: {spring} vs {seep}");
    assert!(seep > pond, "a seep runs harder than standing water: {seep} vs {pond}");

    // And a spring has to be worth camping on, which means giving back more
    // between two passes of the resource tick than a settlement drinks in
    // that time. A drink is a unit or two and a pass comes round every ten
    // ticks; the first cut of this gave back 1.5, which is a twentieth of
    // what a camp takes.
    assert!(
        spring >= 12.0,
        "a spring should carry a settlement, and this one gives {spring} a pass"
    );
}

/// A child beside an adult is kept warm by them.
#[test]
fn the_young_are_kept_warm_by_the_adults_around_them() {
    fn coldest_child(with_adults: bool) -> f32 {
        let world = World::new(WorldConfig::default());
        let mut population = Population::new();
        for _ in 0..6 {
            population.spawn_agent(AgentConfig::default());
        }

        let mut simulation = Simulation::new(world, population);

        // Half of them are small children; the rest are grown, or are not
        // there at all
        for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
            agent.state.position = (25, 25, 0);
            if index < 3 {
                agent.state.age = 800;
            } else {
                agent.state.age = 4000;
            }
            agent.update_life_stage();
        }

        if !with_adults {
            simulation.population.agents.truncate(3);
        }

        for _ in 0..600 {
            // Keep them together: this is about who is standing beside whom
            for agent in &mut simulation.population.agents {
                agent.state.position = (25, 25, 0);
            }
            simulation.tick();
        }

        simulation
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                matches!(agent.state.life_stage, LifeStage::Child | LifeStage::Infant)
            })
            .map(|agent| agent.body_temperature.current)
            .fold(f32::INFINITY, f32::min)
    }

    let looked_after = coldest_child(true);
    let left_alone = coldest_child(false);

    assert!(
        looked_after >= left_alone,
        "a child beside adults should be no colder than one on its own: \
         {looked_after:.1} against {left_alone:.1}"
    );
}

/// A settlement is still there after thirty thousand ticks.
///
/// Ignored by default: it runs thirty thousand ticks, which takes most of a
/// minute on its own against a suite that otherwise finishes in seconds, and
/// the answer is a probability rather than a fact. Run it with
/// `cargo test --release -- --ignored a_settlement_lasts`.
///
/// Measured over twelve independent worlds on the commit that added this,
/// nine were still inhabited at thirty thousand ticks and thirteen of sixteen
/// at twenty thousand. Before the two fixes above, three of three were empty -
/// at 15,994, 18,253 and 26,907 ticks. Once the calendar turned, eleven of
/// twelve were still inhabited at thirty thousand, with 77.7 people on average
/// and a high-water mark of 141. A single world is therefore still worth
/// running by hand and not asserting on: some of them die of their own accord,
/// having first grown past what the land will carry.
#[test]
#[ignore]
fn a_settlement_lasts_thirty_thousand_ticks() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..30000 {
        simulation.tick();
    }

    let alive = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .count();

    assert!(
        alive > 0,
        "the settlement should still be there after thirty thousand ticks"
    );
}

/// A settlement is still growing children into adults well past the point it
/// used to stop.
///
/// The collapse showed up first as births simply stopping: in one traced run
/// the count froze at thirty-three around twelve thousand ticks and never
/// moved again while the last adults aged out. Nearly half of everyone born
/// had died before growing up, so there was never a second generation to take
/// over.
#[test]
fn a_settlement_still_raises_children_late_on() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..9000 {
        simulation.tick();
    }

    let grown_here = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .filter(|agent| agent.state.age < 6500)
        .count();

    assert!(
        grown_here > 0,
        "nine thousand ticks in, the settlement should hold people born into it"
    );
}

/// Where a water source sits decides what refills it.
///
/// A stream coming off the hills is fed by the spring behind it; a pool on
/// open ground lives on the rain. Both were a flat rate before, and before
/// that they were not refilled at all.
#[test]
fn water_is_fed_by_where_it_lies() {
    use crate::world::{Position, ResourceNode, TerrainType};

    let pool = ResourceNode::new(ResourceType::Water, Position::new(10, 10), 100);

    let dry = 0.0;
    let wet = 1.0;

    let river = pool.water_inflow(TerrainType::Water, dry, false);
    let spring = pool.water_inflow(TerrainType::Mountain, dry, false);
    let open = pool.water_inflow(TerrainType::Plains, dry, false);

    assert!(
        river > spring && spring > open,
        "a river should outrun a spring, and a spring open water: {river} {spring} {open}"
    );

    assert!(
        pool.water_inflow(TerrainType::Plains, wet, false) > open,
        "rain should top up a pool on open ground"
    );

    assert!(
        pool.water_inflow(TerrainType::Mountain, dry, true) < spring,
        "a frozen spring should give up less"
    );

    // And nothing but water is fed this way
    let berries = ResourceNode::new(ResourceType::Food, Position::new(10, 10), 100);
    assert_eq!(berries.water_inflow(TerrainType::Water, wet, false), 0.0);
}
