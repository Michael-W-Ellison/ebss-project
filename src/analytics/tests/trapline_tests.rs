// src/analytics/tests/trapline_tests.rs
//! The trapline, and the calendar it is kept on.
//!
//! Trapping is the one food source that does not stop when the hedgerows do:
//! the small life is thinned by winter - `what_a_hectare_of_this_is_worth`
//! takes it to 0.45 - but it is still there, and a snare set in a wood in
//! February catches. What it was not doing was delivering any of it, because
//! the two rates that govern a trapline were written for a twelve-tick day
//! and this world keeps a forty-eight-tick one.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::environment::small_life::SmallLife;
use crate::environment::Action;
use crate::world::{Position, ResourceType, World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

// --------------------------------------------------------------------------
// The calendar
// --------------------------------------------------------------------------

/// A snare on full ground takes about a fifth of a chance a day, so there is
/// something in it inside four or five days.
///
/// The constant used to be written per tick, at a value whose own docstring
/// said "twelve ticks to the day". `TICKS_PER_DAY` is 48, so the snare took
/// four fifths of a chance a day and the sentence beside it was wrong by four
/// times.
#[test]
fn a_snare_is_set_on_the_day_this_world_actually_keeps() {
    let a_day = SmallLife::WHAT_A_SNARE_TAKES_ON_FULL_GROUND * TICKS_PER_DAY as f32;

    assert!(
        (a_day - SmallLife::WHAT_A_SNARE_TAKES_ON_FULL_GROUND_IN_A_DAY).abs() < 1e-6,
        "a snare's rate over a day of this world's ticks should be the rate \
         its own docstring states: {a_day} against {}",
        SmallLife::WHAT_A_SNARE_TAKES_ON_FULL_GROUND_IN_A_DAY
    );

    // "Something in the snare inside four or five days": the chance of still
    // being empty after five days should have fallen below a half.
    let still_empty = (1.0 - SmallLife::WHAT_A_SNARE_TAKES_ON_FULL_GROUND)
        .powi(5 * TICKS_PER_DAY as i32);
    assert!(
        still_empty < 0.5,
        "five days on, a snare on full ground should more likely than not \
         have caught: {still_empty:.3} still empty"
    );
}

/// And a catch in a settled country lasts most of a week before something
/// finds it, which is what makes a line worth keeping at all.
#[test]
fn a_catch_in_a_quiet_country_lasts_most_of_a_week() {
    let a_day = SmallLife::WHAT_A_QUIET_COUNTRY_TAKES * TICKS_PER_DAY as f32;
    assert!(
        (a_day - SmallLife::WHAT_A_QUIET_COUNTRY_TAKES_IN_A_DAY).abs() < 1e-6,
        "the robbing rate over a day should be the rate its docstring states"
    );

    // Still there after a day, which is what a daily round needs.
    let after_a_day =
        (1.0 - SmallLife::WHAT_A_QUIET_COUNTRY_TAKES).powi(TICKS_PER_DAY as i32);
    assert!(
        after_a_day > 0.8,
        "a man who walks his line once a day should find most of what he \
         caught still in it: {after_a_day:.3} survive the day"
    );

    // And most of a week is where it goes, not a day and a half.
    let after_a_week =
        (1.0 - SmallLife::WHAT_A_QUIET_COUNTRY_TAKES).powi(7 * TICKS_PER_DAY as i32);
    assert!(
        after_a_week > 0.3 && after_a_week < 0.7,
        "a week is where a catch is about half gone in a country with plenty \
         in it: {after_a_week:.3}"
    );
}

/// The pinch stays at the other end of the scale: a trapped-out country takes
/// the catch inside a turn.
#[test]
fn a_hungry_country_still_takes_it_off_you() {
    assert!(
        SmallLife::WHAT_A_HUNGRY_COUNTRY_TAKES > SmallLife::WHAT_A_QUIET_COUNTRY_TAKES * 20.0,
        "trapping a ground out has to cost more than a full wood does"
    );
}

// --------------------------------------------------------------------------
// What comes out of it
// --------------------------------------------------------------------------

/// A catch that will not fit in the pack stays in the snare.
///
/// `add_item` returns whether the thing went in and `going_round_the_line`
/// threw that answer away, so a man with a full pack took the catch out of
/// the snare and it stopped existing - the same defect as the store in #180,
/// at the call site that entry named and did not fix.
#[test]
fn a_catch_that_will_not_fit_stays_in_the_snare() {
    let mut simulation = one_person();
    let here = (25, 25);

    simulation.world.snares.push(crate::environment::small_life::Snare {
        at: here,
        set_by: simulation.population.agents[0].id,
        set_at: 0,
        caught_at: Some(1),
    });

    // Fill the pack with something that is not worth less than food, so
    // nothing can be shed to make room.
    {
        let agent = &mut simulation.population.agents[0];
        let capacity = agent.inventory.max_weight;
        let mut ballast =
            InventoryItem::new_with_weight("meat".to_string(), 1, capacity);
        ballast.food_data = simulation
            .food_database
            .create_food_data(&crate::world::ItemType::Meat, 0);
        assert!(agent.inventory.add_item(ballast), "the fixture should fill it");
    }

    let result = simulation.execute_action(&Action::CheckSnares, 0);

    assert!(
        !result.success,
        "a man with no room should be told so, not quietly handed nothing"
    );
    assert!(
        simulation.world.snares[0].is_holding_something(),
        "and what he could not carry is still in the snare, where it will be \
         when he comes back with room"
    );
}

/// And when there is room, the catch comes home.
#[test]
fn a_catch_he_can_carry_comes_home() {
    let mut simulation = one_person();
    let here = (25, 25);

    simulation.world.snares.push(crate::environment::small_life::Snare {
        at: here,
        set_by: simulation.population.agents[0].id,
        set_at: 0,
        caught_at: Some(1),
    });

    let before = simulation.population.agents[0].inventory.count_item("meat");
    let result = simulation.execute_action(&Action::CheckSnares, 0);

    assert!(result.success, "an empty pack and a full snare: {result:?}");
    assert!(
        simulation.population.agents[0].inventory.count_item("meat") > before,
        "the catch should be in the pack"
    );
    assert!(
        !simulation.world.snares[0].is_holding_something(),
        "and out of the snare"
    );
}

// --------------------------------------------------------------------------
// The hedgerow that is not bearing
// --------------------------------------------------------------------------

/// A man is not sent to a bush that cannot be bearing today.
///
/// `known_resources` holds every patch anybody ever walked past, for ever,
/// and nothing asked the calendar of it - so from the first frost a hungry
/// man was still being sent to the bramble he found in September. Measured
/// over twelve worlds: `Gather` is refused 14.6 times a thousand person-ticks
/// in winter and 0.0 in every other season.
#[test]
fn a_bush_out_of_season_is_not_somewhere_to_go_for_supper() {
    use crate::environment::seasons::{DAYS_PER_SEASON, Season};

    // Fruit runs deep summer to late autumn; it is not bearing in winter.
    let midwinter = DAYS_PER_SEASON * 3 + DAYS_PER_SEASON / 2;
    assert_eq!(Season::from_day_of_year(midwinter), Season::Winter);

    assert!(
        !ResourceType::Food.is_it_bearing(midwinter),
        "there is no wild fruit in midwinter, which is the whole point of a store"
    );

    let mut simulation = one_person();
    simulation.world.climate.calendar.day_of_year = midwinter;

    // He remembers a bramble a long way off, and nothing else.
    let bramble = Position::new(40, 40);
    simulation.population.agents[0]
        .exploration_knowledge
        .known_resources
        .insert(bramble, ResourceType::Food);

    let agent = &simulation.population.agents[0];
    let found = simulation.known_source_position(
        agent,
        agent.state.position,
        crate::agents::senses::ScentType::Food,
        crate::core::memory::SpatialMemoryType::Food,
    );

    assert!(
        found.is_none(),
        "a bramble that cannot be bearing is not a place to walk to in \
         February: {found:?}"
    );

    // And in its own season it is.
    let autumn = DAYS_PER_SEASON * 2 + DAYS_PER_SEASON / 2;
    assert!(ResourceType::Food.is_it_bearing(autumn));
    simulation.world.climate.calendar.day_of_year = autumn;
    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.known_source_position(
            agent,
            agent.state.position,
            crate::agents::senses::ScentType::Food,
            crate::core::memory::SpatialMemoryType::Food,
        ),
        Some((bramble.x, bramble.y, agent.state.position.2)),
        "and in October he walks to it"
    );
}
