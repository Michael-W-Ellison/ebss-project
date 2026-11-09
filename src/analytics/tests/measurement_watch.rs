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
    // REDRAW=n keeps the same world and founders and draws the rest of the
    // run's dice afresh, so the spread a counter has between runs of the same
    // code can be measured (#413). Unset, the dice are those they always were.
    let redraw: u64 = std::env::var("REDRAW").ok().and_then(|r| r.parse().ok()).unwrap_or(0);
    crate::core::dice::seed(seed * 7919 + 3 + redraw * 104_729);
    // At the development pace unless told otherwise (#389)
    if let Some(pace) = std::env::var("PACE").ok().and_then(|p| p.parse::<f64>().ok()) {
        crate::core::discovery::set(pace);
    }
    println!("STORE slowness {}", crate::core::discovery::slowness() as u64);
    // FARMING=1 starts the founders as farmers of stage 3 of the
    // agricultural path: they scatter seed, break ground, keep the seasons in
    // mind and save seed (#417). What comes after stage 3 is seldom reached
    // inside a two-year run from foraging (#415), so it is measured from here.
    if std::env::var("FARMING").is_ok() {
        for agent in sim.population.agents.iter_mut() {
            for what in ["do:ScatterSeed", "do:TillSoil", "know:seasons", "know:save_seed"] {
                agent.found_out_how_to(what);
            }
        }
    }

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

    // How far apart partners are, and how many days people spend living
    // off their own reserve, once a day (#376)
    let (mut partner_days, mut partners_apart, mut partners_far, mut wasting_days) = (0u64, 0u64, 0u64, 0u64);
    for day in 0..days {
        for _ in 0..PLANNING_PERIODS_PER_DAY {
            sim.take_a_turn();
        }
        note(&sim, &mut first, day + 1);
        // How the fields stand, once a month (#417)
        if (day + 1) % 30 == 0 {
            farm_line(&sim, day + 1);
        }
        for agent in sim.population.agents.iter().filter(|a| a.state.is_alive) {
            if agent.state.physiology.is_wasting() {
                wasting_days += 1;
            }
            let Some(partner) = agent.my_partner() else { continue };
            let Some(them) = sim.population.agents.iter().find(|a| a.id == partner && a.state.is_alive) else { continue };
            let apart = (agent.state.position.0 - them.state.position.0)
                .abs()
                .max((agent.state.position.1 - them.state.position.1).abs()) as u64;
            partner_days += 1;
            partners_apart += apart;
            if apart > 50 {
                partners_far += 1;
            }
        }
    }
    println!("STORE partner_days {partner_days}");
    println!("STORE partner_cells_apart {partners_apart}");
    println!("STORE partner_days_far_apart {partners_far}");
    println!("STORE wasting_person_days {wasting_days}");

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
        ("beasts_driven_off", sim.world.beasts_driven_off),
        ("thorn_hedges_closed", sim.world.thorn_hedges_closed),
        ("beasts_the_thorns_turned", sim.world.beasts_the_thorns_turned),
        ("watches_kept", sim.world.watches_kept),
        ("woken_by_the_watch", sim.world.woken_by_the_watch),
        ("beasts_struck_by_night", sim.world.beasts_struck_by_night),
        ("beasts_struck_by_day", sim.world.beasts_struck_by_day),
        ("beasts_warned_by_night", sim.world.beasts_warned_by_night),
        ("beasts_warned_by_day", sim.world.beasts_warned_by_day),
        ("sparing_harvests", sim.world.sparing_harvests),
        (
            "stands_kept",
            sim.world.resources.iter().filter(|node| node.tended >= 0.5).count() as u64,
        ),
        ("wildfires", sim.world.wildfires),
        ("burns_set", sim.world.burns_set),
        ("stands_burnt", sim.world.stands_burnt),
        ("cuts_to_the_stool", sim.world.cuts_to_the_stool),
        ("ditches_dug", sim.world.ditches_dug),
        ("compost_dug", sim.world.compost_dug),
        ("lessons_taught", sim.world.lessons_taught),
        ("stores_lined", sim.world.stores_lined),
        ("stores_sealed", sim.world.stores_sealed),
        ("field_crop_fallen", sim.world.field_crop_fallen),
        ("field_crop_brought_in", sim.world.field_crop_brought_in),
        ("crops_that_failed", sim.world.crops_that_failed),
        ("crops_that_paid", sim.world.crops_that_paid),
        ("crops_ripened_full", sim.world.crops_ripened_full),
        ("crops_ripened_short", sim.world.crops_ripened_short),
        ("fields_hedged", sim.world.fields_hedged),
        // The fourth stage of the agricultural path (#417 to #421)
        ("ground_left_unbroken", sim.ground_left_unbroken.get()),
        ("sowings_put_off", sim.sowings_put_off.get()),
        ("worn_fields_mucked", sim.world.worn_fields_mucked),
        ("granaries_built", sim.world.granaries_built),
        ("staple_fields_sown", sim.world.staple_fields_sown),
        ("new_ground_by_keepers_paying", sim.world.new_ground_broken_by[0]),
        ("new_ground_by_keepers_untried", sim.world.new_ground_broken_by[1]),
        ("new_ground_by_keepers_failing", sim.world.new_ground_broken_by[2]),
        ("new_ground_by_others", sim.world.new_ground_broken_by[3]),
        ("new_ground_away_from_home", sim.world.new_ground_broken_away_from_home),
        (
            "put_by_at_the_end",
            sim.world.pits.iter().map(|pit| u64::from(pit.how_much_is_in_it())).sum(),
        ),
        (
            "in_granaries_at_the_end",
            sim.world.pits.iter().filter(|pit| pit.is_a_granary()).map(|pit| u64::from(pit.how_much_is_in_it())).sum(),
        ),
        (
            "fields_at_the_end",
            sim.world.grid.every_field().count() as u64,
        ),
        (
            "fields_worn_at_the_end",
            sim.world.grid.every_field().filter(|(_, field)| field.grade < field.natural).count() as u64,
        ),
        ("wounds_cleaned", sim.world.wounds_cleaned),
        ("wounds_cauterised", sim.world.wounds_cauterised),
        ("wounds_stitched", sim.world.wounds_stitched),
        ("poultices_laid", sim.world.poultices_laid),
        ("teas_made", sim.world.teas_made),
        ("wounds_turned", sim.world.wounds_turned),
        ("bones_broken", sim.world.bones_broken),
        ("bones_set", sim.world.bones_set),
        ("bones_knit_crooked", sim.world.bones_knit_crooked),
        ("bones_knit_straight", sim.world.bones_knit_straight),
        ("hard_births", sim.world.hard_births),
        ("children_lost_at_birth", sim.world.children_lost_at_birth),
        ("mothers_lost_in_childbirth", sim.world.mothers_lost_in_childbirth),
        ("births_with_a_midwife", sim.world.births_with_a_midwife),
        ("calls_raised", sim.world.calls_raised),
        ("heard_a_call", sim.world.heard_a_call),
        ("blows_a_call_would_have_spared", sim.world.blows_a_call_would_have_spared),
        ("ways_weighed_by_the_hand", sim.ways_weighed_by_the_hand.get()),
        ("better_hands_seen", sim.world.better_hands_seen),
        ("stores_reckoned_by_tally", sim.stores_reckoned_by_tally.get()),
        ("stores_found_short", sim.population.stats.stores_found_short),
        ("fields_sown_thin", sim.world.fields_sown_thin),
        ("fields_sown_thick", sim.world.fields_sown_thick),
        ("fields_sown_measured", sim.world.fields_sown_measured),
        // How far the best seed anybody keeps has been bred, in hundredths
        // (#397)
        (
            "best_seed_line_x100",
            (sim.population.agents.iter().filter(|a| a.state.is_alive).map(|a| a.seed_line).fold(0.0f32, f32::max)
                * 100.0) as u64,
        ),
        // The most fields inside an edge and outside one anybody has weeded,
        // which is what the edge is found out from (#398)
        (
            "most_hedged_fields_weeded",
            sim.population.agents.iter().filter_map(|a| a.field_pests.get("hedged")).map(|(n, _)| *n as u64).max().unwrap_or(0),
        ),
        (
            "most_open_fields_weeded",
            sim.population.agents.iter().filter_map(|a| a.field_pests.get("open")).map(|(n, _)| *n as u64).max().unwrap_or(0),
        ),
        (
            "stands_watered",
            sim.world.resources.iter().filter(|node| node.watered >= 0.5).count() as u64,
        ),
        (
            "stools_kept",
            sim.world.resources.iter().filter(|node| node.coppiced >= 0.5).count() as u64,
        ),
    ] {
        println!("STORE {what} {how_many}");
    }
    // How children came about, and how many tries it took (#375)
    for what in ["a coupling tried", "a coupling conceived", "a pair conceived", "a pair did not take"] {
        let how_many = sim.population.stats.how_it_went.get(what).copied().unwrap_or(0);
        println!("STORE {} {}", what.replace(' ', "_"), how_many);
    }
    println!(
        "BD D{:04} births {} deaths {} alive {}",
        days,
        sim.population.stats.total_births,
        sim.population.stats.total_deaths,
        alive.len()
    );
}

/// One line on how a world's farming stands (#417): its fields and how worn
/// they are, what is in crop on them, what has come in and fallen so far,
/// what is put by in stores, and how many live and how many farm.
fn farm_line(sim: &Simulation, day: u32) {
    use crate::world::soil::SoilGrade;
    let mut grades = [0u32; 5];
    for (_, field) in sim.world.grid.every_field() {
        let rung = SoilGrade::LADDER.iter().position(|g| *g == field.grade).unwrap_or(0);
        grades[rung] += 1;
    }
    let crops: Vec<_> = sim.world.resources.iter().filter(|node| node.on_a_field).collect();
    let ripe = crops.iter().filter(|node| node.ripe_stand > 0).count();
    let standing: u32 = crops.iter().map(|node| node.amount).sum();
    let mut by_crop: BTreeMap<String, u32> = BTreeMap::new();
    for node in &crops {
        *by_crop.entry(format!("{:?}", node.resource_type)).or_insert(0) += 1;
    }
    let put_by: u32 = sim
        .world
        .pits
        .iter()
        .flat_map(|pit| pit.holds.iter())
        .filter(|item| item.food_data.is_some())
        .map(|item| item.quantity)
        .sum();
    let alive: Vec<_> = sim.population.agents.iter().filter(|a| a.state.is_alive).collect();
    let farmers = alive
        .iter()
        .filter(|a| a.practices.is_established(crate::agents::practices::Practice::Farming))
        .count();
    let crops_by: Vec<String> = by_crop.iter().map(|(k, v)| format!("{k}:{v}")).collect();
    println!(
        "FARMX D{day:04} fields={} grades={:?} crops={} ripe={ripe} standing={standing} [{}] in={} fallen={} paid={} failed={} put_by={put_by} alive={} farmers={farmers}",
        grades.iter().sum::<u32>(),
        grades,
        crops.len(),
        crops_by.join(","),
        sim.world.field_crop_brought_in,
        sim.world.field_crop_fallen,
        sim.world.crops_that_paid,
        sim.world.crops_that_failed,
        alive.len(),
    );
}
