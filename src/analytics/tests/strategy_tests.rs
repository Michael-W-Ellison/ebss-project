// src/analytics/tests/strategy_tests.rs
//! Layer 3: the ways of answering a drive, named and priced.
//!
//! See `SATISFACTION.md`. What is asserted here is that the ways exist, that
//! they are told apart, that they are learnable, and that they are priced -
//! and, in `a_doubt_outweighs_every_cost_put_together`, how little of that
//! price the sort is currently reading.

use crate::agents::patterns::{Element, Patterns};
use crate::agents::{AgentConfig, Population};
use crate::analytics::wanting::strategy::Strategy;
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

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

/// Drinking what you carry and drinking what is in front of you are two ways,
/// not one.
///
/// They come out as the same verb, which is exactly why the layer is needed: a
/// waterskin runs out and has to be refilled, a river does not and cannot be
/// carried away from. `Did("gather")` cannot tell them apart and never could.
#[test]
fn the_skin_and_the_river_are_two_ways_of_the_same_verb() {
    let ways = Strategy::all_for(DriveType::Thirst);

    assert!(ways.contains(&Strategy::ConsumeCarriedWater));
    assert!(ways.contains(&Strategy::DrinkFromLocalSource));
    assert_ne!(
        Strategy::ConsumeCarriedWater.called(),
        Strategy::DrinkFromLocalSource.called(),
        "two bets that produce one verb still have to be told apart"
    );
}

/// A way is written down as an element like anything else, so the arithmetic
/// that already ranks verbs and places ranks these without a second mechanism.
#[test]
fn a_way_is_an_element_and_survives_being_written_down() {
    let by = Strategy::FetchFromKnownSource.as_element();
    assert_eq!(by, Element::By("fetch-water".to_string()));

    let written: String = by.clone().into();
    assert_eq!(written, "by:fetch-water");
    assert_eq!(Element::try_from(written).unwrap(), by);
}

/// A drive with no ways yet falls through to the arm it always had.
///
/// This is how the layer goes in one drive at a time instead of in one
/// unmeasurable jump.
#[test]
fn a_drive_with_no_ways_yet_answers_as_it_always_did() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    // Curiosity has no ways declared, so it answers the way it always did.
    assert!(Strategy::all_for(DriveType::Curiosity).is_empty());
    assert!(
        simulation
            .the_way_to_answer(DriveType::Curiosity, agent, agent.state.position)
            .is_none(),
        "no ways means no answer from this layer, and the old ladder runs"
    );

    // And the three drives that are wired have ways.
    for need in [DriveType::Thirst, DriveType::Hunger, DriveType::Shelter] {
        assert!(!Strategy::all_for(need).is_empty(), "{need:?}");
    }
}

/// A way whose preconditions are unmet is not a candidate.
///
/// An empty pack cannot be drunk from, whatever it has been worth in the past.
/// Asserted against the precondition check itself rather than against a world
/// arranged to have no water in it: the default test map has a river on it, so
/// arranging the *absence* of a thing is the harder and more brittle half.
#[test]
fn a_way_that_cannot_be_taken_is_not_offered() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    assert_eq!(agent.inventory.available_water(), 0.0, "the pack is empty");
    assert!(
        simulation
            .can_this_way_be_taken(
                Strategy::ConsumeCarriedWater,
                agent,
                agent.state.position
            )
            .is_none(),
        "an empty pack is not something to drink out of"
    );
}

/// And a way whose preconditions are met comes back with what to do.
#[test]
fn a_way_that_can_be_taken_says_what_it_comes_to() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    // Whatever else is true of this map, one of the three ways can be taken -
    // the layer is not allowed to leave a thirsty man with nothing at all
    // while `water_action` would have found him something.
    let any = Strategy::all_for(DriveType::Thirst).iter().any(|way| {
        simulation
            .can_this_way_be_taken(*way, agent, agent.state.position)
            .is_some()
    });
    assert!(any, "some way of answering thirst is open on an ordinary map");

    let chosen = simulation
        .the_way_to_answer(DriveType::Thirst, agent, agent.state.position)
        .expect("and so the layer names one");
    assert!(matches!(
        chosen.doing,
        Action::Gather { .. } | Action::Move { .. }
    ));
}

/// What a body has learned about a way decides between them, and the written
/// order decides when it has learned nothing.
///
/// The ranking is asserted on the trails rather than through a world arranged
/// to make all three ways open at once, which is the brittle part.
#[test]
fn what_worked_before_outranks_the_order_somebody_typed() {
    let mut patterns = Patterns::default();
    let walking = Strategy::FetchFromKnownSource.as_element();

    for round in 0..8u32 {
        patterns.it_worked(
            DriveType::Thirst,
            std::slice::from_ref(&walking),
            0.9,
            round,
        );
    }

    let worth_of = |way: Strategy| {
        patterns
            .trail(DriveType::Thirst, &way.as_element())
            .map(|trail| trail.worth())
            .unwrap_or(0.0)
    };

    assert!(
        worth_of(Strategy::FetchFromKnownSource) > worth_of(Strategy::ConsumeCarriedWater),
        "a way that has paid is worth more than one that has not"
    );
    assert_eq!(
        worth_of(Strategy::ConsumeCarriedWater),
        0.0,
        "and a way nobody has tried is worth nothing, so the written order \
         still decides between the untried ones - which is what makes this \
         step change no behaviour at all"
    );
}

// --- the explicit sets, the scoring, and the horizons ---------------------

/// Every drive named in the specification has its ways declared, all of them,
/// including the ones this world cannot carry out yet.
///
/// A named gap is one somebody can count and go and fill. An unnamed one is a
/// gap nobody knows is there.
#[test]
fn the_ways_are_all_declared_even_the_ones_that_cannot_fire() {
    use crate::analytics::wanting::strategy::Reach;

    for need in [DriveType::Thirst, DriveType::Hunger, DriveType::Shelter] {
        assert!(
            Strategy::all_for(need).len() >= 6,
            "{need:?} has a handful of ways, not one"
        );
    }

    // And each unreachable one says what is missing, rather than being absent.
    let mut reachable = 0;
    let mut declared = 0;
    for way in Strategy::every_one() {
        declared += 1;
        match way.reach() {
            Reach::Now => reachable += 1,
            Reach::NotYet(why) => assert!(
                !why.is_empty(),
                "{} is out of reach and does not say why",
                way.called()
            ),
        }
    }
    assert!(reachable > 0 && reachable < declared, "{reachable} of {declared}");
}

/// Names are unique, because the name is the key the record is kept under.
#[test]
fn no_two_ways_share_a_name() {
    let mut seen = std::collections::BTreeSet::new();
    for way in Strategy::every_one() {
        assert!(seen.insert(way.called()), "{} twice", way.called());
    }
}

/// The formula, and the thing it is for: a mouthful in front of you beats the
/// same mouthful two hours away.
#[test]
fn what_is_near_beats_what_is_far_for_the_same_relief() {
    use crate::analytics::wanting::strategy::Utility;

    let here = Utility::a_sure_thing(0.8);
    let mut two_hours_off = Utility::a_sure_thing(0.8);
    two_hours_off.turns = 20.0;

    assert!(
        here.score() > two_hours_off.score(),
        "{:.3} against {:.3}",
        here.score(),
        two_hours_off.score()
    );
}

/// And not knowing whether it will work discounts what it is worth, rather
/// than being a cost bolted on beside it.
///
/// A half-believed mouthful is worth half a mouthful. Priced as a flat
/// subtraction instead, a big enough relief would swamp any doubt at all.
#[test]
fn doubt_discounts_the_relief_rather_than_being_added_beside_it() {
    use crate::analytics::wanting::strategy::Utility;

    let certain = Utility::a_sure_thing(1.0);
    let mut half_believed = Utility::a_sure_thing(1.0);
    half_believed.confidence = 0.5;

    let lost = certain.score() - half_believed.score();
    assert!(
        (lost - 0.5).abs() < 1e-5,
        "half the belief should cost half the relief, not {lost:.3}"
    );
}

/// A man dying of thirst does not dig a well, however good a well is.
#[test]
fn the_long_work_is_set_aside_when_the_need_is_now() {
    use crate::analytics::wanting::strategy::Horizon;

    assert_eq!(Strategy::ConsumeCarriedWater.horizon(), Horizon::Immediate);
    assert_eq!(Strategy::FetchFromKnownSource.horizon(), Horizon::ShortTerm);
    assert_eq!(Strategy::DigOrRepairWell.horizon(), Horizon::LongTerm);

    // The gate is an ordering, so "as far ahead as he can afford" reads as a
    // comparison rather than a table.
    assert!(Horizon::Immediate < Horizon::ShortTerm);
    assert!(Horizon::ShortTerm < Horizon::LongTerm);
}

/// And the horizon is a gate rather than a term, so no amount of relief buys
/// a well for a man with an hour to live.
#[test]
fn no_relief_is_large_enough_to_buy_a_well_when_the_need_is_now() {
    use crate::analytics::wanting::strategy::Horizon;

    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    // Whatever the world offers, what comes back on a pressing need is
    // something that answers it now.
    let pressing = agent.how_hard_it_presses(DriveType::Thirst);
    if pressing >= Simulation::WHEN_ONLY_NOW_WILL_DO {
        if let Some(way) =
            simulation.the_way_to_answer(DriveType::Thirst, agent, agent.state.position)
        {
            assert_eq!(
                way.by.horizon(),
                Horizon::Immediate,
                "{} was chosen for a need that will not wait",
                way.by.called()
            );
        }
    }
}

// --- what came over with hunger -------------------------------------------

/// The plan and the run decide over the top of the costs, not under them.
///
/// These two readers used to live inside the hunger arm and nowhere else,
/// which meant thirst and shelter could neither hold a plan nor follow a run.
/// They are not about hunger - they are about choosing among things you could
/// do - so they came over with it, and the order they are applied in is the
/// order they were applied in before: what is in his hand, then the plan, then
/// the run, then the cost.
#[test]
fn the_plan_and_the_run_are_asked_before_the_costs_are() {
    // Asserted on the source rather than by arranging a world with a live
    // plan, a worn run and three open ways at once - which needs half a
    // settlement's worth of fixture and asserts the fixture as much as the
    // rule. What matters is that the four rules are in this order, and that is
    // a fact about one function.
    let source = include_str!("../wanting/strategy.rs");
    let where_it_is = |what: &str| {
        source
            .find(what)
            .unwrap_or_else(|| panic!("{what} is not in the chooser any more"))
    };

    let in_hand = where_it_is("is_it_already_in_his_hand");
    let the_plan = where_it_is("what_the_plan_wants_next");
    let the_run = where_it_is("what_usually_comes_next");
    let the_cost = where_it_is("how_far_down_the_list_to_look");

    assert!(
        in_hand < the_plan && the_plan < the_run && the_run < the_cost,
        "the four rules are out of order: {in_hand} {the_plan} {the_run} {the_cost}"
    );
}

/// And every drive that has ways now gets them, not only hunger.
#[test]
fn thirst_and_shelter_can_hold_a_plan_too() {
    // The readers are asked for whatever need is being answered, so this is a
    // question about the signature rather than about a world: `the_way_to_answer`
    // takes the need and passes it to both readers.
    let source = include_str!("../wanting/strategy.rs");
    assert!(
        source.contains("agent.is_the_plan_for(need)"),
        "the plan is asked about the need being answered, whichever it is"
    );
    assert!(
        source.contains("agent.what_usually_comes_next(need)"),
        "and so is the run"
    );
}

/// Hunger's ways cover every rung its old arm had.
///
/// The arm is gone; if a rung did not come over with it, that is a way of
/// answering hunger this world quietly lost.
#[test]
fn every_rung_of_the_old_hunger_arm_came_over() {
    let ways: Vec<&str> = Strategy::all_for(DriveType::Hunger)
        .iter()
        .map(|way| way.called())
        .collect();

    // One for each rung the arm used to try, in its own words: what is at his
    // feet, the ordinary food branch, the round if it is due, the store, the
    // walk out to a catch, the water, and the animal.
    for rung in [
        "eat-carried",
        "gather-wild",
        "walk-the-line",
        "eat-stored",
        "scavenge",
        "fish",
        "hunt",
    ] {
        assert!(ways.contains(&rung), "{rung} did not come over");
    }
}

/// The horizon is a preference, not a prohibition.
///
/// **This is the one that cost three quarters of everything.** Built as a hard
/// filter, the gate could empty the candidate list: a starving man with an
/// empty pack and no store had no *immediate* way of eating, so hunger answered
/// nothing at all and he stood beside a berry bush until he died. Measured over
/// 32 worlds: person-days 105,429 to 24,846, population at month three 11.0 to
/// 1.5, every world emptied.
#[test]
fn a_need_that_will_not_wait_still_gets_an_answer_when_nothing_is_immediate() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    // Whatever the world is like, a drive with reachable ways answers with
    // *something*. The list is allowed to be short; it is not allowed to be
    // empty while a way is open at any horizon.
    for need in [DriveType::Thirst, DriveType::Hunger, DriveType::Shelter] {
        let anything_open = Strategy::all_for(need).iter().any(|way| {
            matches!(way.reach(), crate::analytics::wanting::strategy::Reach::Now)
                && simulation
                    .can_this_way_be_taken(*way, agent, agent.state.position)
                    .is_some()
        });

        if anything_open {
            assert!(
                simulation
                    .the_way_to_answer(need, agent, agent.state.position)
                    .is_some(),
                "{need:?} had a way open and the gate swallowed it"
            );
        }
    }
}

/// And gathering answers hunger now, because in this world you eat what you
/// pick.
#[test]
fn gathering_is_how_a_hungry_man_eats_not_how_he_prepares() {
    use crate::analytics::wanting::strategy::Horizon;

    assert_eq!(Strategy::GatherWildFood.horizon(), Horizon::Immediate);
    assert_eq!(Strategy::EatCarriedFood.horizon(), Horizon::Immediate);
    // A hunt is turns of work before anything is eaten, so it is not.
    assert_eq!(Strategy::HuntLocalAnimals.horizon(), Horizon::ShortTerm);
}

/// **The price of everything the ways differ in is a fifth of the price of the
/// one thing they do not.**
///
/// This is the arithmetic behind what the sort in `the_ways_open` actually
/// reads. Within one need every way relieves the same need by the same amount,
/// so `relief` is common to all of them and cancels; what is left to tell two
/// ways apart is their costs. The whole cost spread between reaching into your
/// own pack and going hunting is about a fifth of a point, and the uncertainty
/// term - `relief * (1 - confidence)` - runs to a full one. So ranking by score
/// is very nearly ranking by how sure the agent is, with the walk and the work
/// as rounding error.
///
/// It measures as neither better nor worse than taking the ways in the order
/// somebody wrote - 209,346 person-days against 211,176 over 64 worlds, which
/// is under a per cent on a measure that swings ten - so the sort stays, and
/// this test holds the arithmetic still until a walk and a doubt are
/// denominated in one currency.
#[test]
fn a_doubt_outweighs_every_cost_put_together() {
    use crate::analytics::wanting::strategy::Utility;

    let pressing = 1.0;

    // The dearest way there is: a hunt, three squares off, with a worn spear.
    let mut dearest = Utility::a_sure_thing(pressing);
    dearest.turns += 3.0;
    dearest.effort += 12.0;
    dearest.wear += 1.0;

    // The cheapest: your own pack, certain.
    let cheapest = Utility::a_sure_thing(pressing);

    let the_whole_cost_spread = cheapest.score() - dearest.score();

    // And the same cheapest way, half believed in.
    let mut half_believed = Utility::a_sure_thing(pressing);
    half_believed.confidence = 0.5;
    let what_a_doubt_costs = cheapest.score() - half_believed.score();

    assert!(
        what_a_doubt_costs > the_whole_cost_spread * 2.0,
        "a half-doubt costs {what_a_doubt_costs}, the whole spread from pack to \
         hunt costs {the_whole_cost_spread} - if that is no longer true the sort \
         can come back, and be measured again"
    );
}
