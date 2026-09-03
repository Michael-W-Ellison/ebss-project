// src/analytics/tests/salt_tests.rs
//! Tests for salt, and for the sea it mostly comes out of.
//!
//! `PreparationState::Salted` was written, tested and unreachable for the
//! whole life of this project, because there was no salt anywhere in the
//! world. There was also only one kind of water: a river, a spring and the
//! sea were the same terrain and the same drink.
//!
//! Salt is now three things — a crust on a flat where a shallow sea dried up,
//! a rare seam in the hills, and what is left when you boil the sea — and the
//! sea is a mistake a thirsty man can make.

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, ResourceType, TerrainType, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 1.0);
    meal.food_data = database.create_food_data(&of, 0);
    meal
}

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

/// Put a stretch of sea beside somebody.
fn a_sea_at(simulation: &mut Simulation, around: Position) {
    for dy in -2..=2 {
        for dx in -2..=2 {
            let there = Position::new(around.x + dx, around.y + dy);
            if let Some(tile) = simulation.world.grid.get_tile_mut(&there) {
                tile.terrain.terrain_type = TerrainType::Sea;
            }
        }
    }
}

// --------------------------------------------------------------------------
// The ground
// --------------------------------------------------------------------------

/// The sea and a salt marsh are salt. A river is not.
#[test]
fn only_some_water_is_salt() {
    use crate::world::Terrain;

    assert!(Terrain::new(TerrainType::Sea).is_the_water_salt());
    assert!(Terrain::new(TerrainType::SaltMarsh).is_the_water_salt());

    assert!(!Terrain::new(TerrainType::Water).is_the_water_salt());
    assert!(!Terrain::new(TerrainType::Wetland).is_the_water_salt());
    assert!(!Terrain::new(TerrainType::Riverbank).is_the_water_salt());
}

/// A salt flat is dry ground you can walk on. The sea is not.
#[test]
fn a_flat_is_walkable_and_a_sea_is_not() {
    use crate::world::Terrain;

    assert!(Terrain::new(TerrainType::SaltFlat).is_walkable());
    assert!(Terrain::new(TerrainType::SaltMarsh).is_walkable());
    assert!(!Terrain::new(TerrainType::Sea).is_walkable());
}

/// All three are places salt can be had.
#[test]
fn salt_can_be_had_in_three_places() {
    use crate::world::Terrain;

    for ground in [TerrainType::Sea, TerrainType::SaltMarsh, TerrainType::SaltFlat] {
        assert!(
            Terrain::new(ground).is_there_salt_here(),
            "{ground:?} should have salt in it"
        );
    }

    assert!(!Terrain::new(TerrainType::Water).is_there_salt_here());
    assert!(!Terrain::new(TerrainType::Plains).is_there_salt_here());
}

/// Nothing grows on a salt flat. That is the point of one.
#[test]
fn nothing_grows_on_a_salt_flat() {
    use crate::world::Soil;

    let flat = Soil::for_terrain(TerrainType::SaltFlat);
    let plains = Soil::for_terrain(TerrainType::Plains);

    assert!(
        flat.nutrients < plains.nutrients / 10.0,
        "a salt flat should be all but dead ground: {} against {}",
        flat.nutrients,
        plains.nutrients
    );
}

// --------------------------------------------------------------------------
// Drinking it
// --------------------------------------------------------------------------

/// Nobody drinks the sea while there is anything else going. This is not a
/// discovery: a mouthful tells you what it is.
#[test]
fn nobody_drinks_the_sea_who_does_not_have_to() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.state.gone_without_water_for(0);
    assert!(
        !agent.would_i_drink_the_sea(),
        "a man with any choice at all leaves it alone"
    );
}

/// And everybody stops knowing better once they are dying of thirst, which is
/// exactly how people have always come to do it.
#[test]
fn a_man_dying_of_thirst_drinks_it_anyway() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.state.gone_without_water_for(100_000);
    assert!(agent.state.is_dehydrated());
    assert!(agent.would_i_drink_the_sea());
}

/// It slakes the thirst on the tick and costs more than it gave over the days
/// after — "even if it seems to temporarily satiate it".
#[test]
fn the_sea_costs_more_than_it_gives() {
    /// What is left in a body's skin after three days, salted or not.
    ///
    /// **The body on its own, and two of them.** This test has been asked
    /// three wrong ways. It followed one man and held his thirst steady
    /// between ticks with `gone_without_water_for(0)`, which fills the skin
    /// back up - so the fixture's own way of holding everything else still
    /// erased the one thing it meant to measure, and it read 0.3 against 0.3.
    /// Asked as the worst thirst two men reach it reads backwards, because
    /// the one the salt is hurting dies sooner and so records a *lower* peak -
    /// 0.57 against 0.80. Asked as time to death in a live world neither man
    /// dies at all, because a live world has water in it and he goes and
    /// drinks.
    ///
    /// What the claim is actually about is what the salt does to a body over
    /// the days after, so the body is what to ask. Neither of these two
    /// drinks; the only difference between them is the salt.
    ///
    /// The mouthful of water a sea drink brings with it is not here on
    /// purpose - it is handled where the drinking happens, in
    /// `Simulation::gathering`, which takes half a drink's worth straight off
    /// the hydration and then calls `drank_salt_water` for the rest.
    fn water_left_after_three_days(drinks_the_sea: bool) -> f32 {
        let mut agent = Agent::new(AgentConfig::default());
        agent.state.physiology.hydration = 1.0;
        agent.state.health = 100.0;

        if drinks_the_sea {
            agent.drank_salt_water(0);
        }

        for tick in 1..=(crate::environment::seasons::TICKS_PER_DAY * 3) {
            agent.state.last_ate_tick = tick;
            agent.state.physiology.reserve = agent.state.physiology.reserve_capacity;
            agent.tick_with_percepts(tick);
            agent.process_survival_tick(tick);
        }

        agent.state.physiology.hydration
    }

    let after_the_sea = water_left_after_three_days(true);
    let left_alone = water_left_after_three_days(false);

    assert!(
        after_the_sea < left_alone,
        "three days on, the man who drank the sea should have less water in \
         him than the man who drank nothing: {after_the_sea} against \
         {left_alone}"
    );
}

/// And the salt goes, eventually.
#[test]
fn the_salt_works_its_way_out() {
    let mut simulation = one_person();
    simulation.population.agents[0].drank_salt_water(0);

    assert!(simulation.population.agents[0].state.salt_in_me > 0.0);

    for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 20) {
        simulation.tick();
        if !simulation.population.agents[0].state.is_alive {
            break;
        }
    }

    assert_eq!(
        simulation.population.agents[0].state.salt_in_me, 0.0,
        "twenty days should be long enough to be rid of one drink of it"
    );
}

/// Drinking it goes in the book against the doing of it.
#[test]
fn drinking_the_sea_is_recorded_against_it() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    let before = agent.lessons.tried_this(Agent::DRINKING_THE_SEA);
    agent.drank_salt_water(0);

    assert!(agent.lessons.tried_this(Agent::DRINKING_THE_SEA) > before);
}

// --------------------------------------------------------------------------
// Boiling for it
// --------------------------------------------------------------------------

/// No sea, no salt.
#[test]
fn boiling_needs_a_sea_to_boil() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    // Make quite sure there is no salt water anywhere near.
    for dy in -6..=6 {
        for dx in -6..=6 {
            let there = Position::new(here.x + dx, here.y + dy);
            if let Some(tile) = simulation.world.grid.get_tile_mut(&there) {
                if tile.terrain.is_the_water_salt() {
                    tile.terrain.terrain_type = TerrainType::Plains;
                }
            }
        }
    }

    let result = simulation.execute_action(&Action::Boil, 0);
    assert!(!result.success, "there is nothing here to boil");
}

/// And a fire to boil it over.
#[test]
fn boiling_needs_a_fire() {
    let mut simulation = one_person();
    a_sea_at(&mut simulation, Position::new(27, 25));

    let result = simulation.execute_action(&Action::Boil, 0);
    assert!(
        !result.success,
        "a pot of sea water and no fire is a pot of sea water"
    );
}

/// Salt is worth what it is because a pot of the sea leaves almost none of it.
#[test]
fn a_pot_of_the_sea_leaves_very_little() {
    assert!(
        Simulation::WHAT_A_POT_OF_THE_SEA_LEAVES <= 3,
        "if boiling the sea were cheap nobody would ever have walked to a flat for it"
    );
}

// --------------------------------------------------------------------------
// Rubbing it in
// --------------------------------------------------------------------------

/// Salting keeps a thing about seven times as long, and unlike drying it
/// needs neither a fortnight of sun nor a fire kept going.
#[test]
fn salting_keeps_a_thing_far_longer() {
    let raw = PreparationState::Raw.spoilage_multiplier();
    let salted = PreparationState::Salted.spoilage_multiplier();

    assert!(
        salted < raw / 5.0,
        "salted should keep several times as long as raw: {salted} against {raw}"
    );
}

/// You cannot salt what you have no salt for.
#[test]
fn salting_needs_salt() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 4));

    let result = simulation.execute_action(
        &Action::Salt {
            what: "meatportions".to_string(),
        },
        0,
    );

    assert!(!result.success);
}

/// With salt, it works, and it costs the salt.
#[test]
fn salting_works_and_uses_the_salt_up() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.add_item(a_meal(ItemType::Meat, "meatportions", 4));
        agent.inventory.add_item(InventoryItem::new_with_weight(
            "salt".to_string(),
            3,
            0.2,
        ));
    }

    let result = simulation.execute_action(
        &Action::Salt {
            what: "meatportions".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        agent
            .inventory
            .get_item("meatportions")
            .and_then(|item| item.food_data.as_ref())
            .map(|food| food.preparation),
        Some(PreparationState::Salted)
    );
    assert_eq!(agent.how_many_i_have("salt"), 2, "and it cost a measure");
}

/// A whole carcass will not take salt any more than it will dry: somebody has
/// to cut it up first.
#[test]
fn a_whole_carcass_will_not_take_salt() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 4));
        agent.inventory.add_item(InventoryItem::new_with_weight(
            "salt".to_string(),
            3,
            0.2,
        ));
    }

    let result = simulation.execute_action(
        &Action::Salt {
            what: "meat".to_string(),
        },
        0,
    );

    assert!(!result.success);
}

/// And an agent picks the right thing to rub it into.
#[test]
fn somebody_with_salt_knows_what_to_put_it_on() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 6));
    assert!(
        agent.what_i_could_salt().is_none(),
        "a whole carcass is not the answer"
    );

    agent
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 6));
    assert_eq!(
        agent.what_i_could_salt().map(|(what, _)| what).as_deref(),
        Some("meatportions")
    );
}

// --------------------------------------------------------------------------
// Where it comes from
// --------------------------------------------------------------------------

/// Salt is a mineral, and it is dear.
#[test]
fn salt_is_a_dear_mineral() {
    assert_eq!(ResourceType::Salt.category(), "Mineral");
}

/// A world with a coast has salt on the ground somewhere.
///
/// Not every world has a coast — the sea is generated from elevation, so a
/// world whose ground never falls that far simply has none, and a people
/// living in it has to boil or go without. So this asks the conditional: if
/// there are flats, there is salt.
#[test]
fn a_world_with_flats_has_salt_on_them() {
    let mut worlds_with_flats = 0;
    let mut worlds_with_salt = 0;

    for _ in 0..8 {
        let world = World::new(WorldConfig::default());

        let has_flats = (0..world.grid.height).any(|y| {
            (0..world.grid.width).any(|x| {
                world
                    .grid
                    .get_tile(&Position::new(x as i32, y as i32))
                    .is_some_and(|tile| tile.terrain.terrain_type == TerrainType::SaltFlat)
            })
        });

        if has_flats {
            worlds_with_flats += 1;
            if world
                .resources
                .iter()
                .any(|resource| resource.resource_type == ResourceType::Salt)
            {
                worlds_with_salt += 1;
            }
        }
    }

    assert_eq!(
        worlds_with_flats, worlds_with_salt,
        "every world that has flats in it should have salt on them"
    );
}
