// src/analytics/tests/keeping_beasts_tests.rs
//! Animal husbandry (#354): a young one left by a kill raised by hand, kept
//! beasts that follow their keeper and are folded at night, milk, wool and
//! dung, the herd culled when it can spare one, and a camp moved on when the
//! herd has eaten the grazing down.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::catalogue;
use crate::environment::Action;
use crate::world::{Position, TerrainType, World, WorldConfig};

/// One grown person at (30, 30) on open plains three hundred cells across,
/// nobody else and no beasts.
fn a_keeper() -> Simulation {
    let mut world = World::new(WorldConfig { size: (300, 300), ..WorldConfig::default() });
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    for x in 0..world.grid.width as i32 {
        for y in 0..world.grid.height as i32 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(25);
    agent.state.position = (30, 30, 0);
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    simulation
}

fn some(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 1.0));
}

/// A beast of this kind at a place, grown or not.
fn a_beast(simulation: &mut Simulation, kind: &str, at: (i32, i32), grown: bool) -> uuid::Uuid {
    let id = simulation.world.spawn_animal(kind.to_string(), at).expect("it spawns");
    let animal = simulation.world.animals.get_mut(&id).unwrap();
    animal.age = if grown { animal.maturity_age } else { 0 };
    animal.hunger = 0.0;
    id
}

/// A beast of this kind, tame and kept by the one person, beside them.
fn a_kept_beast(simulation: &mut Simulation, kind: &str, grown: bool) -> uuid::Uuid {
    let keeper = simulation.population.agents[0].id;
    let id = a_beast(simulation, kind, (31, 30), grown);
    let animal = simulation.world.animals.get_mut(&id).unwrap();
    animal.owner_id = Some(keeper);
    animal.is_domesticated = true;
    animal.tame_level = 1.0;
    id
}

/// Keeping a beast is found out, and the fold and the cull come with it.
#[test]
fn the_catalogue_knows_keeping_is_found_out() {
    for found in ["do:Tame", "do:PutUpFold", "do:Slaughter", "do:MuckOut", "do:CollectAnimalProduct"] {
        assert!(!catalogue::is_born_knowledge(found), "{found}");
    }
    assert_eq!(catalogue::what_comes_before("do:PutUpFold"), ["do:Tame"]);
    assert_eq!(catalogue::what_comes_before("do:Slaughter"), ["do:Tame"]);

    let mut simulation = a_keeper();
    let agent = &mut simulation.population.agents[0];
    assert!(!agent.knows_how_to_do("PutUpFold"));
    agent.found_out_how_to("do:Tame");
    assert!(agent.knows_how_to_do("PutUpFold") && agent.knows_how_to_do("Slaughter"));
    assert!(!agent.knows_how_to_do("MuckOut"), "dung is seen, not reasoned to");
}

/// A ewe killed, and the lamb by her follows the hunter; a grown one does
/// not, nor anything of a kind nobody keeps.
#[test]
fn a_young_one_left_by_a_kill_follows_the_hunter() {
    let mut simulation = a_keeper();
    let hunter = simulation.population.agents[0].id;
    let ewe = a_beast(&mut simulation, "sheep", (31, 30), true);
    let lamb = a_beast(&mut simulation, "sheep", (40, 38), false);
    let ram = a_beast(&mut simulation, "sheep", (32, 30), true);

    let species = simulation.world.animals.get_species("sheep").unwrap().clone();
    simulation.world.animals.get_mut(&ewe).unwrap().take_damage(1.0e6);
    assert!(simulation.a_young_one_left_behind(0, &species, (31, 30)));

    let lamb = simulation.world.animals.get(&lamb).unwrap();
    assert_eq!(lamb.owner_id, Some(hunter));
    assert!(!lamb.is_domesticated, "following is not tame");
    assert_eq!(lamb.position, (30, 30), "it came to the hunter");
    assert_eq!(simulation.world.animals.get(&ram).unwrap().owner_id, None, "a grown one does not follow");

    // A deer is not kept
    let deer = simulation.world.animals.get_species("deer").unwrap().clone();
    a_beast(&mut simulation, "deer", (33, 30), false);
    assert!(!simulation.a_young_one_left_behind(0, &deer, (33, 30)));
}

/// Fed by hand every day, a young one is tame within a fortnight, and its
/// keeper has found out keeping; unfed, it is gone in a few days.
#[test]
fn a_young_one_fed_every_day_stays() {
    let mut simulation = a_keeper();
    let keeper = simulation.population.agents[0].id;
    let lamb = a_beast(&mut simulation, "sheep", (31, 30), false);
    {
        let animal = simulation.world.animals.get_mut(&lamb).unwrap();
        animal.owner_id = Some(keeper);
        animal.tame_level = Simulation::HOW_TAME_A_YOUNG_ONE_TAKEN_IS;
    }
    some(&mut simulation, "greens", 30);
    let feed = Action::Tame { animal_id: lamb, food_type: None };
    let a_day = crate::environment::seasons::TICKS_PER_DAY;

    let mut days = 0;
    while !simulation.world.animals.get(&lamb).unwrap().is_domesticated {
        days += 1;
        assert!(days <= 14, "a fortnight of feeding");
        simulation.current_turn += a_day;
        let fed = simulation.execute_action(&feed, 0);
        assert!(fed.success, "{:?}", fed.message);
        assert!(!simulation.execute_action(&feed, 0).success, "once a day");
    }
    assert!(simulation.population.agents[0].what_i_found_out().contains("do:Tame"));

    // And one nobody feeds runs off
    let other = a_beast(&mut simulation, "goat", (31, 31), false);
    {
        let animal = simulation.world.animals.get_mut(&other).unwrap();
        animal.owner_id = Some(keeper);
        animal.tame_level = Simulation::HOW_TAME_A_YOUNG_ONE_TAKEN_IS;
    }
    let half_hours = 5 * crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
    for _ in 0..half_hours {
        simulation.the_kept_stay_by_their_keepers();
    }
    assert_eq!(simulation.world.animals.get(&other).unwrap().owner_id, None);
    assert_eq!(simulation.world.animals.get(&lamb).unwrap().owner_id, Some(keeper), "the tame one stays");
}

/// By day a kept beast is walked along with its keeper; at night it is shut
/// in the fold its keeper sleeps by, out of the wolves' reckoning, and leaves
/// its dung there.
#[test]
fn a_kept_beast_follows_and_is_folded_at_night() {
    let mut simulation = a_keeper();
    let goat = a_kept_beast(&mut simulation, "goat", true);
    simulation.population.agents[0].state.position = (130, 30, 0);
    simulation.the_kept_stay_by_their_keepers();
    let at = simulation.world.animals.get(&goat).unwrap().position;
    assert!((at.0 - 130).abs() <= 3 && (at.1 - 30).abs() <= 3, "walked along: {at:?}");

    // A fold, put up by somebody who has found out keeping
    simulation.population.agents[0].found_out_how_to("do:Tame");
    assert!(!simulation.execute_action(&Action::PutUpFold, 0).success, "no wood");
    some(&mut simulation, "wood", Simulation::WHAT_A_FOLD_TAKES);
    assert!(simulation.execute_action(&Action::PutUpFold, 0).success);
    assert!(!simulation.execute_action(&Action::PutUpFold, 0).success, "one is enough");

    simulation.population.agents[0].fatigue.is_sleeping = true;
    let a_night = crate::environment::seasons::PLANNING_PERIODS_PER_DAY / 3;
    for _ in 0..a_night {
        simulation.the_kept_stay_by_their_keepers();
    }
    let goat_now = simulation.world.animals.get(&goat).unwrap();
    assert!(goat_now.in_the_fold);
    assert_eq!(goat_now.position, (130, 30));
    let dung = simulation.world.folds[0].dung;
    assert!(dung > 0.0 && dung < 1.0, "a night's dung: {dung}");

    // Awake, it is let out
    simulation.population.agents[0].fatigue.is_sleeping = false;
    simulation.the_kept_stay_by_their_keepers();
    assert!(!simulation.world.animals.get(&goat).unwrap().in_the_fold);
}

/// A ready product waits to be taken: milk only while she nurses, and only
/// by a keeper who has seen her in milk.
#[test]
fn milk_waits_for_her_keeper() {
    let mut simulation = a_keeper();
    let goat = a_kept_beast(&mut simulation, "goat", true);
    let milk = Action::CollectAnimalProduct { animal_id: goat };
    {
        let animal = simulation.world.animals.get_mut(&goat).unwrap();
        animal.product_timers.insert("milk".to_string(), 0);
        for _ in 0..100 {
            animal.turn_products();
        }
        assert!(!animal.has_ready("milk"), "dry");
        animal.nursing_for = crate::environment::fauna::Animal::HOW_LONG_SHE_NURSES;
        for _ in 0..100 {
            animal.turn_products();
        }
        assert!(animal.has_ready("milk"), "and it waited");
    }
    assert!(!simulation.execute_action(&milk, 0).success, "nobody knows to milk her");

    simulation.the_kept_stay_by_their_keepers();
    assert!(simulation.population.agents[0].what_i_found_out().contains("milk"));
    let taken = simulation.execute_action(&milk, 0);
    assert!(taken.success, "{:?}", taken.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("milk"), 1);
    assert!(!simulation.execute_action(&milk, 0).success, "once a day");
}

/// What is born to a kept mother is kept, and she comes into milk.
#[test]
fn the_young_of_a_kept_mother_are_kept() {
    let mut simulation = a_keeper();
    let keeper = simulation.population.agents[0].id;
    let goat = a_kept_beast(&mut simulation, "goat", true);
    {
        let animal = simulation.world.animals.get_mut(&goat).unwrap();
        animal.is_pregnant = true;
        animal.pregnancy_timer = 0;
    }
    let before = simulation.world.animals.get_all().len();
    simulation.world.animals.process_births();
    let born: Vec<_> = simulation.world.animals.get_all()[before..].to_vec();
    assert!(!born.is_empty());
    for kid in born {
        assert_eq!(kid.owner_id, Some(keeper));
        assert!(kid.is_domesticated);
    }
    assert!(simulation.world.animals.get(&goat).unwrap().nursing_for > 0);
}

/// A herd spares one for the pot above the four kept back for breeding, and
/// one starving keeps back only a pair.
#[test]
fn the_herd_spares_one_when_it_can() {
    let mut simulation = a_keeper();
    simulation.population.agents[0].found_out_how_to("do:Tame");
    for _ in 0..4 {
        a_kept_beast(&mut simulation, "sheep", true);
    }
    let here = (30, 30, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_beast_to_spare(&agent, here, false), None, "four are kept back");
    assert!(simulation.a_beast_to_spare(&agent, here, true).is_some(), "but not from a starving man");

    let fifth = a_kept_beast(&mut simulation, "sheep", true);
    simulation.world.animals.get_mut(&fifth).unwrap().age += 1000;
    let Some(Action::Slaughter { animal_id }) = simulation.a_beast_to_spare(&agent, here, false) else {
        panic!("five can spare one");
    };
    assert_eq!(animal_id, fifth, "the oldest goes");

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_durability("knife".to_string(), 1, 25.0, crate::agents::Quality::Common));
    let killed = simulation.execute_action(&Action::Slaughter { animal_id }, 0);
    assert!(killed.success, "{:?}", killed.message);
    assert!(!simulation.world.animals.get(&fifth).unwrap().is_alive());
    assert!(simulation.population.agents[0].how_many_i_have("meat") > 0);
}

/// Dung carried out of a fold burns, for whoever has seen what it is.
#[test]
fn dung_from_the_fold_burns() {
    let mut simulation = a_keeper();
    simulation.world.folds.push(crate::world::Fold { at: Position::new(30, 30), put_up: 0, dung: 5.0 });
    assert!(!simulation.execute_action(&Action::MuckOut, 0).success, "nobody knows");
    simulation.the_kept_stay_by_their_keepers();
    assert!(simulation.population.agents[0].knows_how_to_do("MuckOut"), "seen lying in the fold");
    assert!(simulation.execute_action(&Action::MuckOut, 0).success);
    assert_eq!(simulation.population.agents[0].how_many_i_have("dung"), 5);

    let wanted = simulation.wood_a_fire_here_takes(0);
    some(&mut simulation, "wood", wanted.saturating_sub(5));
    assert_eq!(simulation.population.agents[0].fuel_i_carry(), wanted.max(5));
    let lit = simulation.execute_action(&Action::LightFire, 0);
    assert!(lit.success, "{:?}", lit.message);
    assert!(simulation.population.agents[0].how_many_i_have("dung") < 5, "the dung went on first");
}

/// A herd that has eaten the place down moves its people on to fresh
/// grazing; a field holds them.
#[test]
fn a_hungry_herd_moves_the_camp() {
    let mut simulation = a_keeper();
    simulation.population.agents[0].state.position = (100, 100, 0);
    let ewe = a_kept_beast(&mut simulation, "sheep", true);
    simulation.world.animals.get_mut(&ewe).unwrap().position = (101, 100);
    let agent = simulation.population.agents[0].clone();
    assert!(!simulation.my_herd_is_going_hungry(&agent));

    let animal = simulation.world.animals.get_mut(&ewe).unwrap();
    animal.hunger = animal.max_hunger * 0.9;
    assert!(simulation.my_herd_is_going_hungry(&agent));
    assert!(simulation.fresh_grazing_from((100, 100, 0)).is_some());
    let moved = simulation.moving_on(&agent, (100, 100, 0));
    assert!(matches!(moved, Some(Action::Move { .. })), "got {moved:?}");
}

/// With nothing in the pack to feed it, a young one is led to graze and
/// handled each day, and is tame within a month all the same.
#[test]
fn a_young_one_tended_without_fodder_stays() {
    let mut simulation = a_keeper();
    let keeper = simulation.population.agents[0].id;
    let kid = a_beast(&mut simulation, "goat", (31, 30), false);
    {
        let animal = simulation.world.animals.get_mut(&kid).unwrap();
        animal.owner_id = Some(keeper);
        animal.tame_level = Simulation::HOW_TAME_A_YOUNG_ONE_TAKEN_IS;
    }
    let here = (30, 30, 0);
    let a_day = crate::environment::seasons::TICKS_PER_DAY;
    let half_hours = crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
    let mut days = 0;
    while !simulation.world.animals.get(&kid).unwrap().is_domesticated {
        days += 1;
        assert!(days <= 30, "a month of tending");
        simulation.current_turn += a_day;
        for _ in 0..half_hours {
            simulation.the_kept_stay_by_their_keepers();
        }
        let agent = simulation.population.agents[0].clone();
        let chore = simulation.the_beasts_to_see_to(&agent, here);
        assert!(matches!(chore, Some(Action::Tame { .. })), "day {days}: {chore:?}");
        let tended = simulation.execute_action(&chore.unwrap(), 0);
        assert!(tended.success, "{:?}", tended.message);
    }
}

/// A young one taken in waits two days for its keeper before it begins to
/// drift back to the wild.
#[test]
fn a_young_one_waits_two_days_for_its_keeper() {
    let mut simulation = a_keeper();
    let keeper = simulation.population.agents[0].id;
    let species = simulation.world.animals.get_species("sheep").unwrap().clone();
    let lamb = a_beast(&mut simulation, "sheep", (35, 30), false);
    assert!(simulation.a_young_one_left_behind(0, &species, (35, 30)));
    let start = simulation.world.animals.get(&lamb).unwrap().tame_level;
    let half_hours = crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
    for _ in 0..(2 * half_hours) {
        simulation.current_turn += crate::environment::seasons::TICKS_BETWEEN_PLANS;
        simulation.the_kept_stay_by_their_keepers();
    }
    let lamb_now = simulation.world.animals.get(&lamb).unwrap();
    assert_eq!(lamb_now.owner_id, Some(keeper));
    assert_eq!(lamb_now.tame_level, start, "two days and it has not begun to drift");
    for _ in 0..(5 * half_hours) {
        simulation.current_turn += crate::environment::seasons::TICKS_BETWEEN_PLANS;
        simulation.the_kept_stay_by_their_keepers();
    }
    assert_eq!(simulation.world.animals.get(&lamb).unwrap().owner_id, None, "a week untended and it is gone");
}
