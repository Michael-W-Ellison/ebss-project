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
    // Down to 6,909 for #260: somebody setting out on a walk that will
    // outlast the meal in them takes food along first, and that is decided in
    // the first two and a half days.
    // Down to 6,795 for #261 to #263: a pair ready to conceive no longer
    // rolls for it every turn, only on the one day a month it can happen.
    // Up to 7,749 for the walk at five kilometres an hour (#277): a walk
    // covers what a turn of walking covers, and the minutes it leaves over go
    // on another decision.
    // Down to 7,501 for #280 to #282: a hunt closes and rushes in its half
    // hour, a frightened beast bolts once, animals live by the calendar, and
    // a day's grazing walks on across the nearest patches.
    // Down to 7,226 for #283: a mammoth no longer faces down its own herd,
    // and a grazer walks to what it can crop rather than to a browsed tree.
    // Up to 7,490 for #285: a grazer eats until it is full, takes its share
    // as browse or grass by its kind, and leaves a plant its crown; and a
    // hunter takes another hunter only when it comes upon one.
    // Down to 7,283 for #295: a walk to the water ends on the bank it would
    // reach it from, and the first walks to a river or a fish run are in the
    // first two and a half days.
    // Up to 7,303 for #296: a way of doing a thing is learned once by
    // watching rather than once from every neighbour, talk and a glance
    // bring nobody closer than getting on, and nobody hits a neighbour to see
    // what happens.
    // Down to 7,026 for #299: people sleep from bedtime to first light rather
    // than through the day in snatches, so fewer of the 120 turns are taken.
    // Up to 8,422 for #299 again: a night is slept in two-hour stretches, and
    // each waking is a decision. Down to 7,343 when nobody goes to bed
    // thirsty, or hungry with nothing to eat. Up to 7,392 when somebody on
    // foul ground at bedtime steps off it, and a remedy is taken before bed.
    // Up to 9,667 when sleepers go in under a roof or the trees first, the
    // trees being near on this map.
    // Up to 9,842 for #300: what comes off a fire keeps its cut, and can be
    // eaten. Up to 9,894 for #301: nobody walks to the pit underfoot.
    // Down to 9,333 for #302: the wish to sleep comes from tiredness, not the
    // clock, so nobody takes a five-hour sleep in the afternoon unless tired.
    // Down to 7,686 for #318: worlds are made on smooth ground by default.
    // Down to 7,626 for #320: dried and smoked food are carried under their
    // own names, a merged stack weighs what its parts did, and a pack takes
    // nothing that would overload it once merged.
    // Up to 7,639 for #326: everybody keeps their own half hour, begun at
    // their own minute, and takes it a minute at a time only with something
    // after them.
    // Up to 7,743 for #334: lashing from sinew, strips of bark and rawhide,
    // and only a making with something sharp or hot in it can hurt.
    // Up to 7,934 for #335: the hand at each thing made, and a making that
    // holds an unpractised hand longer than a period.
    // Up to 7,954 for #336: nobody notices anything asleep, and the makings
    // of a discovery somebody wonders about are kept in mind.
    // Down to 7,905 for #338: bonds held to what two natures allow, a few
    // people's company a day, pairs, and a want met the way a nature reaches
    // for.
    // Down to 7,852 for #339: talk makes only a familiar face, a good turn
    // is sized against the fade, and a day's company is kept without a roll
    // for it.
    // Down to 7,457 for #345: the technology catalogue decides what is born
    // knowledge, founders carry Stage 0 (a hand axe and flakes), and the
    // knife, basket and spear are found out.
    // Up to 7,903 for #346: hunter-gatherers first - no longhouse waiting,
    // sowing and gardens found out in stages, and a cache no anchor.
    // Up to 8,703 for #347: the shallows and fish traps, a wasting body wants
    // no child, and partners go halfway to each other.
    // Down to 8,397 for #349: brush shelter and bedding, graded cover, a tent
    // that can be finished, and fish traps that take fish.
    // Down to 7,429 for #352: every spell of weather draws its wind, salt is
    // fetched, a tent asks for what it takes, and a vessel is made for a walk.
    // Up to 8,095 for #353: hides rot unless scraped, rawhide is tanned over
    // a fire, skins make cloaks, wraps and foot-wraps, and a spare hide is
    // slept on.
    const WHAT_SEED_4242_ROLLS_IN_120_TURNS: u64 = 8_095;

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
    // And down 10.6% for #260: food taken along on a walk that will outlast
    // the meal in you, and a body that knows when it will next be hungry.
    // And up 18.9% for #261 to #263: the winter pressed on whoever bears,
    // births spaced by nursing, and conception once a month on each one's own
    // day at a chance of about a quarter. Fewer rolls for conceiving, but a
    // settlement no longer all pregnant at once, and more of it alive to take
    // turns. Not traced to any one of those.
    // And down 2.4% for walking at five kilometres an hour, the animals on
    // the same clock, and backing away from what outweighs you (#277).
    // And up 0.2% for #280 to #282: the half-hour hunt, animal lives on the
    // calendar, packs sharing a kill, and a day's grazing as a walk.
    // And down 3.6% for #283: grazers fed and not facing down their own kind.
    // And down 5.8% for #284: browse at what a wood yields, and no grass
    // coming back in winter.
    // And down 4.9% for #285: feeding niches, eating to the full, a crown
    // left on what is grazed, big animals ranging further, and hunters
    // taking other hunters only when they come upon them.
    // And up 12.4% for #286: plants leave seventy seed a life rather than
    // forty, and the herbs hold their ground.
    // And down 2.4% for #294: nobody is sent off the edge of the map, so
    // walks end somewhere and fewer errands are given up and chosen again.
    // And down 6.7% for #295: a walk to the water ends on the bank, and a
    // bolt may clear a river but not land on a rock in it, so nobody is left
    // standing in the water or stranded on a spit of land for the year.
    // And up 4.1% for #296: bonds are earned rather than talked up to 1.0 by
    // everybody with everybody, and a fight is a bout rather than days of
    // blows, so fewer turns go on feuds and more people are about to take
    // them.
    // And up 3.7% for #297: a wound turns and a soaking tells at their
    // chances a day rather than four times them, so more of the settlement
    // lives through its first year.
    // And up 7.3% for #298: the gate for a child asks what a body really
    // gets through and what a winter really took out of the store, and sleep
    // pays back sleep debt hour for hour, so children are conceived and there
    // are more people about to take turns.
    // And up 6.2% for #299: people sleep at night and are home with the
    // others by dark, cook supper on an evening fire, and the night's sleep
    // pays back its whole length rather than its first half hour, so the
    // year's people are rested and take more turns between them. Then up
    // 6.7% more, to 822,314, when the night is slept in two-hour stretches
    // and each waking is a decision; and 833,642 when nobody goes to bed
    // thirsty, or hungry with nothing to eat; and 881,777 when a coupling
    // outside the fertile window dulls the wish for a child by a day's
    // regrowth rather than a week's; and 949,046 when somebody on foul
    // ground at bedtime steps off it, and a remedy is taken before bed.
    // Down to 912,501 when sleepers go in under a roof or the trees first.
    // Down 18.5% to 743,470 for #300: with no knife, a whole fish or joint
    // is pulled apart by hand and eaten, rather than carried about uneaten
    // while its owner walks to the stores and back. Then up to 786,562 when
    // what comes off a fire keeps its cut, and can be eaten. Up 10.5% to
    // 869,212 for #301: a hungry man with a joint and no knife is offered
    // supper out of it, rather than walked about until his reserve runs out.
    // Then 892,632 when nobody walks to the pit underfoot.
    // Up 24.9% to 1,115,135 for #302: the wish to sleep comes from tiredness,
    // not the clock, and the afternoons that went on sleeping are spent awake.
    // Up 4.9% to 1,169,903 for #308: somebody with a beast on their own tile
    // has somewhere to run, and runs, where they used to freeze on the spot.
    // Down 0.5% to 1,164,176 for #309: a bad plant is an illness, and no
    // longer opens a wound that goes bad.
    // Up 14.0% to 1,327,438 for #310: a sleeper is asleep until the night's
    // stretch is over, rather than woken the turn it began, so nobody tires
    // through the night as though they were up, and the wounded mend.
    // Up 5.3% to 1,397,816 for #311: a coupling off the fertile day answers
    // the wish of whoever was asking for the day, and spends nobody else's.
    // Down 1.6% to 1,374,948 for #312: whichever of a pair is on their
    // fertile day carries, so a pair's chance comes twice a cycle.
    // Down 4.2% to 1,317,154 for #316: a head full of berry patches and
    // fishing spots still keeps the nearest few places it knows of each
    // thing its tools are made of.
    // Up 12.1% to 1,476,142 for #318: worlds are made on smooth ground by
    // default.
    // Up 5.8% to 1,561,205 for #319: a worn-through tool is thrown away
    // rather than carried for the rest of the year.
    // Down 3.3% to 1,509,486 for #320: dried and smoked food are carried
    // under their own names, and a merged stack weighs what its parts did.
    // Down 2.8% to 1,466,916 for #321: nobody sets out for somewhere there
    // is no way to, a walk there is refused before a step is taken, and an
    // errand there is given up.
    // Down 6.4% to 1,372,417 for #323: a way, a drive's answer or something
    // just seen that is a walk to the far bank stands aside before it is
    // chosen, and whoever runs from a beast or an attacker across a river
    // runs somewhere on this bank instead.
    // Up 14.6% to 1,572,784, still #323: nor is water or food known of over
    // the river, prey on the far bank, or cover, clean ground, a fire or home
    // there at night.
    // Down 8.3% to 1,441,966 for #324: a walk that would end where it began
    // is passed over, and running from somebody on the same ground runs
    // somewhere.
    // Up 4.1% to 1,500,653 for #325: a beast has its minutes only against
    // somebody answering a beast or a person, a small child takes no minutes
    // of its own, and the night does not send somebody threatened back to bed.
    // Down 31.2% to 1,032,478 for #326: everybody keeps their own half hour,
    // begun at their own minute, and takes it a minute at a time only with
    // something after them, not for hunger or thirst; those minutes come out
    // of the half hour rather than on top of it.
    // Down 0.4% to 1,027,961 for #327: only a hunter big enough to take
    // somebody frightens them, so nobody squares up to a mammoth or an eagle.
    // Down 7.7% to 948,539, still #327: nobody hunts a beast whose whole
    // blow would kill them, the spear they threw being perhaps gone.
    // Up 7.2% to 1,017,291, still #327: asked again where the hunt is made,
    // and of anything that turns on a hunter, a cow as much as a bear.
    // Up 1.1% to 1,028,941 for #329: a better point brings a beast down in
    // fewer throws, and a bigger beast takes more.
    // Down 1.4% to 1,014,941 for #330: a hunt is weighed by what is on the
    // animal against the throws and blows it costs, so nobody throws at a
    // kestrel and a known patch of food beats most hunts.
    // Up 5.0% to 1,065,822 for #331: a kill leaves a carcass that is gone
    // back to, cut a turn at a time and told of.
    // Down 1.2% to 1,052,931 for #332: an edge in everybody's kit - a struck
    // flake, knapped in a spare turn - and the makings of it fetched.
    // Up 7.9% to 1,135,991 for #333: the edge is taken on after the
    // pottering rather than before it.
    // Down 10.4% to 1,018,105 for #334: lashing from sinew, strips of bark
    // and rawhide, and only a making with something sharp or hot in it hurts.
    // Down 3.3% to 984,596 for #335: the hand at each thing made, the wheel
    // behind pottery and metal, curiosity spent after three, and kills
    // fetched for the store.
    // Down 4.6% to 939,765 for #336: ore is not read in a cold stone, nobody
    // notices anything asleep, and metal waits on twenty evenings at a fire
    // with ore in the pack.
    // Up 6.5% to 1,000,911 for #338: bonds held to what two natures allow,
    // company chosen by how well two natures suit, only a pair has a child,
    // one at a time, and curiosity does not try a neighbour's pack.
    // Down 1.1% to 989,726 for #339: talk makes only a familiar face, a good
    // turn is sized against the fade, a day's company is kept without a roll
    // for it, and a friendship left alone fades over ten weeks, not three.
    // Down 1.3% to 976,851 for #341: somebody chilly with the makings on
    // them dresses on the spot.
    // Up 0.4% to 980,424 for #344: the old technology registry and its
    // discovery roll every few days are gone.
    // Down 0.1% to 979,590 for #345: stages 0-2 from the technology
    // catalogue - Stage 0 born, the rest found out behind its gates.
    // Down 0.2% to 977,779 for #346: hunter-gatherers first, agriculture in
    // the document's three stages.
    // Up 6.3% to 1,039,760 for #347: the shallows and fish traps, a wasting
    // body wants no child, and partners go halfway to each other.
    // Down 2.7% to 1,011,830 for #347's trap robbing: somebody else's trap is
    // theft, and a person is a danger only while they are in sight.
    // Up 1.9% to 1,031,179 for #348: food keeping - smoking, brine, caches,
    // racks, stone boiling and pit ovens, and no drying in the rain.
    // Down 4.9% to 980,526 for #349: windbreaks, lean-tos, brush huts put up
    // in one go, bedding over a sleeping place, graded cover, a tent that can
    // be finished, and fish traps that take fish.
    // Up 3.6% to 1,016,134 for #350: brush does not anchor a camp, a burrow
    // tried for its own sake keeps digging_in's rules, and an open hole
    // fills in.
    // Up 1.9% to 1,035,210 for #351: wounds bleed and are pressed on and
    // bound, a body at rest mends faster and takes one turn a half hour, not
    // two, and a water skin is cut and filled before a walk.
    // Down 1.4% to 1,020,308 for #352: wind that blows, salt fetched from
    // where it lies, skins wanted for a tent, a vessel made for a walk.
    // Up 5.3% to 1,074,317 for #353: hides that rot unless worked, rawhide
    // and tanned leather, the garments a skin makes, and hide bedding.
    // Down 0.3% to 1,071,252 for #354: a young one left by a kill follows
    // the hunter, kept beasts are fed, folded, milked and culled, and dung
    // burns.
    // Down 4.3% to 1,025,389 for #355: pots made by the evening fire, stews,
    // pots that crack and break on a move, and the sea boiled in an empty
    // pot.
    // Up 2.5% to 1,051,389 for #356: salt in the body, sweated out and put
    // back, licked when craved, and journeys made for it.
    // Down 3.7% to 1,012,933 for #357: middens and latrines, fouled water
    // and fouled camps moved off, vermin, theft and scavengers at stores, a
    // storehouse that is a place, and no building making anything from
    // nothing.
    // Down 2.2% to 990,339 for #358: hunters look as far as they can see,
    // remember where game was, and go after it when a week without meat
    // has them craving it.
    // Up to 991,004 for #358's correction: plants carry a little salt, going
    // without it costs a quarter what it did, and meat is not craved for the
    // salt.
    // Down 5.0% to 941,071 for #357's correction: vermin take a third of
    // what they did, a campmate's store is not theft, and foul water sickens
    // a quarter as often.
    // Up 0.5% to 945,540 for #358's husbandry correction: a young one taken
    // from a kill is tended without fodder when there is none to give it.
    const WHAT_SEED_0_ROLLS_IN_A_YEAR: u64 = 945_540;

    let a_year = crate::environment::seasons::PLANNING_PERIODS_PER_YEAR as usize;
    let (_, rolled) = a_world_from(0, a_year);

    assert_eq!(
        rolled, WHAT_SEED_0_ROLLS_IN_A_YEAR,
        "seed 0 rolled {rolled} times over a year where it has always rolled \
         {WHAT_SEED_0_ROLLS_IN_A_YEAR}"
    );
}

/// How long each process runs in `the_same_seed_is_the_same_world_in_another_process`:
/// a month, long enough for the seasons to have begun to turn and the herds
/// and the people to have met, and short enough to run on every change.
const A_MONTH_OF_TURNS: usize =
    (crate::environment::seasons::PLANNING_PERIODS_PER_DAY * 30) as usize;

/// The environment variable that tells the helper below it was started by
/// the two-process test rather than by somebody running `--ignored`.
const SPEAK_UP_FOR_THE_PARENT: &str = "EBSS_REPEATABLE_CHILD";

/// The other half of `the_same_seed_is_the_same_world_in_another_process`:
/// run in a child process, it plays seed 0 for a month and says what came of
/// it on one line the parent can read.
#[test]
#[ignore = "run by the_same_seed_is_the_same_world_in_another_process, in a process of its own"]
fn one_process_reports_seed_zero() {
    if std::env::var(SPEAK_UP_FOR_THE_PARENT).is_err() {
        return;
    }
    let (fingerprint, rolled) = a_world_from(0, A_MONTH_OF_TURNS);
    println!("SEED-ZERO-AFTER-A-MONTH {fingerprint} {rolled}");
}

/// The same seed in two processes is the same world (#342).
///
/// `the_same_seed_is_the_same_world` runs both of its worlds in one process,
/// so it cannot see anything that varies from one run of the program to the
/// next: a hash seeded per process, the operating system's randomness, an
/// address, the clock. Those are what #94 had to find by running one seed as
/// two processes and diffing them, and until now the only standing guard
/// against their coming back was `a_fixed_world_rolls_a_recorded_number_of_
/// times_over_a_whole_year`, which compares against a number recorded in some
/// earlier process - and so has to be re-recorded on every change to the
/// model, and cannot tell a deliberate change from drift when both land in
/// one commit.
///
/// This starts the test binary twice more, each running only
/// `one_process_reports_seed_zero`, and compares what the two report. There is
/// no recorded number: it holds through any deliberate change and fails only
/// when two runs of the same code disagree.
#[test]
fn the_same_seed_is_the_same_world_in_another_process() {
    use std::process::{Command, Stdio};

    let me = std::env::current_exe().expect("the test binary knows where it is");
    let helper = module_path!()
        .split_once("::")
        .map(|(_, rest)| format!("{rest}::one_process_reports_seed_zero"))
        .expect("this test sits inside the crate");

    let start = || {
        Command::new(&me)
            .args([helper.as_str(), "--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .env(SPEAK_UP_FOR_THE_PARENT, "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the test binary can be started again")
    };
    // Both at once: two processes, two hash seeds, two sets of addresses
    let (first, second) = (start(), start());

    let report = |child: std::process::Child| {
        let out = child.wait_with_output().expect("the child process finishes");
        assert!(out.status.success(), "the child process failed: {:?}", out.status);
        String::from_utf8_lossy(&out.stdout)
            .lines()
            // Anywhere on the line: the harness writes its own `test ... `
            // in front of whatever the test prints
            .find_map(|line| {
                line.split_once("SEED-ZERO-AFTER-A-MONTH ")
                    .map(|(_, said)| said.trim().to_string())
            })
            .expect("the child process reports the world it ran")
    };
    let (once, again) = (report(first), report(second));

    assert_eq!(
        once, again,
        "seed 0 came out differently in two runs of the same binary (fingerprint, \
         then draws taken), so something the seed does not fix differs from one \
         process to the next - see the module note above and ISSUES_FOUND #94"
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
