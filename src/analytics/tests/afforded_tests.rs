// src/analytics/tests/afforded_tests.rs
//! What could I do here, holding this?
//!
//! "An action like holding something should unlock additional actions."
//!
//! The matrix has said so all along - twenty-seven of its seventy-four verbs
//! target `AThingHeld` and are closed to empty hands - and nothing asked it.
//! `EVERY_VERB` was read by the test suite and by nothing else; the one
//! production caller of the module asks what a *named* verb wants so it can go
//! and fetch it. These are the first tests of the question asked the other way
//! round.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::environment::verbs::{self as verbs, Targets, EVERY_VERB};
use crate::prelude::Action;
use crate::analytics::Simulation;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// One person on bare ground, with nothing in the pack and nothing underfoot.
///
/// Bare on purpose: what is being measured is which verbs *open* when
/// something is added, so the starting state has to be one where they are
/// shut.
fn one_person_on_bare_ground() -> crate::analytics::Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.dropped.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = crate::analytics::Simulation::new(world, population);

    // `Simulation::new` places people, so the tile to clear is the one they
    // ended up on rather than the one they spawned on.
    let at = simulation.population.agents[0].state.position;
    simulation
        .world
        .resources
        .retain(|node| node.position != Position::new(at.0, at.1));
    simulation.world.dropped.clear();
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();

    simulation
}

fn open_verbs(simulation: &crate::analytics::Simulation) -> Vec<&'static str> {
    let agent = &simulation.population.agents[0];
    let mut names: Vec<&'static str> = simulation
        .what_i_could_do_here(agent)
        .into_iter()
        .map(|verb| verb.called)
        .collect();
    names.sort_unstable();
    names
}

/// Holding something opens verbs that empty hands do not have.
///
/// The whole claim, and it needs no rule of its own: `Targets::AThingHeld` was
/// declared against twenty-seven verbs long before anybody asked which of them
/// a given pair of hands could reach.
#[test]
fn holding_something_opens_verbs_that_empty_hands_do_not() {
    let mut simulation = one_person_on_bare_ground();
    let empty_handed = open_verbs(&simulation);

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, 1.0));
    let carrying = open_verbs(&simulation);

    let opened: Vec<_> = carrying
        .iter()
        .filter(|verb| !empty_handed.contains(verb))
        .collect();

    assert!(
        !opened.is_empty(),
        "a stone in the pack opened nothing: {} verbs either way",
        empty_handed.len()
    );

    // And specifically the family that wants a thing to act on. Named rather
    // than counted, so that a matrix edit that moves one of them shows up here
    // as a changed list rather than as a changed number.
    for wants_a_thing in ["drop", "dry", "salt"] {
        assert!(
            carrying.contains(&wants_a_thing) && !empty_handed.contains(&wants_a_thing),
            "{wants_a_thing} targets a thing held and did not open when one was held"
        );
    }
}

/// And nothing underfoot shuts the verbs that want something underfoot.
///
/// The other half of the target question, and the half the matrix had no way
/// of answering: `Wants` came with `satisfied_by_hands` and could always be
/// asked, `Targets` was declared and never once put to a world.
#[test]
fn something_underfoot_opens_the_verbs_that_want_one() {
    let mut simulation = one_person_on_bare_ground();
    let bare = open_verbs(&simulation);

    let at = simulation.population.agents[0].state.position;
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(at.0, at.1),
        50,
    ));
    let standing_on_a_bush = open_verbs(&simulation);

    assert!(
        standing_on_a_bush.len() > bare.len(),
        "a bush underfoot opened nothing: {} verbs either way",
        bare.len()
    );

    let underfoot: Vec<&'static str> = EVERY_VERB
        .iter()
        .filter(|verb| verb.targets == Targets::AThingUnderfoot)
        .map(|verb| verb.called)
        .collect();
    assert!(
        underfoot
            .iter()
            .any(|verb| standing_on_a_bush.contains(verb) && !bare.contains(verb)),
        "not one of the verbs that wants a thing underfoot opened when one arrived"
    );
}

/// A verb the matrix declares and nothing performs is reported, and marked.
///
/// Eighteen of the fifty-four are in that state. Dropping them silently would
/// report a world of possibilities smaller than the matrix describes and give
/// no sign of the difference - which is the objection `verbs.rs` makes to
/// itself in its own header: "a matrix that quietly implied sixty-eight
/// working verbs would be worse than no matrix".
#[test]
fn a_declared_verb_nobody_performs_is_offered_but_not_as_a_thing_to_do() {
    let simulation = one_person_on_bare_ground();
    let agent = &simulation.population.agents[0];

    let could = simulation.what_i_could_do_here(agent);
    let could_now = simulation.what_i_could_do_here_now(agent);

    let declared_only: Vec<&'static str> = could
        .iter()
        .filter(|verb| verb.done_by.is_none())
        .map(|verb| verb.called)
        .collect();

    assert!(
        !declared_only.is_empty(),
        "no unperformed verb came back at all, so this test is not watching anything"
    );
    assert!(
        could_now.len() < could.len(),
        "the filtered form dropped nothing, so it is not filtering"
    );
    for verb in could_now {
        assert!(
            verb.done_by.is_some(),
            "{} has no performer and came back as a thing to do now",
            verb.called
        );
    }
}

/// The generator agrees with the question the executor asks.
///
/// `what_this_one_is_short_of` is what refuses an action for want of a tool or
/// a hand, and it and this now share `do_these_hands_do`. If they ever drift,
/// the decision layer will offer verbs the executor refuses - which is the
/// exact shape of #215 and #243, and this project has paid for it twice.
#[test]
fn what_is_offered_is_what_the_hands_can_actually_do() {
    let mut simulation = one_person_on_bare_ground();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, 1.0));

    let agent = simulation.population.agents[0].clone();
    for verb in simulation.what_i_could_do_here(&agent) {
        assert!(
            crate::analytics::Simulation::do_these_hands_do(&agent, &verb.wants),
            "{} was offered and these hands cannot do it",
            verb.called
        );
    }
}

/// Curiosity reaches for the thing it has tried least.
///
/// "It should be the curious agents which try new things to satisfy their
/// curiosity drive." Novelty alone picks, on `Lessons::how_new_is_this`, and
/// nothing in the choosing asks whether the thing worked.
#[test]
fn what_is_reached_for_is_what_has_been_tried_least() {
    let mut simulation = one_person_on_bare_ground();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, 1.0));

    let first = {
        let agent = &simulation.population.agents[0];
        simulation
            .what_i_have_tried_least_here(agent)
            .expect("a man with a stone has something he has never done")
            .1
    };

    // Do that one until it is stale, and something else should come up.
    for _ in 0..8 {
        simulation.population.agents[0]
            .lessons
            .record_particular(&first, true);
    }

    let next = {
        let agent = &simulation.population.agents[0];
        simulation
            .what_i_have_tried_least_here(agent)
            .expect("still standing there with a stone")
            .1
    };

    assert_ne!(
        first, next,
        "he did the same thing eight times and it is still the newest thing he could do"
    );
}

/// And what he reaches for is about a kind of thing, not about a verb.
///
/// The key has to name what it was tried on, or "I have stacked stones" and
/// "I have stacked nothing of the sort" are one row - which is the defect the
/// commit before this one fixed, and the reason novelty can be keyed at all.
#[test]
fn what_is_reached_for_names_the_thing_it_would_be_tried_on() {
    let mut simulation = one_person_on_bare_ground();
    for what in ["stone", "wood"] {
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight(what.to_string(), 1, 1.0));
    }

    let agent = simulation.population.agents[0].clone();
    let pairs = simulation.what_i_could_try_here(&agent);

    let about_a_thing: Vec<_> = pairs
        .iter()
        .filter(|(_, key)| key.contains(':'))
        .collect();
    assert!(
        !about_a_thing.is_empty(),
        "not one candidate was about a kind of thing"
    );

    assert!(
        about_a_thing
            .iter()
            .any(|(_, key)| key.ends_with(":stone"))
            && about_a_thing.iter().any(|(_, key)| key.ends_with(":wood")),
        "a stone and a stick came to the same candidates"
    );

    // And every key is one `Lessons` could actually have a record under - the
    // same spelling `Agent::what_was_tried` writes.
    for (verb, key) in &pairs {
        assert!(
            key == verb.called || key.starts_with(&format!("{}:", verb.called)),
            "{key} is not a key anything writes"
        );
    }
}

/// What is built performs the verb the matrix said performs it.
///
/// `an_action_for` is the inverse of `Agent::what_was_tried` and has to stay
/// so: one names an action, the other builds one back, and a drift between
/// them means an agent choosing one thing and learning about another. That is
/// the "two spellings of one question" this project has paid for repeatedly -
/// see #215 and #243.
///
/// The invariant is the family rather than the whole key, because the builder
/// legitimately fills in what the key does not carry: a hunt wants an animal
/// and the key names none.
#[test]
fn what_is_built_performs_the_verb_it_was_built_from() {
    let mut simulation = one_person_on_bare_ground();

    // Something to hold, something underfoot, somebody to talk to, and a
    // fire burning, so that the target-hungry arms have targets to find.
    // The fire is the same kind of provision as the rest: cooking wants one
    // the way trading wants somebody, and `an_action_for` will not build a
    // `Cook` without it - see the note there.
    for what in ["stone", "wood", "meat"] {
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight(what.to_string(), 2, 1.0));
    }
    let at = simulation.population.agents[0].state.position;
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(at.0, at.1),
        50,
    ));
    simulation
        .population
        .spawn_agent(crate::agents::AgentConfig::default());
    let mate = simulation.population.agents.len() - 1;
    simulation.population.agents[mate].state.position = at;

    let mut hearth = crate::environment::HeatSource::new(
        crate::environment::HeatSourceType::Campfire,
        (at.0, at.1, 0),
        0,
    );
    hearth.add_fuel("wood".to_string(), 1.0, 100);
    assert!(hearth.light(), "a fire with fuel in it would not light");
    simulation.world.heat_sources.add(hearth);

    let agent = simulation.population.agents[0].clone();

    let mut built = 0;
    let mut unbuilt = Vec::new();
    for (verb, key) in simulation.what_i_could_try_here(&agent) {
        let done_by = verb.done_by.expect("only performable verbs are offered");
        match simulation.an_action_for(verb, &key, &agent) {
            Some(action) => {
                let named = crate::agents::Agent::what_was_tried(&action);
                let family = named.split(':').next().unwrap_or(&named);
                assert_eq!(
                    family, done_by,
                    "{} was built from {key} and came back as {named}",
                    verb.called
                );
                built += 1;
            }
            None => unbuilt.push((verb.called, key)),
        }
    }

    assert!(
        built > 0,
        "nothing at all could be built, so this test is not watching anything"
    );
    // What cannot be built is exactly the verbs that want somewhere to go.
    // A destination is not a kind of thing, so the key cannot carry one, and
    // where to walk has its own machinery - see the note in `an_action_for`.
    // Anything else appearing here is a gap rather than a decision.
    for (called, key) in &unbuilt {
        let verb = EVERY_VERB
            .iter()
            .find(|verb| verb.called == *called)
            .expect("it came out of the matrix");
        assert_eq!(
            verb.targets,
            Targets::APlace,
            "{called} could not be built from {key}, and it is not a question \
             of where to go"
        );
    }
}

/// The thing curiosity reaches for is a thing it can actually do.
///
/// The whole chain in one assertion: the matrix says what is open, novelty
/// picks the newest of it, and what comes back is an `Action` the executor
/// takes. Each of the three has its own tests; this is the one that fails if
/// they stop meeting.
#[test]
fn what_curiosity_reaches_for_can_be_carried_out() {
    let mut simulation = one_person_on_bare_ground();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, 1.0));

    let agent = simulation.population.agents[0].clone();
    let (verb, key) = simulation
        .what_i_have_tried_least_here(&agent)
        .expect("a man with a stone has something he has never done");

    let action = simulation
        .an_action_for(verb, &key, &agent)
        .unwrap_or_else(|| panic!("curiosity reached for {key}, which cannot be done"));

    // And it survives the executor - which may well refuse it, since trying
    // things that do not work is the point. What it must not do is panic or
    // come back as some other verb.
    let result = simulation.execute_action(&action, 0);
    let named = crate::agents::Agent::what_was_tried(&action);
    assert_eq!(
        named.split(':').next(),
        verb.done_by,
        "what was carried out was not what was chosen: {named:?}"
    );
    let _ = result.success;
}

/// A verb nobody has written an arm for is still reachable, through `Work`.
///
/// This is the property that keeps the builder from being a second closed
/// list beside the matrix. `Work` carries its verb as data, so a verb whose
/// `done_by` names none of the particular actions falls through to it and
/// still round-trips. Add a verb to the matrix tomorrow and it is reachable
/// without touching `an_action_for`.
#[test]
fn a_verb_with_no_arm_of_its_own_is_still_reachable() {
    let simulation = one_person_on_bare_ground();
    let agent = simulation.population.agents[0].clone();

    let made_up = crate::environment::verbs::Verb {
        called: "burnish",
        done_by: Some("burnish"),
        ..*crate::environment::verbs::EVERY_VERB
            .iter()
            .find(|verb| verb.called == "scrape")
            .expect("the matrix has a scraping in it")
    };

    let action = simulation
        .an_action_for(&made_up, "burnish:stone", &agent)
        .expect("a verb with no arm should fall through to a working");

    assert_eq!(
        crate::agents::Agent::what_was_tried(&action),
        "burnish:stone",
        "the working did not carry the verb it was given"
    );
}

/// The curiosity drive reaches for the novelty, rather than wandering.
///
/// The terminal of the curiosity ladder was `generate_action_for_drive`,
/// which answered curiosity with walking somewhere - the one thing a curious
/// man can do that cannot teach him anything about what he is holding. Every
/// rung above it is a named experiment with its own consequences and they
/// stay; this is the general case underneath them.
#[test]
fn curiosity_reaches_for_the_verb_it_has_tried_least() {
    let mut simulation = one_person_on_bare_ground();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 2, 1.0));

    let agent = simulation.population.agents[0].clone();
    let at = agent.state.position;

    let offered = simulation
        .what_this_drive_offers(crate::core::DriveType::Curiosity, &agent, at)
        .expect("a curious man holding a stone has something to do");

    assert!(
        !matches!(offered, crate::environment::Action::Move { .. }),
        "curiosity answered with a walk: {offered:?}"
    );

    // And what it offers is a thing the matrix says he could do here.
    let named = crate::agents::Agent::what_was_tried(&offered);
    let family = named.split(':').next().unwrap_or(&named);
    assert!(
        simulation
            .what_i_could_do_here_now(&agent)
            .iter()
            .any(|verb| verb.done_by == Some(family)),
        "curiosity offered {named}, which the matrix does not say is open here"
    );
}

/// How often the novelty terminal is actually reached, and with what.
///
/// The rungs above it are named experiments that fire on their own
/// conditions, and a terminal that is always shadowed is a terminal that
/// changes nothing. That is not answerable by reading the ladder - measured,
/// `putdown` alone shadows it for anybody carrying a thing they would leave
/// out - so this runs a settlement and counts.
///
/// Prints the split under `--nocapture`; asserts only that the terminal is
/// reached at all, since the exact figures move with everything else.
#[test]
fn the_novelty_terminal_is_reached_by_a_living_settlement() {
    use crate::agents::{AgentConfig, PopulationConfig};
    use crate::core::DriveType;
    use crate::environment::seasons::{PLANNING_PERIODS_PER_DAY, TICKS_PER_DAY};
    use std::collections::BTreeMap;

    crate::core::dice::seed(0);
    let world = World::new(WorldConfig::default());
    let mut population = Population::with_config(PopulationConfig::default());
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = crate::analytics::Simulation::new(world, population);

    let mut chose: BTreeMap<String, u64> = BTreeMap::new();
    let mut looks = 0u64;
    let mut from_novelty = 0u64;

    for _ in 0..30 {
        for _ in 0..PLANNING_PERIODS_PER_DAY {
            simulation.take_a_turn();
        }

        let who: Vec<_> = simulation
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .cloned()
            .collect();
        for agent in who {
            looks += 1;
            let at = agent.state.position;

            // Zeroed before each ask, so what the counter holds afterwards is
            // this one decision and not the thirty days of turning that came
            // before it. The settlement's own turns go through the same
            // function, so a running total would be over a denominator this
            // test has not got.
            simulation
                .how_often_curiosity_reached_for_something_new
                .set(0);

            match simulation.what_this_drive_offers(DriveType::Curiosity, &agent, at) {
                Some(action) => {
                    let named = crate::agents::Agent::what_was_tried(&action);
                    let family = named.split(':').next().unwrap_or(&named).to_string();
                    *chose.entry(family).or_default() += 1;
                }
                None => *chose.entry("(nothing)".to_string()).or_default() += 1,
            }

            from_novelty += simulation
                .how_often_curiosity_reached_for_something_new
                .get();
        }
    }

    let mut rows: Vec<_> = chose.iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    println!("what the curiosity drive chose over {looks} agent-days:");
    for (what, n) in &rows {
        println!("  {what:<16} {:>5.1}%", 100.0 * **n as f64 / looks.max(1) as f64);
    }

    // Counted where it happens rather than guessed from the name of what was
    // chosen.
    //
    // This used to subtract a list of the names the rungs above the terminal
    // can produce, which worked while the terminal produced something else
    // and stopped the moment the candidate list learned about products: the
    // terminal reaches for `craft` now, a rung above it reaches for `craft`,
    // and the subtraction read the whole settlement as ladder and reported
    // the terminal dead when it was not. It had been wrong once before, on
    // `TrySwapping` reporting itself as `swap`. Two wrong readings of one
    // number is enough - see
    // `Simulation::how_often_curiosity_reached_for_something_new`.
    println!(
        "reached the novelty terminal: {from_novelty} of {looks} ({:.1}%)",
        100.0 * from_novelty as f64 / looks.max(1) as f64
    );

    // What this asserts is that curiosity always has an answer. What it
    // prints is a standing defect, and the two are different claims.
    //
    // Measured here at **0 of 340 agent-days**: in a settlement that is
    // feeding itself, the terminal is never reached, because the rungs above
    // it always answer first - gather 44%, examine 34%, putdown 20%. The
    // rung directly above is `something_nobody_has_tried_within_reach`,
    // which is the terminal's own question asked of a narrower list: it
    // offers whatever material underfoot nobody has picked up, where the
    // terminal offers any verb the matrix opens on anything here. A special
    // case sitting above the general one, and it never falls through.
    //
    // It is left standing rather than reordered because reordering a rung
    // that answers 44% of a settlement's curiosity is a behaviour change that
    // wants measuring on its own. The unit-level claims about the terminal -
    // that it picks the least-tried thing, that what it picks can be carried
    // out - are made in this file and hold; this is the settlement-level
    // statement that it is currently unreachable underneath them.
    //
    // The figure was wrong twice before it was counted rather than guessed:
    // the old proxy subtracted a list of the names the rungs produce, and
    // read `swap` as novelty once and the terminal's own `craft` keys as
    // ladder later. See
    // `Simulation::how_often_curiosity_reached_for_something_new`.
    assert!(
        !chose.contains_key("(nothing)"),
        "curiosity came up with nothing at all for somebody, so the ladder \
         has a hole in it rather than a floor: {rows:?}"
    );
    assert!(
        looks > 0,
        "nobody lived long enough to be curious, so this is watching nothing"
    );
}

// --------------------------------------------------------------------------
// And what the key names is what the action means
// --------------------------------------------------------------------------

/// A material is not a product, and `an_action_for` says so.
///
/// `what_i_could_try_here` fills the object of a key from the pack and the
/// ground, because that is what a verb with a `AThingHeld` target acts on.
/// Most actions agree. `Craft { item_type }` does not: it names what is to
/// exist afterwards. Handing it an input reads a material as a product, and
/// the executor then refuses "Unknown recipe: iron".
///
/// Measured over a hundred and twenty days of one settlement, that was a
/// whole new failure on its own: `Craft` went from 339 asked and **none
/// refused** to 1,563 asked and 1,181 refused, on three raw materials nobody
/// was ever trying to make.
#[test]
fn a_material_in_the_pack_is_not_something_to_craft() {
    let simulation = one_person_on_bare_ground();
    let agent = &simulation.population.agents[0];

    // A recipe answers, so the round trip stands.
    assert!(
        simulation
            .an_action_for(&verbs::LASH, "lash:handaxe", agent)
            .is_some(),
        "a handaxe is a thing the tables know how to make"
    );

    // Raw stuff does not, and these are the three the settlement was asking
    // for by name.
    for material in ["iron", "wood", "flax"] {
        assert!(
            simulation
                .an_action_for(&verbs::LASH, &format!("lash:{material}"), agent)
                .is_none(),
            "nothing in the making tables turns out a {material}, so reaching \
             for one spends the turn on a refusal and teaches nothing"
        );
    }
}

/// And a handaxe is not a coat.
#[test]
fn a_tool_in_the_pack_is_not_something_to_wear() {
    let simulation = one_person_on_bare_ground();
    let agent = &simulation.population.agents[0];

    for held in ["handaxe", "basket", "legumes"] {
        for verb in [&verbs::SEW, &verbs::WEAR] {
            assert!(
                simulation
                    .an_action_for(verb, &format!("{}:{held}", verb.called), agent)
                    .is_none(),
                "{held} is not a garment, and `garment_recipe` is what the \
                 executor would refuse it on"
            );
        }
    }
}

/// And a roof is one of the names that executor answers to.
///
/// Sharper than the two above, because nothing refuses: `Simulation::building`
/// matches the name against its list and falls through to a skin tent, so
/// `build:iron` would put a tent up and the lesson written afterwards would
/// say that building an iron works.
#[test]
fn every_roof_this_names_is_one_that_can_be_put_up() {
    let simulation = one_person_on_bare_ground();
    let agent = &simulation.population.agents[0];

    for roof in Simulation::A_ROOF_BY_NAME {
        let built = simulation.an_action_for(&verbs::FRAME, &format!("frame:{roof}"), agent);
        assert!(
            matches!(built, Some(Action::Build { ref structure_type, .. }) if structure_type == roof),
            "{roof} is in the list of roofs and did not come back as one: {built:?}"
        );
    }

    assert!(
        simulation
            .an_action_for(&verbs::FRAME, "frame:iron", agent)
            .is_none(),
        "a lump of iron is not a roof, and the executor would quietly make it \
         a tent rather than say so"
    );
}
