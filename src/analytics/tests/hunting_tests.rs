// src/analytics/tests/hunting_tests.rs
//! Tests for hunting: what an agent will take on, what it has to get close
//! enough to do, and what a kill is worth.
//!
//! Nothing in a run had ever hunted. `Action::Hunt` and the whole fauna model
//! worked, and the one place the action appeared passed a nil animal id that
//! the executor could not resolve, so meat, hides and wool never reached an
//! inventory and the warm half of the garment table was out of reach. These
//! cover:
//! - a kill is butchered into things the rest of the simulation understands
//! - a hunter has to be next to the animal
//! - an unarmed agent leaves the dangerous animals alone
//! - an agent hunts for skins it needs, and stops once it has them

use crate::agents::storage_integration::butchered_item_id;
use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// Every named cut and skin lands in the inventory as something the nutrition
/// database, the cooking rules and the garment table already understand.
#[test]
fn a_kill_is_butchered_into_things_agents_understand() {
    for cut in [
        "mutton",
        "beef",
        "pork",
        "deer_meat",
        "rabbit_meat",
        "bear_meat",
        "elk_meat",
        "blubber",
    ] {
        assert_eq!(butchered_item_id(cut), "meat", "{cut} should butcher to meat");
    }

    assert_eq!(butchered_item_id("fish_meat"), "fish");

    for skin in ["fur", "thick_hide", "snake_skin"] {
        assert_eq!(butchered_item_id(skin), "hides", "{skin} should be a hide");
    }

    // Things that already have a name keep it
    assert_eq!(butchered_item_id("leather"), "leather");
    assert_eq!(butchered_item_id("wool"), "wool");

    // Trophies pass through: no use yet, and inventing one would be worse
    assert_eq!(butchered_item_id("antler"), "antler");
    assert_eq!(butchered_item_id("feathers"), "feathers");
}

/// A hunter has to be within a spear's throw.
///
/// Without this an agent could kill a deer on the far side of the map without
/// leaving where it stood.
#[test]
fn an_animal_across_the_map_cannot_be_hunted() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let deer = world
        .spawn_animal("deer".to_string(), (45, 45))
        .expect("a deer should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (10, 10, 0);

    let result = simulation.execute_action(
        &Action::Hunt {
            animal_id: deer,
            weapon: None,
        },
        0,
    );

    assert!(!result.success, "a deer across the map is not in reach");
    assert!(
        simulation
            .world
            .animals
            .get(&deer)
            .map(|animal| animal.is_alive())
            .unwrap_or(false),
        "the deer should be untouched"
    );
}

/// Standing next to it, a hunter eventually brings it home.
#[test]
fn a_kill_fills_the_pack_with_meat_and_skins() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let deer = world
        .spawn_animal("deer".to_string(), (30, 30))
        .expect("a deer should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;
    simulation.population.agents[0]
        .skills
        .set_skill_level(SkillType::Hunting, 8);

    // Spears, because a hunt is a spear and an animal - the verb matrix
    // refuses one without the other, see `environment::verbs` - and because
    // half the throws that miss leave the shaft in the bracken. One spear and
    // forty throws is one throw.
    for _ in 0..8 {
        simulation.population.agents[0]
            .inventory
            .add_item(crate::agents::InventoryItem::new_with_durability(
                "spear".to_string(),
                1,
                25.0,
                crate::agents::Quality::Common,
            ));
    }

    // Even a practised hunter misses sometimes
    for _ in 0..40 {
        if simulation
            .world
            .animals
            .get(&deer)
            .map(|animal| !animal.is_alive())
            .unwrap_or(true)
        {
            break;
        }

        simulation.execute_action(
            &Action::Hunt {
                animal_id: deer,
                weapon: None,
            },
            0,
        );
    }

    let agent = &simulation.population.agents[0];
    let carried: Vec<&String> = agent.inventory.get_all_items().keys().collect();

    assert!(
        agent.inventory.has_item("meat", 1),
        "a deer should be worth some meat, carrying {carried:?}"
    );
    assert!(
        agent.inventory.has_item("leather", 1),
        "a deer should be worth some leather, carrying {carried:?}"
    );

    // And the meat is meat: it carries nutrition and can go on a fire
    let meat = agent
        .inventory
        .get_item("meat")
        .expect("the meat should be there");
    assert!(
        meat.food_data.is_some(),
        "meat off an animal should be worth eating"
    );
}

/// Hunting teaches hunting, whether or not the animal gets away.
///
/// The old code read MeleeCombat and had no floor on the odds: an untrained
/// agent has that skill at -10, and 0.5 + (-10 x 0.05) is zero, so the first
/// kill an agent ever made created the skill and left it unable to hunt again.
#[test]
fn hunting_teaches_hunting() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let rabbit = world
        .spawn_animal("rabbit".to_string(), (30, 30))
        .expect("a rabbit should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new_with_weight(
            "spear".to_string(),
            1,
            2.0,
        ));

    let before = simulation.population.agents[0]
        .skills
        .get_skill_if_exists(SkillType::Hunting)
        .map(|skill| skill.experience)
        .unwrap_or(0);

    simulation.execute_action(
        &Action::Hunt {
            animal_id: rabbit,
            weapon: None,
        },
        0,
    );

    let after = simulation.population.agents[0]
        .skills
        .get_skill_if_exists(SkillType::Hunting)
        .map(|skill| skill.experience)
        .unwrap_or(0);

    assert!(
        after > before,
        "an attempt should teach something either way: {before} -> {after}"
    );
}

/// An unarmed agent leaves the bears alone.
#[test]
fn nobody_walks_up_to_a_bear_empty_handed() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    // The bear is the only animal in this world, so what the agent does is
    // about the bear and not about a rabbit two fields over
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("bear".to_string(), (30, 31))
        .expect("a bear should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].body_temperature.ideal = 45.0;

    let position = simulation.population.agents[0].state.position;
    let bear = simulation
        .world
        .animals
        .get_all()
        .iter()
        .find(|animal| animal.species_id == "bear")
        .expect("the bear should be there")
        .clone();

    assert!(
        !simulation.worth_hunting(&simulation.population.agents[0], &bear),
        "an unarmed agent should not take on a bear"
    );

    assert!(
        simulation
            .hunting_action(&simulation.population.agents[0], position)
            .is_none(),
        "and should not set out to"
    );
}

/// A cold agent with no skins goes after an animal; one that already has
/// enough stays where it is.
#[test]
fn an_agent_hunts_for_the_skins_it_needs() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    // The deer is the only animal in this world, so what the agent does is
    // about the deer. It was not, and the test was green on an accident: an
    // unarmed man has never been able to take a deer - `could_bring_it_down`
    // asks for a hunting tool for anything a thrown stone will not kill - and
    // what the agent was actually setting out after was a hawk eleven tiles
    // off that the world happened to have stocked. Change what the world is
    // stocked with and the assertion says nothing at all.
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("deer".to_string(), (30, 33))
        .expect("a deer should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;

    // And something to take it with, for the same reason.
    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new_with_durability(
            "spear".to_string(),
            1,
            25.0,
            crate::agents::Quality::Common,
        ));

    // Wants to be warmer than the weather will ever make it
    simulation.population.agents[0].body_temperature.ideal = 45.0;

    let position = simulation.population.agents[0].state.position;

    let going = simulation.hunting_action(&simulation.population.agents[0], position);
    assert!(
        matches!(going, Some(Action::Move { .. }) | Some(Action::Hunt { .. })),
        "a cold agent with no skins should go after the deer, got {going:?}"
    );

    // Give it more hides than any garment asks for
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("hides".to_string(), 30, 2.0));

    assert!(
        simulation
            .hunting_action(&simulation.population.agents[0], position)
            .is_none(),
        "an agent with a pack full of hides has no reason to hunt"
    );
}

/// Skins off an animal become the warm clothing that plants cannot make.
#[test]
fn skins_become_the_warm_clothing() {
    use crate::agents::equipment::garment_recipe;

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("hides".to_string(), 20, 2.0));
        agent.body_temperature.ideal = 45.0;
    }

    let position = simulation.population.agents[0].state.position;
    let making = simulation
        .clothing_action(&simulation.population.agents[0], position, true)
        .expect("a cold agent with hides should make something of them");

    let garment = match &making {
        Action::MakeClothing { garment } => garment.clone(),
        other => panic!("expected to make a garment, got {other:?}"),
    };

    let recipe = garment_recipe(&garment).expect("the garment should have a recipe");
    assert_eq!(
        recipe.material_item, "hides",
        "the hides should be what it reaches for"
    );

    // A raw beginner spoils half of what they attempt, and this test is about
    // what fur is worth rather than about whose hands it is in, so give the
    // work to somebody who can be relied on to finish it
    simulation.population.agents[0]
        .skills
        .get_skill_mut(crate::agents::skills::SkillType::Leatherworking)
        .level = 9;

    simulation.execute_action(&making, 0);

    let agent = &simulation.population.agents[0];
    assert!(
        agent.body.total_cold_insulation() > 0.4,
        "fur should be warmer than anything woven from flax, got {:.2}",
        agent.body.total_cold_insulation()
    );
}


/// A man already hurt does not go looking for a bear, spear or no spear.
///
/// `worth_hunting` asked two things: can this tool kill it, and if it fights
/// back is there a weapon in hand. Nothing about the man. So **an agent at ten
/// health with a spear set off after a bear exactly as a whole man would**,
/// and so did a child, and the settlement lost people to fights they had gone
/// out of their way to start.
///
/// The reactive half of the question has always been asked - between_us::threat
/// calls `could_i_fight_at_all(species.attack_damage)` to decide whether a man
/// stands his ground when something comes at him. This is the same question on
/// the other side of it: whether he goes looking for the fight. One predicate,
/// both directions.
#[test]
fn a_hurt_man_with_a_spear_still_leaves_the_bear_alone() {
    use crate::agents::equipment::WeaponTemplate;
    use crate::agents::skills::Quality;

    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    world.animals.get_all_mut().clear();
    world
        .spawn_animal("bear".to_string(), (30, 31))
        .expect("a bear should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].body_temperature.ideal = 45.0;
    // Two things are wanted and they are asked of two different places, which
    // is its own smell and not this test's business: `could_bring_it_down`
    // looks for a hunting tool in the **pack**, and the dangerous-animal arm
    // looks for a weapon in the **equipment**. A bear needs both.
    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new_with_weight(
            "spear".to_string(),
            1,
            2.0,
        ));
    simulation.population.agents[0]
        .equipment
        .equip(WeaponTemplate::wooden_spear(Quality::Common))
        .expect("the spear should go in his hand");

    let bear = simulation
        .world
        .animals
        .get_all()
        .iter()
        .find(|animal| animal.species_id == "bear")
        .expect("the bear should be there")
        .clone();

    // Whole and armed, he will take it on: a gate that says no to everything
    // would pass the second half of this test and starve him.
    assert!(
        simulation.worth_hunting(&simulation.population.agents[0], &bear),
        "a whole man with a spear should still be willing to hunt a bear"
    );

    // Now hurt him. Not a scratch - down to less than a bear's blow, which is
    // what `could_i_fight_at_all` weighs it against.
    simulation.population.agents[0].state.health = 5.0;

    assert!(
        !simulation.worth_hunting(&simulation.population.agents[0], &bear),
        "a man with five points of health walked up to a bear because the \
         decision never asked how he was"
    );

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .hunting_action(&simulation.population.agents[0], position)
            .is_none(),
        "and should not set out to"
    );
}

/// A hunter fit to take a beast's blow is not killed by it.
///
/// A hunt is a spear thrown, and a throw that misses can leave the spear in
/// the bracken, so the beast that turns on the hunter may find nothing in their
/// hands. The hunt was weighed against the blow as the spear would turn it, and
/// a man with forty-odd health went after a mammoth, lost his spear and died of
/// the whole blow, on two worlds of sixteen (#327).
#[test]
fn a_hunter_fit_for_the_blow_survives_it() {
    use crate::agents::equipment::WeaponTemplate;
    use crate::agents::skills::Quality;

    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("mammoth".to_string(), (30, 31))
        .expect("a mammoth should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    let arm = |simulation: &mut Simulation| {
        let hunter = &mut simulation.population.agents[0];
        if !hunter.inventory.has_item("spear", 1) {
            hunter
                .inventory
                .add_item(InventoryItem::new_with_weight("spear".to_string(), 1, 2.0));
        }
        if hunter.equipment.get_weapon().is_none() {
            hunter
                .equipment
                .equip(WeaponTemplate::wooden_spear(Quality::Common))
                .expect("the spear should go in his hand");
        }
    };
    arm(&mut simulation);

    let mammoth = simulation.world.animals.get_all()[0].clone();
    let its_blow = simulation
        .world
        .animals
        .get_species(&mammoth.species_id)
        .expect("a mammoth is a species")
        .attack_damage;
    let through_the_spear = simulation.population.agents[0].what_a_blow_costs_me(its_blow);
    assert!(through_the_spear < its_blow, "the spear is meant to turn some of it");

    // Fit for the blow as the spear turns it, and not for the whole of it:
    // no hunt.
    simulation.population.agents[0].state.health = through_the_spear + 1.0;
    assert!(
        !simulation.worth_hunting(&simulation.population.agents[0], &mammoth),
        "fit only for the blow the spear turns, and the spear may be gone"
    );

    // Fit for the whole blow: a hunt, and never killed by it.
    for _ in 0..200 {
        arm(&mut simulation);
        let hunter = &mut simulation.population.agents[0];
        hunter.state.health = its_blow + 1.0;
        hunter.busy_until = 0;
        assert!(simulation.worth_hunting(&simulation.population.agents[0], &mammoth));
        simulation.execute_action(
            &Action::Hunt {
                animal_id: mammoth.id,
                weapon: Some("spear".to_string()),
            },
            0,
        );
        assert!(
            simulation.population.agents[0].state.health > 0.0,
            "killed by the blow he was judged fit to take"
        );
        if let Some(animal) = simulation.world.animals.get_mut(&mammoth.id) {
            animal.current_health = 300.0;
        }
    }
}

/// And a hunt made by somebody no longer fit for it is not made, however it
/// was chosen.
///
/// A hunt chosen at full health can come round again after a blow, from a
/// plan or a way already in hand, without being chosen again: a hunter at
/// thirty-six health went after a mammoth a second time and died of it (#327).
#[test]
fn a_hunt_after_the_blow_is_not_made() {
    use crate::agents::equipment::WeaponTemplate;
    use crate::agents::skills::Quality;

    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("mammoth".to_string(), (30, 31))
        .expect("a mammoth should spawn");

    let mut simulation = Simulation::new(world, population);
    let hunter = &mut simulation.population.agents[0];
    hunter.state.position = (30, 30, 0);
    hunter
        .inventory
        .add_item(InventoryItem::new_with_weight("spear".to_string(), 1, 2.0));
    hunter
        .equipment
        .equip(WeaponTemplate::wooden_spear(Quality::Common))
        .expect("the spear should go in his hand");
    hunter.state.health = 36.0;

    let mammoth = simulation.world.animals.get_all()[0].id;
    let result = simulation.execute_action(
        &Action::Hunt {
            animal_id: mammoth,
            weapon: Some("spear".to_string()),
        },
        0,
    );

    assert!(
        !result.success && result.message.as_deref().is_some_and(|said| said.starts_with("Not fit")),
        "a hunter at 36 health took on a mammoth: {:?}",
        result.message
    );
    assert_eq!(simulation.population.agents[0].state.health, 36.0, "and was hurt by it");
}

/// How many throws that tell it takes to bring one of these down, with this
/// in the pack, averaged over a few hunts.
fn throws_that_tell(species: &str, in_the_pack: &str) -> f32 {
    const HUNTS: u64 = 12;
    let mut told = 0u32;
    for seed in 0..HUNTS {
        crate::core::dice::seed(7_000 + seed);
        let mut world = World::new(WorldConfig::default());
        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        world.animals.get_all_mut().clear();
        world.spawn_animal(species.to_string(), (30, 31)).expect("it should spawn");
        let mut simulation = Simulation::new(world, population);
        let hunter = &mut simulation.population.agents[0];
        hunter.state.position = (30, 30, 0);
        hunter
            .inventory
            .add_item(InventoryItem::new_with_weight(in_the_pack.to_string(), 1, 2.0));
        let quarry = simulation.world.animals.get_all()[0].id;

        for _ in 0..400 {
            let before = simulation.world.animals.get(&quarry).map(|a| a.current_health);
            let hunter = &mut simulation.population.agents[0];
            hunter.state.health = 100.0;
            hunter.busy_until = 0;
            if !hunter.inventory.has_item(in_the_pack, 1) {
                hunter
                    .inventory
                    .add_item(InventoryItem::new_with_weight(in_the_pack.to_string(), 1, 2.0));
            }
            if let Some(beast) = simulation.world.animals.get_mut(&quarry) {
                beast.position = (30, 31);
            }
            simulation.execute_action(&Action::Hunt { animal_id: quarry, weapon: None }, 0);
            let after = simulation.world.animals.get(&quarry);
            if after.map(|a| a.current_health) != before {
                told += 1;
            }
            if after.is_none_or(|a| !a.is_alive()) {
                break;
            }
        }
    }
    told as f32 / HUNTS as f32
}

/// A better point brings a beast down in fewer throws, and a bigger beast
/// takes more.
///
/// "A wooden spear is enough, but should take several attacks to kill the
/// animal, depending on the size of the animal. A flint spear should reduce
/// the number of attacks." A throw that told took a third of whatever it hit,
/// so everything went down to three, whatever threw at it (#329).
#[test]
fn a_better_point_takes_fewer_throws_and_a_bigger_beast_more() {
    let stick_on_a_deer = throws_that_tell("deer", "sharpenedstick");
    let flint_on_a_deer = throws_that_tell("deer", "spear");
    let flint_on_a_cow = throws_that_tell("cow", "spear");

    assert!(
        flint_on_a_deer < stick_on_a_deer,
        "a flint spear took {flint_on_a_deer:.1} throws to a deer, a sharpened stick {stick_on_a_deer:.1}"
    );
    assert!(
        flint_on_a_cow > flint_on_a_deer,
        "a cow took {flint_on_a_cow:.1} throws and a deer {flint_on_a_deer:.1}"
    );
}

/// A hunter with a flint spear, alone in a world holding only these animals.
fn a_hunter_among(animals: &[(&str, (i32, i32))]) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    for (species, at) in animals {
        world.spawn_animal(species.to_string(), *at).expect("it should spawn");
    }
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("spear".to_string(), 1, 2.0));
    simulation
}

/// Nobody throws at a kestrel for supper: there is no meat on it.
///
/// Prey was the nearest animal there was a means of killing, and six of the
/// eleven kills in twelve settlement-years were kestrels (#330).
#[test]
fn nobody_hunts_a_kestrel_for_supper() {
    let simulation = a_hunter_among(&[("kestrel", (30, 31))]);
    let hunter = &simulation.population.agents[0];

    assert!(
        simulation.the_best_prey_to_eat(hunter, hunter.state.position, 5).is_none(),
        "a kestrel has nothing on it to eat"
    );
}

/// A deer a few paces further off is worth more than a rabbit at your feet,
/// for what is on it - and it is the deer that is gone after.
#[test]
fn the_deer_is_worth_more_than_the_rabbit_at_your_feet() {
    let simulation = a_hunter_among(&[("rabbit", (30, 31)), ("deer", (30, 34))]);
    let hunter = &simulation.population.agents[0];
    let here = hunter.state.position;

    let worth_of = |species: &str| {
        let animal = simulation
            .world
            .animals
            .get_all()
            .iter()
            .find(|animal| animal.species_id == species)
            .expect("it should be there");
        let kind = simulation.world.animals.get_species(species).unwrap();
        simulation.what_this_hunt_is_worth_to_eat(hunter, here, animal, kind)
    };
    let deer = worth_of("deer");
    let rabbit = worth_of("rabbit");
    assert!(deer > rabbit, "a deer was worth {deer:.0} a turn and a rabbit {rabbit:.0}");

    let (chosen, _, _) = simulation
        .the_best_prey_to_eat(hunter, here, 5)
        .expect("something here is worth a throw");
    let chosen = simulation.world.animals.get(&chosen).unwrap();
    assert_eq!(chosen.species_id, "deer", "the hunter went after the {}", chosen.species_id);
}

/// A man after a coat goes after something with a skin on it, not the nearer
/// bird with feathers.
#[test]
fn a_skin_hunt_goes_after_a_skin() {
    let simulation = a_hunter_among(&[("kestrel", (30, 31)), ("deer", (30, 36))]);
    let hunter = &simulation.population.agents[0];

    let (chosen, _) = simulation
        .the_best_prey_for_its_skin(hunter, hunter.state.position)
        .expect("the deer has a skin worth having");
    let chosen = simulation.world.animals.get(&chosen).unwrap();
    assert_eq!(chosen.species_id, "deer", "went after the {} for its skin", chosen.species_id);
}

