// src/analytics/tests/beast_tests.rs
//! Tests for what the beasts make of us.
//!
//! An animal has two drives worth the name — eat, and do not be eaten — and
//! until now it had no opinion about people at all. `AnimalState::Fleeing`
//! and `AnimalState::Attacking` have been in the model since the model had
//! animals and nothing had ever set either of them, so a deer stood placidly
//! in a field while somebody walked up to it with a spear.
//!
//! Temper decides how kindly the odds get read, and a Passive thing never
//! stands its ground however the arithmetic comes out: a rabbit that fights a
//! wolf is not a rabbit.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::fauna::{AnimalBehavior, AnimalState};
use crate::world::{World, WorldConfig};

fn an_empty_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

/// One person at (30, 30) and whatever the test puts near them.
fn one_person(world: World) -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.health = 100.0;
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

fn arm_them(simulation: &mut Simulation) {
    let mut spear = InventoryItem::new_with_weight("spear".to_string(), 1, 1.0);
    spear.current_durability = Some(25.0);
    spear.max_durability = Some(25.0);
    let _ = simulation.population.agents[0].inventory.add_item(spear);
}

fn how_it_feels(simulation: &mut Simulation) -> AnimalState {
    simulation.what_the_beasts_make_of_us();
    simulation.world.animals.get_all()[0].state.clone()
}

// --------------------------------------------------------------------------
// Running
// --------------------------------------------------------------------------

/// A deer with a man beside it goes.
#[test]
fn a_deer_runs_from_a_man() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (32, 30))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    assert!(
        matches!(how_it_feels(&mut simulation), AnimalState::Fleeing { .. }),
        "it should be away"
    );
}

/// And it runs away from him rather than past him.
#[test]
fn what_runs_puts_ground_between_itself_and_the_thing() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (32, 30))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    simulation.what_the_beasts_make_of_us();
    let before = simulation.world.animals.get_all()[0].position;
    simulation.the_beasts_act_on_it();
    let after = simulation.world.animals.get_all()[0].position;

    assert!(
        after.0 > before.0,
        "the man is to the west, so the deer goes east: {before:?} to {after:?}"
    );
}

/// A rabbit never stands its ground, whatever the odds say. That is what
/// Passive means, and a rabbit that fights a wolf is not a rabbit.
#[test]
fn a_rabbit_never_turns_and_faces_anything() {
    assert_eq!(
        AnimalBehavior::Passive.how_readily_it_stands_its_ground(),
        0.0,
        "there is no arithmetic that makes a rabbit brave"
    );

    let mut world = an_empty_country();
    world
        .spawn_animal("rabbit".to_string(), (31, 30))
        .expect("a rabbit should spawn");
    let mut simulation = one_person(world);

    assert!(
        matches!(how_it_feels(&mut simulation), AnimalState::Fleeing { .. }),
        "it runs"
    );
}

// --------------------------------------------------------------------------
// Standing its ground
// --------------------------------------------------------------------------

/// A bear does not run from one man.
#[test]
fn a_bear_stands_its_ground() {
    let mut world = an_empty_country();
    world
        .spawn_animal("bear".to_string(), (31, 30))
        .expect("a bear should spawn");
    let mut simulation = one_person(world);

    assert!(
        matches!(how_it_feels(&mut simulation), AnimalState::Attacking { .. }),
        "a bear is not afraid of a man"
    );
}

/// And what it turns on is the man it saw.
#[test]
fn what_stands_its_ground_names_what_it_is_facing() {
    let mut world = an_empty_country();
    world
        .spawn_animal("bear".to_string(), (31, 30))
        .expect("a bear should spawn");
    let mut simulation = one_person(world);
    let who = simulation.population.agents[0].id;

    let AnimalState::Attacking { target_id } = how_it_feels(&mut simulation) else {
        panic!("a bear stands its ground");
    };

    assert_eq!(target_id, who, "it is facing the man who walked up to it");
}

/// A thing in the hand changes the arithmetic. The same wolf that would take
/// on an unarmed man goes when he has a spear.
#[test]
fn a_spear_changes_what_a_wolf_thinks_of_you() {
    let with_a_spear = |armed: bool| {
        let mut world = an_empty_country();
        world
            .spawn_animal("wolf".to_string(), (31, 30))
            .expect("a wolf should spawn");
        let mut simulation = one_person(world);
        if armed {
            arm_them(&mut simulation);
        }
        how_it_feels(&mut simulation)
    };

    assert!(
        matches!(with_a_spear(false), AnimalState::Attacking { .. }),
        "a wolf will take on a man with nothing in his hands"
    );
    assert!(
        matches!(with_a_spear(true), AnimalState::Fleeing { .. }),
        "and thinks better of it when he has a spear"
    );
}

/// A wounded thing is worth less in a fight, and knows it.
#[test]
fn a_wounded_beast_reads_the_odds_differently() {
    let sound = Simulation::what_a_beast_is_worth_in_a_fight(100.0, 100.0, 20.0);
    let hurt = Simulation::what_a_beast_is_worth_in_a_fight(10.0, 100.0, 20.0);

    assert!(
        hurt < sound,
        "a bear with one leg is not a bear: {hurt} against {sound}"
    );
}

// --------------------------------------------------------------------------
// Each other
// --------------------------------------------------------------------------

/// A deer runs from a wolf, and not only from us.
#[test]
fn a_deer_runs_from_a_wolf_too() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (10, 10))
        .expect("a deer should spawn");
    world
        .spawn_animal("wolf".to_string(), (12, 10))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    // The man is right across the map and is nobody's business
    simulation.population.agents[0].state.position = (60, 60, 0);

    simulation.what_the_beasts_make_of_us();

    let deer = simulation
        .world
        .animals
        .get_all()
        .iter()
        .find(|animal| animal.species_id == "deer")
        .expect("the deer is there");

    assert!(
        matches!(deer.state, AnimalState::Fleeing { .. }),
        "there is a wolf two paces off: {:?}",
        deer.state
    );
}

/// And nothing minds a thing right across the country.
#[test]
fn nothing_minds_a_man_across_the_map() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (5, 5))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    let before = simulation.world.animals.get_all()[0].state.clone();
    simulation.what_the_beasts_make_of_us();
    let after = simulation.world.animals.get_all()[0].state.clone();

    assert_eq!(
        before, after,
        "twenty-five paces is somebody else's problem"
    );
}

/// Running costs something. A deer that has been bolting all day is a tired
/// deer, which is what makes a hunt possible at all.
#[test]
fn bolting_costs_a_beast_its_wind() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (32, 30))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    let before = simulation.world.animals.get_all()[0].stamina;
    simulation.what_the_beasts_make_of_us();
    simulation.the_beasts_act_on_it();
    let after = simulation.world.animals.get_all()[0].stamina;

    assert!(after < before, "{after} against {before}");
}

// --- what a beast makes of what is in front of it -------------------------

/// A deer with a wolf on it runs, and does not go on grazing.
///
/// `update_animal_behavior_with_hunger` was dice keyed on `AnimalBehavior` and
/// nothing else - it could not see what was standing next to the animal at
/// all, so a deer with a wolf over it did exactly what a deer alone in a
/// meadow did. Animals read the same appraisal agents do now: see
/// `AnimalManager::what_each_animal_is_facing`.
#[test]
fn a_deer_with_a_wolf_on_it_runs() {
    use crate::environment::{AnimalState, GrazingWeather, Season};

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    world.spawn_animal("deer".to_string(), (30, 30)).expect("a deer");
    world.spawn_animal("wolf".to_string(), (33, 30)).expect("a wolf");

    let weather = GrazingWeather { precipitation: 1.0, now: 0, season: Season::Summer };
    world.animals.tick_in_world(&mut world.grid, &mut world.plants, 1.0, weather);

    let deer = world
        .animals
        .get_all()
        .iter()
        .find(|a| a.species_id == "deer")
        .expect("the deer should still be there");

    assert!(
        deer.what_is_on_me > 0.0,
        "the deer should read the wolf as something on it"
    );
    assert!(
        !deer.could_face_it,
        "and should not reckon it can take a wolf"
    );
    assert!(
        matches!(deer.state, AnimalState::Fleeing { .. }),
        "so it should be running, not {:?}",
        deer.state
    );
}

/// And a flock standing together turns round on what one of them runs from.
///
/// The other half of the split: the appraisal is one reading, and being able
/// to face a thing is what makes it anger rather than fear. Every one of its
/// own kind standing near it counts towards what it brings.
///
/// Goats, for two reasons. A wolf never reads a cow as food in the first
/// place - its prey tops out at `AnimalSize::Medium` and a cow is Large,
/// which is this model's way of saying a lone wolf does not take cattle. And
/// a sheep is `AnimalBehavior::Passive`, which is nought nerve: a passive
/// thing never turns round however many of it there are, which is the point
/// of `a_rabbit_never_stands_its_ground`. A goat is Defensive and on the
/// menu, so what decides it here is the flock.
#[test]
fn a_flock_standing_together_turns_on_what_one_of_them_runs_from() {
    use crate::environment::{AnimalState, GrazingWeather, Season};

    let how_they_take_it = |how_many: u32| {
        let mut world = World::new(WorldConfig::default());
        world.animals.get_all_mut().clear();

        for i in 0..how_many {
            world
                .spawn_animal("goat".to_string(), (30 + i as i32 % 2, 30))
                .expect("a goat");
        }
        world.spawn_animal("wolf".to_string(), (33, 30)).expect("a wolf");

        let weather = GrazingWeather { precipitation: 1.0, now: 0, season: Season::Summer };
        world.animals.tick_in_world(&mut world.grid, &mut world.plants, 1.0, weather);

        let goat = world
            .animals
            .get_all()
            .iter()
            .find(|a| a.species_id == "goat")
            .expect("a goat should still be there")
            .clone();
        (goat.could_face_it, goat.state)
    };

    let (alone_could, alone_does) = how_they_take_it(1);
    let (together_could, together_does) = how_they_take_it(8);

    assert!(
        !alone_could,
        "one goat should not reckon it can see off a wolf"
    );
    assert!(
        matches!(alone_does, AnimalState::Fleeing { .. }),
        "so it should run, not {alone_does:?}"
    );

    assert!(
        together_could,
        "eight of them standing together should"
    );
    assert!(
        matches!(together_does, AnimalState::Attacking { .. }),
        "so the flock should turn on it, not {together_does:?}"
    );
}

/// A rabbit never turns round, whatever the arithmetic says.
///
/// Temperament is the per-species baseline, and at the bottom of it
/// `AnimalBehavior::Passive` is nought: a passive thing brings nothing to a
/// stand-off, so `could_face_it` cannot come out true for one however small
/// the thing in front of it is. A rabbit that fights a wolf is not a rabbit.
#[test]
fn a_rabbit_never_stands_its_ground() {
    use crate::environment::{FaunaRegistry, GrazingWeather, Season};

    let registry = FaunaRegistry::new();
    assert_eq!(
        registry.get("rabbit").expect("a rabbit").behavior,
        AnimalBehavior::Passive,
        "this test is about what Passive means"
    );

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    // A whole warren of them, so the herd bonus is as generous as it gets.
    for i in 0..8 {
        world
            .spawn_animal("rabbit".to_string(), (30 + i % 2, 30))
            .expect("a rabbit");
    }
    world.spawn_animal("stoat".to_string(), (32, 30)).expect("a stoat");

    let weather = GrazingWeather { precipitation: 1.0, now: 0, season: Season::Summer };
    world.animals.tick_in_world(&mut world.grid, &mut world.plants, 1.0, weather);

    for rabbit in world.animals.get_all().iter().filter(|a| a.species_id == "rabbit") {
        assert!(
            !rabbit.could_face_it,
            "a rabbit reckoned it could take a stoat"
        );
        assert!(
            !matches!(rabbit.state, AnimalState::Attacking { .. }),
            "and went for it: {:?}",
            rabbit.state
        );
    }
}

// --- a hunting ground is a living, not a queue -----------------------------

/// Two wolves on ground thick with deer are neighbours; two on ground that
/// has been eaten out are rivals.
///
/// "Predators should have ranges and hunting grounds which they defend
/// against other predators. As prey species decrease in number, this should
/// cause predators to attack each other for food." The second sentence is the
/// one the model kept getting wrong: crowding was a flat count of hunters -
/// three to a ground, whatever was on it - so the pressure came from how many
/// hunters happened to be standing about and never from the game running out.
/// Winter could not cause it, a good year could not relieve it, and a hard
/// year and an easy one looked identical.
#[test]
fn what_makes_a_hunting_ground_crowded_is_the_game_on_it() {
    use crate::environment::fauna::AnimalManager;

    let per_hunter = AnimalManager::WHAT_A_HUNTER_WANTS_UNDER_IT as usize;

    // Thick with game, and a good many hunters
    assert!(
        !AnimalManager::is_the_ground_crowded(per_hunter * 20, 6),
        "six hunters over plenty of game are not in each other's way"
    );

    // The same six, after the game has gone
    assert!(
        AnimalManager::is_the_ground_crowded(per_hunter, 6),
        "and the same six on a picked-over ground are"
    );

    // Two on an empty moor: the fewest possible rivals, and the worst
    // possible living. A crowd rule would call this quiet.
    assert!(
        AnimalManager::is_the_ground_crowded(0, 2),
        "an eaten-out ground is crowded at two, which is the whole point"
    );
}

/// A hunter leaving bad ground goes where the living is better, not where the
/// rivals are fewest.
///
/// Those come apart exactly where it matters. The ground with no rivals on it
/// is very often the ground with nothing on it at all, and a hunter that walks
/// to one has swapped competition for famine.
#[test]
fn a_hunter_reckons_ground_by_what_it_feeds_not_by_who_is_on_it() {
    use crate::environment::fauna::AnimalManager;

    // An empty moor with nobody on it
    let a_moor = AnimalManager::how_good_a_living(0, 1);
    // Ground thick with game and four hunters already working it
    let good_ground = AnimalManager::how_good_a_living(120, 4);

    assert!(
        good_ground > a_moor,
        "crowded good ground beats an empty moor: {good_ground} against {a_moor}"
    );

    // And between two grounds carrying the same game, the emptier one wins
    assert!(
        AnimalManager::how_good_a_living(60, 2) > AnimalManager::how_good_a_living(60, 6),
        "of two equally stocked grounds, the one with fewer hunters on it"
    );
}

// --- the winter half of what a burrow is for -------------------------------

/// A rabbit in a bank pays a third of what a deer standing out in it pays.
///
/// "The burrows would offer shelter from weather, predators, and places to
/// hibernate in the winter." The predator half was already in - a hole is the
/// whole of a rabbit's answer to a wolf - and this is the winter half, which
/// is what decides who a hard year takes.
#[test]
fn what_can_lie_up_gets_through_a_winter_cheaper() {
    use crate::environment::fauna::{what_this_ground_offers, AnimalManager, FaunaRegistry};
    use crate::environment::Season;
    use crate::world::TerrainType;

    let registry = FaunaRegistry::new();
    let diggable = what_this_ground_offers(TerrainType::Plains);

    let winter_for = |id: &str| {
        let species = registry.get(id).unwrap_or_else(|| panic!("{id} exists"));
        AnimalManager::what_a_winter_costs(species, diggable, Season::Winter)
    };

    assert!(
        winter_for("rabbit") < 1.0,
        "a rabbit can dig itself in: {}",
        winter_for("rabbit")
    );
    assert!(
        winter_for("bear") < 1.0,
        "and a bear dens: {}",
        winter_for("bear")
    );
    assert_eq!(
        winter_for("deer"),
        1.0,
        "a deer stands out in it and pays for it"
    );
    assert_eq!(
        winter_for("wolf"),
        1.0,
        "and a wolf's winter is a wolf's winter"
    );
}

/// A rabbit on bare rock has no more hole than a deer does, and nobody lies
/// up in July.
///
/// Both of these are the difference between a rule about the world and a flag
/// on a species. A burrow is somewhere an animal is standing, not something
/// it carries about with it.
#[test]
fn lying_up_wants_the_right_ground_and_the_right_season() {
    use crate::environment::fauna::{what_this_ground_offers, AnimalManager, FaunaRegistry};
    use crate::environment::Season;
    use crate::world::TerrainType;

    let registry = FaunaRegistry::new();
    let rabbit = registry.get("rabbit").expect("rabbits exist");

    let bare_rock = what_this_ground_offers(TerrainType::Mountain);
    let diggable = what_this_ground_offers(TerrainType::Plains);

    assert!(
        !bare_rock.can_be_dug,
        "the fixture wants ground a rabbit cannot get into"
    );
    assert_eq!(
        AnimalManager::what_a_winter_costs(rabbit, bare_rock, Season::Winter),
        1.0,
        "no hole, no shelter"
    );
    assert_eq!(
        AnimalManager::what_a_winter_costs(rabbit, diggable, Season::Summer),
        1.0,
        "and nobody hibernates in July"
    );
    assert!(
        AnimalManager::what_a_winter_costs(rabbit, diggable, Season::Winter) < 1.0,
        "the right ground in the right season, and only then"
    );
}
