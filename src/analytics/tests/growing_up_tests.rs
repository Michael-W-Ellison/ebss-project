// src/analytics/tests/growing_up_tests.rs
//! What being young costs, and what it is owed.
//!
//! Three things the lifecycle described and nothing read. The age capability
//! curve was written, hung on nothing, and deleted as dead code in the sweep
//! of #93 — which left a six-year-old carrying what a grown man carried,
//! walking as fast, working as hard and hitting as heavily, on a fifth of his
//! food. The supervision bands have been in `LifeStage`'s own doc comment
//! since the lifecycle was written and nothing ever consulted them. And a
//! parent had no way to hand a hungry child anything short of the sacrifice
//! branch, which waits until somebody is starving.

use crate::agents::{Agent, AgentConfig, InventoryItem, LifeStage, Population};
use crate::agents::agent::{what_a_body_this_age_can_do, what_a_body_this_age_eats};
use crate::agents::emotions::{Relationship, RelationshipType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn a_world() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

fn somebody_of(years: u32, at: (i32, i32, i32)) -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.now_this_many_years_old(years);
    agent.state.position = at;
    agent.state.health = 100.0;
    agent.state.energy = 100.0;
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.recalculate_weight();
    agent.take_up_the_cart();
    agent
}

// --------------------------------------------------------------------------
// The capability curve
// --------------------------------------------------------------------------

/// The specification's table: nothing at one, a fifth at four, all of it from
/// sixteen, and falling away after forty.
#[test]
fn the_curve_is_the_table_the_specification_gives() {
    assert_eq!(what_a_body_this_age_can_do(1), 0.0);
    assert_eq!(what_a_body_this_age_can_do(4), 0.2);
    assert_eq!(what_a_body_this_age_can_do(8), 0.4);
    assert_eq!(what_a_body_this_age_can_do(16), 1.0);
    assert_eq!(what_a_body_this_age_can_do(39), 1.0);

    // And it falls away rather than stopping
    let mut last = 1.0;
    for years in 40..=70 {
        let now = what_a_body_this_age_can_do(years);
        assert!(now <= last, "it should only fall after forty: {years}");
        assert!(now > 0.0, "an old man is not nothing at {years}");
        last = now;
    }
}

/// A child carries less than a grown man, which is what it was not doing.
#[test]
fn a_child_carries_less_than_a_grown_man() {
    let child = somebody_of(6, (0, 0, 0));
    let grown = somebody_of(30, (0, 0, 0));

    assert!(
        child.total_carrying_capacity() < grown.total_carrying_capacity(),
        "a six-year-old's two hands are not a man's: {:.1} against {:.1}",
        child.total_carrying_capacity(),
        grown.total_carrying_capacity()
    );
}

/// And walks slower, and hits softer.
#[test]
fn a_child_walks_slower_and_hits_softer() {
    let child = somebody_of(6, (0, 0, 0));
    let grown = somebody_of(30, (0, 0, 0));

    assert!(
        child.movement_speed() < grown.movement_speed(),
        "short legs cover less ground: {:.2} against {:.2}",
        child.movement_speed(),
        grown.movement_speed()
    );
    assert!(
        child.own_strength() < grown.own_strength(),
        "and a child is not his father in a fight: {:.2} against {:.2}",
        child.own_strength(),
        grown.own_strength()
    );
}

/// An old man is worth less than he was, and the table takes him down to
/// exactly half and no further.
///
/// Which puts a man of sixty-five on the same rung as a ten-year-old - and
/// that is the specification's table rather than an accident of it. It is
/// worth writing down rather than asserting away: the curve's floor is where
/// a boy is on his way up, and the two meet there.
#[test]
fn strength_falls_away_after_forty() {
    let young = somebody_of(30, (0, 0, 0));
    let older = somebody_of(60, (0, 0, 0));
    let old = somebody_of(65, (0, 0, 0));
    let boy = somebody_of(10, (0, 0, 0));

    assert!(older.own_strength() < young.own_strength(), "sixty is not thirty");
    assert!(old.own_strength() < older.own_strength(), "and sixty-five is not sixty");
    assert!(older.own_strength() > boy.own_strength(), "sixty still beats ten");

    assert_eq!(what_a_body_this_age_can_do(65), what_a_body_this_age_can_do(10));
    assert_eq!(what_a_body_this_age_can_do(69), 0.5, "and it stops at half");
}

/// The two age tables answer different questions and must not be confused: one
/// is what a body needs, the other is what it can give.
///
/// A six-year-old eats about a third of a grown share and can do about three
/// tenths of a grown day's work, which is roughly a wash. A two-year-old eats
/// a fifth and can do a tenth, which is not — and that is the shape of a
/// child: it costs more than it brings in until it is grown.
#[test]
fn a_child_costs_more_than_it_brings_in() {
    for years in [2u32, 4, 6, 8, 10] {
        let eats = what_a_body_this_age_eats(years);
        let does = what_a_body_this_age_can_do(years);
        assert!(
            does <= eats,
            "at {years} a body does {does:.2} of a grown day's work and eats \
             {eats:.2} of a grown share, so it feeds itself and more"
        );
    }

    // And by sixteen it is the other way about
    assert_eq!(what_a_body_this_age_can_do(16), 1.0);
    assert_eq!(what_a_body_this_age_eats(16), 1.0);
}

// --------------------------------------------------------------------------
// The supervision bands
// --------------------------------------------------------------------------

/// The three bands, and no leash on a grown person.
#[test]
fn the_bands_are_the_ones_the_lifecycle_describes() {
    let leash = Simulation::how_far_from_a_grown_person_this_one_may_be;

    let arms = leash(LifeStage::Infant).expect("under six is kept close");
    let sight = leash(LifeStage::Child).expect("under eleven is kept in sight");
    let errand = leash(LifeStage::Adolescent).expect("under sixteen is kept within a walk");

    assert!(arms < sight, "with a parent is closer than within sight");
    assert!(sight < errand, "within sight is closer than an hour's walk");

    assert!(leash(LifeStage::Adult).is_none(), "a grown person goes where they like");
    assert!(leash(LifeStage::Elderly).is_none());
}

/// A child too far from anybody grown, and out of sight of the camp, heads
/// back.
#[test]
fn a_child_on_its_own_goes_back_to_somebody_grown() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (10, 10, 0)));
    population.agents.push(somebody_of(7, (0, 45, 0)));

    let simulation = Simulation::new(a_world(), population);
    let child = &simulation.population.agents[1];

    // The rule is eyesight of an adult *or* of the camp, so the fixture has to
    // put him out of both - the first cut of this stood him at (40, 40), which
    // is within sight of a longhouse at the middle of the world.
    let leash = Simulation::how_far_from_a_grown_person_this_one_may_be(LifeStage::Child).unwrap();
    assert!(
        !simulation.world.buildings.iter().any(|roof| Simulation::within(
            (child.state.position.0, child.state.position.1),
            (roof.position.x, roof.position.y),
            leash
        )),
        "he has to be out of sight of any roof too"
    );

    let answer = simulation
        .keeping_close_to_somebody_grown(child, child.state.position)
        .expect("he is out of sight of the camp and of anybody grown");

    match answer {
        Action::Move { target } => assert_eq!((target.0, target.1), (10, 10)),
        other => panic!("he should be walking back: {other:?}"),
    }
}

/// And one already beside an adult carries on with its day.
#[test]
fn a_child_beside_an_adult_is_left_alone() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (10, 10, 0)));
    population.agents.push(somebody_of(7, (11, 10, 0)));

    let simulation = Simulation::new(a_world(), population);
    let child = &simulation.population.agents[1];

    assert!(
        simulation
            .keeping_close_to_somebody_grown(child, child.state.position)
            .is_none(),
        "he is standing next to his father"
    );
}

/// A child with no adult left alive is not marched across the map to a corpse.
#[test]
fn a_child_with_nobody_left_is_on_its_own() {
    let mut population = Population::new();
    population.agents.push(somebody_of(7, (40, 40, 0)));
    population.agents.push(somebody_of(9, (10, 10, 0)));

    let simulation = Simulation::new(a_world(), population);
    let child = &simulation.population.agents[0];

    assert!(
        simulation
            .keeping_close_to_somebody_grown(child, child.state.position)
            .is_none(),
        "there is nobody grown to go to"
    );
}

// --------------------------------------------------------------------------
// Feeding a child from a parent's stores
// --------------------------------------------------------------------------

fn a_parent_and_a_hungry_child() -> Simulation {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (10, 10, 0)));
    population.agents.push(somebody_of(7, (11, 10, 0)));

    let parent = population.agents[0].id;
    let child = population.agents[1].id;

    // A pack with food to spare in it
    let database = crate::world::nutrition::FoodDatabase::new();
    let mut supper = InventoryItem::new_with_weight("food".to_string(), 20, 0.5);
    supper.food_data = database.create_food_data(&crate::world::ItemType::Food, 0);
    let _ = population.agents[0].inventory.add_item(supper);

    // Who they are to one another
    population.agents[0]
        .relationships
        .add_relationship(Relationship::new(child, RelationshipType::Child));
    population.agents[1]
        .relationships
        .add_relationship(Relationship::new(parent, RelationshipType::Parent));

    // And the child is hungry
    population.agents[1]
        .drives
        .get_mut(crate::core::DriveType::Hunger)
        .unwrap()
        .value = 0.9;

    Simulation::new(a_world(), population)
}

/// A parent with food to spare feeds a hungry child of their own.
#[test]
fn a_parent_feeds_a_hungry_child() {
    let simulation = a_parent_and_a_hungry_child();
    let parent = &simulation.population.agents[0];

    let fed = simulation
        .a_child_of_mine_to_feed(parent, parent.state.position)
        .expect("his own child is hungry and he has twenty in his pack");

    assert_eq!(fed, simulation.population.agents[1].id);
}

/// And it reaches the turn: the decision layer hands over rather than
/// carrying on with the day.
#[test]
fn feeding_a_child_reaches_the_turn() {
    let simulation = a_parent_and_a_hungry_child();
    let parent = &simulation.population.agents[0];

    let (action, _) = simulation.generate_non_emotional_action(parent, parent.state.position);

    assert!(
        matches!(action, Action::GiveTo { to } if to == simulation.population.agents[1].id),
        "he should be handing his child something: {action:?}"
    );
}

/// A hungry child that is nothing to this agent gets nothing. A gift is one
/// thing and feeding your own is another.
#[test]
fn somebody_elses_hungry_child_is_not_this_ones_business() {
    let mut simulation = a_parent_and_a_hungry_child();
    simulation.population.agents[0].relationships = Default::default();

    let parent = &simulation.population.agents[0];
    assert!(
        simulation
            .a_child_of_mine_to_feed(parent, parent.state.position)
            .is_none(),
        "this branch is about a parent and a child, not about charity"
    );
}

/// And a parent with nothing to spare does not hand over what they have not
/// got. The sacrifice branch is where that decision belongs.
#[test]
fn a_parent_with_an_empty_pack_gives_nothing() {
    let mut simulation = a_parent_and_a_hungry_child();
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();

    let parent = &simulation.population.agents[0];
    assert!(
        simulation
            .a_child_of_mine_to_feed(parent, parent.state.position)
            .is_none()
    );
}

// --------------------------------------------------------------------------
// Against the specification, clause by clause
// --------------------------------------------------------------------------

/// The capability table, read straight off the specification.
#[test]
fn the_capability_table_is_the_specification_verbatim() {
    for (years, out_of_ten) in [
        (2u32, 1),
        (4, 2),
        (6, 3),
        (8, 4),
        (10, 5),
        (12, 6),
        (13, 7),
        (14, 8),
        (15, 9),
        (16, 10),
        (40, 9),
        (50, 8),
        (55, 7),
        (60, 6),
        (65, 5),
    ] {
        assert_eq!(
            what_a_body_this_age_can_do(years),
            out_of_ten as f32 / 10.0,
            "age {years} should be {out_of_ten} out of ten"
        );
    }
}

/// And the food table, likewise - including the fifteenth year, which the
/// specification leaves between "14-15: 90%" and "16+: 100%".
#[test]
fn the_food_table_is_the_specification_verbatim() {
    for (years, share) in [
        (0u32, 0.20),
        (3, 0.20),
        (4, 0.25),
        (5, 0.30),
        (6, 0.35),
        (7, 0.40),
        (8, 0.45),
        (9, 0.50),
        (10, 0.55),
        (11, 0.60),
        (12, 0.70),
        (13, 0.80),
        (14, 0.90),
        (16, 1.00),
        (30, 1.00),
    ] {
        assert!(
            (what_a_body_this_age_eats(years) - share).abs() < 1e-6,
            "age {years} should eat {share} of a grown share, not {}",
            what_a_body_this_age_eats(years)
        );
    }

    // The gap: the last child band runs to the adult boundary rather than a
    // fifteen-year-old eating a full share while doing nine tenths of the work
    assert!((what_a_body_this_age_eats(15) - 0.90).abs() < 1e-6);
}

/// A whole life is 36,288,000 of the specification's ticks, and a year is
/// 518,400 - which are this model's *minutes*, because a turn is a decision
/// and not a minute.
#[test]
fn a_life_is_the_length_the_specification_gives() {
    use crate::environment::seasons::{MINUTES_IN_A_WHOLE_LIFE, MINUTES_PER_YEAR};

    assert_eq!(MINUTES_PER_YEAR, 518_400);
    assert_eq!(MINUTES_IN_A_WHOLE_LIFE, 36_288_000);
    assert_eq!(MINUTES_IN_A_WHOLE_LIFE / MINUTES_PER_YEAR, 70);
}

/// A child in sight of the camp is where it is supposed to be, with every
/// adult out foraging.
#[test]
fn a_child_by_the_camp_is_left_alone() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (45, 45, 0)));
    population.agents.push(somebody_of(7, (25, 25, 0)));

    let simulation = Simulation::new(a_world(), population);
    let child = &simulation.population.agents[1];

    let leash = Simulation::how_far_from_a_grown_person_this_one_may_be(LifeStage::Child).unwrap();
    assert!(
        simulation.world.buildings.iter().any(|roof| Simulation::within(
            (child.state.position.0, child.state.position.1),
            (roof.position.x, roof.position.y),
            leash
        )),
        "the fixture should put him in sight of the longhouse at the world's centre"
    );

    assert!(
        simulation
            .keeping_close_to_somebody_grown(child, child.state.position)
            .is_none(),
        "in eyesight of camp *or* of an adult, and he has the first"
    );
}

/// But a child under six is kept with a parent, and a camp is not a parent.
#[test]
fn the_camp_is_not_a_parent_for_the_very_young() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (45, 45, 0)));
    population.agents.push(somebody_of(3, (25, 25, 0)));

    let simulation = Simulation::new(a_world(), population);
    let infant = &simulation.population.agents[1];

    assert!(
        simulation
            .keeping_close_to_somebody_grown(infant, infant.state.position)
            .is_some(),
        "under six the rule is to be *with* a parent, not near a building"
    );
}

/// A parent carrying somebody under two has one hand occupied and carries half
/// of what two hands hold.
#[test]
fn a_child_in_arms_takes_up_a_hand() {
    let mut free = somebody_of(30, (0, 0, 0));
    let mut carrying = somebody_of(30, (0, 0, 0));

    carrying.hands_full_of_child = true;
    carrying.take_up_the_cart();
    free.take_up_the_cart();

    assert!(
        carrying.total_carrying_capacity() < free.total_carrying_capacity(),
        "one hand is not two: {:.1} against {:.1}",
        carrying.total_carrying_capacity(),
        free.total_carrying_capacity()
    );
}

/// The feeding bands, read straight off the specification.
#[test]
fn a_small_child_gets_what_its_parent_can_spare() {
    let share = Simulation::what_share_a_small_child_gets;

    assert_eq!(share(0.9), 1.0, "a parent four fifths full feeds it fully");
    assert_eq!(share(0.7), 0.75);
    assert_eq!(share(0.5), 0.5);
    assert_eq!(share(0.3), 0.25);
    assert_eq!(share(0.1), 0.0, "and below a fifth there is nothing to give");

    // Monotone, and never more than the child asked for
    let mut last = 0.0;
    for step in 0..=20 {
        let now = share(step as f32 / 20.0);
        assert!(now >= last, "it should not go down as the parent fills up");
        assert!(now <= 1.0);
        last = now;
    }
}

/// And it actually reaches the child, out of the parent.
#[test]
fn feeding_a_small_child_comes_out_of_the_parent() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (25, 25, 0)));
    let parent = population.agents[0].id;

    let mut child = somebody_of(3, (25, 25, 0));
    child.parent_ids = vec![parent];
    child.state.physiology.reserve = 0.0;
    population.agents.push(child);

    let mut simulation = Simulation::new(a_world(), population);
    let was = simulation.population.agents[0].state.physiology.reserve;

    simulation.feed_the_small_children();

    assert!(
        simulation.population.agents[1].state.physiology.reserve > 0.0,
        "a three-year-old beside a full parent should have been fed"
    );
    assert!(
        simulation.population.agents[0].state.physiology.reserve < was,
        "and it came out of the parent: {was:.1} to {:.1}",
        simulation.population.agents[0].state.physiology.reserve
    );
}

/// A parent with almost nothing inside them feeds nobody, and both of them are
/// still standing.
#[test]
fn a_parent_below_a_fifth_has_nothing_to_give() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (25, 25, 0)));
    let parent = population.agents[0].id;

    let mut child = somebody_of(3, (25, 25, 0));
    child.parent_ids = vec![parent];
    child.state.physiology.reserve = 0.0;
    population.agents.push(child);

    let mut simulation = Simulation::new(a_world(), population);
    let capacity = simulation.population.agents[0].state.physiology.reserve_capacity;
    simulation.population.agents[0].state.physiology.reserve = capacity * 0.1;

    simulation.feed_the_small_children();

    assert_eq!(
        simulation.population.agents[1].state.physiology.reserve, 0.0,
        "below a fifth the child receives nothing"
    );
    assert!(simulation.population.agents[0].state.is_alive);
    assert!(simulation.population.agents[1].state.is_alive);
}

/// Nobody under ten puts anything on a fire.
#[test]
fn a_child_under_ten_cannot_cook() {
    let mut population = Population::new();
    population.agents.push(somebody_of(7, (25, 25, 0)));
    let mut simulation = Simulation::new(a_world(), population);

    let mut rng = crate::core::dice::roll();
    let refused = simulation.cooking(&"generic".to_string(), 0, &mut rng);

    assert!(!refused.success, "seven is too young for a fire");
    assert!(
        refused.message.as_deref().is_some_and(|m| m.contains("young")),
        "and it should say why: {:?}",
        refused.message
    );
}
