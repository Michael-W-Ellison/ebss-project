// src/analytics/tests/off_the_map_tests.rs
//! Nobody is put where a body cannot stand.
//!
//! `Move: No passable route toward destination` was 63,922 refusals over
//! eight seeded world-years - **half of every refusal left in the model** -
//! and it is not a pathfinding failure. By the time it fires the direct step,
//! a breadth-first search of four thousand tiles and all four neighbours have
//! been tried. Every one of those 63,922 reported the same thing: **standing
//! off the map, with 0 ways out.** `is_passable_tile` refuses everything
//! outside the grid, so an agent one pace past the edge has no neighbour it
//! can step to, in any direction, for the rest of its life. It cannot walk to
//! food or to water. It starves where it lies.
//!
//! Two places put them there, and neither asked where the edge was: a newborn
//! was placed at its mother's position plus a random pace in each axis, and
//! `Explore` stepped in a direction without checking anything at all.

use crate::agents::{AgentConfig, Population};
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// A world and one person standing in its very first corner.
fn somebody_in_the_corner() -> crate::analytics::Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    simulation.population.agents[0].state.position = (0, 0, 0);
    crate::analytics::tests::plain_ground_about(&mut simulation, (0, 0), 8);
    simulation
}

/// Exploring off the edge of the world leaves you where you were.
#[test]
fn nobody_explores_off_the_edge_of_the_world() {
    let mut simulation = somebody_in_the_corner();

    let result = simulation.execute_action(&Action::Explore { direction: (-1, -1, 0) }, 0);

    assert!(
        result.success,
        "looking about is still a turn spent: {:?}",
        result.message
    );
    assert_eq!(
        simulation.population.agents[0].state.position,
        (0, 0, 0),
        "he walked off the map and can never walk back"
    );
}

/// And having stayed on it, he can still walk somewhere.
#[test]
fn somebody_in_the_corner_can_still_get_out_of_it() {
    let mut simulation = somebody_in_the_corner();
    simulation.execute_action(&Action::Explore { direction: (-1, -1, 0) }, 0);

    let result = simulation.execute_action(&Action::Move { target: (5, 5, 0) }, 0);

    assert!(
        result.success,
        "boxed in after one look about: {:?}",
        result.message
    );
}

/// A child is born where its mother is standing, and not a pace to the left.
///
/// A mother on the first column put one child in three at x = -1.
#[test]
fn a_child_is_born_where_its_mother_stands() {
    use crate::agents::Agent;

    let mut mother = Agent::new(AgentConfig::default());
    let mut father = Agent::new(AgentConfig::default());
    mother.state.position = (0, 0, 0);
    father.state.position = (0, 0, 0);

    for _ in 0..32 {
        let child = crate::agents::reproduction::reproduce(&mother, &father, 0);
        assert_eq!(
            child.state.position,
            (0, 0, 0),
            "a child was put off the map, where it can never take a step"
        );
    }

    // And it is the mother's ground, not the father's. `reproduce` takes the
    // pregnant one's position where there is one and the first parent's
    // otherwise, so this is the first parent standing somewhere else.
    mother.state.position = (7, 9, 0);
    let child = crate::agents::reproduction::reproduce(&mother, &father, 0);
    assert_eq!(child.state.position, (7, 9, 0));
}

/// A target one pace past the edge does not walk anybody off it.
///
/// This is the one that was actually happening. The breadth-first search
/// exempts the goal tile from the passability check, because a goal may be a
/// barn door or a berry bush rather than open ground - and nothing under that
/// exemption asked whether the goal was on the map at all. A decision naming
/// (50, 10) on a fifty-wide grid walked the agent onto it, and the next turn
/// named (51, 10), and so out into nowhere.
#[test]
fn a_target_past_the_edge_does_not_walk_anybody_off_it() {
    let mut simulation = somebody_in_the_corner();
    let wide = simulation.world.grid.width as i32;
    simulation.population.agents[0].state.position = (wide - 1, 10, 0);

    for _ in 0..8 {
        let here = simulation.population.agents[0].state.position;
        simulation.execute_action(&Action::Move { target: (here.0 + 1, here.1, here.2) }, 0);

        let (x, y, _) = simulation.population.agents[0].state.position;
        assert!(
            x >= 0 && y >= 0 && x < wide && y < simulation.world.grid.height as i32,
            "walked off the map to {:?}, where there is no ground to stand on",
            simulation.population.agents[0].state.position
        );
    }
}

/// And the search itself never offers a step outside the world.
#[test]
fn the_search_never_offers_a_step_off_the_map() {
    let simulation = somebody_in_the_corner();
    let wide = simulation.world.grid.width as i32;

    let step = simulation.next_step_toward((wide - 1, 10, 0), (wide, 10, 0));

    assert!(
        step.is_none_or(|(x, y, _)| x >= 0 && y >= 0 && x < wide),
        "the search offered {step:?}, which is off the map"
    );
}

/// And nobody is walked into the sea.
///
/// The same fault one terrain along, and it took a year of a settlement's
/// life. `Terrain::is_walkable` has said `Water | Sea => false` since the sea
/// was split off from fresh water; `Simulation::is_passable_tile` named
/// `TerrainType::Water` and nothing else, so the pathfinder walked people
/// into salt water and left them living there - three of twelve at once,
/// one of them on the same sea tile from day 120 to day 190. A man who
/// reaches a one-cell pocket of sea ringed by fresh water has no neighbour
/// the pathfinder will accept in any direction, for ever. See #227.
///
/// Asked of every terrain rather than of the sea, because the fault is two
/// answers to one question and naming the second one would be a third.
#[test]
fn what_a_body_can_stand_on_has_one_answer() {
    use crate::world::{Position, TerrainType};

    let mut simulation = somebody_in_the_corner();
    let (wide, high) = (
        simulation.world.grid.width as i32,
        simulation.world.grid.height as i32,
    );

    // Somewhere to put each terrain in turn, well inside the map so that the
    // edge is not what answers.
    let bench = Position::new(wide / 2, high / 2);

    for terrain in [
        TerrainType::Plains,
        TerrainType::Forest,
        TerrainType::Mountain,
        TerrainType::Water,
        TerrainType::Desert,
        TerrainType::Wetland,
        TerrainType::Meadow,
        TerrainType::Hills,
        TerrainType::Beach,
        TerrainType::Riverbank,
        TerrainType::Sea,
        TerrainType::SaltMarsh,
        TerrainType::SaltFlat,
        TerrainType::Farmland,
    ] {
        simulation
            .world
            .grid
            .get_tile_mut(&bench)
            .expect("the middle of the map is on the map")
            .terrain
            .terrain_type = terrain;

        let walkable = simulation
            .world
            .grid
            .get_tile(&bench)
            .expect("still there")
            .terrain
            .is_walkable();

        assert_eq!(
            simulation.is_passable_tile(bench.x, bench.y),
            walkable,
            "{terrain:?} is walkable={walkable} to the terrain and the other \
             way to the pathfinder"
        );
    }
}

/// A walk to the water ends on the bank, and nobody is left on a rock.
///
/// The route searches let the last pace of a walk onto its goal whatever the
/// goal is, and a fish run or the sea someone wants to drink from stands on
/// water. So a walk to one left somebody standing in the river, and the next
/// step off it could be onto a spit of land with water all round, which
/// nobody can ever step off. Traced on one world: onto water at (25, 40),
/// onto desert at (25, 39), and there for the rest of the year (#295).
///
/// The same shape here: a pool with a one-cell rock in the middle of it, and
/// somebody sent to the water at its edge. They stop on the bank, and set foot
/// on neither the water nor the rock.
#[test]
fn a_walk_to_the_water_ends_on_the_bank() {
    use crate::world::{Position, TerrainType};

    let mut simulation = somebody_in_the_corner();
    let (cx, cy) = (
        simulation.world.grid.width as i32 / 2,
        simulation.world.grid.height as i32 / 2,
    );
    for x in cx - 15..=cx + 15 {
        for y in cy - 15..=cy + 15 {
            let ground = if (x - cx).abs() <= 2 && (y - cy).abs() <= 2 {
                if (x, y) == (cx, cy) {
                    TerrainType::Desert
                } else {
                    TerrainType::Water
                }
            } else {
                TerrainType::Plains
            };
            simulation
                .world
                .grid
                .get_tile_mut(&Position::new(x, y))
                .expect("well inside the map")
                .terrain
                .terrain_type = ground;
        }
    }
    simulation.population.agents[0].state.position = (cx - 10, cy, 0);
    let the_water = (cx - 2, cy, 0);

    for _ in 0..6 {
        simulation.execute_action(&Action::Move { target: the_water }, 0);

        let (x, y, _) = simulation.population.agents[0].state.position;
        assert!(
            simulation.is_passable_tile(x, y),
            "walked into the water, to ({x}, {y})"
        );
        assert_ne!((x, y), (cx, cy), "walked across the water onto the rock");
    }

    let (x, y, _) = simulation.population.agents[0].state.position;
    assert!(
        (x - the_water.0).abs() <= 1 && (y - the_water.1).abs() <= 1
            && [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]
                .iter()
                .any(|(dx, dy)| {
                    simulation
                        .world
                        .grid
                        .get_tile(&Position::new(x + dx, y + dy))
                        .is_some_and(|tile| tile.terrain.terrain_type == TerrainType::Water)
                }),
        "ended at ({x}, {y}), which is not on the bank of the pool"
    );
}

/// A bolt may clear the water; it may not land on a rock in it.
///
/// Somebody frightened runs over whatever is in the way, and a river between
/// them and what is after them is worth having. But a landing with water all
/// round it is somewhere nobody can walk off, and twice in sixteen worlds a
/// bolt put somebody on a rock in the sea at the corner of the map for the
/// rest of the year (#295). Here the only ground within a bolt is such a rock,
/// so there is nowhere to run.
#[test]
fn nobody_bolts_onto_a_rock_in_the_water() {
    use crate::world::{Position, TerrainType};

    let mut simulation = somebody_in_the_corner();
    let (cx, cy) = (
        simulation.world.grid.width as i32 / 2,
        simulation.world.grid.height as i32 / 2,
    );
    let bolt = crate::analytics::Simulation::HOW_FAR_A_FRIGHTENED_PERSON_GETS;
    let rock = (cx + bolt, cy);
    let reach = bolt + 2;
    for x in cx - reach..=cx + reach {
        for y in cy - reach..=cy + reach {
            let ground = if (x, y) == (cx, cy) || (x, y) == rock {
                TerrainType::Mountain
            } else {
                TerrainType::Water
            };
            if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = ground;
            }
        }
    }

    let remembers = simulation.population.agents[0].exploration_knowledge.clone();
    let landed = simulation.where_this_one_would_run(&remembers, (cx, cy, 0), (cx - 1, cy));

    assert_ne!(
        landed.map(|(x, y, _)| (x, y)),
        Some(rock),
        "bolted onto the rock, and nobody can walk off it"
    );
}

/// Ground with water across it: a river from one edge of the map to the
/// other, or a pond three cells long, a few paces east of somebody frightened
/// from the west.
fn somebody_by_the_water(the_water_runs_the_whole_way: bool) -> (crate::analytics::Simulation, (i32, i32)) {
    use crate::world::{Position, TerrainType};

    let mut simulation = somebody_in_the_corner();
    let (w, h) = (simulation.world.grid.width as i32, simulation.world.grid.height as i32);
    let (cx, cy) = (w / 2, h / 2);
    for x in 0..w {
        for y in 0..h {
            let in_the_water = (cx + 3..=cx + 5).contains(&x)
                && (the_water_runs_the_whole_way || (cy - 1..=cy + 1).contains(&y));
            simulation
                .world
                .grid
                .get_tile_mut(&Position::new(x, y))
                .expect("on the map")
                .terrain
                .terrain_type = if in_the_water { TerrainType::Water } else { TerrainType::Plains };
        }
    }
    simulation.world.the_lie_of_the_land_changed();
    (simulation, (cx, cy))
}

/// A bolt does not clear a river it could not walk back over.
///
/// On smooth country a river runs for kilometres, and some close round an
/// island with no ford. People who bolted across one were on the far bank
/// for good, and two who could not get to the pits on the other side starved
/// walking at the water (#317).
#[test]
fn a_bolt_does_not_clear_a_river_with_no_way_back() {
    let (simulation, (cx, cy)) = somebody_by_the_water(true);
    let remembers = simulation.population.agents[0].exploration_knowledge.clone();
    let landed = simulation
        .where_this_one_would_run(&remembers, (cx, cy, 0), (cx - 1, cy))
        .expect("there is ground to run to on this bank");

    assert!(landed.0 < cx + 3, "bolted across the river to {landed:?}");
}

/// But a pond with a way round it is still worth putting between you and
/// whatever is after you.
#[test]
fn a_bolt_still_clears_a_pond() {
    let (simulation, (cx, cy)) = somebody_by_the_water(false);
    let remembers = simulation.population.agents[0].exploration_knowledge.clone();
    let landed = simulation
        .where_this_one_would_run(&remembers, (cx, cy, 0), (cx - 1, cy))
        .expect("there is ground to run to");

    assert!(landed.0 > cx + 5, "stopped short of the pond at {landed:?}");
}

/// Running from something with a river at your back is running somewhere on
/// this bank.
///
/// Running away was a straight line, and the straight line went over the
/// river: a walk with no way there, refused at its first step, and somebody
/// frightened stood still for it, hundreds of turns a year on some worlds
/// (#323).
#[test]
fn running_away_with_a_river_behind_stays_on_this_bank() {
    let (simulation, (cx, cy)) = somebody_by_the_water(true);
    let agent = &simulation.population.agents[0];
    let straight = crate::analytics::Simulation::put_ground_between((cx, cy, 0), (cx - 1, cy));
    assert!(
        !simulation.can_this_be_got_to((cx, cy, 0), &straight),
        "the straight line away is meant to cross the river: {straight:?}"
    );

    match simulation.away_on_this_side(agent, (cx, cy, 0), (cx - 1, cy), straight) {
        Action::Move { target } => assert!(
            simulation.world.can_walk_between((cx, cy), (target.0, target.1)),
            "ran for the far bank at {target:?}"
        ),
        other => panic!("did not run at all: {other:?}"),
    }
}

/// A walk to somewhere there is no way to is refused, and nobody walks at the
/// water.
///
/// With no route, a walk took single steps towards the place, and against a
/// river they went one way and back again, each step a success, until the
/// turn was spent, and the next turn the same. On one world that was a walk
/// in six (#321).
#[test]
fn a_walk_with_no_way_there_is_refused_rather_than_walked_at() {
    let (mut simulation, (cx, cy)) = somebody_by_the_water(true);
    simulation.population.agents[0].state.position = (cx, cy, 0);
    let over_the_river = (cx + 10, cy, 0);

    let result = simulation.execute_action(&Action::Move { target: over_the_river }, 0);

    assert!(!result.success, "there is no way over: {:?}", result.message);
    assert_eq!(
        simulation.population.agents[0].state.position,
        (cx, cy, 0),
        "and nobody has walked anywhere"
    );
}

/// And what is on the far bank is not somewhere to set out for.
#[test]
fn nobody_sets_out_for_what_is_on_the_far_bank() {
    use crate::world::{Position, ResourceNode, ResourceType};

    let (mut simulation, (cx, cy)) = somebody_by_the_water(true);
    simulation.population.agents[0].state.position = (cx, cy, 0);
    simulation.world.resources.clear();
    let this_side = Position::new(cx - 8, cy);
    let far_side = Position::new(cx + 8, cy);
    simulation.world.resources.push(ResourceNode::new(ResourceType::Water, this_side, 50));
    simulation.world.resources.push(ResourceNode::new(ResourceType::Water, far_side, 50));
    simulation.world.file_the_nodes();

    let agent = &simulation.population.agents[0];
    let known: Vec<Position> = simulation
        .nodes_known_to(agent, Position::new(cx, cy), 25)
        .map(|node| node.position)
        .collect();

    assert!(known.contains(&this_side), "the spring on this bank is known: {known:?}");
    assert!(!known.contains(&far_side), "and the one over the river is no use: {known:?}");
}

/// And an errand to there is over, rather than refused again every turn
/// until it runs out of patience (#321).
#[test]
fn an_errand_to_somewhere_there_is_no_way_to_is_given_up() {
    let (mut simulation, (cx, cy)) = somebody_by_the_water(true);
    simulation.population.agents[0].state.position = (cx, cy, 0);
    let over_the_river = (cx + 10, cy, 0);
    simulation.population.agents[0].errand = Some(crate::agents::Errand {
        going_to: over_the_river,
        set_out_from: (cx, cy, 0),
        to_make: None,
        for_drive: crate::core::DriveType::Curiosity,
        turns_on_it: 0,
        set_aside: 0,
    });

    let _ = simulation.stick_to_the_errand(0, Action::Wait, false);

    assert!(
        simulation.population.agents[0].errand.is_none(),
        "the errand is still held: {:?}",
        simulation.population.agents[0].errand
    );
}


/// Water known of over a river is not where somebody parched heads.
///
/// Smelled, remembered or seen, the nearest water was often on the far bank
/// of a river, and somebody starving or parched was sent to it and refused at
/// the bank every turn: 471 turns in half a year on one world (#323).
#[test]
fn the_water_a_thirsty_one_heads_for_is_on_this_bank() {
    use crate::core::memory::SpatialMemoryType;
    use crate::agents::senses::ScentType;
    use crate::world::{Position, ResourceType};

    let (mut simulation, (cx, cy)) = somebody_by_the_water(true);
    let agent = &mut simulation.population.agents[0];
    agent.state.position = (cx, cy, 0);
    let known = &mut agent.exploration_knowledge.known_resources;
    known.insert(Position::new(cx + 7, cy), ResourceType::Water);
    known.insert(Position::new(cx - 12, cy), ResourceType::Water);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.known_source_position(agent, (cx, cy, 0), ScentType::Water, SpatialMemoryType::Water),
        Some((cx - 12, cy, 0)),
        "the nearer water is over the river"
    );
}

/// Nor does anybody go in out of the weather under trees on the far bank.
///
/// At night a sleeper out in the open goes to the nearest cover, and the
/// nearest was over the river often enough to be refused 1,400 times in a
/// year on one world (#323).
#[test]
fn the_cover_a_sleeper_goes_to_is_on_this_bank() {
    use crate::world::{Position, TerrainType};

    let (mut simulation, (cx, cy)) = somebody_by_the_water(true);
    simulation.world.buildings.clear();
    for (x, y) in [(cx + 7, cy), (cx - 9, cy)] {
        simulation
            .world
            .grid
            .get_tile_mut(&Position::new(x, y))
            .expect("on the map")
            .terrain
            .terrain_type = TerrainType::Forest;
    }
    simulation.world.the_lie_of_the_land_changed();

    assert_eq!(
        simulation.somewhere_out_of_the_weather((cx, cy, 0)),
        Some((cx - 9, cy, 0)),
        "the nearer trees are over the river"
    );
}

/// A walk out into open water is passed over, rather than walked to where
/// you already stand.
///
/// Somewhere in the water with no dry ground beside it is clamped back along
/// the line to the first ground a foot can go on, which for somebody on the
/// bank is the ground under their feet. The walk "succeeded", went nowhere,
/// and was booked as "move: already there", never as a refusal: 2,173 turns
/// in half a year on one world, a third of them somebody running from danger
/// straight at a lake (#323). Asked where it is decided, a place with no
/// ground beside it is somewhere nobody can get to.
#[test]
fn a_walk_out_into_open_water_is_passed_over() {
    let (simulation, (cx, cy)) = somebody_by_the_water(true);
    let on_the_bank = (cx + 2, cy, 0);
    let mid_river = Action::Move { target: (cx + 4, cy, 0) };

    assert_eq!(
        simulation.where_a_walk_can_end(on_the_bank, (cx + 4, cy, 0)),
        on_the_bank,
        "clamped, the walk ends where it began"
    );
    assert!(
        !simulation.can_this_be_got_to(on_the_bank, &mid_river),
        "so it is not somewhere to set out for"
    );
}

/// A walk that would end where it began is not somewhere to set out for.
///
/// Something seen in the water a pace off was answered with a walk to it,
/// the walk ended on the bank, which was where the walker stood, and the
/// turn went on standing there; started as an errand, the next turn went on
/// arriving (#324).
#[test]
fn a_walk_that_would_end_where_it_began_is_passed_over() {
    let (simulation, (cx, cy)) = somebody_by_the_water(true);
    let on_the_bank = (cx + 2, cy, 0);
    let walk_to = |x: i32| Action::Move { target: (x, cy, 0) };

    assert!(!simulation.can_this_be_got_to(on_the_bank, &walk_to(cx + 3)), "the water a pace off");
    assert!(!simulation.can_this_be_got_to(on_the_bank, &walk_to(cx + 2)), "the ground underfoot");
    assert!(simulation.can_this_be_got_to(on_the_bank, &walk_to(cx + 1)), "but a pace back from the bank is a walk");
}

/// Running from somebody on the same ground is running somewhere.
///
/// The way away from an attacker was a straight line from where they stood,
/// and from somebody on the very same tile that was no line at all: a walk
/// to where this one already was, and an errand to it (#324).
#[test]
fn running_from_somebody_on_the_same_ground_goes_somewhere() {
    let (simulation, (cx, cy)) = somebody_by_the_water(true);
    let here = (cx - 20, cy, 0);
    let away = crate::analytics::Simulation::put_ground_between(here, (here.0, here.1));

    assert!(
        simulation.can_this_be_got_to(here, &away),
        "running from somebody standing here goes nowhere: {away:?}"
    );
}

/// A walk is judged where it will end, not where it was aimed.
///
/// Water that touches this bank only at a corner counts as somewhere to get
/// to, since the ground beside it is this side's. But no route steps into it
/// corner-wise, so the walk ends on the ground beside it that is square on,
/// which is the far bank, and is refused there. Somebody running from an
/// attacker straight at such a river was refused 117 times in a year, with
/// open ground to run to all round (#324).
#[test]
fn a_walk_is_judged_where_it_will_end() {
    use crate::world::{Position, TerrainType};

    let (mut simulation, (cx, cy)) = somebody_by_the_water(true);
    // The river two wide, at cx + 3 and cx + 4, with a spit of this bank
    // reaching out to touch the water at (cx + 4, cy) at its corner.
    let high = simulation.world.grid.height as i32;
    let mut make_ground = |x: i32, y: i32| {
        simulation.world.grid.get_tile_mut(&Position::new(x, y)).expect("on the map").terrain.terrain_type =
            TerrainType::Plains;
    };
    for y in 0..high {
        make_ground(cx + 5, y);
    }
    make_ground(cx + 3, cy + 1);
    simulation.world.the_lie_of_the_land_changed();

    let here = (cx, cy, 0);
    let in_the_water = (cx + 4, cy, 0);
    assert!(
        simulation.world.can_walk_between((cx, cy), (in_the_water.0, in_the_water.1)),
        "the water is meant to touch this bank at a corner"
    );
    let end = simulation.where_a_walk_can_end(here, in_the_water);
    assert!(
        !simulation.world.can_walk_between((cx, cy), (end.0, end.1)),
        "the walk is meant to end on the far bank, at {end:?}"
    );

    assert!(!simulation.can_this_be_got_to(here, &Action::Move { target: in_the_water }));
}
