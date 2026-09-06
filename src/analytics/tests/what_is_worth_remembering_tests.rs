// src/analytics/tests/what_is_worth_remembering_tests.rs
//! How long a place stays known, and what it costs to forget one.
//!
//! Every settlement emptied between day 315 and day 350 with food still in the
//! ground. The reason was not the store, the appetite, or the calorie tables:
//! it was that **nobody could remember where the store was**.
//!
//! `SpatialMemory::decay` took a flat thousandth of confidence a tick with no
//! notion that some places matter more than others, and
//! `batch_decay_and_prune` - the path that is actually live, since
//! `batch_decay` defaults to true - had a second copy of that rule which
//! multiplied by `prune_interval` on top of the elapsed time. That
//! double-counts, and with the default interval of a hundred it came to a
//! tenth of confidence per tick elapsed: **anywhere a person had not looked in
//! the last four hours was gone**.

use crate::core::memory::{Memory, MemoryImportance, SpatialMemory, SpatialMemoryType};
use crate::environment::seasons::TICKS_PER_DAY;

/// A store is the one place a person does not forget.
#[test]
fn the_store_outlasts_the_winter_it_was_laid_down_for() {
    let lean = crate::agents::provision::how_long_the_land_gives_nothing();
    let mut buried = SpatialMemory::new(SpatialMemoryType::Storage, (10, 10, 0), 0);

    buried.forget_a_little(lean * TICKS_PER_DAY);

    assert!(
        buried.confidence > 0.3,
        "a man who buries food in October knows where it is in February: \
         {lean} days on, confidence {:.2}",
        buried.confidence
    );
}

/// A bush is not.
#[test]
fn a_bush_somebody_walked_past_is_forgotten_in_a_fortnight() {
    let mut noticed = SpatialMemory::new(SpatialMemoryType::Food, (10, 10, 0), 0);

    noticed.forget_a_little(10 * TICKS_PER_DAY);
    assert!(
        noticed.confidence > 0.3,
        "ten days is not long enough to forget a berry patch: {:.2}",
        noticed.confidence
    );

    noticed.forget_a_little(20 * TICKS_PER_DAY);
    assert!(
        noticed.confidence <= 0.3,
        "and twenty days is: {:.2}",
        noticed.confidence
    );
}

/// The two paths through forgetting agree.
///
/// `Memory::tick` decays either every tick or in batches, and the batch path
/// had its own arithmetic. Whichever way the clock is run, the same elapsed
/// time has to leave the same memory.
#[test]
fn forgetting_in_batches_is_forgetting_at_the_same_rate() {
    use crate::core::memory::MemoryConfig;

    let every_tick = MemoryConfig { batch_decay: false, ..Default::default() };
    let in_batches = MemoryConfig { batch_decay: true, ..Default::default() };

    let mut one = Memory::with_config(every_tick);
    let mut other = Memory::with_config(in_batches);

    one.remember_location(SpatialMemoryType::Food, (5, 5, 0));
    other.remember_location(SpatialMemoryType::Food, (5, 5, 0));

    for _ in 0..(5 * TICKS_PER_DAY) {
        one.tick();
        other.tick();
    }

    let confidence = |memory: &Memory| {
        memory
            .recall_locations(SpatialMemoryType::Food)
            .first()
            .map(|place| place.confidence)
            .unwrap_or(0.0)
    };

    let (a, b) = (confidence(&one), confidence(&other));
    assert!(
        (a - b).abs() < 0.05,
        "one rule, two spellings: {a:.2} against {b:.2}"
    );
    assert!(a > 0.0, "and five days does not empty either of them: {a:.2}");
}

/// What each kind of place is worth remembering.
#[test]
fn a_store_matters_more_than_a_bush() {
    let store = SpatialMemoryType::Storage.how_much_this_matters();
    let bush = SpatialMemoryType::Food.how_much_this_matters();
    let water = SpatialMemoryType::Water.how_much_this_matters();

    assert_eq!(store, MemoryImportance::Critical);
    assert!(
        store.decay_multiplier() < water.decay_multiplier(),
        "a store you made outlasts a spring you noticed"
    );
    assert!(
        water.decay_multiplier() < bush.decay_multiplier(),
        "and a spring outlasts a berry patch, because thirst is the faster death"
    );
}

/// A place looked at and found empty stops being remembered - for water as
/// well as for food.
#[test]
fn standing_somewhere_empty_corrects_the_memory() {
    use crate::agents::{AgentConfig, Population};
    use crate::world::{World, WorldConfig};

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);

    simulation.population.agents[0].state.position = (20, 20, 0);
    for what in [SpatialMemoryType::Water, SpatialMemoryType::Food] {
        simulation.population.agents[0]
            .memory
            .remember_location(what, (20, 20, 0));
    }

    simulation.forget_what_is_not_there(0, SpatialMemoryType::Water);

    assert!(
        simulation.population.agents[0]
            .memory
            .recall_locations(SpatialMemoryType::Water)
            .is_empty(),
        "he is standing on it and there is nothing there"
    );
    assert!(
        !simulation.population.agents[0]
            .memory
            .recall_locations(SpatialMemoryType::Food)
            .is_empty(),
        "and being wrong about the water says nothing about the berries"
    );
}
