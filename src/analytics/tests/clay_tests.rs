// src/analytics/tests/clay_tests.rs
//! Tests for clay, and for the fire that stops it being clay.
//!
//! `ResourceType::Clay`, `Pottery` and `Bricks` were three enum variants with
//! nothing whatever behind them. Clay had been spawning on every riverbank and
//! every marsh in every world since the project began and no agent could ever
//! pick any of it up: "clay" was missing from the vocabulary `Gather` answers
//! to, which is the only vocabulary it has.
//!
//! Nobody is handed pottery. A curious agent fetches a handful of something
//! nobody here has ever done anything with, finds that it holds a shape, and
//! then — either by trying it or by leaving a lump too near the fire — finds
//! out what a fire does to it. That is the technology, and it gets better with
//! repetition like everything else in this chain.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::making;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

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

fn some_clay(how_many: u32) -> InventoryItem {
    InventoryItem::new_with_weight("clay".to_string(), how_many, 1.0)
}

/// A fire, lit, where somebody is standing.
fn a_lit_fire_at(simulation: &mut Simulation, where_it_is: (i32, i32, i32)) {
    // Idempotent, because these tests keep it burning for a season and a
    // hearth cannot be built twice on the same ground.
    let fire = match simulation.world.build_heat_source(
        crate::environment::HeatSourceType::Campfire,
        where_it_is,
        None,
    ) {
        Ok(fire) => fire,
        Err(_) => simulation
            .world
            .heat_sources
            .get_at_position(where_it_is)
            .map(|source| source.id)
            .expect("there is one here already"),
    };

    let _ = simulation
        .world
        .add_fuel_to_heat_source(&fire, "wood".to_string(), 200.0);
    let _ = simulation.world.light_heat_source(&fire);
}

// --------------------------------------------------------------------------
// Getting hold of it at all
// --------------------------------------------------------------------------

/// Clay is in the ground, in marshes and along riverbanks.
#[test]
fn clay_is_where_the_wet_ground_is() {
    use crate::world::resource_spawning::TerrainResourceMapper;

    let ground = TerrainResourceMapper::preferred_terrains(ResourceType::Clay);
    assert!(ground.contains(&TerrainType::Wetland));
    assert!(ground.contains(&TerrainType::Riverbank));
}

/// And `Gather` knows the word for it, which for the whole life of this
/// project it did not.
#[test]
fn clay_can_actually_be_gathered() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Clay, here, 50));

    let result = simulation.execute_action(
        &Action::Gather {
            resource_type: "clay".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    assert!(
        simulation.population.agents[0].how_many_i_have("clay") > 0,
        "and it should be in the pack"
    );
}

/// A curious agent fetches a handful of something nobody here has ever done
/// anything with. Nothing else in the model would ever put clay in a pack:
/// every other material is gathered by somebody who already wants what it
/// makes, and nobody can want a pot before anybody has made one.
#[test]
fn curiosity_fetches_a_material_nobody_has_tried() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Clay, here, 50));

    let position = simulation.population.agents[0].state.position;
    let fetched = simulation
        .something_nobody_has_tried_within_reach(&simulation.population.agents[0], position);

    assert_eq!(
        fetched.as_deref(),
        Some("clay"),
        "there is clay underfoot and nobody here has ever done anything with any"
    );
}

/// And walks past it once there is nothing left to find out about it.
#[test]
fn somebody_who_has_tried_everything_walks_past_it() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Clay, here, 50));

    for working in making::EVERY_WORKING {
        simulation.population.agents[0].found_out_how_to(working.makes);
    }

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .something_nobody_has_tried_within_reach(&simulation.population.agents[0], position)
            .is_none(),
        "a man who has tried everything clay does walks past the clay"
    );
}

// --------------------------------------------------------------------------
// Playing with it
// --------------------------------------------------------------------------

/// Clay holds a shape. Nobody is born knowing that, and finding it out costs
/// nothing but an idle afternoon.
#[test]
fn clay_holds_a_shape_and_it_is_a_discovery() {
    let working = making::how_to_work("mold", "clay").expect("clay molds");

    assert_eq!(working.makes, "claypot");
    assert!(
        !working.obvious,
        "nobody arrives knowing what clay does; it has to be found out"
    );
    assert!(
        !working.over_a_fire,
        "pressing a lump into a shape wants no fire at all"
    );
    assert_eq!(
        working.wants_water, 0.0,
        "asking for carried water here would need a vessel to make the vessel"
    );
}

/// A curious agent with clay in its pack tries it.
#[test]
fn a_curious_agent_with_clay_tries_molding_it() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(some_clay(6));

    // No fire, so the only thing clay can be tried for is molding: firing it
    // is refused without one, and the decision knows that rather than
    // spending the turn finding out.
    let tried = simulation.population.agents[0].what_working_i_would_try_out(false);

    assert_eq!(
        tried,
        Some(("mold".to_string(), "clay".to_string())),
        "a lump of clay and an idle afternoon"
    );
}

/// What comes out is worth almost nothing until a fire has had it.
#[test]
fn an_unfired_shape_holds_nothing() {
    let working = making::how_to_work("mold", "clay").expect("clay molds");
    assert!(
        working.holds.is_none(),
        "a shape in unfired clay comes apart in the rain"
    );
}

// --------------------------------------------------------------------------
// Firing it
// --------------------------------------------------------------------------

/// A fired pot holds things, which is the whole of the technology: the first
/// thing this people can make that keeps something else.
#[test]
fn a_fired_pot_holds_things() {
    let working = making::how_to_work("fire", "claypot").expect("a pot fires");

    assert_eq!(working.makes, "stoneware");
    assert!(working.over_a_fire, "a fire is the whole point");
    assert!(!working.obvious, "and nobody is born knowing it");
    assert!(
        working.holds.is_some_and(|held| held > 0.0),
        "what comes out holds water"
    );
}

/// Bricks are a separate thing to find out, off the same material and the same
/// fire. A people that has fired a pot has not thereby learned to make a wall.
#[test]
fn bricks_are_their_own_discovery() {
    let working = making::how_to_work("fire", "clay").expect("clay fires");

    assert_eq!(working.makes, "bricks");
    assert!(working.over_a_fire);
    assert!(!working.obvious);
    assert_ne!(
        working.makes,
        making::how_to_work("fire", "claypot").unwrap().makes,
        "knowing one is not knowing the other"
    );
}

/// Firing wants a fire, and says so rather than quietly working without one.
#[test]
fn firing_without_a_fire_is_refused() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("claypot".to_string(), 2, 1.0));
    simulation.population.agents[0].found_out_how_to("stoneware");

    let result = simulation.execute_action(
        &Action::Work {
            verb: "fire".to_string(),
            to: "claypot".to_string(),
        },
        0,
    );

    assert!(!result.success, "{:?}", result.message);
}

// --------------------------------------------------------------------------
// The accident
// --------------------------------------------------------------------------

/// Nobody intends this. Somebody is sitting at a fire with clay in their pack
/// because they picked it up walking past a riverbank, a lump of it ends up in
/// the embers, and in the morning it is not clay any more.
#[test]
fn a_lump_left_at_the_fire_comes_out_hard() {
    let mut found_out = 0;

    for _ in 0..40 {
        let mut simulation = one_person();
        let here = simulation.population.agents[0].state.position;

        simulation.population.agents[0]
            .inventory
            .add_item(some_clay(30));

        a_lit_fire_at(&mut simulation, here);

        for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 60) {
            simulation.population.agents[0].state.position = here;
            a_lit_fire_at(&mut simulation, here);
            simulation.tick();

            if !simulation.population.agents[0].state.is_alive {
                break;
            }

            if simulation.population.agents[0]
                .what_i_found_out()
                .contains(Simulation::THAT_FIRE_HARDENS_CLAY)
            {
                found_out += 1;
                break;
            }
        }
    }

    assert!(
        found_out > 0,
        "forty people sat at a fire with clay in the pack for two months apiece \
         and not one lump ever found the embers"
    );
}

/// And what it teaches is the same thing the working makes, so having seen it
/// is the same thing as knowing how.
#[test]
fn what_the_embers_teach_is_what_the_working_makes() {
    assert_eq!(
        Simulation::THAT_FIRE_HARDENS_CLAY,
        making::how_to_work("fire", "claypot").unwrap().makes,
        "seeing clay come out of a fire hard is knowing how to fire clay"
    );
}

/// Nobody with no clay finds any in the fire.
#[test]
fn nothing_comes_out_of_a_fire_nobody_put_clay_in() {
    let mut simulation = one_person();

    // And no clay anywhere to be curious about, or the agent fetches some and
    // the test measures the opposite of what it means to.
    simulation
        .world
        .resources
        .retain(|resource| resource.resource_type != ResourceType::Clay);

    let here = simulation.population.agents[0].state.position;
    a_lit_fire_at(&mut simulation, here);

    for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 20) {
        simulation.population.agents[0].state.position = here;
        simulation.population.agents[0].inventory.remove_item("clay", 99);
        a_lit_fire_at(&mut simulation, here);
        simulation.tick();
        if !simulation.population.agents[0].state.is_alive {
            break;
        }
    }

    assert!(
        !simulation.population.agents[0]
            .what_i_found_out()
            .contains(Simulation::THAT_FIRE_HARDENS_CLAY),
        "there was no clay to harden"
    );
}

// --------------------------------------------------------------------------
// What it is worth
// --------------------------------------------------------------------------

/// Whatever came off the fire is still something the rest of the model knows
/// how to price and store.
#[test]
fn what_comes_off_the_fire_is_a_thing_the_world_knows() {
    use crate::agents::storage_integration::id_to_item_type;
    use crate::world::ItemType;

    assert_eq!(id_to_item_type("claypot"), Some(ItemType::Clay));
    assert_eq!(id_to_item_type("stoneware"), Some(ItemType::Pottery));
    assert_eq!(id_to_item_type("bricks"), Some(ItemType::Bricks));
}

/// And molding is a verb the matrix knows something performs, which it did
/// not before: `MOLD` had sat in the matrix with nothing carrying it out.
#[test]
fn molding_is_a_live_verb() {
    let mold = crate::environment::verbs::EVERY_VERB
        .iter()
        .find(|verb| verb.called == "mold")
        .expect("the matrix has always had a word for it");

    assert!(mold.is_live(), "and now something does it");
}
