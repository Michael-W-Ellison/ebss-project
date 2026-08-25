// src/analytics/tests/discovery_tests.rs
//! Tests that an outcome is something an agent has to find out.
//!
//! "Some actions might not have readily apparent results (e.g., rock + fire =
//! ?) until the right conditions apply (shiny rock + fire = shiny lump). This
//! can lead to: shiny lump + hammer = crude metal knife blade."

use crate::agents::{Agent, AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::making::{self, METAL_BLADE, METAL_KNIFE, SHINY_LUMP};
use crate::environment::{Action, HeatSourceType};
use crate::world::{World, WorldConfig};

fn carrying(agent: &mut Agent, what: &str, how_many: u32) {
    agent
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.5));
}

fn one_agent_world() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let world = World::new(WorldConfig::default());
    Simulation::new(world, population)
}

/// Keep every fire in the world fuelled and burning.
fn keep_the_fire_in(simulation: &mut Simulation) {
    let fires: Vec<uuid::Uuid> = simulation
        .world
        .heat_sources
        .all()
        .into_iter()
        .map(|fire| fire.id)
        .collect();

    for fire in fires {
        let _ = simulation
            .world
            .add_fuel_to_heat_source(&fire, "wood".to_string(), 40.0);
        let _ = simulation.world.light_heat_source(&fire);
    }
}

/// Light a fire on the tile the agent is standing on.
fn a_fire_where_he_stands(simulation: &mut Simulation) {
    let where_he_is = simulation.population.agents[0].state.position;
    let fire = simulation
        .world
        .build_heat_source(HeatSourceType::Campfire, where_he_is, None)
        .expect("a fire can be built here");
    let _ = simulation
        .world
        .add_fuel_to_heat_source(&fire, "wood".to_string(), 40.0);
    simulation
        .world
        .light_heat_source(&fire)
        .expect("and lit");
}

// --- what a people arrives knowing -----------------------------------------

/// The metal chain is not something anybody is born knowing.
#[test]
fn nobody_is_born_knowing_how_to_work_metal() {
    for step in [SHINY_LUMP, METAL_BLADE, METAL_KNIFE] {
        assert!(
            !step.obvious,
            "{} should be a thing somebody has to find out",
            step.makes
        );
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    assert!(!founder.knows_how_to_make("shinylump"));
    assert!(!founder.knows_how_to_make("metalknife"));
    assert!(
        founder.knows_how_to_make("spear"),
        "but he can still make the things his people brought with them"
    );
}

/// A man who does not know how cannot plan towards it either.
#[test]
fn what_nobody_knows_is_not_a_step_anybody_plans() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "iron", 8);

    let holding = |what: &str| agent.how_many_i_have(what);
    let knows = |step: &making::Making| agent.knows_how_to(step);

    assert!(
        making::what_to_do_first_knowing("metalknife", &holding, &knows).is_none(),
        "a pack full of iron is not a step towards a knife nobody has seen"
    );
    assert!(
        making::what_to_do_first("shinylump", &holding).is_none(),
        "and the default is knowing only what a founder knows"
    );
}

/// Asking for it outright fails, and says why.
#[test]
fn making_what_nobody_knows_fails_saying_so() {
    let mut simulation = one_agent_world();
    carrying(&mut simulation.population.agents[0], "iron", 4);
    a_fire_where_he_stands(&mut simulation);

    let result = simulation.execute_action(
        &Action::Craft { item_type: "shinylump".to_string() },
        0,
    );
    assert!(!result.success);
    assert!(
        result
            .message
            .as_deref()
            .is_some_and(|said| said.contains("knows how to make")),
        "the failure should say nobody knows how: {:?}",
        result.message
    );
}

// --- finding out ------------------------------------------------------------

/// A curious man, holding a bright stone, at a fire, works it out.
#[test]
fn a_bright_stone_at_a_fire_is_eventually_understood() {
    let mut simulation = one_agent_world();
    a_fire_where_he_stands(&mut simulation);

    {
        let agent = &mut simulation.population.agents[0];
        carrying(agent, "iron", 8);
        if let Some(curiosity) = agent.drives.get_mut(DriveType::Curiosity) {
            curiosity.value = 1.0;
        }
    }

    let mut worked_it_out = false;
    for _ in 0..4000 {
        simulation.somebody_notices_something();
        if let Some(curiosity) = simulation.population.agents[0]
            .drives
            .get_mut(DriveType::Curiosity)
        {
            curiosity.value = 1.0;
        }
        if simulation.population.agents[0].knows_how_to_make("shinylump") {
            worked_it_out = true;
            break;
        }
    }

    assert!(
        worked_it_out,
        "a curious man with iron in his hands at a burning fire should \
         eventually see what the fire does to it"
    );
}

/// And away from a fire he never does.
#[test]
fn the_same_stone_in_the_cold_teaches_nothing() {
    let mut simulation = one_agent_world();

    {
        let agent = &mut simulation.population.agents[0];
        carrying(agent, "iron", 8);
        if let Some(curiosity) = agent.drives.get_mut(DriveType::Curiosity) {
            curiosity.value = 1.0;
        }
    }

    for _ in 0..4000 {
        simulation.somebody_notices_something();
        if let Some(curiosity) = simulation.population.agents[0]
            .drives
            .get_mut(DriveType::Curiosity)
        {
            curiosity.value = 1.0;
        }
    }

    assert!(
        !simulation.population.agents[0].knows_how_to_make("shinylump"),
        "rock and no fire is still just a rock"
    );
}

/// Nothing is worked out by somebody with other things on their mind.
#[test]
fn a_man_with_no_curiosity_in_him_notices_nothing() {
    let mut simulation = one_agent_world();
    a_fire_where_he_stands(&mut simulation);

    {
        let agent = &mut simulation.population.agents[0];
        carrying(agent, "iron", 8);
        if let Some(curiosity) = agent.drives.get_mut(DriveType::Curiosity) {
            curiosity.value = 0.0;
        }
    }

    for _ in 0..2000 {
        simulation.somebody_notices_something();
        if let Some(curiosity) = simulation.population.agents[0]
            .drives
            .get_mut(DriveType::Curiosity)
        {
            curiosity.value = 0.0;
        }
    }

    assert!(!simulation.population.agents[0].knows_how_to_make("shinylump"));
}

/// The chain only opens one link at a time: no lump, no blade.
#[test]
fn the_blade_cannot_be_found_out_before_the_lump() {
    let mut simulation = one_agent_world();

    {
        let agent = &mut simulation.population.agents[0];
        carrying(agent, "iron", 8);
        if let Some(curiosity) = agent.drives.get_mut(DriveType::Curiosity) {
            curiosity.value = 1.0;
        }
    }

    for _ in 0..2000 {
        simulation.somebody_notices_something();
        if let Some(curiosity) = simulation.population.agents[0]
            .drives
            .get_mut(DriveType::Curiosity)
        {
            curiosity.value = 1.0;
        }
    }

    assert!(
        !simulation.population.agents[0].knows_how_to_make("metalblade"),
        "the blade wants a lump, and there are no lumps in the world yet"
    );
}

// --- doing it once it is known ---------------------------------------------

/// Having found it out, he can do it - and it still wants the fire.
#[test]
fn once_known_the_work_still_wants_its_conditions() {
    let mut simulation = one_agent_world();
    simulation.population.agents[0].found_out_how_to("shinylump");
    carrying(&mut simulation.population.agents[0], "iron", 4);

    let cold = simulation.execute_action(
        &Action::Craft { item_type: "shinylump".to_string() },
        0,
    );
    assert!(!cold.success, "no fire, no lump");
    assert!(
        cold.message
            .as_deref()
            .is_some_and(|said| said.contains("fire")),
        "and it should say so: {:?}",
        cold.message
    );

    a_fire_where_he_stands(&mut simulation);
    let hot = simulation.execute_action(
        &Action::Craft { item_type: "shinylump".to_string() },
        0,
    );
    assert!(hot.success, "{:?}", hot.message);
    assert_eq!(
        simulation.population.agents[0].inventory.count_item("shinylump"),
        SHINY_LUMP.how_many
    );
}

/// A blade is beaten out with something, and beating wears it.
#[test]
fn a_blade_wants_a_hammer_in_the_hand() {
    let mut simulation = one_agent_world();
    simulation.population.agents[0].found_out_how_to("metalblade");
    carrying(&mut simulation.population.agents[0], "shinylump", 2);

    // Take his axe away and there is nothing to beat it out with.
    let had = simulation.population.agents[0].inventory.count_item("handaxe");
    simulation.population.agents[0]
        .inventory
        .remove_item("handaxe", had);

    let barehanded = simulation.execute_action(
        &Action::Craft { item_type: "metalblade".to_string() },
        0,
    );
    assert!(!barehanded.success);
    assert!(
        barehanded
            .message
            .as_deref()
            .is_some_and(|said| said.contains("handaxe")),
        "it should name what it wants: {:?}",
        barehanded.message
    );

    let axe = simulation.population.agents[0].a_tool_fresh_from_these_hands("handaxe", 1, 2.0);
    let before = axe.current_durability.unwrap();
    simulation.population.agents[0].inventory.add_item(axe);

    let result = simulation.execute_action(
        &Action::Craft { item_type: "metalblade".to_string() },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    let agent = &simulation.population.agents[0];
    assert_eq!(agent.inventory.count_item("metalblade"), 1);
    assert_eq!(
        agent.inventory.count_item("handaxe"),
        1,
        "the hammerstone is not used up by the beating"
    );
    assert!(
        agent.inventory.get_item("handaxe").unwrap().current_durability.unwrap() < before,
        "but it is worn by it"
    );
}

/// The whole chain, end to end, for somebody who has found it all out.
#[test]
fn a_people_that_has_found_it_all_out_can_make_a_metal_knife() {
    let mut simulation = one_agent_world();
    a_fire_where_he_stands(&mut simulation);

    {
        let agent = &mut simulation.population.agents[0];
        for step in making::everything_to_find_out() {
            agent.found_out_how_to(step.makes);
        }
        carrying(agent, "iron", 4);
        carrying(agent, "flax", 4);
    }

    for what in ["shinylump", "metalblade", "lashing", "metalknife"] {
        let result = simulation.execute_action(
            &Action::Craft { item_type: what.to_string() },
            0,
        );
        assert!(result.success, "making {what} failed: {:?}", result.message);
    }

    let agent = &simulation.population.agents[0];
    assert_eq!(agent.inventory.count_item("metalknife"), 1);
    assert!(
        agent.how_much_my_tools_help(SkillType::Leatherworking)
            > 1.0 + (making::KNIFE_FOR_BUTCHERING.how_much_better - 1.0),
        "a metal knife should beat a stone one at butchering"
    );
}

/// What one man found out is his, not everybody's.
#[test]
fn finding_something_out_is_one_man_finding_it_out() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    population.agents[0].found_out_how_to("shinylump");

    assert!(population.agents[0].knows_how_to_make("shinylump"));
    assert!(
        !population.agents[1].knows_how_to_make("shinylump"),
        "his neighbour has not seen it done"
    );
}

/// Knowing it opens the planning that was closed before.
#[test]
fn knowing_it_opens_the_chain_that_was_shut() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "iron", 4);
    carrying(agent, "flax", 4);

    for step in making::everything_to_find_out() {
        agent.found_out_how_to(step.makes);
    }

    let holding = |what: &str| agent.how_many_i_have(what);
    let knows = |step: &making::Making| agent.knows_how_to(step);

    let step = making::what_to_do_first_knowing("metalknife", &holding, &knows)
        .expect("iron and flax are now steps towards a metal knife");
    assert!(
        ["shinylump", "lashing"].contains(&step.makes),
        "he should start at one end of the chain or the other, not {}",
        step.makes
    );
}

/// A man who has found something out does it again to see it happen.
///
/// This is what puts the next link of the chain in anybody's hands: nothing
/// in a settlement wants a shiny lump, so unless curiosity makes one for its
/// own sake, nobody ever holds one and the blade is never found out.
#[test]
fn a_new_trick_gets_done_again_for_its_own_sake() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "iron", 4);

    assert_eq!(
        agent.what_i_would_try_out(),
        None,
        "there is nothing to try out until something has been found out"
    );

    agent.found_out_how_to("shinylump");
    assert_eq!(
        agent.what_i_would_try_out().as_deref(),
        Some("shinylump"),
        "and then there is"
    );
}

/// Once a people knows metal, a metal knife is what its hands want.
#[test]
fn knowing_metal_changes_what_a_pair_of_hands_wants() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    let stone = agent
        .what_i_would_rather_have(SkillType::Leatherworking)
        .map(|tool| tool.called);
    assert_eq!(
        stone, None,
        "a founder carries a stone knife already and wants nothing better"
    );

    for step in making::everything_to_find_out() {
        agent.found_out_how_to(step.makes);
    }

    let metal = agent
        .what_i_would_rather_have(SkillType::Leatherworking)
        .map(|tool| tool.called);
    assert_eq!(
        metal,
        Some("metalknife"),
        "having seen metal, a stone knife is no longer good enough"
    );
}

/// The whole thing, in a running world: iron goes into a pack, somebody
/// notices what a fire does to it, and the knowledge is real.
#[test]
fn a_settlement_can_find_metal_out_for_itself() {
    let mut population = Population::new();
    for _ in 0..6 {
        population.spawn_agent(AgentConfig::default());
    }
    let world = World::new(WorldConfig::default());
    let mut simulation = Simulation::new(world, population);

    // Put iron and a fire where the people are, so that this tests the
    // mechanism rather than whether a random world happens to put a mountain
    // within walking distance.
    a_fire_where_he_stands(&mut simulation);
    for agent in simulation.population.agents.iter_mut() {
        carrying(agent, "iron", 6);
    }

    let hearth = simulation.population.agents[0].state.position;

    let mut anybody_knows = false;
    for _ in 0..600 {
        simulation.tick();

        // Keep the conditions in place: a fire that stays lit, iron in every
        // pack, and people who have not wandered off. What is under test is
        // whether the noticing happens at all, not whether a settlement
        // happens to sit still.
        keep_the_fire_in(&mut simulation);
        for agent in simulation.population.agents.iter_mut() {
            agent.state.position = hearth;
            if agent.inventory.count_item("iron") < 2 {
                carrying(agent, "iron", 4);
            }
            if let Some(curiosity) = agent.drives.get_mut(DriveType::Curiosity) {
                if curiosity.value < 0.5 {
                    curiosity.value = 0.5;
                }
            }
        }
        if simulation
            .population
            .agents
            .iter()
            .any(|agent| agent.knows_how_to_make("shinylump"))
        {
            anybody_knows = true;
            break;
        }
    }

    assert!(
        anybody_knows,
        "six curious people at a fire with iron in their packs should work \
         out what the fire does to it inside fifty days"
    );
}
