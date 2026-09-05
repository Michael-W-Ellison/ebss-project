// examples/climate_demo.rs
//! Comprehensive demonstration of the climate and exposure systems
//!
//! This example shows:
//! - Seasonal progression
//! - Weather changes
//! - Biome variations
//! - Temperature effects
//! - Exposure damage accumulation
//! - Climate impact on agents

use ebss::environment::{
    BiomeType, Biome, Weather, WeatherType, WeatherGenerator,
    Season, SeasonalCalendar, ExposureStatus, ExposureProtection, ExposureType,
    DAYS_PER_SEASON, DAYS_PER_YEAR,
};
use ebss::agents::temperature::BodyTemperature;
use ebss::world::{ClimateManager, Position, TerrainType, terrain_to_biome};

fn main() {
    println!("=== EBSS Climate and Exposure System Demonstration ===\n");

    // ===== Part 1: Seasonal Calendar =====
    println!("--- Part 1: Seasonal Calendar and Time ---");

    let mut calendar = SeasonalCalendar::default();

    println!("Starting conditions:");
    println!("  {}", calendar.date_string());
    println!("  Season: {:?}", calendar.current_season());
    println!("  Daytime: {}", calendar.is_daytime());
    println!("  Sun intensity: {:.2}", calendar.sun_intensity());
    println!();

    // Simulate one day
    println!("Simulating one day...");
    for _ in 0..calendar.ticks_per_day() {
        calendar.tick();
    }

    println!("After 24 hours:");
    println!("  {}", calendar.date_string());
    println!("  Day of year: {}", calendar.day_of_year);
    println!();

    // Jump to different seasons
    println!("Seasonal characteristics:");
    for season in [Season::Spring, Season::Summer, Season::Fall, Season::Winter] {
        println!("  {:?}:", season);
        println!(
            "    How far into the year: {:.2}",
            season.how_far_into_the_year_it_is()
        );
        println!("    Day length: {:.1} hours", season.day_length());
        println!("    Plant growth: {:.1}x", season.plant_growth_modifier());
        println!("    Precipitation: {:.1}x", season.precipitation_modifier());
    }
    println!();

    // ===== Part 2: Biome System =====
    println!("--- Part 2: Biome Characteristics ---");

    let biomes = [
        BiomeType::Tundra,
        BiomeType::Desert,
        BiomeType::Tropical,
        BiomeType::TemperateForest,
        BiomeType::Alpine,
    ];

    for biome_type in biomes {
        let (min_temp, max_temp) = biome_type.temperature_range();
        println!("{:?}:", biome_type);
        println!("  Temperature range: {:.1}°C to {:.1}°C", min_temp, max_temp);
        println!("  Humidity: {:.0}%", biome_type.average_humidity() * 100.0);
        println!("  Exposure risk: {:.1}/10", biome_type.exposure_risk() * 10.0);
        println!("  Shelter availability: {:.1}/10", biome_type.shelter_availability() * 10.0);
        println!("  Resource abundance: {:.1}/10", biome_type.resource_abundance() * 10.0);
    }
    println!();

    // ===== Part 3: Weather System =====
    println!("--- Part 3: Weather System ---");

    let weather_types = [
        WeatherType::Clear,
        WeatherType::Rain,
        WeatherType::Thunderstorm,
        WeatherType::Snow,
        WeatherType::Blizzard,
        WeatherType::Fog,
    ];

    for weather_type in weather_types {
        println!("{:?}:", weather_type);
        println!("  Visibility reduction: {:.0}%", weather_type.visibility_reduction() * 100.0);
        println!("  Movement modifier: {:.0}%", weather_type.movement_modifier() * 100.0);
        println!("  Temperature change: {:+.1}°C", weather_type.temperature_modifier());
        println!("  Precipitation: {:.2}", weather_type.precipitation_intensity());
        println!("  Exposure damage: {:.3}/day", weather_type.exposure_damage_in_a_day());
    }
    println!();

    // ===== Part 4: Weather Generation =====
    println!("--- Part 4: Weather Generation ---");

    let mut weather_gen = WeatherGenerator::new(Season::Winter, true, true); // Cold, wet climate
    println!("Generating weather for cold, wet climate in winter:");

    for i in 1..=5 {
        let weather = weather_gen.generate_weather();
        println!("  {}. {:?} - lasts {} ticks", i, weather.weather_type, weather.duration_remaining);
    }
    println!();

    weather_gen = WeatherGenerator::new(Season::Summer, false, false); // Warm, dry climate
    println!("Generating weather for warm, dry climate in summer:");

    for i in 1..=5 {
        let weather = weather_gen.generate_weather();
        println!("  {}. {:?} - lasts {} ticks", i, weather.weather_type, weather.duration_remaining);
    }
    println!();

    // ===== Part 5: Exposure Damage =====
    println!("--- Part 5: Exposure Damage System ---");

    let mut exposure = ExposureStatus::new();
    let mut body_temp = BodyTemperature::new();

    // Simulate cold exposure
    println!("Scenario 1: Cold exposure without shelter");
    body_temp.current = 34.0; // Hypothermia
    let cold_weather = Weather::new(WeatherType::Blizzard);

    for tick in 0..10 {
        let damage = exposure.update(&body_temp, -10.0, &cold_weather, false, true, 12.0);
        if tick % 3 == 0 {
            println!("  Tick {}: Damage {:.3}, Wetness {:.2}, Active exposures: {:?}",
                tick, damage, exposure.wetness, exposure.active_exposures);
        }
    }

    if let Some(action) = exposure.recommended_action() {
        println!("  Recommended action: {}", action);
    }
    println!("  Total damage: {:.2}", exposure.exposure_damage);
    println!("  Critical condition: {}", exposure.is_critical());
    println!();

    // Simulate recovery
    println!("Recovering in shelter...");
    exposure.recover(0.5);
    println!("  Damage after recovery: {:.2}", exposure.exposure_damage);
    println!();

    // Simulate heat exposure
    println!("Scenario 2: Heat exposure in desert");
    let mut heat_exposure = ExposureStatus::new();
    let mut hot_body_temp = BodyTemperature::new();
    hot_body_temp.current = 40.0; // Hyperthermia
    let hot_weather = Weather::new(WeatherType::Clear);

    for tick in 0..20 {
        let damage = heat_exposure.update(&hot_body_temp, 45.0, &hot_weather, false, false, 14.0);
        if tick % 5 == 0 {
            println!("  Tick {}: Damage {:.3}, Sun exposure {:.2}, Active: {:?}",
                tick, damage, heat_exposure.sun_exposure, heat_exposure.active_exposures);
        }
    }

    if let Some(action) = heat_exposure.recommended_action() {
        println!("  Recommended action: {}", action);
    }
    println!();

    // ===== Part 6: Clothing Protection =====
    println!("--- Part 6: Clothing Protection ---");

    let naked = ExposureProtection::new();
    let basic = ExposureProtection::basic_clothing();
    let winter_gear = ExposureProtection::winter_gear();
    let desert_gear = ExposureProtection::desert_clothing();

    println!("Protection levels:");
    println!("  Naked:");
    println!("    Overall rating: {:.1}/10", naked.overall_rating() * 10.0);
    println!("  Basic clothing:");
    println!("    Cold insulation: {:.1}/10", basic.cold_insulation * 10.0);
    println!("    Sun protection: {:.1}/10", basic.sun_protection * 10.0);
    println!("  Winter gear:");
    println!("    Cold insulation: {:.1}/10", winter_gear.cold_insulation * 10.0);
    println!("    Water resistance: {:.1}/10", winter_gear.water_resistance * 10.0);
    println!("  Desert clothing:");
    println!("    Heat resistance: {:.1}/10", desert_gear.heat_resistance * 10.0);
    println!("    Sun protection: {:.1}/10", desert_gear.sun_protection * 10.0);
    println!();

    // Test wetness impact
    println!("Effect of wetness on insulation:");
    println!("  Winter gear (dry): {:.1}/10", winter_gear.effective_cold_insulation(0.0) * 10.0);
    println!("  Winter gear (50% wet): {:.1}/10", winter_gear.effective_cold_insulation(0.5) * 10.0);
    println!("  Winter gear (fully wet): {:.1}/10", winter_gear.effective_cold_insulation(1.0) * 10.0);
    println!();

    // ===== Part 7: Climate Manager Integration =====
    println!("--- Part 7: Climate Manager ---");

    let mut climate_mgr = ClimateManager::new(false, false); // Temperate, moderate

    println!("Initial state:");
    println!("  {}", climate_mgr.date_time_string());
    println!("  Season: {:?}", climate_mgr.current_season());
    println!("  Visibility range: {} tiles", climate_mgr.visibility_range());
    println!("  Movement modifier: {:.0}%", climate_mgr.movement_modifier() * 100.0);
    println!();

    // Simulate time passage
    println!("Simulating 10 hours (1000 ticks)...");
    for _ in 0..1000 {
        climate_mgr.tick();
    }

    println!("After 10 hours:");
    println!("  {}", climate_mgr.date_time_string());
    println!();

    // Check different terrain biomes
    println!("Temperature by terrain type:");
    for terrain in [TerrainType::Plains, TerrainType::Forest, TerrainType::Mountain, TerrainType::Water] {
        let pos = Position::new(0, 0);
        let temp = climate_mgr.get_temperature(pos, terrain);
        let biome = terrain_to_biome(terrain);
        println!("  {:?} ({:?}): {:.1}°C", terrain, biome, temp);
    }
    println!();

    // ===== Part 8: Seasonal Progression =====
    println!("--- Part 8: Full Year Simulation ---");

    let mut year_calendar = SeasonalCalendar::default();
    let mut season_days = vec![0, 0, 0, 0]; // Count days in each season

    println!("Simulating one full year ({} days)...", DAYS_PER_YEAR);

    for day in 0..DAYS_PER_YEAR {
        for _ in 0..year_calendar.ticks_per_day() {
            year_calendar.tick();
        }

        let season_idx = match year_calendar.current_season() {
            Season::Winter => 0,
            Season::Spring => 1,
            Season::Summer => 2,
            Season::Fall => 3,
        };
        season_days[season_idx] += 1;

        // Print milestone days
        if (day + 1) % DAYS_PER_SEASON == 0 {
            println!("  Day {}: {} ({} progress: {:.0}%)",
                day + 1,
                year_calendar.current_season().name(),
                year_calendar.date_string(),
                year_calendar.season_progress() * 100.0);
        }
    }

    println!("\nDays per season:");
    println!("  Winter: {}", season_days[0]);
    println!("  Spring: {}", season_days[1]);
    println!("  Summer: {}", season_days[2]);
    println!("  Fall: {}", season_days[3]);
    println!();

    // ===== Part 9: Combined Scenario =====
    println!("--- Part 9: Realistic Agent Scenario ---");

    println!("An agent travels through a tundra biome in winter...\n");

    let mut scenario_climate = ClimateManager::new(true, true); // Cold, wet
    let mut scenario_exposure = ExposureStatus::new();
    let mut scenario_body_temp = BodyTemperature::new();
    let protection = ExposureProtection::winter_gear();

    let tundra_pos = Position::new(10, 10);

    println!("Equipment: Winter gear");
    println!("Starting conditions:");
    println!("  Weather: {:?}", scenario_climate.weather.weather_type);
    println!("  Body temperature: {:.1}°C", scenario_body_temp.current);
    println!();

    println!("Hour-by-hour progression:");

    for hour in 0..12 {
        // Tick one hour
        for _ in 0..100 {
            scenario_climate.tick();
        }

        let env_temp = scenario_climate.get_temperature(tundra_pos, TerrainType::Plains);
        scenario_body_temp.update(env_temp, protection.cold_insulation, 0.1);

        let damage = scenario_exposure.update(
            &scenario_body_temp,
            env_temp,
            &scenario_climate.weather,
            false,
            true,
            scenario_climate.calendar.time_of_day,
        );

        if hour % 3 == 0 {
            println!("  Hour {}:", hour);
            println!("    Time: {:.0}:00", scenario_climate.calendar.time_of_day.floor());
            println!("    Environment: {:.1}°C", env_temp);
            println!("    Body temp: {:.1}°C", scenario_body_temp.current);
            println!("    Damage this hour: {:.3}", damage);
            println!("    Total damage: {:.2}", scenario_exposure.exposure_damage);
            println!("    Wetness: {:.0}%", scenario_exposure.wetness * 100.0);

            if !scenario_exposure.active_exposures.is_empty() {
                println!("    Active exposures: {:?}", scenario_exposure.active_exposures);
            }

            if let Some(action) = scenario_exposure.recommended_action() {
                println!("    ⚠ {}", action);
            }
        }
    }

    println!("\nFinal status:");
    println!("  Survival status: {}",
        if scenario_exposure.is_critical() { "CRITICAL" } else { "Stable" });
    println!("  Total exposure damage: {:.2}", scenario_exposure.exposure_damage);
    println!();

    // ===== Summary =====
    println!("=== Key Features Demonstrated ===");
    println!("✓ Seasonal calendar with time progression");
    println!("✓ 10 distinct biome types with unique characteristics");
    println!("✓ 13 weather conditions with dynamic generation");
    println!("✓ Temperature system affected by season and weather");
    println!("✓ 6 exposure damage types (hypothermia, hyperthermia, frostbite, etc.)");
    println!("✓ Clothing protection system with wetness effects");
    println!("✓ Climate manager integrating all systems");
    println!("✓ Realistic environmental hazards for agents");
    println!("✓ Recovery mechanics");
    println!("✓ Context-aware recommendations");

    println!("\n=== Demonstration Complete ===");
}
