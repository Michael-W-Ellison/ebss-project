// src/bin/test_simulation.rs
//! Comprehensive test executable for EBSS simulation
//!
//! This executable provides easy testing and troubleshooting of the simulation
//! with full visibility into agent death mechanics (aging, starvation, damage).
//!
//! Usage:
//!   cargo run --bin test_simulation
//!   cargo run --bin test_simulation -- --agents 20 --ticks 5000

use ebss::prelude::*;
use ebss::agents::PopulationConfig;
use std::env;

fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let num_agents = parse_arg(&args, "--agents").unwrap_or(10) as u32;
    let num_ticks = parse_arg(&args, "--ticks").unwrap_or(1000) as u32;
    let report_interval = parse_arg(&args, "--report").unwrap_or(100) as u32;

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║   EBSS - Emergent Behavior Society Simulator              ║");
    println!("║   Test & Troubleshooting Executable                       ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("Configuration:");
    println!("  • Agents: {}", num_agents);
    println!("  • Ticks: {}", num_ticks);
    println!("  • Report Interval: {} ticks", report_interval);
    println!();

    // Create world
    println!("🌍 Creating world...");
    let world = World::new(WorldConfig::default());
    println!("   ✓ World initialized");

    // Create population with configured settings
    println!();
    println!("👥 Spawning population...");
    let config = PopulationConfig::default();
    let mut population = Population::with_config(config);

    for _ in 0..num_agents {
        population.spawn_agent(AgentConfig::default());
    }
    println!("   ✓ {} agents spawned", num_agents);

    // Display initial population statistics
    print_population_status(&population, 0);

    // Create simulation
    println!();
    println!("⚙️  Initializing simulation...");
    let mut sim = Simulation::new(world, population);
    println!("   ✓ Simulation ready");
    println!();
    println!("▶️  Starting simulation for {} ticks...", num_ticks);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Run simulation with periodic reporting
    for tick in 0..num_ticks {
        sim.tick();

        // Report at intervals
        if (tick + 1) % report_interval == 0 {
            print_world_status(&sim.world, tick + 1);
            print_population_status(&sim.population, tick + 1);
            print_death_watch(&sim.population, tick + 1);
        }

        // Check if population died out
        if sim.population.agents.is_empty() {
            println!();
            println!("⚠️  SIMULATION ENDED: Population extinct at tick {}", tick + 1);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            print_final_statistics(&sim.population, tick + 1);
            return;
        }
    }

    println!();
    println!("✓ Simulation completed successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print_final_statistics(&sim.population, num_ticks);
}

/// Parse command line argument
fn parse_arg(args: &[String], flag: &str) -> Option<usize> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|val| val.parse().ok())
}

/// Print world resource status
fn print_world_status(world: &World, tick: u32) {
    use ebss::world::ResourceType;

    // Count all resources
    let mut wood_nodes = 0;
    let mut wood_amount = 0;
    let mut stone_nodes = 0;
    let mut stone_amount = 0;
    let mut iron_nodes = 0;
    let mut iron_amount = 0;
    let mut food_nodes = 0;
    let mut food_amount = 0;

    for resource in &world.resources {
        match resource.resource_type {
            ResourceType::Wood => {
                wood_nodes += 1;
                wood_amount += resource.amount;
            }
            ResourceType::Stone => {
                stone_nodes += 1;
                stone_amount += resource.amount;
            }
            ResourceType::Iron => {
                iron_nodes += 1;
                iron_amount += resource.amount;
            }
            ResourceType::Food => {
                food_nodes += 1;
                food_amount += resource.amount;
            }
            _ => {}
        }
    }

    println!("🌍 World Resources at Tick {}:", tick);
    println!("   Wood:  {} nodes with {} total", wood_nodes, wood_amount);
    println!("   Stone: {} nodes with {} total", stone_nodes, stone_amount);
    println!("   Iron:  {} nodes with {} total", iron_nodes, iron_amount);
    println!("   Food:  {} nodes with {} total", food_nodes, food_amount);

    // Count buildings
    if !world.buildings.is_empty() {
        use ebss::world::BuildingState;

        let mut completed = 0;
        let mut under_construction = 0;

        for building in &world.buildings {
            match building.state {
                BuildingState::Completed => completed += 1,
                BuildingState::UnderConstruction { .. } => under_construction += 1,
            }
        }

        println!();
        println!("🏗️  Buildings:");
        println!("   Total: {} buildings", world.buildings.len());
        println!("   Completed: {}", completed);
        if under_construction > 0 {
            println!("   Under Construction: {}", under_construction);
        }
    }

    println!();
}

/// Print current population status
fn print_population_status(population: &Population, tick: u32) {
    let stats = &population.stats;

    println!("📊 Tick {}: Population Status", tick);
    println!("   Population: {} agents", population.agents.len());
    println!("   Life Stages:");
    println!("     • Infants:     {}", stats.infants);
    println!("     • Children:    {}", stats.children);
    println!("     • Adolescents: {}", stats.adolescents);
    println!("     • Adults:      {}", stats.adults);
    println!("     • Elderly:     {}", stats.elderly);
    println!("   Lifetime Stats:");
    println!("     • Total Births: {}", stats.total_births);
    println!("     • Total Deaths: {}", stats.total_deaths);
    println!("     • Abandonments: {}", stats.total_abandonments);

    // Calculate average hunger statistics
    if !population.agents.is_empty() {
        let mut total_energy = 0.0;
        let mut total_hunger = 0.0;
        let mut starving_count = 0;

        for agent in &population.agents {
            total_energy += agent.state.energy;
            if let Some(hunger) = agent.drives.get(DriveType::Hunger) {
                total_hunger += hunger.value;
            }
            if agent.state.is_starving() {
                starving_count += 1;
            }
        }

        let count = population.agents.len() as f32;
        println!("   Survival Stats:");
        println!("     • Avg Energy:  {:.1}/100.0", total_energy / count);
        println!("     • Avg Hunger:  {:.2}", total_hunger / count);
        println!("     • Starving:    {}", starving_count);

        // Calculate inventory statistics
        let mut total_wood = 0;
        let mut total_stone = 0;
        let mut total_iron = 0;
        let mut total_food_inv = 0;
        let mut total_weight = 0.0;
        let mut total_max_weight = 0.0;

        for agent in &population.agents {
            if let Some(wood) = agent.inventory.get_item("wood") {
                total_wood += wood.quantity;
            }
            if let Some(stone) = agent.inventory.get_item("stone") {
                total_stone += stone.quantity;
            }
            if let Some(iron) = agent.inventory.get_item("iron") {
                total_iron += iron.quantity;
            }
            if let Some(food) = agent.inventory.get_item("food") {
                total_food_inv += food.quantity;
            }
            total_weight += agent.inventory.current_weight;
            total_max_weight += agent.inventory.max_weight;
        }

        if total_wood > 0 || total_stone > 0 || total_iron > 0 || total_food_inv > 0 {
            println!("   Gathered Resources:");
            if total_wood > 0 {
                println!("     • Wood:  {}", total_wood);
            }
            if total_stone > 0 {
                println!("     • Stone: {}", total_stone);
            }
            if total_iron > 0 {
                println!("     • Iron:  {}", total_iron);
            }
            if total_food_inv > 0 {
                println!("     • Food:  {}", total_food_inv);
            }
            println!("     • Total Weight: {:.1}/{:.1} kg", total_weight, total_max_weight);
        }

        // Calculate injury and combat statistics
        let mut total_injuries = 0;
        let mut agents_with_injuries = 0;
        let mut disabled_parts = 0;
        let mut crippled_parts = 0;
        let mut total_body_health = 0.0;
        let mut agents_with_armor = 0;

        for agent in &population.agents {
            let mut agent_injury_count = 0;

            // Count injuries across all body parts
            for part in agent.body.parts.values() {
                agent_injury_count += part.injuries.len();
                total_injuries += part.injuries.len();

                // Check body part status
                match part.status {
                    ebss::agents::body::BodyPartStatus::Disabled |
                    ebss::agents::body::BodyPartStatus::Missing => {
                        disabled_parts += 1;
                    }
                    ebss::agents::body::BodyPartStatus::Crippled => {
                        crippled_parts += 1;
                    }
                    _ => {}
                }
            }

            if agent_injury_count > 0 {
                agents_with_injuries += 1;
            }

            // Track armor
            if !agent.body.equipment.is_empty() {
                agents_with_armor += 1;
            }

            // Sum overall body health
            total_body_health += agent.body.overall_health();
        }

        // Display injury statistics if any exist
        if total_injuries > 0 || disabled_parts > 0 || crippled_parts > 0 {
            println!("   Combat & Injuries:");
            println!("     • Total Injuries:     {}", total_injuries);
            println!("     • Agents Injured:     {}", agents_with_injuries);
            if crippled_parts > 0 {
                println!("     • Crippled Parts:     {}", crippled_parts);
            }
            if disabled_parts > 0 {
                println!("     • Disabled/Missing:   {}", disabled_parts);
            }
            println!("     • Avg Body Health:    {:.1}%", (total_body_health / count) * 100.0);
            if agents_with_armor > 0 {
                println!("     • Agents w/ Armor:    {}", agents_with_armor);
            }
        }

        // Calculate crafting statistics
        let mut total_tools = 0;
        let mut total_weapons = 0;
        let mut total_crafted_items = 0;

        for agent in &population.agents {
            // Count tools and weapons in inventory
            let tool_types = vec!["woodenaxe", "woodenpickaxe", "woodenhammer",
                                  "stoneaxe", "stonepickaxe", "stonehammer",
                                  "ironaxe", "ironpickaxe", "ironhammer"];
            let weapon_types = vec!["woodenspear", "ironsword"];

            for tool in &tool_types {
                if let Some(item) = agent.inventory.get_item(tool) {
                    total_tools += item.quantity;
                    total_crafted_items += item.quantity;
                }
            }

            for weapon in &weapon_types {
                if let Some(item) = agent.inventory.get_item(weapon) {
                    total_weapons += item.quantity;
                    total_crafted_items += item.quantity;
                }
            }
        }

        // Display crafting statistics if any items crafted
        if total_crafted_items > 0 {
            println!("   Crafting & Production:");
            if total_tools > 0 {
                println!("     • Tools Crafted:      {}", total_tools);
            }
            if total_weapons > 0 {
                println!("     • Weapons Crafted:    {}", total_weapons);
            }
            if total_crafted_items > 0 {
                println!("     • Total Items:        {}", total_crafted_items);
            }
        }

        // Calculate movement and position statistics
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        let mut total_distance_from_center = 0.0;
        let mut agents_with_leg_injuries = 0;
        let mut total_movement_speed = 0.0;

        // Get world center for reference
        let center_x = 25; // Default world size is 50x50
        let center_y = 25;

        for agent in &population.agents {
            let pos = agent.state.position;
            min_x = min_x.min(pos.0);
            max_x = max_x.max(pos.0);
            min_y = min_y.min(pos.1);
            max_y = max_y.max(pos.1);

            // Calculate distance from center
            let dx = (pos.0 - center_x) as f32;
            let dy = (pos.1 - center_y) as f32;
            total_distance_from_center += (dx * dx + dy * dy).sqrt();

            // Check leg health for movement capability
            let movement_speed = agent.body.movement_speed_multiplier();
            total_movement_speed += movement_speed;

            if movement_speed < 1.0 {
                agents_with_leg_injuries += 1;
            }
        }

        // Display movement statistics if agents have spread out
        let spread_x = (max_x - min_x) as u32;
        let spread_y = (max_y - min_y) as u32;
        if spread_x > 5 || spread_y > 5 {
            println!("   Movement & Exploration:");
            println!("     • Position Range:");
            println!("       X: {} to {} (spread: {})", min_x, max_x, spread_x);
            println!("       Y: {} to {} (spread: {})", min_y, max_y, spread_y);
            println!("     • Avg Distance from Center: {:.1} tiles",
                total_distance_from_center / count);
            println!("     • Avg Movement Speed:      {:.2}x",
                total_movement_speed / count);
            if agents_with_leg_injuries > 0 {
                println!("     • Agents w/ Leg Injuries:  {}", agents_with_leg_injuries);
            }
        }

        // Calculate relationship statistics
        use std::collections::HashMap;
        let mut total_relationships = 0;
        let mut total_positive = 0;
        let mut total_negative = 0;
        let mut total_loved_ones = 0;
        let mut total_family = 0;
        let mut total_hostile = 0;
        let mut total_bond_strength = 0.0;

        for agent in &population.agents {
            let relationships = agent.relationships.get_all();
            total_relationships += relationships.len();

            for (_, rel) in relationships {
                if rel.bond_strength > 0.0 {
                    total_positive += 1;
                    total_bond_strength += rel.bond_strength;
                } else if rel.bond_strength < 0.0 {
                    total_negative += 1;
                    total_bond_strength += rel.bond_strength;
                }

                if rel.is_loved_one() {
                    total_loved_ones += 1;
                }

                if rel.is_family() {
                    total_family += 1;
                }

                if rel.is_hostile() {
                    total_hostile += 1;
                }
            }
        }

        // Display relationship statistics if there are any
        if total_relationships > 0 {
            let avg_bond = total_bond_strength / total_relationships as f32;

            println!("   Relationships & Social Bonds:");
            println!("     • Total Relationships: {}", total_relationships);
            println!("     • Positive Bonds:      {} ({:.1}%)",
                total_positive,
                (total_positive as f32 / total_relationships as f32) * 100.0);
            println!("     • Negative Bonds:      {} ({:.1}%)",
                total_negative,
                (total_negative as f32 / total_relationships as f32) * 100.0);
            println!("     • Average Bond Strength: {:.3}", avg_bond);

            if total_loved_ones > 0 {
                println!("     • Loved Ones:          {}", total_loved_ones);
            }
            if total_family > 0 {
                println!("     • Family Bonds:        {}", total_family);
            }
            if total_hostile > 0 {
                println!("     • Hostile Relationships: {}", total_hostile);
            }
        }

        // Calculate technology statistics
        use std::collections::HashSet;
        use ebss::environment::technology::TechnologyState;

        let mut all_known_techs: HashSet<String> = HashSet::new();
        let mut tech_discovery_counts: HashMap<String, usize> = HashMap::new();
        let mut total_original_discoveries = 0;
        let mut agents_with_tech = 0;

        for agent in &population.agents {
            let agent_techs: Vec<_> = agent.technology_knowledge
                .known_technologies
                .keys()
                .collect();

            if !agent_techs.is_empty() {
                agents_with_tech += 1;
            }

            for tech_id in agent_techs {
                all_known_techs.insert(tech_id.clone());
                *tech_discovery_counts.entry(tech_id.clone()).or_insert(0) += 1;
            }

            total_original_discoveries += agent.technology_knowledge.original_discoveries.len();
        }

        // Display technology statistics
        if !all_known_techs.is_empty() {
            println!("   Technology & Knowledge:");
            println!("     • Discovered Technologies: {}", all_known_techs.len());
            println!("     • Agents with Knowledge:   {}/{}", agents_with_tech, count);
            println!("     • Original Discoveries:    {}", total_original_discoveries);

            // Sort technologies by how many agents know them
            let mut tech_vec: Vec<_> = tech_discovery_counts.iter().collect();
            tech_vec.sort_by(|a, b| b.1.cmp(a.1));

            println!("     • Most Known Technologies:");
            for (tech_id, agent_count) in tech_vec.iter().take(5) {
                let percentage = (**agent_count as f32 / count) * 100.0;
                println!("       - {}: {} agents ({:.0}%)", tech_id, agent_count, percentage);
            }
        }
    }

    println!();
}

/// Print agents approaching death (for early warning)
fn print_death_watch(population: &Population, _tick: u32) {
    let mut critical_agents = Vec::new();

    for agent in &population.agents {
        let state = &agent.state;
        let age_percent = (state.age as f32 / state.max_age as f32) * 100.0;

        // Check for critical conditions
        let mut reasons = Vec::new();

        if state.health < 30.0 {
            reasons.push(format!("Low Health: {:.1}", state.health));
        }

        if state.is_starving() {
            let days_without_food = state.ticks_without_food / 1440;
            reasons.push(format!("Starving ({}d)", days_without_food));
        }

        if state.energy < 20.0 {
            reasons.push(format!("Low Energy: {:.1}", state.energy));
        }

        if age_percent > 90.0 {
            reasons.push(format!("Old Age: {:.0}%", age_percent));
        }

        if !reasons.is_empty() {
            critical_agents.push((agent.id, state.life_stage, reasons));
        }
    }

    if !critical_agents.is_empty() {
        println!("⚠️  Critical Agents (Death Watch):");
        for (id, life_stage, reasons) in critical_agents {
            println!("   • {:?} ({:?})", id, life_stage);
            for reason in reasons {
                println!("     - {}", reason);
            }
        }
        println!();
    }
}

/// Print final statistics
fn print_final_statistics(population: &Population, total_ticks: u32) {
    println!();
    println!("📈 Final Statistics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Simulation Duration: {} ticks ({:.1} sim-days)",
             total_ticks, total_ticks as f32 / 1440.0);
    println!();
    println!("Population Metrics:");
    println!("  Final Population:  {} agents", population.agents.len());
    println!("  Total Births:      {}", population.stats.total_births);
    println!("  Total Deaths:      {}", population.stats.total_deaths);
    println!("  Abandonments:      {}", population.stats.total_abandonments);
    println!();

    if population.stats.total_deaths > 0 {
        let death_rate = (population.stats.total_deaths as f32 / total_ticks as f32) * 1000.0;
        println!("  Death Rate:        {:.2} deaths per 1000 ticks", death_rate);
    }

    if population.stats.total_births > 0 {
        let birth_rate = (population.stats.total_births as f32 / total_ticks as f32) * 1000.0;
        println!("  Birth Rate:        {:.2} births per 1000 ticks", birth_rate);
    }

    println!();

    // Detailed agent breakdown
    if !population.agents.is_empty() {
        println!("Surviving Agents:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        for (i, agent) in population.agents.iter().enumerate().take(10) {
            let state = &agent.state;
            let age_percent = (state.age as f32 / state.max_age as f32) * 100.0;

            println!("Agent #{} ({:?}):", i + 1, state.life_stage);
            println!("  Age:     {} / {} ticks ({:.1}%)", state.age, state.max_age, age_percent);
            println!("  Health:  {:.1}/100.0", state.health);
            println!("  Energy:  {:.1}/100.0", state.energy);

            if state.ticks_without_food > 0 {
                let days = state.ticks_without_food / 1440;
                println!("  Hunger:  {} days without food", days);
            } else {
                println!("  Hunger:  Well fed");
            }
            println!();
        }

        if population.agents.len() > 10 {
            println!("... and {} more agents", population.agents.len() - 10);
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
