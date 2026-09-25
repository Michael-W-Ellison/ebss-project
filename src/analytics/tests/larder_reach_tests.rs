// src/analytics/tests/larder_reach_tests.rs
//! A full larder five paces off beats a bush across the valley.
//!
//! Measured at the last look anybody got before they died, over thirty-two
//! worlds: the settlement's pits held **805.7 items among 6.68 mouths** - ten
//! days of food for everybody - the larder was wholly empty in under one per
//! cent of those samples, and the dying were carrying one item, eleven days
//! into a three-week reserve. They starved walking somewhere.
//!
//! The store sits behind the ordinary food branch, which is right and was
//! measured. What broke it was taking the limit off the range of the food
//! search: `food_action` could no longer return `None`, so the branch behind
//! it stopped existing. It is compared on distance now.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{Pit, Position, ResourceNode, ResourceType, World, WorldConfig};

/// A country with one bush in it, at whatever distance is asked for, and a
/// larder with ten days of food in it five paces from where the man stands.
fn one_bush_and_a_full_pit(bush_at: (i32, i32)) -> crate::analytics::Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let here = population.agents[0].state.position;

    world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(here.0 + bush_at.0, here.1 + bush_at.1),
        500,
    ));

    let mut simulation = crate::analytics::Simulation::new(world, population);

    // A pit only counts as a larder if what is in it is a meal, which means
    // a stack with a clock on it that has not gone over - see `is_it_a_meal`.
    let mut buried = InventoryItem::new_with_weight("food".to_string(), 150, 0.5);
    buried.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Food, 0);

    let mut pit = Pit {
        where_it_is: Position::new(here.0 + PACES_TO_THE_PIT, here.1),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: crate::world::Belongs::ToNobody,
    };
    pit.put_in(buried);
    simulation.world.pits.push(pit);

    // A pit is a place somebody has to have *learned* about - by seeing it,
    // or by having dug or filled it - rather than a fact about the world that
    // every mind has free of charge; see `nearest_pit_i_remember`. This one
    // was pushed straight into the world, so hand him the knowledge a man
    // would have had of the store he buried.
    simulation.population.agents[0]
        .memory
        .remember_how_much_is_there(
            crate::core::memory::SpatialMemoryType::Storage,
            (here.0 + PACES_TO_THE_PIT, here.1, 0),
            150,
        );

    // Starving, which is what opens a store while the hedgerows are bearing.
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 1.0;
    }
    // `is_starving` is the body's own reckoning, and energy is the half of
    // it a fixture can set without waiting a fortnight.
    simulation.population.agents[0].state.energy = 5.0;

    simulation
}

/// Near enough that nobody would walk past it, far enough to be a walk.
const PACES_TO_THE_PIT: i32 = 5;

/// The bush is across the valley and the pit is at hand: he opens the pit.
#[test]
fn a_starving_man_walks_to_the_larder_rather_than_across_the_valley() {
    let simulation = one_bush_and_a_full_pit((30, 0));
    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;

    let what = simulation
        .food_action(&agent, here, true)
        .expect("a starving man with a full pit should be doing something");

    match what {
        Action::Move { target } => assert_eq!(
            (target.0, target.1),
            (here.0 + PACES_TO_THE_PIT, here.1),
            "he set off for the far bush with ten days of food five paces away"
        ),
        Action::PickUp { .. } => {}
        other => panic!("neither the pit nor a walk to it: {other:?}"),
    }
}

/// And the bush at his feet still beats the pit, which is the ordering that
/// was measured and is not being changed: a meal out of a hole costs two
/// turns where a berry costs one.
#[test]
fn a_bush_underfoot_still_beats_the_larder() {
    let simulation = one_bush_and_a_full_pit((0, 0));
    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;

    let what = simulation
        .food_action(&agent, here, true)
        .expect("a starving man on a berry patch should be doing something");

    assert!(
        matches!(what, Action::Eat { .. } | Action::Gather { .. }),
        "he walked to the larder with a bush under his feet: {what:?}"
    );
}

/// An emptied hole does not stand between a starving man and the next one.
///
/// `something_out_of_the_store` took the nearest pit this one remembers and
/// stopped there. A memory is a record of what *was* in a hole, so a man
/// standing on one he or somebody else has already emptied got nothing from
/// the whole branch - though he might remember three more with food in them.
///
/// Measured across a settlement's winter, over the samples where a body under
/// a quarter of its own reserve was carrying nothing: the branch answered 38
/// times and came back empty 13, and in every one of those 13 the man
/// remembered a pit that had food in it. See ISSUES_FOUND #228.
#[test]
fn an_empty_hole_underfoot_does_not_hide_the_full_one_behind_it() {
    use crate::core::memory::SpatialMemoryType;
    use crate::world::{Belongs, Pit, Position};

    let mut simulation = one_bush_and_a_full_pit((30, 0));
    let here = simulation.population.agents[0].state.position;

    // An empty hole right where he is standing, and he remembers it.
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(here.0, here.1),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: Belongs::ToNobody,
    });
    simulation.population.agents[0]
        .memory
        .remember_how_much_is_there(SpatialMemoryType::Storage, (here.0, here.1, 0), 150);

    // The full one five paces off is already remembered by the fixture.
    let agent = simulation.population.agents[0].clone();
    let what = simulation
        .something_out_of_the_store(&agent, here)
        .expect("he remembers a full pit five paces away");

    match what {
        Action::Move { target } => assert_eq!(
            (target.0, target.1),
            (here.0 + PACES_TO_THE_PIT, here.1),
            "he was sent somewhere other than the pit that has food in it"
        ),
        Action::PickUp { .. } => panic!("there is nothing in the hole he is standing on"),
        other => panic!("neither the full pit nor a walk to it: {other:?}"),
    }
}

/// And with nothing standing anywhere at all, the larder is still the answer.
///
/// The store was reachable from the hunger drive only through
/// `the_larder_or_this_walk`, which weighs the larder against a **walk** and
/// so needs somewhere to walk to. In deep winter there is nowhere: both
/// callers of it fall through, and what was left was the starvation override
/// at the head of the decision, which fires only below a quarter of the
/// reserve.
///
/// So a body spent the first three weeks of a seventy-five day hungry gap
/// burning itself with the settlement's whole winter store in the ground
/// behind it. Measured over three seeded settlements across the gap, 92,249
/// agent-turns: **27.0% on SeekShelter against 2.0% on the store**, and 4.2
/// items a person-day out of the pits against the 11.5 a grown body burns.
/// See ISSUES_FOUND #231.
#[test]
fn a_bare_country_still_leaves_the_larder() {
    // Nothing growing anywhere, which is what the hungry gap is.
    let mut simulation = one_bush_and_a_full_pit((30, 0));
    simulation.world.resources.clear();

    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;

    assert!(
        simulation.the_best_food_anywhere(&agent, here).is_none(),
        "the fixture is not testing anything: something is still standing"
    );

    // Merely hungry, not desperate - which is the case this had no rung for.
    let what = simulation
        .food_action(&agent, here, false)
        .expect("a hungry man with a full pit five paces off should be doing something");

    match what {
        Action::Move { target } => assert_eq!(
            (target.0, target.1),
            (here.0 + PACES_TO_THE_PIT, here.1),
            "he set off somewhere that is not the larder"
        ),
        Action::PickUp { .. } => {}
        other => panic!("neither the pit nor a walk to it: {other:?}"),
    }
}

/// A parent whose small child is on short rations opens the larder in any
/// season.
///
/// The store is kept shut while the hedgerows bear, and a small child is fed
/// through its parent's body at less than a full share as soon as the parent
/// is under four-fifths of their reserve. So a parent eating for two off the
/// hedge sat at seven-tenths all autumn beside a full larder, and the child
/// went on three-quarter rations into the winter. See ISSUES_FOUND #255.
#[test]
fn a_parent_whose_child_is_going_short_opens_the_larder_in_summer() {
    use crate::agents::Agent;

    let mut simulation = one_bush_and_a_full_pit((30, 0));
    simulation.world.climate.calendar.day_of_year = 150;
    assert!(simulation.are_the_hedgerows_bearing(), "the fixture wants the hedgerows bearing");

    let capacity = simulation.population.agents[0].state.physiology.reserve_capacity;
    simulation.population.agents[0].state.physiology.reserve = capacity * 0.7;
    simulation.population.agents[0].state.energy = 100.0;
    if let Some(hunger) = simulation.population.agents[0].drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.0;
    }
    // Breakfast in them, so that nobody is acutely starving and the only
    // question is the reserve.
    simulation.population.agents[0].state.physiology.eat(100.0, 1.0);

    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;
    assert!(
        simulation.something_out_of_the_store(&agent, here).is_none(),
        "somebody at seven-tenths with nobody to feed keeps the summer larder shut"
    );

    let mut child = Agent::with_parents(AgentConfig::default(), vec![agent.id], simulation.current_turn);
    child.state.position = here;
    simulation.population.agents.push(child);

    assert!(
        simulation.something_out_of_the_store(&agent, here).is_some(),
        "a parent at seven-tenths, and so a child on three-quarter rations, was kept out of a full larder"
    );
}
