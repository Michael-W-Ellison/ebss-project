// src/analytics/tests/sickness_tests.rs
//! Tests for the first illness in this model.
//!
//! "Eating raw meat, spending time near dead bodies or fresh waste, and eating
//! spoiling food should have a chance to cause sickness."
//!
//! Before this there was no sickness at all. The only health consequence
//! anywhere in the project was a flat ten damage for eating something already
//! past `is_harmful`, taken in one tick and done with — so a settlement could
//! live on raw flesh and sleep in its own midden and never know the
//! difference, and a fire was worth 2.7 times the nutrition and nothing else.
//!
//! An ailment lasts. What makes sickness cost a settlement is not the damage,
//! it is the days: somebody laid up is somebody not gathering, not building,
//! and eating anyway.

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, Soil, World, WorldConfig};

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

// --------------------------------------------------------------------------
// Being ill at all
// --------------------------------------------------------------------------

/// Nobody starts out ill.
#[test]
fn a_healthy_person_is_not_ailing() {
    let simulation = one_person();
    assert!(!simulation.population.agents[0].is_ailing());
    assert!(simulation.population.agents[0].what_ails_me().is_none());
}

/// An ailment lasts days rather than a tick, which is the whole point of it.
#[test]
fn being_ill_lasts_days() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.taken_ill_with(Agent::OFF_RAW_FLESH, 0.5, 100);

    let ailing = agent.what_ails_me().expect("should be ill");
    assert_eq!(ailing.from, Agent::OFF_RAW_FLESH);
    assert!(
        ailing.until - ailing.since >= crate::agents::Ailment::THE_SHORTEST_IT_LASTS,
        "an illness that is over inside a day is not an illness, it is a bad hour"
    );
}

/// It costs health and it costs the days' work, and then it is over.
#[test]
fn being_ill_costs_and_then_passes() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent.state.health = 100.0;
        agent.taken_ill_with(Agent::OFF_RAW_FLESH, 1.0, simulation.current_tick);
    }

    let started_at = simulation.population.agents[0].state.health;
    let runs_until = simulation.population.agents[0]
        .what_ails_me()
        .expect("ill")
        .until;

    let mut worst = started_at;
    while simulation.current_tick < runs_until + 2 {
        simulation.tick();
        let agent = &simulation.population.agents[0];
        if !agent.state.is_alive {
            break;
        }
        worst = worst.min(agent.state.health);
    }

    assert!(
        worst < started_at,
        "being ill should take something off: {started_at} -> {worst}"
    );
    assert!(
        !simulation.population.agents[0].is_ailing(),
        "and it should run its course"
    );
}

/// Nothing stacks. Somebody already ill stays ill rather than catching a
/// second thing on top of the first.
#[test]
fn one_thing_at_a_time() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.taken_ill_with(Agent::OFF_RAW_FLESH, 0.5, 100);
    let first = agent.what_ails_me().expect("ill").until;

    agent.taken_ill_with(Agent::OFF_FOUL_GROUND, 1.0, 100);

    let still = agent.what_ails_me().expect("still ill");
    assert_eq!(still.from, Agent::OFF_RAW_FLESH);
    assert_eq!(still.until, first);
}

// --------------------------------------------------------------------------
// Raw flesh
// --------------------------------------------------------------------------

/// Raw flesh is a gamble. Over enough meals it tells.
#[test]
fn living_on_raw_meat_makes_people_ill() {
    let mut ill = 0;

    for _ in 0..60 {
        let mut simulation = one_person();
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(a_meal(ItemType::Meat, "meatportions", 1));
        agent.eat_food_item("meatportions", 0);

        if agent.is_ailing() {
            ill += 1;
        }
    }

    assert!(
        ill > 0,
        "sixty meals of raw flesh and nobody the worse for it"
    );
    assert!(
        ill < 40,
        "raw flesh is a gamble, not poison: {ill} in 60 is too many"
    );
}

/// And a fire settles it. This is the reason cooking exists beyond the
/// nutrition: it was worth 2.7 times the utilization and nothing else.
#[test]
fn cooked_flesh_is_not_a_gamble() {
    for _ in 0..60 {
        let mut simulation = one_person();
        let agent = &mut simulation.population.agents[0];

        let mut cooked = a_meal(ItemType::Meat, "meatportions", 1);
        if let Some(food) = cooked.food_data.as_mut() {
            food.preparation = PreparationState::Cooked;
        }
        agent.inventory.add_item(cooked);
        agent.eat_food_item("meatportions", 0);

        assert!(
            !agent.is_ailing(),
            "nobody gets ill off a joint that has been over a fire"
        );
    }
}

/// A berry is not flesh, however raw it is.
#[test]
fn nobody_gets_ill_off_a_fresh_berry() {
    for _ in 0..60 {
        let mut simulation = one_person();
        let agent = &mut simulation.population.agents[0];
        agent.inventory.add_item(a_meal(ItemType::Food, "berries", 1));
        agent.eat_food_item("berries", 0);

        assert!(!agent.is_ailing(), "a raw berry is just a berry");
    }
}

// --------------------------------------------------------------------------
// Food on the turn
// --------------------------------------------------------------------------

/// Food that has started to go is a worse gamble than fresh food.
///
/// The band this works in is narrower than it looks: raw food below 0.3
/// freshness is already `is_harmful`, which is a separate and worse matter
/// handled before any of this. So "on the turn" for raw food means 0.3 to
/// 0.5, and for anything that has been cooked or dried it means the whole way
/// down to nothing.
#[test]
fn food_on_the_turn_is_worse_than_fresh_food() {
    let count_ill = |freshness: f32| {
        let mut ill = 0;
        for _ in 0..120 {
            let mut simulation = one_person();
            let agent = &mut simulation.population.agents[0];

            let mut going = a_meal(ItemType::Food, "berries", 1);
            if let Some(food) = going.food_data.as_mut() {
                food.freshness = freshness;
            }
            agent.inventory.add_item(going);
            agent.eat_food_item("berries", 0);

            if agent.is_ailing() {
                ill += 1;
            }
        }
        ill
    };

    let fresh = count_ill(1.0);
    let going = count_ill(0.35);

    assert_eq!(fresh, 0, "fresh food is not a gamble at all");
    assert!(
        going > 0,
        "a hundred and twenty helpings of food on the turn and nobody the worse for it"
    );
}

// --------------------------------------------------------------------------
// The ground underfoot
// --------------------------------------------------------------------------

/// A body fouls the ground it falls on, which is what makes it a thing to be
/// away from rather than a nutrient deposit.
#[test]
fn a_body_fouls_the_ground_it_falls_on() {
    let mut simulation = one_person();
    let here = Position::new(30, 30);

    let clean = simulation
        .world
        .grid
        .get_tile(&here)
        .map(|tile| tile.soil.fouling)
        .unwrap_or(0.0);

    simulation
        .population
        .bodies_where_they_fell
        .push(((30, 30, 0), 4.0, 1.0));
    simulation.tick();

    let after = simulation
        .world
        .grid
        .get_tile(&here)
        .map(|tile| tile.soil.fouling)
        .unwrap_or(0.0);

    assert!(
        after > clean,
        "somebody died here and the ground says nothing about it: {clean} -> {after}"
    );
    assert!(
        after >= Soil::FOUL_ENOUGH_TO_WALK_AWAY_FROM,
        "and it should be foul enough that people move off it: {after}"
    );
}

/// Living on a midden tells, eventually.
#[test]
fn living_on_a_midden_makes_people_ill() {
    let mut ill = 0;

    for _ in 0..12 {
        let mut simulation = one_person();
        let here = Position::new(25, 25);

        // Pin them to the worst ground there is, and keep it that way: the
        // fouling breaks down, and an agent that wanders off is not living
        // on it any more.
        for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 30) {
            if let Some(tile) = simulation.world.grid.get_tile_mut(&here) {
                tile.soil.fouling = Soil::AS_FOUL_AS_IT_GETS;
            }
            simulation.population.agents[0].state.position = (25, 25, 0);
            simulation.tick();

            // A dead man is taken out of the population, so this has to ask
            // whether he is still there before asking how he is. It used to
            // index straight in, and only stayed up because a lone agent
            // happened to survive its month.
            let Some(agent) = simulation.population.agents.first() else {
                break;
            };
            if agent.is_ailing() {
                ill += 1;
                break;
            }
            if !agent.state.is_alive {
                break;
            }
        }
    }

    assert!(
        ill > 0,
        "twelve people living a month apiece on a midden and not one of them the worse for it"
    );
}

/// Clean ground does not.
#[test]
fn clean_ground_does_not_make_anybody_ill() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 20) {
        if let Some(tile) = simulation.world.grid.get_tile_mut(&here) {
            tile.soil.fouling = 0.0;
        }
        simulation.population.agents[0].state.position = (25, 25, 0);
        simulation.tick();

        if let Some(ailing) = simulation.population.agents[0].what_ails_me() {
            assert_ne!(
                ailing.from,
                Agent::OFF_FOUL_GROUND,
                "there is nothing wrong with this ground"
            );
        }
    }
}

// --------------------------------------------------------------------------
// What somebody learns from it
// --------------------------------------------------------------------------

/// Being ill off a thing goes in the book, so it can be learned from. This is
/// the ordinary lessons machinery — nothing here knows what "raw" means, only
/// that this agent has a bad history with something by that name.
#[test]
fn being_ill_off_a_thing_is_recorded_against_it() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    assert_eq!(agent.how_often_this_has_laid_me_up(Agent::OFF_RAW_FLESH), 0);

    agent.taken_ill_with(Agent::OFF_RAW_FLESH, 0.5, 0);

    assert_eq!(
        agent.how_often_this_has_laid_me_up(Agent::OFF_RAW_FLESH),
        1,
        "the illness should be counted against what caused it"
    );
}

/// And somebody who has learned it leaves raw flesh alone, while there is
/// anything else going.
#[test]
fn somebody_who_has_learned_it_leaves_raw_flesh_alone() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 4));

    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("meatportions"),
        "somebody who has never been ill eats what is in front of them"
    );

    // Now give them the history: two weeks in bed off the same thing.
    for _ in 0..Agent::TWICE_IS_A_PATTERN {
        agent.taken_ill_with(Agent::OFF_RAW_FLESH, 0.5, 0);
        agent.state.ailing = None;
    }

    assert!(
        agent.has_this_made_me_ill(Agent::OFF_RAW_FLESH),
        "two weeks in bed off the same thing is a pattern by anybody's reckoning"
    );
    assert!(
        agent.find_best_food_to_eat().is_none(),
        "and having learned it, they leave it alone"
    );
}

/// Unless they are starving, in which case a strong enough survival drive
/// overrides the risk — which is the rule the specification already set for
/// theft and applies just as well here.
#[test]
fn a_starving_person_eats_it_anyway() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 4));
    for _ in 0..Agent::TWICE_IS_A_PATTERN {
        agent.taken_ill_with(Agent::OFF_RAW_FLESH, 0.5, 0);
        agent.state.ailing = None;
    }

    assert!(agent.find_best_food_to_eat().is_none());

    agent.state.gone_without_food_for(10_000);
    assert!(agent.state.is_starving(), "and now they are desperate");

    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("meatportions"),
        "a man three days without food eats what is in front of him"
    );
}

// --- and something to do about it -----------------------------------------

/// A remedy eases an illness. It does not cure one, and no amount of it will.
///
/// This is the whole of the treatment in this model and it is deliberately
/// not very much. The specification is unusually careful about its ten
/// medicinal plants - aloe is "not a replacement for burn or wound care",
/// echinacea's "clinical benefits remain uncertain", garlic is not "an
/// antibiotic substitute" - and that care is the model: a settlement can have
/// the whole hedgerow and still bury people.
#[test]
fn a_remedy_eases_and_does_not_cure() {
    use crate::environment::remedies;

    let mut agent = Agent::new(AgentConfig::default());
    agent.taken_ill_with(Agent::OFF_RAW_FLESH, 0.8, 100);

    let at_its_worst = agent.what_ails_me().expect("ill").severity;
    let until = agent.what_ails_me().expect("ill").until;

    // Mint is for the gut, and a bad gut is what everybody in this model has.
    let eased = agent.take_a_remedy("mint_leaves", 100).expect("a remedy");
    assert!(eased > 0.0, "mint should be worth something for a bad gut");

    let after = agent.what_ails_me().expect("still ill");
    assert!(
        after.severity < at_its_worst,
        "it should be easier to bear: {} against {at_its_worst}",
        after.severity
    );
    assert_eq!(
        after.until, until,
        "and not one tick shorter: a remedy is not a cure"
    );

    // Take it over and over and it stops helping. The cap is against the
    // illness at its worst, not against what it has already been eased to,
    // which is what stops a sixth dose curing.
    for _ in 0..20 {
        agent.take_a_remedy("mint_leaves", 100);
    }
    let dosed = agent.what_ails_me().expect("still ill");
    assert!(
        dosed.severity >= at_its_worst * (1.0 - remedies::THE_MOST_A_HERBAL_CAN_DO) - 0.001,
        "no amount of herbs takes off more than a third: {} against {at_its_worst}",
        dosed.severity
    );
    assert!(agent.is_ailing(), "and they are still ill at the end of it");
}

/// The right remedy for the trouble is worth four times the wrong one.
///
/// Aloe is for the skin and a bad gut is not the skin. It is still worth
/// something - somebody has been looked after - and that is
/// `WHAT_THE_WRONG_REMEDY_IS_STILL_WORTH`, which is what makes knowing one
/// herb from another worth having without making it the difference between
/// living and dying.
#[test]
fn the_right_herb_for_the_trouble_is_worth_the_most() {
    use crate::environment::remedies::{self, WhatARemedyEases};

    let eased_by = |remedy: &str, cause: &str| {
        let mut agent = Agent::new(AgentConfig::default());
        agent.taken_ill_with(cause, 0.8, 100);
        agent.take_a_remedy(remedy, 100).unwrap_or(0.0)
    };

    // A bad gut, treated with a gut herb and with a skin one.
    let right = eased_by("mint_leaves", Agent::OFF_RAW_FLESH);
    let wrong = eased_by("aloe_gel", Agent::OFF_RAW_FLESH);
    assert!(right > wrong, "mint beats aloe for a bad gut: {right} against {wrong}");
    assert!(wrong > 0.0, "and being looked after is worth something");

    // And the other way about: a wound that turned wants the topical.
    let on_a_wound = eased_by("aloe_gel", Agent::OFF_A_WOUND_THAT_TURNED);
    assert!(
        on_a_wound > eased_by("mint_leaves", Agent::OFF_A_WOUND_THAT_TURNED),
        "aloe beats mint for a wound"
    );

    // The unproven ones are worth almost nothing, whatever they are given
    // for, which is what the specification says about them.
    let ginseng = remedies::what_this_is_good_for("ginseng_root").expect("in the table");
    assert_eq!(ginseng.eases, WhatARemedyEases::NothingAnybodyCanShow);
    assert!(
        ginseng.takes_off
            < remedies::what_this_is_good_for("mint_leaves").unwrap().takes_off,
        "prized, rare, expensive, and never shown to do this"
    );

    // Nothing in this model is ill in the nerves, so lavender is always the
    // wrong remedy - which is the honest place for it.
    let lavender = remedies::what_this_is_good_for("lavender").expect("in the table");
    assert_eq!(lavender.eases, WhatARemedyEases::TheNerves);
}

/// Every remedy in the table is something a plant in this world actually
/// drops, or comes off a patch of herbs.
///
/// A table of medicines nobody can get hold of is the same defect as a biome
/// no map can produce.
#[test]
fn every_remedy_is_something_you_could_actually_pick() {
    use crate::environment::flora::FloraRegistry;
    use crate::environment::remedies;

    let flora = FloraRegistry::new();
    let mut droppable: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for plant in flora.all_species() {
        for drop in &plant.drops {
            droppable.insert(drop.material_id.clone());
        }
    }
    // And what a patch of herbs gathers as.
    droppable.insert("herbs".to_string());

    for remedy in remedies::EVERY_REMEDY {
        assert!(
            droppable.contains(remedy.id),
            "{} is in the remedy table and nothing in this world yields it",
            remedy.id
        );
    }
}

/// A wound that does not close can turn, and that is what killed people who
/// survived the animal.
#[test]
fn a_wound_can_turn() {
    crate::core::dice::seed(5_100);

    let mut agent = Agent::new(AgentConfig::default());
    agent.state.take_damage(30.0);
    assert!(
        agent.state.an_open_wound > 0.9,
        "a heavy blow leaves an open wound: {}",
        agent.state.an_open_wound
    );

    // Hunger and cold do not leave wounds - only blows do.
    let mut worn_down = Agent::new(AgentConfig::default());
    worn_down.state.lose_health(30.0, "hunger");
    assert_eq!(
        worn_down.state.an_open_wound, 0.0,
        "being starved does not leave anything to fester"
    );

    // Over a fortnight of an open wound, somebody comes down with something.
    let mut came_down = 0;
    for seed in 0..24u64 {
        crate::core::dice::seed(5_200 + seed);
        let mut one = Agent::new(AgentConfig::default());
        one.state.take_damage(30.0);
        for tick in 0..(14 * crate::environment::seasons::TICKS_PER_DAY) {
            one.tick_with_time(tick);
            if one.is_ailing() {
                break;
            }
        }
        if one.what_ails_me().map(|a| a.from == Agent::OFF_A_WOUND_THAT_TURNED).unwrap_or(false) {
            came_down += 1;
        }
    }
    assert!(
        came_down > 0 && came_down < 24,
        "most people were all right and some were not: {came_down} of 24"
    );
}

/// A soaking in the cold is a thing you come down with, not only a thing that
/// wears you down.
#[test]
fn a_soaking_in_the_cold_tells() {
    let mut came_down = 0;
    for seed in 0..24u64 {
        crate::core::dice::seed(6_100 + seed);
        let mut agent = Agent::new(AgentConfig::default());
        // A hard day of it: what `update_exposure` would be handing over on a
        // January night in the open.
        for tick in 0..crate::environment::seasons::TICKS_PER_DAY {
            agent.a_soaking_may_tell(0.8, tick);
        }
        if agent.what_ails_me().map(|a| a.from == Agent::OFF_A_SOAKING).unwrap_or(false) {
            came_down += 1;
        }
    }
    assert!(
        came_down > 0,
        "somebody out in it all day should come down with something"
    );

    // And it is a chest complaint, which is not what a gut herb is for.
    let mut chilled = Agent::new(AgentConfig::default());
    chilled.taken_ill_with(Agent::OFF_A_SOAKING, 0.6, 10);
    assert_eq!(
        chilled.what_ails_me().unwrap().what_sort_it_is(),
        crate::environment::remedies::WhatARemedyEases::TheChest
    );
    let sage = chilled.take_a_remedy("sage_leaves", 10).unwrap_or(0.0);
    let mint = {
        let mut other = Agent::new(AgentConfig::default());
        other.taken_ill_with(Agent::OFF_A_SOAKING, 0.6, 10);
        other.take_a_remedy("mint_leaves", 10).unwrap_or(0.0)
    };
    assert!(sage > mint, "a gargle beats a stomach herb for a chill: {sage} against {mint}");
}

/// An ill agent with something in the pack takes it, rather than only lying
/// down.
#[test]
fn somebody_ill_reaches_for_what_they_have() {
    let mut agent = Agent::new(AgentConfig::default());
    assert!(agent.what_i_have_for_it().is_none(), "nothing wrong, nothing wanted");

    agent.taken_ill_with(Agent::OFF_RAW_FLESH, 0.6, 50);
    assert!(
        agent.what_i_have_for_it().is_none(),
        "ill with an empty pack is still nothing to take"
    );

    agent
        .inventory
        .add_item(InventoryItem::new("mint_leaves".to_string(), 3));
    assert_eq!(agent.what_i_have_for_it().as_deref(), Some("mint_leaves"));

    // A taught hand reaches past the aloe for the mint; an untaught one takes
    // whatever is called medicine.
    agent
        .inventory
        .add_item(InventoryItem::new("aloe_gel".to_string(), 3));
    agent
        .skills
        .practise(crate::agents::SkillType::Herbalism, 400, 50);
    assert_eq!(
        agent.what_i_have_for_it().as_deref(),
        Some("mint_leaves"),
        "a herbalist knows which is which"
    );
}
