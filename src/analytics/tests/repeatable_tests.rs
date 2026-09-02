//! The same seed is the same world - and the two ways that stops being true.
//!
//! A run of this model was never repeatable, and the cost was paid on every
//! change: a settlement that dies at 1,102 turns in one run and 1,199 in the
//! next cannot tell a regression from a coin, so every measurement had to be
//! a mean over thirty-two worlds and anything worth less than a hundred and
//! twenty turns could not be seen at all.
//!
//! Two separate faults, and both had to go:
//!
//! **Randomness taken outside the stream.** `thread_rng()`, `rand::random()`
//! and `Uuid::new_v4()` all ask the operating system and none of them can be
//! seeded. The last ten of those - every wander an animal takes, and whether
//! it grazes, rests or hunts - moved the beasts differently in every run, and
//! by the fiftieth tick it had reached the people through the Safety drive of
//! anybody who could see one.
//!
//! **Order taken from a `HashMap`.** Rust seeds hash iteration *per process*,
//! so a `max_by` over an unordered table is decided by the process's hash seed
//! whenever two candidates tie. That cannot be caught by a test inside one
//! process - the seed is fixed for its lifetime - so the guard for it is the
//! source-level one below, not the world one.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Long enough for the fauna, the weather and the people to be interacting.
const LONG_ENOUGH_TO_TELL: usize = 120;

/// Everything about a world that anything downstream reads.
fn fingerprint(sim: &Simulation) -> u64 {
    let mut h = DefaultHasher::new();
    for agent in sim.population.agents.iter() {
        agent.id.hash(&mut h);
        agent.state.is_alive.hash(&mut h);
        agent.state.position.hash(&mut h);
        ((agent.state.health * 100.0) as i64).hash(&mut h);
        ((agent.state.physiology.reserve * 100.0) as i64).hash(&mut h);
        ((agent.state.physiology.hydration * 1000.0) as i64).hash(&mut h);
        for (name, item) in agent.inventory.get_all_items() {
            name.hash(&mut h);
            (item.quantity as i64).hash(&mut h);
        }
    }
    for beast in sim.world.animals.get_all().iter() {
        beast.id.hash(&mut h);
        beast.position.hash(&mut h);
        beast.is_alive().hash(&mut h);
    }
    for resource in sim.world.resources.iter() {
        (resource.position.x, resource.position.y, resource.amount).hash(&mut h);
    }
    for pit in sim.world.pits.iter() {
        (pit.how_much_is_in_it() as i64).hash(&mut h);
    }
    h.finish()
}

/// A world's fingerprint, and how many times it rolled to get there.
fn a_world_from(seed: u64, ticks: usize) -> (u64, u64) {
    crate::core::dice::seed(seed);
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for _ in 0..ticks {
        simulation.tick();
    }
    (fingerprint(&simulation), crate::core::dice::draws_taken())
}

/// The same seed twice is the same world down to the last berry.
///
/// This is what catches a roll taken outside `core::dice`: a new
/// `thread_rng()`, `rand::random()` or `Uuid::new_v4()` anywhere in the model
/// makes the second world differ from the first.
#[test]
fn the_same_seed_is_the_same_world() {
    let (once, rolled_once) = a_world_from(4_242, LONG_ENOUGH_TO_TELL);
    let (again, rolled_again) = a_world_from(4_242, LONG_ENOUGH_TO_TELL);

    // The count first, because it says *which* fault this is. A world that
    // rolled a different number of times took a branch the other did not, and
    // the thing that decided the branch is what to go and look at.
    assert_eq!(
        rolled_once, rolled_again,
        "the second run of seed 4242 rolled a different number of times, so \
         something decided a branch on an input the seed does not fix"
    );
    assert_eq!(
        once, again,
        "two runs of seed 4242 came out differently, so something in the model \
         is rolling outside `core::dice` - see the module note above"
    );
}

/// And a different seed is a different world, or seeding would prove nothing.
#[test]
fn a_different_seed_is_a_different_world() {
    assert_ne!(
        a_world_from(4_242, LONG_ENOUGH_TO_TELL).0,
        a_world_from(9_001, LONG_ENOUGH_TO_TELL).0,
    );
}

/// Nothing in the model reaches for randomness the seed cannot reach.
///
/// A source-level guard rather than a behavioural one, because that is the
/// only kind that works here: the world test above catches a stray
/// `thread_rng` only if the code path happens to run in a hundred and twenty
/// ticks, and a new one in a rarely-taken branch would sit undetected until it
/// spoiled somebody's measurement months later.
#[test]
fn every_roll_comes_from_the_one_stream() {
    let banned = ["thread_rng", "rand::random", "Uuid::new_v4"];
    let mut found = Vec::new();

    for (path, text) in every_source_file() {
        if path.ends_with("core/dice.rs") || path.ends_with("repeatable_tests.rs") {
            continue; // where the vocabulary is defined, described and guarded
        }
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") || line.trim_start().starts_with("///") {
                continue;
            }
            for word in banned {
                if line.contains(word) {
                    found.push(format!("{path}:{} {word}", n + 1));
                }
            }
        }
    }

    assert!(
        found.is_empty(),
        "randomness outside `core::dice`, which no seed can reach:\n  {}",
        found.join("\n  ")
    );
}

/// And nothing decides anything by walking an unordered table.
///
/// `HashMap` and `HashSet` iterate in an order Rust seeds per process, so a
/// `max_by` over one is settled by the hash seed whenever two candidates tie -
/// which is how, for a long time, **what an agent was most afraid of was
/// decided by a coin**. The ordered forms cost a little speed and are the only
/// ones this model uses.
#[test]
fn nothing_decides_anything_by_walking_an_unordered_table() {
    let mut found = Vec::new();

    for (path, text) in every_source_file() {
        if path.ends_with("repeatable_tests.rs") {
            continue; // this file names them in order to forbid them
        }
        for (n, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            // The hasher vocabulary is not a collection: `DefaultHasher` is
            // fixed-key and perfectly repeatable.
            if line.contains("hash_map::") || line.contains("DefaultHasher") {
                continue;
            }
            for word in ["HashMap", "HashSet"] {
                if line.contains(word) {
                    found.push(format!("{path}:{} {word}", n + 1));
                }
            }
        }
    }

    assert!(
        found.is_empty(),
        "unordered collections in the model - use BTreeMap/BTreeSet:\n  {}",
        found.join("\n  ")
    );
}

/// Every `.rs` file under `src/`, as (path, contents).
fn every_source_file() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    out.push((path.display().to_string(), text));
                }
            }
        }
    }

    let mut out = Vec::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut out,
    );
    out
}
