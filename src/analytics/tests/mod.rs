mod afforded_tests;
mod terrain_tests;
mod camp_tests;
mod coupling_tests;
mod workings_tests;
pub mod save_load_tests;
pub mod autosave_tests;
pub mod config_tests;
pub mod agent_building_integration_tests;
pub mod survival_loop_tests;
pub mod shelter_and_exposure_tests;
pub mod thirst_tests;
pub mod sight_tests;
pub mod scent_tests;
pub mod cooking_tests;
pub mod clothing_tests;
pub mod hunting_tests;
pub mod predator_prey_tests;
pub mod longevity_tests;
pub mod husbandry_tests;
pub mod land_tests;
pub mod calendar_tests;
pub mod survival_pressure_tests;
pub mod nutrient_loop_tests;
pub mod rotation_tests;
pub mod herbal_tests;
pub mod fishery_tests;
pub mod personality_tests;
pub mod specialisation_tests;
pub mod fight_or_flight_tests;
pub mod relationship_graph_tests;
pub mod distrust_tests;
pub mod news_tests;
pub mod stacking_tests;
pub mod stone_age_tests;
pub mod making_tests;
pub mod tool_wear_tests;
pub mod discovery_tests;
pub mod pattern_tests;
pub mod whereabouts_tests;
pub mod worry_tests;
pub mod lean_season_tests;
pub mod midden_tests;
pub mod farming_tests;
pub mod nomad_tests;
pub mod sprouting_tests;
pub mod transplant_tests;
pub mod tasting_tests;
pub mod swap_tests;
pub mod verb_tests;
pub mod burrow_tests;
pub mod clay_tests;
pub mod cornered_tests;
pub mod mortality_tests;
pub mod short_handed_tests;
pub mod danger_cadence_tests;
pub mod deposit_state_tests;
pub mod tally_tests;
pub mod danger_map_tests;
pub mod keeping_it_tests;
pub mod portion_tests;
pub mod salt_tests;
pub mod scarcity_tests;
pub mod soil_ladder_tests;
pub mod sleeping_tests;
pub mod node_index_tests;
pub mod knowing_where_things_are_tests;
pub mod worked_out_tests;
pub mod through_the_gap_tests;
pub mod the_first_children_tests;
pub mod sickness_tests;
mod full_pack_tests;
mod larder_reach_tests;
mod off_the_map_tests;
mod what_a_meal_answers_tests;
mod several_larders_tests;
mod a_roof_and_a_pack_tests;
mod what_is_worth_remembering_tests;
mod weather_tests;
pub mod working_tests;
pub mod barter_tests;
pub mod ground_tests;
pub mod combat_verb_tests;
pub mod examine_tests;
pub mod fluid_tests;
pub mod hands_tests;
pub mod errand_tests;
pub mod tool_planning_tests;
pub mod bearing_tests;
pub mod beast_tests;
pub mod belonging_tests;
pub mod innovation_tests;
pub mod map_memory_tests;
pub mod goal_tests;
pub mod stage_zero_tests;
pub mod technology_catalogue_tests;
pub mod durability_tests;
pub mod quality_tests;
pub mod retention_tests;
pub mod working_together_tests;
pub mod keeping_tests;
pub mod food_keeping_tests;
pub mod shelter_on_the_move_tests;
pub mod water_and_care_tests;
pub mod the_gaps_tests;
pub mod measurement_watch;
pub mod leatherworking_tests;
pub mod keeping_beasts_tests;
pub mod pottery_tests;
pub mod salt_in_the_body_tests;
pub mod sanitation_and_stores_tests;
pub mod hunting_for_meat_tests;
pub mod loose_ends_tests;
pub mod dead_subsystems_tests;
pub mod behaviour_gaps_tests;
pub mod stamina_tests;
pub mod larder_tests;
pub mod sacrifice_tests;
pub mod theft_tests;
pub mod threat_tests;
pub mod situation_tests;
pub mod picked_out_tests;
pub mod wondering_tests;
pub mod asking_tests;
pub mod carrying_tests;
pub mod preparation_tests;
pub mod table_order_tests;
pub mod knife_chain_tests;
pub mod repeatable_tests;
pub mod armed_tests;
pub mod earshot_tests;
pub mod growing_up_tests;
pub mod anticipation_tests;
pub mod ecology_tests;
pub mod trapline_tests;
pub mod trying_something_else_tests;
pub mod cadence_tests;
pub mod composition_tests;
pub mod planning_period_tests;
pub mod planning_reach_tests;
pub mod crafting_composition_tests;
pub mod strategy_tests;
pub mod night_tests;
pub mod by_hand_tests;
pub mod minding_tests;
pub mod young_parents_tests;
pub mod carcass_tests;
pub mod kit_tests;
pub mod practice_tests;
pub mod social_rework_tests;

/// What founders walked in with before Stage 0 took the basket and the stone
/// knife out of the kit (ISSUES_FOUND #345): a hand axe, a knife and a
/// basket, and no loose flakes.
///
/// For a fixture that is about something else and was built on that kit - a
/// pack with a basket's room in it, or a knife to reach for. Founders now
/// carry a hand axe and two flakes (`Agent::WHAT_THEY_CARRY`), and their
/// people find the knife and the basket out within a fortnight or so.
pub(crate) fn the_kit_before_stage_zero(agent: &mut crate::agents::Agent) {
    agent.inventory.remove_item("knappedtip", 2);
    for (what, how_many, each) in [("stoneknife", 1, 0.5), ("basket", 1, 1.0)] {
        let item = agent.a_tool_fresh_from_these_hands(what, how_many, each);
        agent.inventory.add_item(item);
    }
    agent.take_up_the_cart();
}

/// `the_kit_before_stage_zero` for everybody in a simulation.
pub(crate) fn everybody_with_the_kit_before_stage_zero(simulation: &mut crate::analytics::Simulation) {
    for agent in simulation.population.agents.iter_mut() {
        the_kit_before_stage_zero(agent);
    }
}

/// Plains for `reach` paces about a point, so a fixture that is about walking
/// or running is not about whatever the generated map happened to put there.
/// Agents are spawned before the world is built and draw dice doing it, so a
/// change to what they carry deals a fixture a different map.
pub(crate) fn plain_ground_about(simulation: &mut crate::analytics::Simulation, at: (i32, i32), reach: i32) {
    use crate::world::{Position, Terrain, TerrainType};
    for dx in -reach..=reach {
        for dy in -reach..=reach {
            let here = Position::new(at.0 + dx, at.1 + dy);
            if let Some(tile) = simulation.world.grid.get_tile_mut(&here) {
                tile.terrain = Terrain::new(TerrainType::Plains);
            }
        }
    }
}

/// The finished longhouse every world used to start with at its centre, for a
/// fixture built on it. Worlds start with nothing built now: founders arrive
/// as hunters and gatherers (ISSUES_FOUND #346).
pub(crate) fn the_longhouse_there_used_to_be(world: &mut crate::world::World) {
    use crate::world::{Building, BuildingType, Position};
    let (width, height) = (world.grid.width as i32, world.grid.height as i32);
    world.add_building(Building::new(
        BuildingType::Longhouse,
        Position::new(width / 2, height / 2),
    ));
}
