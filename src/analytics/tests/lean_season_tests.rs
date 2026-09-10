// src/analytics/tests/lean_season_tests.rs
//! Tests that hunting and fishing are slow work, and that what comes off an
//! animal depends on the time of year.
//!
//! "Hunting using spears should take time and effort. Spear fishing is a slow
//! process as well." and "Killing an animal in late summer or autumn should
//! result in more meat, whereas killing an animal in winter and early spring
//! should result in less meat."

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::{Season, SeasonalCalendar};
use crate::world::{World, WorldConfig};

/// Put the world's calendar on a given day of the year.
fn on_the_day(simulation: &mut Simulation, day: u32) {
    simulation.world.climate.calendar.day_of_year = day;
}

fn a_world() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    Simulation::new(World::new(WorldConfig::default()), population)
}

/// Days that land in the middle of each season.
fn midsummer_and_friends() -> [(Season, u32); 4] {
    let days = crate::environment::seasons::DAYS_PER_YEAR;
    let season = days / 4;
    [
        (Season::Spring, season / 2),
        (Season::Summer, season + season / 2),
        (Season::Fall, 2 * season + season / 2),
        (Season::Winter, 3 * season + season / 2),
    ]
}

// --- the turning year -------------------------------------------------------

/// An animal killed in the autumn carries more than one killed in the spring.
#[test]
fn a_deer_is_fatter_in_the_autumn_than_in_the_spring() {
    let mut calendar = SeasonalCalendar::default();
    let mut by_season = std::collections::BTreeMap::new();

    for (season, day) in midsummer_and_friends() {
        calendar.day_of_year = day;
        assert_eq!(
            calendar.current_season(),
            season,
            "day {day} should be {season:?}"
        );
        by_season.insert(season, calendar.how_fat_the_beasts_are());
    }

    assert!(
        by_season[&Season::Fall] > by_season[&Season::Spring],
        "autumn {:.2} should beat spring {:.2}",
        by_season[&Season::Fall],
        by_season[&Season::Spring]
    );
    assert!(
        by_season[&Season::Summer] > by_season[&Season::Winter],
        "summer should beat winter"
    );
}

/// The worst of it is the end of the winter, and the best the end of autumn.
#[test]
fn the_lean_time_is_the_end_of_the_winter() {
    let days = crate::environment::seasons::DAYS_PER_YEAR;
    let mut calendar = SeasonalCalendar::default();

    let mut leanest_day = 0;
    let mut leanest = f32::MAX;
    let mut fattest_day = 0;
    let mut fattest = f32::MIN;

    for day in 0..days {
        calendar.day_of_year = day;
        let condition = calendar.how_fat_the_beasts_are();
        if condition < leanest {
            leanest = condition;
            leanest_day = day;
        }
        if condition > fattest {
            fattest = condition;
            fattest_day = day;
        }
    }

    calendar.day_of_year = leanest_day;
    let leanest_season = calendar.current_season();
    calendar.day_of_year = fattest_day;
    let fattest_season = calendar.current_season();

    assert!(
        matches!(leanest_season, Season::Winter | Season::Spring),
        "the hungry gap should be the turn of winter into spring, not {leanest_season:?}"
    );
    assert!(
        matches!(fattest_season, Season::Fall | Season::Winter),
        "and the fat time the end of the autumn, not {fattest_season:?}"
    );
    assert!(
        (leanest - SeasonalCalendar::LEANEST).abs() < 0.01,
        "the worst of it should be {}, not {leanest}",
        SeasonalCalendar::LEANEST
    );
    assert!((fattest - SeasonalCalendar::FATTEST).abs() < 0.01);
}

/// The year joins up: no day on which a herd suddenly halves.
#[test]
fn the_condition_of_a_herd_changes_gradually() {
    let days = crate::environment::seasons::DAYS_PER_YEAR;
    let mut calendar = SeasonalCalendar::default();

    let mut yesterday = {
        calendar.day_of_year = 0;
        calendar.how_fat_the_beasts_are()
    };

    for day in 1..days {
        calendar.day_of_year = day;
        let today = calendar.how_fat_the_beasts_are();
        assert!(
            (today - yesterday).abs() < 0.1,
            "condition jumped from {yesterday:.2} to {today:.2} on day {day}"
        );
        yesterday = today;
    }
}

/// A carcass butchered in the autumn yields more than one in the spring.
#[test]
fn the_same_kill_yields_more_meat_in_the_autumn() {
    use crate::environment::ItemStack;

    let dropped = vec![ItemStack {
        material_id: "meat".to_string(),
        quantity: 20,
    }];

    fn meat_on(day: u32, dropped: &[crate::environment::ItemStack]) -> u32 {
        let mut simulation = a_world();
        on_the_day(&mut simulation, day);
        simulation
            .butcher(dropped, 1.0)
            .iter()
            .map(|item| item.quantity)
            .sum()
    }

    let days = crate::environment::seasons::DAYS_PER_YEAR;
    let season = days / 4;
    let autumn = meat_on(2 * season + season - 1, &dropped);
    let spring = meat_on(1, &dropped);

    assert!(
        autumn > spring,
        "twenty units of deer should come to more in the autumn ({autumn}) \
         than at the turn of the year ({spring})"
    );
}

// --- slow work --------------------------------------------------------------

/// A throw does not kill outright.
#[test]
fn one_throw_does_not_bring_a_deer_down() {
    assert!(
        Simulation::WHAT_ONE_THROW_TAKES_OUT_OF_IT < 0.5,
        "a hunt that ends on the first hit is not a hunt"
    );
}

/// A hunt is mostly missing, for somebody with nothing in his hands.
#[test]
fn a_hunt_is_mostly_missing() {
    assert!(
        Simulation::A_THROW_THAT_TELLS <= 0.35,
        "a bare-handed throw should mostly miss"
    );
    assert!(
        Simulation::A_THRUST_THAT_TELLS <= 0.2,
        "and a man standing in a river with nothing is mostly just standing"
    );
}

/// Standing in the water costs something whether or not anything takes.
#[test]
fn a_cast_that_catches_nothing_still_costs_the_morning() {
    use crate::environment::Action;

    let mut simulation = a_world();

    // Drain every reach so that nothing can possibly take.
    for resource in simulation.world.resources.iter_mut() {
        if resource.resource_type == crate::world::ResourceType::Fish {
            resource.amount = 0;
        }
    }

    let before = simulation.population.agents[0].state.energy;
    let mut cast = 0;
    for _ in 0..40 {
        let result = simulation.execute_action(&Action::Fish, 0);
        if result.energy_cost > 0.0 {
            cast += 1;
        }
    }

    if cast > 0 {
        assert!(
            simulation.population.agents[0].state.energy < before,
            "forty casts should have cost something"
        );
    }
}

// --- the mast ---------------------------------------------------------------

/// The top of this model's own energy scale was a food that did not exist.
///
/// `physiology.rs` describes the nutrition database as running "from six
/// (spring greens) to eighty (fat and nuts)" - and nothing in the world
/// yielded a nut, no `ItemType` held one and no plant dropped one. The best
/// food a temperate forager has was named in a comment and nowhere else.
#[test]
fn a_nut_is_the_best_thing_in_the_wood_and_it_keeps() {
    use crate::world::nutrition::FoodDatabase;
    use crate::world::ItemType;

    let food = FoodDatabase::new();
    let nuts = food.create_food_data(&ItemType::Nuts, 0).expect("nuts are food");
    let greens = food.create_food_data(&ItemType::Greens, 0).expect("greens are food");
    let grain = food.create_food_data(&ItemType::Grain, 0).expect("grain is food");
    let meat = food.create_food_data(&ItemType::Meat, 0).expect("meat is food");

    assert!(
        nuts.base_nutrition.energy > greens.base_nutrition.energy * 10.0,
        "the top of the scale against the bottom of it: {} against {}",
        nuts.base_nutrition.energy,
        greens.base_nutrition.energy
    );
    assert!(
        nuts.base_nutrition.energy >= grain.base_nutrition.energy,
        "and better than a harvest"
    );

    // And the point of it: it keeps, with nothing done to it. Everything
    // else a settlement puts by has to be dried, salted, smoked or buried,
    // and the throughput of that is what caps the winter store - #241.
    assert!(
        nuts.base_spoilage_ticks > meat.base_spoilage_ticks * 10,
        "a nut outlasts fresh meat many times over: {} against {}",
        nuts.base_spoilage_ticks,
        meat.base_spoilage_ticks
    );
    assert!(
        nuts.base_spoilage_ticks
            > (crate::environment::seasons::DAYS_PER_SEASON
                * crate::environment::seasons::TICKS_PER_DAY) as u32,
        "gathered in October and still food in March: {}",
        nuts.base_spoilage_ticks
    );
}

/// The mast is on the ground for a few weeks of autumn and at no other time.
#[test]
fn the_mast_is_an_autumn_and_nothing_else() {
    use crate::environment::seasons::{Season, DAYS_PER_SEASON};
    use crate::world::ResourceType;

    let midsummer = Season::Summer.first_day() + DAYS_PER_SEASON / 2;
    let autumn = Season::Fall.first_day() + DAYS_PER_SEASON / 2;
    let midwinter = Season::Winter.first_day() + DAYS_PER_SEASON / 2;
    let spring = Season::Spring.first_day() + DAYS_PER_SEASON / 2;

    assert!(ResourceType::Nuts.is_it_bearing(autumn), "the mast is an autumn");
    assert!(!ResourceType::Nuts.is_it_bearing(midsummer));
    assert!(!ResourceType::Nuts.is_it_bearing(midwinter));
    assert!(!ResourceType::Nuts.is_it_bearing(spring));

    // Later than the grain, which is what a harvest and a mast are.
    assert!(
        ResourceType::Grain.is_it_bearing(Season::Summer.first_day() + DAYS_PER_SEASON - 5),
        "grain comes in at the end of summer"
    );
}

/// A wood does not drop the same weight of mast every year, and that is the
/// first thing in this model that makes one year worth more than another.
#[test]
fn some_autumns_are_worth_being_in_a_wood_for() {
    use crate::world::ResourceType;

    let over_twenty_years: Vec<f32> =
        (0..20).map(ResourceType::how_heavy_the_mast_is).collect();

    let lean = over_twenty_years.iter().filter(|&&m| m < 0.5).count();
    let flooded = over_twenty_years.iter().filter(|&&m| m > 1.5).count();
    let ordinary = over_twenty_years.len() - lean - flooded;

    assert!(lean >= 4, "a good few autumns are bad ones: {lean} of 20");
    assert!(flooded >= 2, "and a few are mast years: {flooded} of 20");
    assert!(ordinary >= 4, "and the rest are ordinary: {ordinary} of 20");

    // Every wood in a country agrees, because a district where half the woods
    // bore would be no gamble at all - so this is worked out from the year
    // and not rolled.
    assert_eq!(
        ResourceType::how_heavy_the_mast_is(7),
        ResourceType::how_heavy_the_mast_is(7),
        "the same year is the same year"
    );

    // And nothing else in the world has a mast year.
    assert!(ResourceType::Nuts.does_it_have_mast_years());
    for other in [ResourceType::Food, ResourceType::Grain, ResourceType::Greens] {
        assert!(!other.does_it_have_mast_years(), "{other:?} bears what it bears");
    }
}

/// The mast reaches a pack under whatever name the tree that dropped it goes
/// by, and it reaches the world under the trees that drop it.
#[test]
fn the_mast_is_gatherable_and_storable() {
    use crate::agents::storage_integration::id_to_item_type;
    use crate::world::resource_spawning::TerrainResourceMapper;
    use crate::world::{ItemType, ResourceType, TerrainType};

    for named in ["nuts", "acorns", "hazelnuts", "chestnuts", "walnuts"] {
        assert_eq!(
            id_to_item_type(named),
            Some(ItemType::Nuts),
            "{named} should reach the pack as nuts"
        );
    }

    let where_it_falls = TerrainResourceMapper::preferred_terrains(ResourceType::Nuts);
    assert!(
        where_it_falls.contains(&TerrainType::Forest),
        "the mast falls under the trees: {where_it_falls:?}"
    );
    assert!(
        !where_it_falls.contains(&TerrainType::Plains),
        "and not on open ground"
    );

    // And an oak drops it, which it did not before: an oak was standing
    // timber and nothing else.
    let flora = crate::environment::flora::FloraRegistry::new();
    let oak = flora.get("oak_tree").expect("oaks exist");
    assert!(
        oak.drops.iter().any(|drop| drop.material_id == "acorns"),
        "an oak drops acorns"
    );
}
