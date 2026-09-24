// src/analytics/tests/rotation_tests.rs
//! Tests for the crop that pays the ground rent.
//!
//! "Legumes: Beans, peas, lentils, chickpeas. Cover crops / green manure:
//! clover, vetch, alfalfa, rye."
//!
//! On the soil ladder: a whole crop's worth taken off a field wears it down a
//! rung, and a bean crop that comes to maturity builds it back up one. A pod
//! crop takes nothing from the ground it grows in, so what comes off one does
//! not wear it. That one fact is what makes a rotation worth knowing - a bean
//! crop after a cereal holds a field where it was. See ISSUES_FOUND #246.
//!
//! This file used to hold the same ideas as nutrient arithmetic: what a pod
//! crop fixed per unit grown against what a cereal drew, a pool that took no
//! more once full. The rungs have taken over from the pool.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::soil::{Field, SoilGrade, A_SEASON};
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// A field of ordinary loam broken out of open plains, at the given grade,
/// with a crop standing on it and one person in it.
fn a_field_of(crop: ResourceType, grade: SoilGrade) -> (Simulation, Position) {
    let mut world = World::new(WorldConfig::default().with_size(40, 40));
    world.animals.get_all_mut().clear();
    world.resources.clear();

    let here = Position::new(20, 20);
    if let Some(tile) = world.grid.get_tile_mut(&here) {
        tile.terrain = crate::world::Terrain::new(TerrainType::Plains);
        tile.soil.weeds = 0.0;
        tile.soil.pests = 0.0;
    }
    assert!(world.grid.break_ground(&here, 0), "open plains will break");
    world
        .grid
        .field_at_mut(&here)
        .expect("just broken")
        .grade = grade;

    let mut node = ResourceNode::new(crop, here.clone(), 60);
    node.amount = 0;
    world.resources.push(node);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (20, 20, 0);
    (simulation, here)
}

fn the_grade(simulation: &Simulation, where_it_is: &Position) -> SoilGrade {
    simulation
        .world
        .grid
        .field_at(where_it_is)
        .expect("a field")
        .grade
}

/// Grow what is on the field until it is ripe, in the weeks both crops bear.
fn grow_it_until_it_is_ripe(simulation: &mut Simulation) {
    let midsummer = crate::environment::seasons::Season::Summer.first_day()
        + crate::environment::seasons::DAYS_PER_SEASON
        - 10;
    simulation.world.climate.calendar.day_of_year = midsummer;

    for _ in 0..20_000 {
        simulation.world.take_a_turn();
        if simulation.world.resources[0].ripe_stand > 0 {
            return;
        }
    }
    panic!("the crop never ripened");
}

/// Bring the harvest in by hand - all a hand can take, which is three
/// quarters of it - and let a day go by for the ground to reckon it.
fn bring_the_harvest_in(simulation: &mut Simulation) {
    let the_harvest = simulation.world.resources[0].what_can_be_taken();
    assert!(the_harvest > 0, "a ripe crop is there to be taken");
    simulation.world.resources[0].harvest(the_harvest);
    assert!(!simulation.world.resources[0].anything_to_take(), "and then the harvest is in");
    for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY {
        simulation.world.take_a_turn();
    }
}

// --- the crop that gives back -----------------------------------------------

/// Cropping wheat costs the ground a rung; cropping beans gains it one.
///
/// The same ground, grown until ripe and harvested by the same hand, and the
/// only difference is which plant was standing on it.
#[test]
fn a_pod_row_leaves_the_ground_better_than_it_found_it() {
    let (mut wheat, field) = a_field_of(ResourceType::Grain, SoilGrade::Ordinary);
    let (mut beans, bean_field) = a_field_of(ResourceType::Legumes, SoilGrade::Ordinary);

    grow_it_until_it_is_ripe(&mut wheat);
    bring_the_harvest_in(&mut wheat);

    grow_it_until_it_is_ripe(&mut beans);
    bring_the_harvest_in(&mut beans);

    assert_eq!(
        the_grade(&wheat, &field),
        SoilGrade::Depleted,
        "a whole crop of wheat off ordinary ground takes it down a rung"
    );
    assert_eq!(
        the_grade(&beans, &bean_field),
        SoilGrade::Rich,
        "and a bean crop brings it up one, and taking it off costs nothing"
    );
}

/// A cereal after beans, or beans after a cereal, and the field is where it
/// started. That is the whole of a two-course rotation.
#[test]
fn a_year_of_beans_answers_a_year_of_wheat() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    let started = field.grade;

    field.the_harvest_is_in(false, 10);
    assert_eq!(field.grade, started.poorer(), "the wheat took a rung");

    field.a_bean_crop_came_in(20);
    assert_eq!(field.grade, started, "and the beans gave it back");
}

/// A bean crop counts once, when it ripens: a ripe crop does not grow, so a
/// stand nobody picks cannot ripen again and again and climb to the top on
/// its own.
#[test]
fn a_bean_crop_standing_there_counts_once() {
    let (mut beans, field) = a_field_of(ResourceType::Legumes, SoilGrade::Depleted);

    grow_it_until_it_is_ripe(&mut beans);
    for _ in 0..(30 * crate::environment::seasons::PLANNING_PERIODS_PER_DAY) {
        beans.world.take_a_turn();
    }

    assert_eq!(the_grade(&beans, &field), SoilGrade::Ordinary, "one crop, one rung");
}

/// The top of the ladder is the top: nobody makes an infinite larder out of
/// one tile by leaving beans on it, or by tipping muck on it.
#[test]
fn ground_that_is_at_the_top_takes_no_more() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    field.grade = SoilGrade::the_top();

    field.a_bean_crop_came_in(10);
    assert_eq!(field.grade, SoilGrade::the_top(), "beans cannot lift it past the top");

    assert!(
        !field.somebody_mucked_it(Field::ENOUGH_MUCK_FOR_A_RUNG * 10.0, 20),
        "and muck on it will come to nothing"
    );
    assert!(field.mucked_ready_at.is_none());
}

/// One answer to which crops feed the ground, and nothing else claims to.
#[test]
fn only_a_pod_crop_feeds_the_ground() {
    assert!(ResourceType::Legumes.feeds_the_ground());

    for other in ResourceType::all() {
        if other == ResourceType::Legumes {
            continue;
        }
        assert!(
            !other.feeds_the_ground(),
            "{other:?} claims to feed the ground and nothing has been written \
             to make it do so"
        );
    }

    // And it is a crop in every other respect - food, grown, and it comes
    // back. A ground-feeder that fell through those lists would be deleted
    // out of season the way the mast was; see ISSUES_FOUND.md #164.
    assert!(ResourceType::Legumes.is_it_food());
    assert!(ResourceType::Legumes.is_it_grown());
    assert!(ResourceType::Legumes.how_fast_it_comes_back() > 0.0);
}

// --- green manure -----------------------------------------------------------

/// Turning a standing crop under is muck, and comes to a rung a season later.
#[test]
fn ploughing_a_crop_in_feeds_the_ground_it_stood_on() {
    let (mut simulation, field) = a_field_of(ResourceType::Legumes, SoilGrade::Depleted);

    // Something worth turning under
    if let Some(node) = simulation.world.resources.first_mut() {
        node.amount = 40;
    }

    let result = simulation.execute_action(&crate::environment::Action::TillSoil, 0);
    assert!(result.success, "{}", result.message.clone().unwrap_or_default());

    // The crop is gone, because that is what it cost.
    assert!(
        !simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.position == field),
        "what was turned under is not still standing there to be eaten"
    );

    // The ground is broken, and has a season's rot ahead of it.
    let ready = simulation
        .world
        .grid
        .field_at(&field)
        .and_then(|field| field.mucked_ready_at)
        .expect("forty units of vetch under the plough is a rung's worth of muck");
    assert_eq!(the_grade(&simulation, &field), SoilGrade::Depleted, "not yet");

    simulation.world.grid.a_day_goes_by_for_the_fields(ready);
    assert_eq!(
        the_grade(&simulation, &field),
        SoilGrade::Ordinary,
        "and a season on, the crop is in the ground"
    );
    assert!(ready >= A_SEASON, "which took a season");
}

/// A berry bush is not a green manure. Breaking ground that carries something
/// which does not feed the ground is still refused.
#[test]
fn a_hungry_crop_is_not_turned_under() {
    let (mut simulation, _field) = a_field_of(ResourceType::Food, SoilGrade::Depleted);
    if let Some(node) = simulation.world.resources.first_mut() {
        node.amount = 40;
    }

    let result = simulation.execute_action(&crate::environment::Action::TillSoil, 0);
    assert!(
        !result.success,
        "a bush somebody could be eating off is not ploughed in"
    );
    assert_eq!(simulation.world.resources.len(), 1, "and it is still standing");
}

// --- the reading ------------------------------------------------------------

/// A man with both in his pack puts pods in tired ground and wheat in good.
#[test]
fn tired_ground_gets_the_pod_row() {
    use crate::agents::{Agent, InventoryItem};

    let mut agent = Agent::new(AgentConfig::default());
    agent.inventory.add_item(InventoryItem::new_with_weight("grain".to_string(), 10, 0.1));
    agent.inventory.add_item(InventoryItem::new_with_weight("legumes".to_string(), 10, 0.1));

    let on_good_ground = Simulation::what_this_one_would_sow(&agent, 0.9);
    let on_tired_ground = Simulation::what_this_one_would_sow(&agent, 0.1);

    assert_eq!(
        on_good_ground,
        ResourceType::Grain,
        "good ground carries a harvest and a hungry people wants one"
    );
    assert_eq!(
        on_tired_ground,
        ResourceType::Legumes,
        "and worked-out ground is where a pod row is worth more than a thin crop"
    );
}

/// It is a reading and not a rule: a man who has sown wheat here three years
/// running and carried it home each time goes on sowing wheat.
#[test]
fn a_settled_opinion_still_beats_the_reading() {
    use crate::agents::{Agent, InventoryItem};

    let mut agent = Agent::new(AgentConfig::default());
    agent.inventory.add_item(InventoryItem::new_with_weight("grain".to_string(), 10, 0.1));
    agent.inventory.add_item(InventoryItem::new_with_weight("legumes".to_string(), 10, 0.1));

    for _ in 0..12 {
        agent.lessons.record_particular("sow:grain", true);
        agent.lessons.record_particular("sow:legumes", false);
    }

    assert_eq!(
        Simulation::what_this_one_would_sow(&agent, 0.1),
        ResourceType::Grain,
        "what a man has actually seen work outweighs the look of the ground"
    );
}
