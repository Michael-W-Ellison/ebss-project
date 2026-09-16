// src/analytics/tests/stage_zero_tests.rs
//! What a people starts with, and whether the recipe tables agree.
//!
//! The stage table is a declaration, and a declaration that nothing checks is
//! a wish list. Two things are checked here. The first is that the table is
//! whole: thirty-five paths, each with a Stage 0, each saying honestly
//! whether this world carries it. The second is the one that has teeth -
//! **a product the table says everybody is born knowing how to make must
//! actually be obvious in `making.rs`**, so the two cannot drift apart
//! without the suite going red.

use crate::environment::making;
use crate::environment::stage::{
    everything_a_people_starts_knowing, how_much_of_stage_zero_stands,
    what_is_missing_from_stage_zero, Path, Standing, EVERY_PATH,
};

// --------------------------------------------------------------------------
// The table is whole
// --------------------------------------------------------------------------

/// Every path in the specification is here, once.
#[test]
fn every_development_path_is_declared_exactly_once() {
    assert_eq!(
        EVERY_PATH.len(),
        35,
        "thirty-five development paths were specified"
    );

    let mut seen: Vec<Path> = EVERY_PATH.to_vec();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), EVERY_PATH.len(), "and none of them twice");
}

/// Every path has a Stage 0, and it is stage zero.
#[test]
fn every_path_starts_at_stage_zero() {
    for path in EVERY_PATH {
        let stage = path.stage_zero();
        assert_eq!(
            stage.number,
            0,
            "{} starts somewhere other than the beginning",
            path.called()
        );
        assert!(
            !stage.called.is_empty(),
            "{} has no name for where it starts",
            path.called()
        );
        assert!(
            !stage.starts_with.is_empty(),
            "{} starts with nothing at all, which is not a stage",
            path.called()
        );
    }
}

/// Every path is named distinctly, because the name is what a report prints.
#[test]
fn no_two_paths_answer_to_the_same_name() {
    let mut names: Vec<&str> = EVERY_PATH.iter().map(|path| path.called()).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "two paths share a name");
}

/// Every path says what holds it back, because that is what the next stage is
/// for and the user is going to write the next stage.
#[test]
fn every_stage_says_what_it_cannot_do() {
    for path in EVERY_PATH {
        let stage = path.stage_zero();
        assert!(
            !stage.held_back_by.is_empty(),
            "{} names nothing it cannot do, so nothing can advance past it",
            path.called()
        );
    }
}

// --------------------------------------------------------------------------
// The table and the recipes cannot drift
// --------------------------------------------------------------------------

/// **The one that has teeth.**
///
/// A path that says a people is born knowing how to make a thing, and a
/// making table that says the thing has to be found out first, are two
/// answers to one question. This is what stops them being given.
#[test]
fn the_stage_table_and_the_making_tables_cannot_drift() {
    for path in EVERY_PATH {
        let stage = path.stage_zero();
        for born in stage.born_knowing {
            let by_making = making::EVERY_STEP
                .iter()
                .any(|step| step.makes == *born && step.obvious);
            let by_working = making::EVERY_WORKING
                .iter()
                .any(|working| working.makes == *born && working.obvious);

            assert!(
                by_making || by_working,
                "the {} path says everybody is born knowing how to make {born}, and the \
                 recipe tables either do not make it at all or still want it discovered",
                path.called()
            );
        }
    }
}

/// Where a Stage 0 product is *also* something to discover, the discovery is
/// a better road to it and never the only road.
///
/// The other direction of the same rule, and it caught what it was written to
/// catch on the first run. A knapped tip is born knowledge off ordinary stone
/// and a discovery off flint - one product, two roads, the second better than
/// the first. That is the whole of the innovation path and it is correct.
/// What would be wrong is a product the table hands a people at Stage 0 that
/// can *only* be reached by finding something out, because then a people
/// would spend afternoons rediscovering what it was said to have been born
/// with.
#[test]
fn a_stage_zero_product_that_is_also_discovered_has_a_road_that_needs_no_discovery() {
    let starts_knowing = everything_a_people_starts_knowing();

    for step in making::everything_to_find_out() {
        if !starts_knowing.contains(step.makes) {
            continue;
        }

        let obvious_road = making::EVERY_STEP
            .iter()
            .any(|other| other.makes == step.makes && other.obvious)
            || making::EVERY_WORKING
                .iter()
                .any(|working| working.makes == step.makes && working.obvious);

        assert!(
            obvious_road,
            "{} is claimed at Stage 0 and the only way to it is a discovery",
            step.makes
        );
    }
}

/// The one place the specification and this model disagree, held open rather
/// than papered over.
///
/// Stage 0 of the stoneware path is "hand-shaped clay vessels, pit firing,
/// low temperature firing, porous pottery". In this model shaping clay is a
/// discovery, and it was made obvious to match: **it cost 4.9% of person-days
/// and eight of twenty-one first winters over sixty-four seeded worlds**,
/// because `what_i_would_work_on` spends a turn on any working it knows the
/// input for without asking whether the output is worth having.
///
/// So the path reads `Short`, with that as the reason, and this test is what
/// stops somebody quietly flipping the flag back without re-measuring.
#[test]
fn the_stoneware_path_is_declared_short_and_the_recipe_agrees() {
    let shaping = making::how_to_work("mold", "clay").expect("clay molds");
    assert_eq!(shaping.makes, "claypot");
    assert!(
        !shaping.obvious,
        "shaping clay stays a discovery until something asks whether what a \
         working makes is worth making"
    );

    let firing = making::how_to_work("fire", "claypot").expect("a pot fires");
    assert_eq!(firing.makes, "stoneware");
    assert!(
        !firing.obvious,
        "and a kiln and a fuel supply are what stoneware wants, so firing was \
         never Stage 0 either"
    );

    let stage = Path::Stoneware.stage_zero();
    assert!(
        matches!(stage.standing, Standing::Short(_)),
        "the table says out loud that this world does not start where the \
         specification says it should"
    );
    assert!(
        stage.born_knowing.is_empty(),
        "and claims no born knowledge it cannot back"
    );
}

// --------------------------------------------------------------------------
// The honest half
// --------------------------------------------------------------------------

/// Every path says whether this world carries it, and says something when it
/// does not.
#[test]
fn every_path_says_where_this_world_actually_stands() {
    for path in EVERY_PATH {
        let said = path.stage_zero().standing.what_was_said();
        assert!(
            said.len() > 20,
            "{} says almost nothing about where it stands: {said:?}",
            path.called()
        );
    }
}

/// A path that claims to stand whole claims no missing half.
#[test]
fn a_path_that_stands_whole_is_not_also_short() {
    for path in EVERY_PATH {
        let stage = path.stage_zero();
        if matches!(stage.standing, Standing::Stands(_)) {
            assert!(
                stage.standing.any_of_it(),
                "{} both stands and does not",
                path.called()
            );
        }
    }
}

/// The gap list is what it says it is: everything not whole, and nothing else.
#[test]
fn the_gap_list_holds_every_path_that_is_not_whole() {
    let (whole, half, none) = how_much_of_stage_zero_stands();
    assert_eq!(
        whole + half + none,
        EVERY_PATH.len(),
        "every path is counted once"
    );

    let missing = what_is_missing_from_stage_zero();
    assert_eq!(
        missing.len(),
        half + none,
        "the gap list is exactly the paths that are not whole"
    );

    for (path, _) in &missing {
        assert!(
            !matches!(path.stage_zero().standing, Standing::Stands(_)),
            "{} is in the gap list and stands whole",
            path.called()
        );
    }
}

/// A people that has nothing to carry water in has half a water path.
///
/// Not a decoration: this is ISSUES_FOUND #292 stated where somebody counting
/// Stage 0 will trip over it, rather than only in a report nobody reads.
#[test]
fn the_water_path_is_only_half_here() {
    assert!(
        matches!(Path::WaterSystems.stage_zero().standing, Standing::Part(_)),
        "drinking works and carrying barely does"
    );
    assert!(
        Path::WaterSystems
            .stage_zero()
            .born_knowing
            .contains(&"bowl"),
        "the one vessel a people is born able to make"
    );
}

/// Nothing crosses water and nothing is stacked into a wall, and the table
/// says so out loud rather than quietly leaving the paths out.
#[test]
fn the_paths_this_world_does_not_carry_at_all_are_named_anyway() {
    for path in [
        Path::MaritimeAndBoatbuilding,
        Path::Masonry,
        Path::PotteryBeyondStoneware,
        Path::Stoneware,
    ] {
        assert!(
            matches!(path.stage_zero().standing, Standing::Short(_)),
            "{} is declared and empty",
            path.called()
        );
        assert!(
            path.stage_zero().born_knowing.is_empty(),
            "{} cannot claim born knowledge it has no machinery for",
            path.called()
        );
    }
}

// --------------------------------------------------------------------------
// And the other way round: what a people starts with *is* Stage 0
// --------------------------------------------------------------------------
//
// The drift test above holds one direction - a product the table claims must
// be obvious. That leaves the direction that actually decides what agents
// start with: something the recipe tables call obvious that no stage grants
// is technology a people has and the table never gave it. Audited when these
// were written, the two sets are exactly equal at twenty products apiece, so
// what is pinned here is a state the model is already in rather than one it
// is being moved to.

/// Nothing is obvious that no stage grants.
///
/// `Agent::knows_how_to` is `step.obvious || self.found_out.contains(..)`, so
/// the obvious flag *is* what a founder starts knowing. If a step is obvious
/// and no path claims it, a people begins with a technology the Stage 0 table
/// never handed them - which is the table quietly not being the answer to
/// what agents start with.
#[test]
fn nothing_is_born_known_that_no_stage_grants() {
    let granted = everything_a_people_starts_knowing();

    let mut ungranted: Vec<&str> = making::EVERY_STEP
        .iter()
        .filter(|step| step.obvious)
        .map(|step| step.makes)
        .chain(
            making::EVERY_WORKING
                .iter()
                .filter(|working| working.obvious)
                .map(|working| working.makes),
        )
        .filter(|makes| !granted.contains(*makes))
        .collect();
    ungranted.sort_unstable();
    ungranted.dedup();

    assert!(
        ungranted.is_empty(),
        "the recipe tables call these obvious - so every founder is born able \
         to make them - and no path in the Stage 0 table grants them: \
         {ungranted:?}"
    );
}

/// What founders carry is what their people know how to replace.
///
/// `Agent::WHAT_THEY_CARRY` says so in a doc comment - "the same named things
/// the chain in `environment::making` turns out, so that what a founder wears
/// through is a thing his people know how to replace" - and nothing checked
/// it. A kit item that is not Stage 0 is a people starting above its stage
/// with a thing it cannot make again, which is the sharper half of the same
/// question: a stoneknife in the pack now decides whether anybody can sew
/// (see `verbs::SEW`), so a kit that drifted above the stage table would hand
/// out a capability the stage never granted.
#[test]
fn what_founders_carry_is_something_their_people_can_make() {
    let granted = everything_a_people_starts_knowing();

    for (what, _, _) in crate::agents::Agent::WHAT_THEY_CARRY {
        assert!(
            granted.contains(what),
            "founders walk in carrying a {what} and no Stage 0 path says \
             anybody knows how to make one"
        );
    }
}

/// And the one lesson everybody is born with is one a stage grants.
///
/// The third surface, and the one the product-keyed drift test cannot see:
/// `found_out` is seeded with `THAT_LAYING_IT_OUT_KEEPS_IT`, which is a fact
/// rather than a product, so nothing tied it to the table. `FoodPreservation`
/// begins at "sun drying, air drying", which is exactly that lesson - so the
/// seed is right, and this is what keeps it right.
#[test]
fn the_lesson_founders_are_seeded_with_belongs_to_a_stage() {
    let born = crate::agents::Agent::what_anybody_is_born_knowing();

    assert_eq!(
        born.len(),
        1,
        "somebody added to what everybody is born knowing without saying \
         which stage grants it: {born:?}"
    );

    let preservation = Path::FoodPreservation.stage_zero();
    assert!(
        preservation
            .starts_with
            .iter()
            .any(|had| had.contains("drying")),
        "founders are seeded with the lesson that laying food out keeps it, \
         and food preservation does not start with drying"
    );
    assert!(
        matches!(preservation.standing, Standing::Stands(_)),
        "the stage that grants the one born-known lesson does not stand"
    );
}
