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

// --------------------------------------------------------------------------
// A spring is a flow, not a barrel
//
// Raising the rate (ISSUES #46) was only half of it. Water was still a stock
// with a `max_amount` that drinking decremented, so a big enough camp could
// still draw one down inside a single pass. A spring does not hold a set
// amount of water: it recharges, out of a catchment that is not in this model,
// and what limits what you can draw from it in an afternoon is its rate.
// Twelve people cannot drain a decent spring. See ISSUES_FOUND #53.
// --------------------------------------------------------------------------

/// A spring cannot be drunk below what it puts out.
#[test]
fn a_spring_cannot_be_drunk_below_what_it_puts_out() {
    use crate::world::{Position, ResourceNode};

    let mut spring = ResourceNode::new(ResourceType::Water, Position::new(0, 0), 400);
    spring.flow = 20.0;

    let all_of_it = spring.harvest(10_000);

    assert_eq!(all_of_it, 380, "he draws everything above the springline");
    assert_eq!(spring.amount, 20, "and the spring goes on running");

    let and_again = spring.harvest(10_000);

    assert_eq!(and_again, 0, "there is nothing more to be had out of it today");
    assert_eq!(spring.amount, 20, "and it is still there tomorrow");
}

/// Everything that is a stock still is one. A berry patch stripped bare is
/// bare and a seam mined out is mined out; that is what those things are.
#[test]
fn everything_that_is_a_stock_can_still_be_taken_to_nothing() {
    use crate::world::{Position, ResourceNode};

    for what in [ResourceType::Food, ResourceType::Clay, ResourceType::Wood] {
        let mut node = ResourceNode::new(what, Position::new(0, 0), 40);
        node.flow = 20.0;

        assert_eq!(node.harvest(10_000), 40, "{what:?} is a stock");
        assert_eq!(node.amount, 0, "{what:?} strips bare");
    }
}

/// And a spring knows its own rate before anybody has drunk from it. The
/// regeneration pass sets this and does not run until the tenth tick, which
/// is ten ticks in which the founders could drink one dry.
#[test]
fn a_spring_knows_its_rate_from_the_moment_the_world_is_made() {
    let world = World::new(WorldConfig::default());

    let springs: Vec<f32> = world
        .resources
        .iter()
        .filter(|resource| resource.resource_type == ResourceType::Water)
        .map(|resource| resource.flow)
        .collect();

    assert!(!springs.is_empty(), "a world should have water in it");
    assert!(
        springs.iter().all(|flow| *flow > 0.0),
        "every source should be running before anybody drinks: {springs:?}"
    );
}

/// Which together mean a world cannot lose its water, however many people
/// stand in it.
#[test]
fn a_settlement_cannot_drink_a_world_dry() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..24 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..3000 {
        simulation.tick();
    }

    let emptiest = simulation
        .world
        .resources
        .iter()
        .filter(|resource| resource.resource_type == ResourceType::Water)
        .map(|resource| resource.amount)
        .min()
        .expect("a world should have water in it");

    assert!(
        emptiest > 0,
        "no source anywhere should be drunk to nothing, and the emptiest \
         holds {emptiest}"
    );
}

/// A reach of running water keeps nothing back, because there is nothing to
/// protect: it is full again by morning whatever was taken out of it.
///
/// The first cut of the springline set it to the flow for every source, and a
/// river's flow is larger than its bed — so rivers became undrinkable and the
/// failure rate went up rather than down. See ISSUES_FOUND #53.
#[test]
fn a_river_holds_nothing_back() {
    use crate::world::{Position, ResourceNode};

    let mut river = ResourceNode::new(ResourceType::Water, Position::new(0, 0), 400);
    river.flow = ResourceNode::WHATEVER_WAS_DRAWN;

    assert_eq!(
        river.what_can_be_taken(),
        400,
        "a man at a river can drink his fill"
    );
    assert_eq!(river.harvest(10_000), 400, "and does");
}

/// A spring down to its springline still gives a drink. The pool is what has
/// gathered; the springline is what is arriving, and a man kneeling at a
/// running spring drinks it as it comes.
///
/// This is the difference between a spring having a rate and a spring having a
/// closing time. Without it, "Gather: Resource source was empty" became the
/// fourth largest refusal in the model — a strange thing to be able to say
/// about a running spring — and the flow model cost half a point of failure
/// rate on its own. See ISSUES_FOUND #53.
#[test]
fn nobody_is_turned_away_from_a_running_spring() {
    use crate::world::{Position, ResourceNode};

    let mut spring = ResourceNode::new(ResourceType::Water, Position::new(0, 0), 400);
    spring.flow = 20.0;
    spring.harvest(10_000);

    assert_eq!(spring.what_can_be_taken(), 0, "the pool is down to its springline");
    assert_eq!(spring.a_mouthful_from_the_flow(), 1, "and it is still running");

    let pool = spring.amount;
    for _ in 0..50 {
        assert_eq!(
            spring.a_mouthful_from_the_flow(),
            1,
            "a queue at a spring all get a drink"
        );
    }
    assert_eq!(spring.amount, pool, "and none of it comes out of the pool");
}

/// A source that really has nothing in it — a seep frozen solid in February —
/// gives nothing, and should not pretend otherwise.
#[test]
fn a_spring_that_is_not_running_gives_nothing() {
    use crate::world::{Position, ResourceNode};

    let mut frozen = ResourceNode::new(ResourceType::Water, Position::new(0, 0), 400);
    frozen.flow = 0.0;
    frozen.amount = 0;

    assert_eq!(frozen.a_mouthful_from_the_flow(), 0, "there is nothing there");
}
