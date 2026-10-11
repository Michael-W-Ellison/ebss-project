// src/analytics/tests/terrain_tests.rs
//! The lie of the land.
//!
//! Terrain was drawn tile by tile from a hash with no seed: neighbouring
//! tiles agreed one time in five, as random tiles do, and every world had the
//! same ground. There was nowhere better to live and nowhere worse. It is
//! smooth now, drawn from each world's own dice, with the same mix of ground
//! as before. See ISSUES_FOUND #315.

use crate::world::grid::Grid;
use crate::world::{Position, TerrainType};

fn a_country(seed: u64, across: usize) -> Grid {
    crate::core::dice::seed(seed);
    let mut grid = Grid::new(across, across);
    grid.generate_smooth_terrain();
    grid
}

fn kind(grid: &Grid, x: i32, y: i32) -> TerrainType {
    grid.get_tile(&Position::new(x, y)).unwrap().terrain.terrain_type
}

/// Neighbouring ground is mostly the same ground.
#[test]
fn the_country_is_smooth() {
    let grid = a_country(1, 300);
    let (mut same, mut pairs) = (0, 0);
    for x in 0..299 {
        for y in 0..300 {
            pairs += 1;
            if kind(&grid, x, y) == kind(&grid, x + 1, y) {
                same += 1;
            }
        }
    }
    let share = same as f32 / pairs as f32;
    assert!(share > 0.85, "neighbours agree {share:.2} of the time; random ground agrees about 0.2");
}

/// Each world has its own country, and the same seed makes the same one.
#[test]
fn every_world_has_its_own_country() {
    let differ = |a: &Grid, b: &Grid| {
        (0..200).flat_map(|x| (0..200).map(move |y| (x, y))).filter(|&(x, y)| kind(a, x, y) != kind(b, x, y)).count()
    };
    let (one, again, other) = (a_country(3, 200), a_country(3, 200), a_country(4, 200));
    assert_eq!(differ(&one, &again), 0, "the same seed makes the same country");
    assert!(differ(&one, &other) > 200 * 200 / 4, "a different seed makes a different country");
}

/// About the mix of ground the old generator gave, which everything that
/// grows was set against: two fifths mountain, a tenth hills, a tenth or so
/// of water.
#[test]
fn the_mix_of_ground_is_what_it_was() {
    let grid = a_country(5, 400);
    let share = |wanted: &[TerrainType]| {
        let n = (0..400).flat_map(|x| (0..400).map(move |y| (x, y))).filter(|&(x, y)| wanted.contains(&kind(&grid, x, y))).count();
        n as f32 / (400.0 * 400.0)
    };
    assert!((share(&[TerrainType::Mountain]) - 0.40).abs() < 0.02);
    assert!((share(&[TerrainType::Hills]) - 0.10).abs() < 0.02);
    let wet = share(&[TerrainType::Water, TerrainType::Sea]);
    assert!((0.06..0.14).contains(&wet), "water and sea {wet:.3}");
    let tillable = share(&[TerrainType::Plains, TerrainType::Meadow]);
    assert!((0.12..0.30).contains(&tillable), "ploughland {tillable:.3}");
}

/// And there is good country and bad: some stretches are mostly ploughland
/// and some have none, where before every stretch had a fifth.
#[test]
fn there_is_good_country_and_bad() {
    let grid = a_country(6, 400);
    let mut shares: Vec<f32> = Vec::new();
    for bx in (0..400).step_by(50) {
        for by in (0..400).step_by(50) {
            let n = (bx..bx + 50)
                .flat_map(|x| (by..by + 50).map(move |y| (x, y)))
                .filter(|&(x, y)| grid.get_tile(&Position::new(x, y)).unwrap().terrain.can_be_tilled())
                .count();
            shares.push(n as f32 / 2500.0);
        }
    }
    shares.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(shares[0] < 0.02, "the worst stretch is {:.2} ploughland", shares[0]);
    assert!(shares[shares.len() - 1] > 0.5, "the best stretch is {:.2} ploughland", shares[shares.len() - 1]);
}

/// Most springs are on the shore of a lake or river, where the water is.
#[test]
fn springs_are_where_the_water_is() {
    use crate::world::{ResourceType, World, WorldConfig};

    crate::core::dice::seed(7);
    let mut config = WorldConfig::default().with_smooth_terrain();
    config.size = (200, 200);
    let world = World::new(config);
    let springs: Vec<_> = world
        .resources
        .iter()
        .filter(|node| node.resource_type == ResourceType::Water)
        .filter(|node| world.grid.get_tile(&node.position).is_some_and(|t| t.terrain.terrain_type != TerrainType::Sea))
        .collect();
    let on_a_shore = springs
        .iter()
        .filter(|node| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                world
                    .grid
                    .get_tile(&Position::new(node.position.x + dx, node.position.y + dy))
                    .is_some_and(|t| t.terrain.terrain_type == TerrainType::Water)
            })
        })
        .count();
    assert!(
        on_a_shore * 2 > springs.len(),
        "{on_a_shore} of {} springs are on a shore",
        springs.len()
    );
}

/// A people arriving stops on open country nearest the middle, not on the
/// mountain that happens to be there, with woods and fresh water in reach.
#[test]
fn a_people_arrives_on_open_country() {
    use crate::agents::{AgentConfig, Population};
    use crate::analytics::Simulation;
    use crate::world::{World, WorldConfig};

    for seed in 0..3 {
        crate::core::dice::seed(seed);
        let mut config = WorldConfig::default().with_smooth_terrain();
        config.size = (200, 200);
        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        let mut simulation = Simulation::new(World::new(config), population);
        let at = simulation.bring_everybody_to_the_middle();
        let tile = simulation.world.grid.get_tile(&Position::new(at.0, at.1)).unwrap();
        assert!(tile.terrain.can_be_tilled(), "seed {seed}: arrived on {:?}", tile.terrain.terrain_type);
        let reach = Simulation::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS;
        let woods = (-reach..=reach).any(|dx| (-reach..=reach).any(|dy| {
            simulation.world.grid.get_tile(&Position::new(at.0 + dx, at.1 + dy)).is_some_and(|t| t.terrain.terrain_type == TerrainType::Forest)
        }));
        assert!(woods, "seed {seed}: no woods within a sleeper's walk of where they arrived");
        let water = (-reach..=reach).any(|dx| (-reach..=reach).any(|dy| {
            simulation.world.grid.get_tile(&Position::new(at.0 + dx, at.1 + dy)).is_some_and(|t| t.terrain.terrain_type == TerrainType::Water)
        }));
        assert!(water, "seed {seed}: no fresh water within reach of where they arrived");
    }
}

/// A river is not a wall: walking from one side of the country to the other,
/// the dry ground reached from the middle holds nearly all the dry ground
/// there is. Unbroken rivers cut the country into pieces nobody could cross.
#[test]
fn a_river_can_be_crossed() {
    for seed in 0..3 {
        let grid = a_country(seed, 300);
        let dry = |x: i32, y: i32| {
            (0..300).contains(&x)
                && (0..300).contains(&y)
                && !matches!(kind(&grid, x, y), TerrainType::Water | TerrainType::Sea)
        };
        let total = (0..300).flat_map(|x| (0..300).map(move |y| (x, y))).filter(|&(x, y)| dry(x, y)).count();
        // The largest piece of dry ground, by flood fill from every unvisited
        // dry tile
        let mut seen = vec![false; 300 * 300];
        let mut largest = 0;
        for start in 0..300 * 300 {
            let (sx, sy) = ((start % 300) as i32, (start / 300) as i32);
            if seen[start] || !dry(sx, sy) {
                continue;
            }
            let (mut size, mut stack) = (0, vec![(sx, sy)]);
            seen[start] = true;
            while let Some((x, y)) = stack.pop() {
                size += 1;
                // Four ways, as a route is searched (`the_way_there`)
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if dry(nx, ny) && !seen[(ny * 300 + nx) as usize] {
                        seen[(ny * 300 + nx) as usize] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            largest = largest.max(size);
        }
        let share = largest as f32 / total as f32;
        assert!(share > 0.9, "seed {seed}: the largest piece of dry ground is {share:.2} of it");
    }
}

/// Most ploughland has woods within a sleeper's walk: trees to get under in
/// foul weather and wood to burn. Woods grown in a band up the slopes left
/// plains kilometres from a tree, and a people camped on one died of the
/// weather in its first winter (#315).
#[test]
fn most_ploughland_has_woods_in_reach() {
    let grid = a_country(8, 300);
    let reach = 20;
    let (mut ploughland, mut near_woods) = (0, 0);
    for x in (0..300).step_by(3) {
        for y in (0..300).step_by(3) {
            if !grid.get_tile(&Position::new(x, y)).unwrap().terrain.can_be_tilled() {
                continue;
            }
            ploughland += 1;
            let woods = (-reach..=reach).any(|dx| (-reach..=reach).any(|dy| {
                grid.get_tile(&Position::new(x + dx, y + dy)).is_some_and(|t| t.terrain.terrain_type == TerrainType::Forest)
            }));
            if woods {
                near_woods += 1;
            }
        }
    }
    let share = near_woods as f32 / ploughland as f32;
    assert!(share > 0.5, "only {share:.2} of ploughland has woods within {reach} cells");
}
