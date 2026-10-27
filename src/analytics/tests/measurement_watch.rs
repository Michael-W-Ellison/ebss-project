// src/analytics/tests/measurement_watch.rs
//! The settlement measurement every ISSUES_FOUND entry since #345 reports:
//! one seeded world of twelve founders, run for `DAYS` days, printing what
//! was found out and when, and the births, deaths and survivors at the end.
//!
//! Ignored, because it is a measurement and not a test. Run one world with
//!
//! ```text
//! SMOOTH=1 SEEDS=3 DAYS=720 cargo test --release --lib -- --ignored --nocapture zz_discovery_watch
//! ```
//!
//! and forty (smooth 1-24, scattered 1-16) side by side for two builds to
//! compare them. It lived outside the repository until #352, and was lost
//! with the container that held it.

use crate::analytics::Simulation;
use crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
use std::collections::BTreeMap;

#[test]
#[ignore]
fn zz_discovery_watch() {
    let seed: u64 = std::env::var("SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let days: u32 = std::env::var("DAYS").ok().and_then(|s| s.parse().ok()).unwrap_or(720);
    crate::core::dice::seed(seed);
    let mut cfg = crate::world::WorldConfig::big_enough_for_an_ecology();
    if std::env::var("SMOOTH").is_ok() {
        cfg = cfg.with_smooth_terrain();
    }
    if std::env::var("SCATTERED").is_ok() {
        cfg = cfg.with_scattered_terrain();
    }
    let mut population = crate::agents::Population::new();
    for _ in 0..12 {
        population.spawn_agent(crate::agents::AgentConfig::default());
    }
    let mut sim = Simulation::new(crate::world::World::new(cfg), population);
    sim.bring_everybody_to_the_middle();
    crate::core::dice::seed(seed * 7919 + 3);

    let mut first: BTreeMap<String, u32> = BTreeMap::new();
    let note = |sim: &Simulation, first: &mut BTreeMap<String, u32>, day: u32| {
        for agent in sim.population.agents.iter().filter(|a| a.state.is_alive) {
            for what in agent.what_i_found_out() {
                first.entry(what.clone()).or_insert(day);
            }
        }
    };
    note(&sim, &mut first, 0);
    let at_start: Vec<String> = first.keys().cloned().collect();
    println!("START {}", at_start.join(","));

    for day in 0..days {
        for _ in 0..PLANNING_PERIODS_PER_DAY {
            sim.take_a_turn();
        }
        note(&sim, &mut first, day + 1);
    }

    let alive: Vec<_> = sim.population.agents.iter().filter(|a| a.state.is_alive).collect();
    for animal in sim.world.animals.get_all().iter().filter(|a| a.is_alive() && a.is_kept()) {
        let how = if animal.is_domesticated { "tame" } else { "raising" };
        println!("HERDX kept {how} {}", animal.species_id);
    }
    for (what, day) in first.iter() {
        let holders = alive.iter().filter(|a| a.what_i_found_out().contains(what)).count();
        println!("DISC {} day={} holders={} of {}", what, day, holders, alive.len());
    }
    // What became of what was put by, in units, as lines the compare script
    // counts one per unit (#357)
    for (what, how_many) in [
        ("rotted", sim.world.food_that_rotted_in_the_ground),
        ("vermin", sim.world.food_the_vermin_took),
        ("scavengers", sim.world.food_the_scavengers_took),
        ("meat_left_on_kills", sim.world.meat_left_on_kills),
        ("meat_the_crows_had", sim.world.meat_the_crows_had),
        ("carriers_taken_on", sim.world.carriers_taken_on),
        ("joints_carried_for_a_share", sim.world.joints_carried_for_a_share),
        ("joints_kept_as_pay", sim.world.joints_kept_as_pay),
    ] {
        println!("STORE {what} {how_many}");
    }
    println!(
        "BD D{:04} births {} deaths {} alive {}",
        days,
        sim.population.stats.total_births,
        sim.population.stats.total_deaths,
        alive.len()
    );
}
