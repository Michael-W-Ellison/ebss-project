// src/analytics/tests/social_paths_tests.rs
//! The social paths' first stages (ISSUES_FOUND #406 to #409).
//!
//! - a call raised at a beast tells everybody in earshot where it is, and
//!   wakes whoever sleeps near it; the cold-hearted raise none
//! - a blow from a beast somebody else was warned off, and nobody called, is
//!   how calling out is found out
//! - somebody who divides the work leans to the work their hand stands best
//!   at against their camp's; the stubborn do not
//! - seeing better hands bring food in is how dividing it is found out
//! - somebody who keeps a tally reckons the store by what is in it, anybody
//!   else by what they last saw; the lazy keep none
//! - a pit found well short of what it was last seen to hold is how the
//!   tally is found out
//! - seed thrown by the eye goes in thin a third of the time and thick a
//!   third, and by the measure never; a thin field carries seven tenths
//! - bringing in fields one sowed thin is how the measure is found out

use crate::agents::{Agent, AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::wanting::strategy::Strategy;
use crate::analytics::Simulation;
use crate::core::memory::SpatialMemoryType;
use crate::core::traits::Trait;
use crate::environment::{catalogue, Action};
use crate::world::{Pit, Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig};

/// Plains with nothing on them, and `how_many` grown people in a row at
/// home, a pace apart, with nothing in their hands and no traits.
fn a_camp(how_many: usize) -> Simulation {
    let mut world = World::new(WorldConfig::default().with_size(80, 80));
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.pits.clear();
    for x in 0..80 {
        for y in 0..80 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain = Terrain::new(TerrainType::Plains);
            }
        }
    }
    let mut population = Population::new();
    for _ in 0..how_many {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
        agent.state.now_this_many_years_old(30);
        agent.state.position = (20 + index as i32, 20, 0);
        agent.hearth = Some((20, 20));
        agent.inventory.get_all_items_mut().clear();
        agent.inventory.max_weight = 500.0;
        agent.traits = crate::core::traits::TraitSet::new();
    }
    simulation
}

fn knows_of_danger_at(agent: &Agent, at: (i32, i32)) -> bool {
    agent.exploration_knowledge.where_it_went_badly.contains_key(&Position::new(at.0, at.1))
}

/// The four are found out, none is born knowing, and the first stages of
/// the trade and communication paths are what people already do.
#[test]
fn the_catalogue_knows_the_social_paths() {
    for (link, id) in [
        ("know:calls", "communication.1.standardized_calls_for_danger_assembly_hunt"),
        ("know:division_of_labour", "governance_and_labor_organization.1.division_of_labor_in_food_gathering"),
        ("know:tally", "writing_counting_and_recordkeeping.1.tally_sticks"),
        ("know:measured_sowing", "measurement_and_standardization.1.known_seed_amounts_for_planting"),
    ] {
        assert!(!catalogue::is_born_knowledge(link), "{link} is found out");
        let tech = catalogue::the_catalogue().tech.iter().find(|t| t.id == id).expect(id);
        assert!(tech.model.iter().any(|m| m == link), "{id} is {link}");
    }
    for born in ["do:ShareInformation", "do:AskAbout", "do:GoWithout", "do:GiveTo", "do:Trade"] {
        assert!(catalogue::is_born_knowledge(born), "{born} is stage 0");
    }
}

/// Somebody who knows calls out a beast that warned them off: everybody
/// within a call knows where it is, nobody beyond it does, the deaf do not,
/// and whoever slept near it is woken.
#[test]
fn a_call_tells_everybody_in_earshot_where_the_beast_is() {
    let mut simulation = a_camp(4);
    let beast = (24, 22);
    simulation.population.agents[0].found_out_how_to("know:calls");
    simulation.population.agents[2].traits.add_trait(Trait::Deaf);
    simulation.population.agents[3].state.position = (20 + Simulation::A_CALL_CARRIES + 2, 20, 0);
    simulation.population.agents[1].busy_until = 9999;

    simulation.a_beast_was_seen(0, "wolf", "wolf", beast);

    let agents = &simulation.population.agents;
    assert!(knows_of_danger_at(&agents[1], beast), "in earshot");
    assert_eq!(agents[1].busy_until, 0, "near the beast, and woken");
    assert!(!knows_of_danger_at(&agents[2], beast), "deaf");
    assert!(!knows_of_danger_at(&agents[3], beast), "beyond a call");
    assert_eq!(simulation.world.calls_raised, 1);
    assert_eq!(simulation.world.heard_a_call, 1);
}

/// Knowing it, the cold-hearted get themselves clear and call nothing.
#[test]
fn the_cold_hearted_call_nothing() {
    let mut simulation = a_camp(2);
    simulation.population.agents[0].found_out_how_to("know:calls");
    simulation.population.agents[0].traits.add_trait(Trait::ColdHearted);
    simulation.a_beast_was_seen(0, "wolf", "wolf", (24, 22));
    assert!(!knows_of_danger_at(&simulation.population.agents[1], (24, 22)));
    assert_eq!(simulation.world.calls_raised, 0);
}

/// Struck, twice, by a kind somebody else was warned off near there that
/// day, and nobody called it: and the one struck is apt to call out. A blow
/// from a beast somebody did call teaches nothing.
#[test]
fn blows_nobody_called_are_how_calling_is_found_out() {
    let mut simulation = a_camp(2);
    simulation.a_beast_was_seen(1, "wolf", "wolf", (24, 22));
    for _ in 0..Simulation::BLOWS_BEFORE_A_CALL {
        assert!(!simulation.population.agents[0].knows_the_way("calls"));
        simulation.a_blow_from_a_beast_seen_before(0, "wolf");
    }
    assert!(simulation.population.agents[0].knows_the_way("calls"));
    assert_eq!(simulation.world.blows_a_call_would_have_spared, 2);

    let mut simulation = a_camp(2);
    simulation.population.agents[1].found_out_how_to("know:calls");
    simulation.a_beast_was_seen(1, "wolf", "wolf", (24, 22));
    for _ in 0..Simulation::BLOWS_BEFORE_A_CALL {
        simulation.a_blow_from_a_beast_seen_before(0, "wolf");
    }
    assert!(!simulation.population.agents[0].knows_the_way("calls"), "it was called");
}

/// Three in a camp. The first is a fair fisher and a poor forager among
/// worse fishers and better foragers: dividing the work, they lean to the
/// fishing and away from the foraging. Not knowing to, or stubborn, or
/// alone, it weighs nothing.
#[test]
fn the_work_goes_to_the_hand_that_stands_best_at_it() {
    let mut simulation = a_camp(3);
    for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
        let (fish, forage) = if index == 0 { (2, -4) } else { (-6, 6) };
        agent.skills.set_skill_level(SkillType::Fishing, fish);
        agent.skills.set_skill_level(SkillType::Herbalism, forage);
        agent.skills.set_skill_level(SkillType::Hunting, 0);
    }
    let worth = |simulation: &Simulation, way| {
        simulation.what_my_hand_is_worth_to_the_camp(&simulation.population.agents[0], way, 0.0)
    };
    assert_eq!(worth(&simulation, Strategy::FishLocalWaters), 1.0, "nobody told them");

    simulation.population.agents[0].found_out_how_to("know:division_of_labour");
    let fishing = worth(&simulation, Strategy::FishLocalWaters);
    let foraging = worth(&simulation, Strategy::GatherWildFood);
    assert!(fishing > 1.0, "{fishing}");
    assert!(foraging < 1.0, "{foraging}");
    assert!(fishing <= 1.0 + Simulation::THE_MOST_A_ROLE_TIPS_IT);
    assert_eq!(worth(&simulation, Strategy::EatCarriedFood), 1.0, "no hand decides it");

    // Weighing less as the need presses, and nothing to somebody starving
    let agent = &simulation.population.agents[0];
    let half = simulation.what_my_hand_is_worth_to_the_camp(agent, Strategy::GatherWildFood, 0.5);
    assert!(foraging < half && half < 1.0, "{foraging} {half}");
    assert_eq!(
        simulation.what_my_hand_is_worth_to_the_camp(agent, Strategy::GatherWildFood, 1.0),
        1.0,
        "a starving one eats what is in reach"
    );

    simulation.population.agents[0].traits.add_trait(Trait::Stubborn);
    assert_eq!(worth(&simulation, Strategy::FishLocalWaters), 1.0, "the stubborn do as they would");

    let mut alone = a_camp(1);
    alone.population.agents[0].found_out_how_to("know:division_of_labour");
    alone.population.agents[0].skills.set_skill_level(SkillType::Fishing, 8);
    assert_eq!(
        alone.what_my_hand_is_worth_to_the_camp(&alone.population.agents[0], Strategy::FishLocalWaters, 0.0),
        1.0,
        "nobody to divide it with"
    );
}

/// Gathering is bringing in a field as well as stripping wild plants: the
/// camp's best farmer, a poor forager and a better hunter, does not mark
/// their harvest down for the foraging hand (#407, after #410).
#[test]
fn a_good_farmer_is_not_put_off_their_harvest() {
    let mut simulation = a_camp(3);
    for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
        let (farm, forage, hunt) = if index == 0 { (8, -4, 4) } else { (-2, 2, -2) };
        agent.skills.set_skill_level(SkillType::Farming, farm);
        agent.skills.set_skill_level(SkillType::Herbalism, forage);
        agent.skills.set_skill_level(SkillType::Hunting, hunt);
        agent.skills.set_skill_level(SkillType::Fishing, 0);
    }
    simulation.population.agents[0].found_out_how_to("know:division_of_labour");
    let gathering = simulation.what_my_hand_is_worth_to_the_camp(
        &simulation.population.agents[0],
        Strategy::GatherWildFood,
        0.0,
    );
    assert!(gathering >= 1.0, "the field is theirs to bring in: {gathering}");
}

/// Seeing a plainly better hand bring food in, five times, at work one has
/// been at oneself that half day, and somebody is apt to leave that work to
/// them; a hand no better teaches nothing, and nor does a catch seen by
/// somebody who has not been fishing.
#[test]
fn better_hands_seen_are_how_dividing_the_work_is_found_out() {
    let mut simulation = a_camp(4);
    simulation.current_turn = Simulation::AT_THE_SAME_WORK_WITHIN * 4;
    let now = simulation.current_turn;
    simulation.population.agents[0].skills.set_skill_level(SkillType::Fishing, 8);
    simulation.population.agents[1].skills.set_skill_level(SkillType::Fishing, -4);
    simulation.population.agents[1].skills.get_skill_mut(SkillType::Fishing).last_used = now;
    simulation.population.agents[2].skills.set_skill_level(SkillType::Fishing, 8);
    simulation.population.agents[2].skills.get_skill_mut(SkillType::Fishing).last_used = now;
    simulation.population.agents[3].skills.set_skill_level(SkillType::Fishing, -4);
    for _ in 0..Simulation::BETTER_HANDS_SEEN_BEFORE_A_ROLE {
        assert!(!simulation.population.agents[1].knows_the_way("division_of_labour"));
        simulation.a_hand_seen_at_work(0, SkillType::Fishing);
    }
    assert!(simulation.population.agents[1].knows_the_way("division_of_labour"));
    assert!(!simulation.population.agents[2].knows_the_way("division_of_labour"), "as good a hand");
    assert!(!simulation.population.agents[3].knows_the_way("division_of_labour"), "not at the work");
}

/// A pit last seen with a hundred in it and holding ten: somebody who keeps a
/// tally reckons ten, anybody else a hundred, and the lazy keep no tally.
#[test]
fn a_tally_reckons_the_store_by_what_is_in_it() {
    let mut simulation = a_camp(1);
    let at = Position::new(20, 20);
    simulation.world.pits.push(Pit {
        gnawed: 0,
        last_opened: None,
        where_it_is: at,
        holds: vec![InventoryItem::new_with_weight("food".to_string(), 10, 0.5)],
        covered: true,
        dug: 0,
        belongs: crate::world::Belongs::ToNobody,
    });
    let in_it = simulation.world.how_much_is_in_the_ground_near(at, Simulation::WORTH_WALKING_TO_THE_STORE);
    simulation.population.agents[0]
        .memory
        .remember_how_much_is_there(SpatialMemoryType::Storage, (20, 20, 0), 100);

    let reckons = |simulation: &Simulation| simulation.what_i_reckon_is_put_by(&simulation.population.agents[0], at);
    assert_eq!(reckons(&simulation), 100, "by what was seen");
    simulation.population.agents[0].found_out_how_to("know:tally");
    assert_eq!(reckons(&simulation), in_it, "by the tally");
    simulation.population.agents[0].traits.add_trait(Trait::Lazy);
    assert_eq!(reckons(&simulation), 100, "the lazy keep none");
}

/// Seeing a pit hold well under half what it was last seen to, three times,
/// and somebody is apt to keep a tally.
#[test]
fn a_store_found_short_is_how_the_tally_is_found_out() {
    let mut simulation = a_camp(1);
    simulation.world.pits.push(Pit {
        gnawed: 0,
        last_opened: None,
        where_it_is: Position::new(21, 20),
        holds: vec![InventoryItem::new_with_weight("food".to_string(), 10, 0.5)],
        covered: true,
        dug: 0,
        belongs: crate::world::Belongs::ToNobody,
    });
    for _ in 0..Population::STORES_FOUND_SHORT_BEFORE_A_TALLY {
        assert!(!simulation.population.agents[0].knows_the_way("tally"));
        simulation.population.agents[0]
            .memory
            .remember_how_much_is_there(SpatialMemoryType::Storage, (21, 20, 0), 100);
        simulation.population.process_exploration_of(&mut simulation.world, None);
    }
    assert!(simulation.population.agents[0].knows_the_way("tally"));
    assert_eq!(
        simulation.population.stats.stores_found_short,
        u64::from(Population::STORES_FOUND_SHORT_BEFORE_A_TALLY)
    );
}

/// By the eye, a third of fields go in thin and a third thick, a second
/// handful spent; by the measure, every one right.
#[test]
fn seed_thrown_by_the_eye_goes_in_thin_or_thick() {
    crate::core::dice::seed(13);
    let mut simulation = a_camp(1);
    let trials = 3000;
    let mut thin = 0;
    for _ in 0..trials {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.get_all_items_mut().clear();
        agent.inventory.add_item(InventoryItem::new("grain".to_string(), 1));
        if simulation.sown_at_a_guess(0, "grain") {
            thin += 1;
        }
    }
    let share = |n: u64| n as f64 / trials as f64;
    assert!((share(thin) - Simulation::A_GUESS_SOWS_THIN).abs() < 0.03, "{thin}");
    assert!((share(simulation.world.fields_sown_thick) - Simulation::A_GUESS_SOWS_THICK).abs() < 0.03);
    assert_eq!(simulation.world.fields_sown_thin, thin);

    simulation.population.agents[0].found_out_how_to("know:measured_sowing");
    for _ in 0..100 {
        assert!(!simulation.sown_at_a_guess(0, "grain"));
    }
    assert_eq!(simulation.world.fields_sown_measured, 100);
}

/// A field sown thin carries seven tenths of one sown right.
#[test]
fn a_thin_sown_field_carries_seven_tenths() {
    let mut field = ResourceNode::new(ResourceType::Grain, Position::new(5, 5), Simulation::FIELD_YIELD);
    field.on_a_field = true;
    let right = field.how_heavy_a_crop_it_carries(1.0);
    field.sown_thin_by = Some(uuid::Uuid::from_u128(0x7417));
    let thin = field.how_heavy_a_crop_it_carries(1.0);
    let carries = thin as f32 / right as f32;
    assert!((carries - ResourceNode::A_THIN_SOWN_FIELD_CARRIES).abs() < 0.05, "{carries}");
}

/// Bringing in a field one sowed thin oneself, three times, and somebody is
/// apt to sow by the measure.
#[test]
fn bringing_in_thin_fields_is_how_the_measure_is_found_out() {
    let mut simulation = a_camp(1);
    let me = simulation.population.agents[0].id;
    for x in 0..Simulation::THIN_FIELDS_BEFORE_A_MEASURE as i32 {
        assert!(!simulation.population.agents[0].knows_the_way("measured_sowing"));
        let at = Position::new(30 + 3 * x, 30);
        assert!(simulation.world.grid.break_ground(&at, 0));
        let mut node = ResourceNode::new(ResourceType::Grain, at, Simulation::FIELD_YIELD);
        node.on_a_field = true;
        node.sown_thin_by = Some(me);
        let yields = simulation.world.grid.what_it_yields_here_for(&at, ResourceType::Grain);
        node.amount = node.how_heavy_a_crop_it_carries(yields);
        node.it_has_ripened();
        simulation.world.put_a_node_down(node);
        simulation.population.agents[0].state.position = (at.x, at.y, 0);
        let result = simulation.execute_action(&Action::Gather { resource_type: "grain".to_string() }, 0);
        assert!(result.success, "{:?}", result.message);
    }
    assert!(simulation.population.agents[0].knows_the_way("measured_sowing"));
}
