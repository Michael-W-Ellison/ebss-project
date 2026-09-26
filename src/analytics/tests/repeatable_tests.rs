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
//! by the fiftieth turn it had reached the people through the Safety drive of
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
fn a_world_from(seed: u64, turns: usize) -> (u64, u64) {
    crate::core::dice::seed(seed);
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for _ in 0..turns {
        simulation.take_a_turn();
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

/// And how many times a fixed world rolls is a *recorded* number, not merely
/// one that agrees with itself.
///
/// `the_same_seed_is_the_same_world` runs both worlds in one process on one
/// thread, so it cannot see anything that varies *between* runs - and that is
/// exactly the kind of drift that has been hardest to pin down here. A
/// settlement test that asserts "at least one of thirty-two survives" fails
/// intermittently, takes half an hour to do it, and says nothing at all about
/// why; this says it in a second, and says which fault it is. A count that has
/// moved means something took a branch on an input the seed does not fix.
///
/// If the model is deliberately changed, this number changes with it, and the
/// new one goes here. That is the point: it is a fact about the model, so it
/// should have to be restated when the model is restated.
#[test]
fn a_fixed_world_rolls_a_recorded_number_of_times() {
    // 8,887 until the curiosity terminal was wired into the drive ladder and
    // the candidate list stopped offering verbs whose action names a product
    // rather than a target - see `wanting::afforded::what_i_could_try_here`.
    // Both change which branch a turn takes, and so how many times it rolls.
    //
    // Then 8,717 until the clock was split into ticks and turns, and the
    // constants derived from `TICKS_PER_DAY` were put back on the one they
    // are counted in. The dread horizon is the one that shows here: every
    // agent read itself as half a day from dying, so every agent took the
    // frightened branch, and the minute-by-minute danger cadence rolled for
    // each of them. Fewer rolls now because fewer people are terrified.
    //
    // Then 7,894 until how fast a thing goes off became the product of its
    // tags and what it is kept in. What a pack holds now keeps at a rate that
    // depends on what is in the pack, and what is buried keeps at a rate
    // rather than by having its own clock wound forward - so what a person
    // finds worth eating, worth burying and worth carrying is a different
    // set, and a different set of branches gets taken.
    // And down 3.9% for the clock audit - the five places a tick stood where a
    // pass belonged, ISSUES_FOUND #218. The *short* count moving at all is the
    // interesting part and is what tells this apart from #217, which moved
    // only the year: a hundred and twenty turns is two and a half days, and
    // two and a half days is long enough for a spell of weather to end now
    // that a ten-hour front lasts ten hours instead of twelve days. A world
    // whose weather turns over draws for its weather.
    // And down 4.2% again for the placement and firewood work of #220-#222.
    // The short count moves because what an agent keeps in its pack changed:
    // it holds ten wood now rather than six, so it banks less, carries more,
    // and decides differently about both.
    // And down 4.7% for the fire chain of #224. Both counts move and the long
    // one moves further, which is the shape to expect from a change that adds
    // a thing to the world rather than only a branch to a turn: over two and a
    // half days a handful of turns go on lighting a fire and cooking at it
    // instead of on drying and pottering, while over a year the fires
    // themselves burn, consume and go out.
    // And up 2.0% for the wander that had a direction in it (#225). The short
    // count rises and the long one falls, which is the shape of a herd that
    // stays on the map: more animals alive to take turns over two and a half
    // days, and over a year the beasts that used to stand off the edge where
    // nothing grows are standing on ground that feeds them, so fewer of them
    // are drawing for a hungry beast's search.
    // And down 2.7% for the sea (#227), where the year is up 6.0%. A person
    // who cannot be walked into the water takes a different step on the few
    // turns of two and a half days that a coast comes into, and over a year
    // lives long enough to take a great many more of them.
    // And **up 37%** for the soil ladder (#246). The largest move this count
    // has taken, and in two and a half days, because it changes the world
    // before anybody moves in it: what a patch opens carrying was read off a
    // nutrient pool and is read off a grade now, what every wild plant grows
    // on was a pool its neighbours drew down and is its terrain's grade, and
    // no daily pass rots litter across the map. A different country from
    // turn nought is a different set of draws from turn nought.
    // And down a quarter for #252. Two and a half days is long enough to
    // see it: a man who stepped round a half-built burrow now walks over it,
    // somebody hungry eats before he huddles, and a pack of rot is emptied to
    // make room for supper - each a different turn from the first morning.
    // And up 3% for #254: a walk that will not turn straight back onto the
    // tile it just left, and a store filled to what the breeding gate asks.
    const WHAT_SEED_4242_ROLLS_IN_120_TURNS: u64 = 7_244;

    let (_, rolled) = a_world_from(4_242, LONG_ENOUGH_TO_TELL);

    assert_eq!(
        rolled, WHAT_SEED_4242_ROLLS_IN_120_TURNS,
        "seed 4242 rolled {rolled} times where it has always rolled {WHAT_SEED_4242_ROLLS_IN_120_TURNS}. \
         Either the model was changed on purpose - in which case put {rolled} in \
         the constant - or something is deciding a branch on an input the seed \
         does not fix, which is what this is here to catch."
    );
}

/// And the same, out to the length a settlement test actually runs.
///
/// Kept separate from the short one because it costs about a minute. The
/// short one catches anything that goes wrong early; a whole year is where
/// the seasons turn, the herds move and a settlement dies, and a drift that
/// only shows up out there would otherwise be found by a half-hour test that
/// says "not one settlement of thirty-two came out".
#[test]
fn a_fixed_world_rolls_a_recorded_number_of_times_over_a_whole_year() {
    use crate::environment::seasons::{DAYS_PER_YEAR, TICKS_PER_DAY};
    // 732,915, then 793,014 for the curiosity and candidate-list changes, and
    // now this for the lifecycle work: a child under six takes no turn of its
    // own and is put where its keeper is, which moves both how many turns a
    // year contains and where the people in it are standing.
    // And down again with the clock fixes - see the note on seed 4242. This
    // figure also stopped meaning what it said: the run above it asked for a
    // year and took `DAYS_PER_YEAR * TICKS_PER_DAY` steps, which is thirty
    // years. Counted in planning periods, a year is a year again.
    //
    // Down eleven per cent again for the decay conversion. A year is where
    // that one shows: food in a pack, food in a hole and food lying in the
    // weather all go off at rates that are now read off the same three
    // modifiers, and over a whole year the difference is a settlement holding
    // a different amount of different things and deciding differently about
    // all of it.
    //
    // And up a third from there when the fishery went back to holding a
    // season's run rather than a rate per pass. That is the largest single
    // move any of these has made, and it is the one to be least surprised by:
    // a full spring used to bring ninety fish into a reach that holds sixty,
    // so a river was never empty, and it is 28.8 now. A year is exactly where
    // that tells - the short run above did not move at all - because what it
    // changes is whether standing in the water goes on being the answer after
    // the run is past. It is not, now, and a people who cannot fish in July
    // do something else in July.
    //
    // And down 23.8% from there when the grazing intake and the plant growth
    // rates were put back onto the clock they are counted against - see
    // ISSUES_FOUND #217. The short count above did not move at all, which is
    // the expected shape: nothing about a person's half hour changed, and
    // everything about what the country will feed did. A world with four
    // hundred head of stock on it instead of seventeen hundred has fewer
    // animals taking turns, fewer of them being born and dying, and a
    // different amount of forage standing where the people are walking.
    //
    // The short count and the long one moving separately is the useful part.
    // A change that moves both is in the decision loop; one that moves only
    // the year is in the world.
    // And down 8.2% again for the clock audit (#218). Less than the ecology
    // fix cost and in the same direction, which is the shape to expect: a
    // world with a fifth of the animals on it and wild food coming back at
    // the rate it was actually measured at has fewer things happening in it
    // to draw for.
    // And a tenth of a per cent for #220-#222, which is the shape to expect
    // from a change that alters what one pass does rather than what the world
    // is: the short count moved thirty times further than the long one.
    // And down 8.7% for the hydration cost of dried food (#223), with the
    // short count above **not moving at all** - which is exactly the shape to
    // expect. A hundred and twenty turns is two and a half days and nobody
    // lives on dried meat for two and a half days; a year is long enough for
    // what a winter store does to a body's water to tell.
    //
    // And **up 18.7%** for the fire chain of #224 - the first rise either of
    // these counts has recorded, and the only one so far that is not a
    // correction. Every previous move was a rate being put back on its proper
    // clock, and every one of those took things *out* of the world. This puts
    // something in: fires are lit now, and a fire is a thing that goes on
    // happening. It burns through its fuel, it heats what is left at it, it
    // draws for how the cooking comes out, and it goes out - a year of that
    // is a year of rolls that were never made before, on top of the turns the
    // people themselves spend lighting and cooking rather than drying.
    // And down 3.6% for #225 - see the short count above, which moved the
    // other way.
    // And **up 6.0%** for the sea (#227). A settlement that is not standing
    // in salt water keeps three more people alive through the summer and
    // twice as much in its pits, and a living settlement rolls.
    // And down 2.6% for the larder work of #228, with the short count above
    // **not moving at all** - which is the shape to expect. None of it can
    // fire inside two and a half days: a body does not get a quarter of the
    // way through a three-week reserve in that time, and the pits are not
    // empty enough for the second one to matter. Over a year, a man who gets
    // what he came for out of the first hole stops walking to the next.
    // And **up 11.8%** for the giving of #230, the largest single move either
    // count has taken. The short count again does not move: a gift that lands
    // rather than being refused changes what a settlement *is* over a year -
    // nine children born across six seeded years against two - and changes
    // almost nothing about a particular afternoon.
    // And down a *hundredth* of a per cent - fifty draws in six hundred and
    // eighty-nine thousand - for the larder rung of #231. A man who walks to
    // the pit instead of casting about does not roll for where to cast.
    // And down 0.9% for #232, where the larder offers the biggest stack
    // rather than the first one: a trip that brings back a load is a trip
    // that is not made again tomorrow.
    // And down 0.45% for #238, where the crop is handed back to the bush
    // once rather than twice. The short count above does not move, which is
    // the shape to expect: nobody has a pack full enough to be refused inside
    // two and a half days. Over a year the bushes and the quarries stop
    // growing back what was taken off them, so there is less standing in the
    // country to walk to and roll over.
    // And **up 4.6%** for #243, where a percept that reports the ground
    // underfoot no longer answers with a walk to it. The short count above
    // does not move: over two and a half days nobody has lived long enough
    // for the difference to compound. Over a year it is the largest rise
    // since the giving of #230, and for the same reason - a settlement that
    // spends a turn in six on something rather than on standing still is a
    // settlement with more people in it at the end, and a living settlement
    // rolls. Gathering is up 3.9% across twelve seeds and person-turns are
    // flat, so what the turn bought was work rather than survival.
    // And **down 16%** for the soil ladder (#246), which moved the short
    // count above the other way. Over a year the settlement farms: fields
    // yield four times the wild, wear a rung a crop, and are ploughed in and
    // sown again - a different year's work, and a year in which the country
    // round it no longer changes under its plants.
    // And down 1.6% for #247, where a field crop is picked ripe or not at
    // all and a harvest is three quarters of it. The short count does not
    // move: no field ripens inside two and a half days. Over a year a farmer
    // waits for the crop instead of picking at it green, and walks to fewer
    // fields that have nothing on them he may take.
    // And up 6.6% for #250, where nobody sets out for anything they cannot
    // see, remember, smell or reach. The short count does not move: nothing
    // in the first two and a half days is chosen from past what a man can
    // see. Over a year the search for the best food anywhere is what moves,
    // because it read the whole map and now reads a man's memory.
    // And up 1.2% for #252 and #253: people get into the stores through the
    // winter and out of the rot in their packs, fewer die of a strange plant,
    // and a pregnancy lasts nine months rather than a night.
    // And up 1.9% for #254: the store is filled for a child as well, from
    // summer; walks leave pockets; small children are fed all through and
    // live; and an illness costs a week what it says it does.
    // And down 13.7% for #255: the store opens to a wasting body and to a
    // parent whose child is short, the winter presses from two months out,
    // a parent holding a child is not walked to their own feet, and wasting
    // hurts by depth. Not traced to any one of those; the short count does
    // not move.
    // And up 22.7% for #256 to #259: a small child with one parent and handed
    // between them, a parent eating for it, poison plants passed on in talk,
    // the walk priced into what is learned. Not traced to any one of those.
    const WHAT_SEED_0_ROLLS_IN_A_YEAR: u64 = 683_166;

    let a_year = crate::environment::seasons::PLANNING_PERIODS_PER_YEAR as usize;
    let (_, rolled) = a_world_from(0, a_year);

    assert_eq!(
        rolled, WHAT_SEED_0_ROLLS_IN_A_YEAR,
        "seed 0 rolled {rolled} times over a year where it has always rolled \
         {WHAT_SEED_0_ROLLS_IN_A_YEAR}"
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
/// turns, and a new one in a rarely-taken branch would sit undetected until it
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
