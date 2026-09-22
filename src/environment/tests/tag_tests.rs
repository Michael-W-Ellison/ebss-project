// src/environment/tests/tag_tests.rs
//! What a thing is, and what a job wants.
//!
//! Two vocabularies, and the tests that keep them honest: that a tag says
//! something true of a thing, that a capability ranks answers rather than
//! merely listing them, and - the one that matters most - that the four
//! capabilities with a trade behind them cannot drift away from the trade.

use super::*;

fn holding<'a>(these: &'a [(&'a str, u32)]) -> impl Fn(&str) -> u32 + 'a {
    move |what: &str| {
        these
            .iter()
            .find(|(called, _)| *called == what)
            .map(|(_, how_many)| *how_many)
            .unwrap_or(0)
    }
}

// --------------------------------------------------------------------------
// What a thing is
// --------------------------------------------------------------------------

/// The specification's three worked examples, against this world's names.
#[test]
fn the_three_worked_examples_read_the_way_the_specification_writes_them() {
    // The gourd: a vessel that holds water, holds food, and breaks.
    let pot = what_this_is("claypot");
    assert!(pot.contains(&Tag::WaterContainer));
    assert!(pot.contains(&Tag::FoodContainer));
    assert!(
        pot.contains(&Tag::FragileContainer),
        "the third tag is the one that earns its place: without it nothing \
         can explain why a leather bag is ever preferred to a pot"
    );

    // The flint spear.
    let spear = what_this_is("spear");
    assert!(spear.contains(&Tag::HuntingWeapon));
    assert!(spear.contains(&Tag::PiercingWeapon));
    assert!(spear.contains(&Tag::MediumRangeMelee));

    // The cordage grass, which this world grows as flax and cotton.
    let flax = what_this_is("flax");
    assert!(flax.contains(&Tag::FiberSource));
    assert!(flax.contains(&Tag::LowStrengthCordageMaterial));

    // And a fibre source is not itself cordage: the distinction the recipe
    // chain already draws in named steps, now drawn in classes.
    assert!(!flax.contains(&Tag::Cordage));
    assert!(is_this_a("lashing", Tag::Cordage));
}

/// An untagged thing says nothing rather than guessing.
#[test]
fn a_thing_nobody_has_described_does_not_get_described_by_its_name() {
    assert!(what_this_is("gruntlebuck").is_empty());
    assert!(!is_this_a("gruntlebuck", Tag::WaterContainer));
}

/// Every tag in the table is one something actually is.
///
/// A vocabulary with entries nothing uses is a vocabulary that reads as richer
/// than the world it describes.
///
/// The keeping tags are exempt and are held to the harder version of the same
/// rule below: they are not what a thing is *called*, they are what has been
/// done to it, and what has been done to it comes from a preparation rather
/// than from a name.
#[test]
fn every_tag_declared_is_a_tag_something_carries() {
    let every_tag = [
        Tag::FoodContainer,
        Tag::WaterContainer,
        Tag::FragileContainer,
        Tag::CarryingContainer,
        Tag::HuntingWeapon,
        Tag::PiercingWeapon,
        Tag::MediumRangeMelee,
        Tag::ShortRangeMelee,
        Tag::ThrownWeapon,
        Tag::FiberSource,
        Tag::LowStrengthCordageMaterial,
        Tag::Cordage,
        Tag::FlexibleCovering,
        Tag::Pole,
        Tag::RigidBuildingMaterial,
        Tag::Knappable,
        Tag::Metallic,
        Tag::Hide,
        Tag::Timber,
        Tag::FiredEarth,
        Tag::Preserved,
        Tag::Perishable,
    ];

    for tag in every_tag {
        assert!(
            everything_that_is(tag).next().is_some(),
            "nothing in the world is {}",
            tag.called()
        );
    }
}

/// No thing is listed twice, and no thing carries the same tag twice.
#[test]
fn the_table_says_each_thing_once() {
    let mut seen: Vec<&str> = EVERYTHING_TAGGED.iter().map(|t| t.called).collect();
    let how_many = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), how_many, "something is described twice");

    for tagged in EVERYTHING_TAGGED {
        let mut tags = tagged.is.to_vec();
        let how_many = tags.len();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), how_many, "{} carries a tag twice", tagged.called);
    }
}

// --------------------------------------------------------------------------
// What a job wants
// --------------------------------------------------------------------------

/// The specification's ladder: a best answer at 1.0, worse ones below it, and
/// bare hands at the bottom.
///
/// "copper shovel = digging_tool 1.0, wooden shovel = 0.7, wooden stick =
/// 0.3". The names differ - this world has no copper - but the shape is the
/// thing, and the shape has to hold on every capability.
#[test]
fn every_capability_ranks_its_answers_from_the_best_down() {
    for capability in Capability::every_one() {
        let answers = everything_that_answers(*capability);

        assert!(
            !answers.is_empty(),
            "nothing in the world answers {}",
            capability.called()
        );

        assert_eq!(
            answers[0].1, 1.0,
            "the best answer to {} should sit at the top of the scale",
            capability.called()
        );

        for pair in answers.windows(2) {
            assert!(
                pair[0].1 >= pair[1].1,
                "{} is not ranked: {:?} before {:?}",
                capability.called(),
                pair[0],
                pair[1]
            );
        }

        for (called, how_well) in &answers {
            assert!(
                *how_well > WHAT_EMPTY_HANDS_SERVE && *how_well <= 1.0,
                "{called} answers {} at {how_well}, which is off the scale",
                capability.called()
            );
        }
    }
}

/// Empty hands answer nothing, and neither does the wrong thing.
#[test]
fn bare_hands_sit_at_the_bottom_of_every_scale() {
    assert_eq!(WHAT_EMPTY_HANDS_SERVE, 0.0);

    for capability in Capability::every_one() {
        assert_eq!(
            how_well_this_serves("nothing at all", *capability),
            WHAT_EMPTY_HANDS_SERVE
        );
    }

    // A basket is a fine thing and is no use for digging.
    assert_eq!(
        how_well_this_serves("basket", Capability::DiggingTool),
        WHAT_EMPTY_HANDS_SERVE
    );
}

/// **The test this module exists for.** A capability with a trade behind it
/// reads the trade, and cannot say anything the trade does not say.
///
/// The alternative - a second hand-written table of digging coefficients
/// beside the tool table that already prices digging - is two spellings of one
/// question, which this codebase has paid for at least three times. If
/// somebody makes a better shovel, every coefficient on that capability has to
/// move on its own.
#[test]
fn a_capability_with_a_trade_behind_it_agrees_with_the_trade() {
    for capability in Capability::every_one() {
        let Some(trade) = capability.the_trade_behind_it() else {
            continue;
        };

        // Nothing may be declared by hand for a capability a trade answers.
        assert!(
            !EVERYTHING_THAT_SERVES
                .iter()
                .any(|serves| serves.capability == *capability),
            "{} is both derived from {trade:?} and declared by hand",
            capability.called()
        );

        // And the ordering has to be the tool table's ordering.
        let answers = everything_that_answers(*capability);
        for pair in answers.windows(2) {
            let better = super::EVERY_TOOL
                .iter()
                .filter(|t| t.helps == trade && t.called == pair[0].0)
                .map(|t| t.how_much_better)
                .fold(0.0_f32, f32::max);
            let worse = super::EVERY_TOOL
                .iter()
                .filter(|t| t.helps == trade && t.called == pair[1].0)
                .map(|t| t.how_much_better)
                .fold(0.0_f32, f32::max);

            assert!(
                better >= worse,
                "{} ranks {} above {}, and the tool table does not",
                capability.called(),
                pair[0].0,
                pair[1].0
            );
        }
    }
}

/// Conversely: a capability with no trade behind it must be declared.
#[test]
fn a_capability_with_no_trade_behind_it_is_declared_outright() {
    for capability in Capability::every_one() {
        if capability.the_trade_behind_it().is_some() {
            continue;
        }

        assert!(
            EVERYTHING_THAT_SERVES
                .iter()
                .any(|serves| serves.capability == *capability),
            "{} has neither a trade nor a declaration",
            capability.called()
        );
    }
}

/// The best thing to hand is taken, not the first one somebody typed.
#[test]
fn the_best_answer_to_hand_is_the_one_that_comes_back() {
    let pack = holding(&[("diggingstick", 1), ("shovel", 1)]);
    let (what, how_well) =
        the_best_to_hand(Capability::DiggingTool, &pack).expect("something to dig with");

    assert_eq!(what, "shovel", "a shovel beside a stick is the shovel");
    assert!(how_well > how_well_this_serves("diggingstick", Capability::DiggingTool));

    // And with only the stick, the stick - which is worth something, and much
    // less than the shovel.
    let stick_only = holding(&[("diggingstick", 1)]);
    assert_eq!(
        the_best_to_hand(Capability::DiggingTool, &stick_only).map(|(what, _)| what),
        Some("diggingstick")
    );

    let nothing = holding(&[]);
    assert!(the_best_to_hand(Capability::DiggingTool, &nothing).is_none());
}

/// The ranking is the same on every run.
///
/// Two things worth the same would otherwise come back in whatever order the
/// table happened to hold them, and a world that chooses differently between
/// runs is a world that cannot be measured. See `repeatable_tests`.
#[test]
fn the_ranking_does_not_depend_on_the_order_of_the_table() {
    for capability in Capability::every_one() {
        let once = everything_that_answers(*capability);
        let twice = everything_that_answers(*capability);
        assert_eq!(once, twice);

        let mut named: Vec<&str> = once.iter().map(|(called, _)| *called).collect();
        let how_many = named.len();
        named.sort_unstable();
        named.dedup();
        assert_eq!(
            named.len(),
            how_many,
            "{} lists something twice",
            capability.called()
        );
    }
}

// --------------------------------------------------------------------------
// Asking for a class rather than for a name
// --------------------------------------------------------------------------

/// A tent is poles, a covering and cordage, and anything of those classes does.
#[test]
fn a_tent_can_be_built_out_of_whatever_answers_the_classes() {
    let with_hides = holding(&[("wood", 8), ("hides", 4), ("lashing", 2)]);
    let using = what_would_be_used(WHAT_A_TENT_TAKES, &with_hides).expect("enough for a tent");

    assert!(using.contains(&("wood", 8)));
    assert!(using.contains(&("hides", 4)));
    assert!(using.contains(&("lashing", 2)));

    // **The thing classes buy.** A people who scraped their hides into leather
    // can roof with the leather. Asked by name, they had nothing.
    let with_leather = holding(&[("wood", 8), ("leather", 4), ("lashing", 2)]);
    let using = what_would_be_used(WHAT_A_TENT_TAKES, &with_leather)
        .expect("leather is a flexible covering too");
    assert!(using.contains(&("leather", 4)));
}

/// And what is short comes back as the next job rather than as a refusal.
#[test]
fn a_shortfall_says_what_class_is_missing_and_by_how_much() {
    let no_covering = holding(&[("wood", 8), ("lashing", 2)]);

    match what_would_be_used(WHAT_A_TENT_TAKES, &no_covering) {
        Err((short_of, how_many)) => {
            assert_eq!(short_of, Tag::FlexibleCovering);
            assert_eq!(how_many, 4);
        }
        Ok(using) => panic!("built a tent with no covering, out of {using:?}"),
    }
}

/// What one want takes is not available to the next one.
///
/// Wood is both a pole and - by a different tag - a roofing material, so a
/// list that wanted both could be met twice over by one pile of wood if
/// nothing kept count.
#[test]
fn the_same_pile_cannot_answer_two_wants_at_once() {
    let wants = [
        Wanted { is: Tag::Pole, how_much: 6 },
        Wanted { is: Tag::RigidBuildingMaterial, how_much: 6 },
    ];

    let only_six = holding(&[("wood", 6)]);
    assert!(
        what_would_be_used(&wants, &only_six).is_err(),
        "six pieces of wood cannot be six poles and six walls at the same time"
    );

    let twelve = holding(&[("wood", 12)]);
    assert!(what_would_be_used(&wants, &twelve).is_ok());
}

/// A want is met out of several piles where no one pile covers it.
#[test]
fn several_things_of_a_class_add_up() {
    let scraps = holding(&[("wood", 8), ("hides", 1), ("leather", 3), ("lashing", 2)]);
    let using = what_would_be_used(WHAT_A_TENT_TAKES, &scraps).expect("one hide and three leathers");

    let covering: u32 = using
        .iter()
        .filter(|(called, _)| is_this_a(called, Tag::FlexibleCovering))
        .map(|(_, how_many)| *how_many)
        .sum();

    assert_eq!(covering, 4);
}

// --------------------------------------------------------------------------
// How fast a thing goes off, and what makes it faster or slower
// --------------------------------------------------------------------------

/// Every tag that carries a keeping number is one some preparation leaves.
///
/// The counterpart of `every_tag_declared_is_a_tag_something_carries` for the
/// tags nothing is *called*. A priced tag nothing can arrive at is a number
/// that will never be read, and the way a thing arrives at one of these is by
/// having something done to it.
#[test]
fn every_priced_tag_is_one_something_can_actually_become() {
    use crate::world::nutrition::PreparationState;

    const EVERY_PREPARATION: &[PreparationState] = &[
        PreparationState::Raw,
        PreparationState::Cooked,
        PreparationState::Dried,
        PreparationState::Smoked,
        PreparationState::Salted,
        PreparationState::Pickled,
        PreparationState::Ground,
        PreparationState::Fermented,
        PreparationState::Ruined,
    ];

    let reachable: Vec<Tag> = EVERY_PREPARATION
        .iter()
        .flat_map(|how| how.what_this_leaves_it().iter().copied())
        .collect();

    for tag in [
        Tag::Wet,
        Tag::Dry,
        Tag::Cooked,
        Tag::Smoked,
        Tag::Salted,
        Tag::Soured,
        Tag::Fermented,
        Tag::Ground,
        Tag::Spoiled,
    ] {
        assert!(
            tag.what_it_does_to_keeping().is_some(),
            "{} is a keeping tag and should carry a number",
            tag.called()
        );
        assert!(
            reachable.contains(&tag) || everything_that_is(tag).next().is_some(),
            "nothing in this world can come to be {}",
            tag.called()
        );
    }
}

/// A tag with no opinion about keeping is not given one.
///
/// The difference between `None` and `Some(1.0)` is the whole of why this is
/// an `Option`: the arithmetic is the same and the statement is not, and a
/// default of one would quietly swallow a keeping tag somebody added and
/// forgot to price.
#[test]
fn a_tag_that_says_nothing_about_keeping_says_nothing() {
    assert_eq!(Tag::Pole.what_it_does_to_keeping(), None);
    assert_eq!(Tag::Knappable.what_it_does_to_keeping(), None);
    assert_eq!(Tag::FoodContainer.what_it_does_to_keeping(), None);

    // And it changes nothing when it is in the list anyway.
    assert_eq!(
        how_fast_these_go_off(&[Tag::Pole, Tag::Timber]),
        WHAT_NOTHING_SAYS
    );
}

/// The coarse question and the numbers must never disagree.
///
/// `Preserved` and `Perishable` are what a verb asks for - is this the kind of
/// thing meant to keep - and they are deliberately unpriced, because the
/// number belongs to the *reason* a thing keeps and not to the claim that it
/// does. That is only safe while the two say the same thing, which is what
/// this asserts: two spellings of one question is the mistake this module's
/// own docstring says the project has paid for three times.
#[test]
fn what_is_called_preserved_is_what_actually_keeps() {
    use crate::world::nutrition::PreparationState;

    // Nothing that keeps, keeps by being called something.
    for called in everything_that_is(Tag::Preserved) {
        assert_eq!(
            how_fast_this_goes_off(called, &[]),
            WHAT_NOTHING_SAYS,
            "{called} is called preserved, and the way a thing comes to keep              is that something is done to it - the name must not do it too"
        );
    }

    // And drying one is what makes it keep.
    for called in everything_that_is(Tag::Preserved) {
        let dried = how_fast_this_goes_off(called, PreparationState::Dried.what_this_leaves_it());
        assert!(
            dried < WHAT_NOTHING_SAYS,
            "{called} dried should keep better than {called} raw"
        );
    }

    for called in everything_that_is(Tag::Perishable) {
        let raw = how_fast_this_goes_off(called, PreparationState::Raw.what_this_leaves_it());
        assert!(
            raw >= WHAT_NOTHING_SAYS,
            "{called} is called perishable and should not keep: {raw}"
        );
    }
}

/// The tags compose, and a tag arriving twice counts once.
///
/// The doubling is not hypothetical: a dried strip of meat is `Dry` because of
/// what was done to it and would be `Dry` again the day somebody says that is
/// what a strip *is*. Twenty times squared is four hundred, and a four-hundred
/// times multiplier on a winter store is a settlement that never goes hungry.
#[test]
fn what_arrives_twice_is_counted_once() {
    let once = how_fast_these_go_off(&[Tag::Dry]);
    let twice = how_fast_these_go_off(&[Tag::Dry, Tag::Dry]);

    assert_eq!(once, twice, "one fact, counted once");

    // Two different facts do compose: over a fire and then laid in the sun.
    let both = how_fast_these_go_off(&[Tag::Cooked, Tag::Dry]);
    assert!(
        both < once,
        "two things done to it should keep it better than one: {both} against {once}"
    );
}

/// A change of tag is a change of rate, with nothing told about it.
///
/// **The thing the whole conversion is for.** Meat goes from wet to dry and
/// goes off twenty times slower, and no part of the model was informed: the
/// clock reads the tags, and the tags changed.
#[test]
fn meat_that_goes_from_wet_to_dry_keeps_twenty_times_longer() {
    use crate::world::nutrition::PreparationState;

    let wet = how_fast_this_goes_off(
        "meatportions",
        PreparationState::Raw.what_this_leaves_it(),
    );
    let dry = how_fast_this_goes_off(
        "meatportions",
        PreparationState::Dried.what_this_leaves_it(),
    );

    assert_eq!(wet, 1.0, "raw flesh is the baseline");
    assert!(
        (wet / dry - 20.0).abs() < 0.01,
        "drying should be worth twenty times: {wet} against {dry}"
    );
}

/// A preparation says nothing the tags do not.
///
/// `PreparationState::spoilage_multiplier` was the table these numbers came
/// out of, and is now derived from them. The test is that it still answers,
/// and answers the same thing - because the alternative is two tables, which
/// is what this is all in aid of avoiding.
#[test]
fn a_preparation_is_only_a_way_of_getting_a_tag() {
    use crate::world::nutrition::PreparationState;

    for how in [
        PreparationState::Raw,
        PreparationState::Cooked,
        PreparationState::Dried,
        PreparationState::Smoked,
        PreparationState::Salted,
        PreparationState::Pickled,
        PreparationState::Ground,
        PreparationState::Fermented,
        PreparationState::Ruined,
    ] {
        assert_eq!(
            how.spoilage_multiplier(),
            how_fast_these_go_off(how.what_this_leaves_it()),
            "{} should say what its tags say and nothing else",
            how.name()
        );
    }
}

// --------------------------------------------------------------------------
// Where a thing is kept
// --------------------------------------------------------------------------

/// Everything priced as somewhere to keep food is somewhere food goes.
#[test]
fn nothing_is_a_larder_that_will_not_hold_food() {
    for keeps in EVERYTHING_THAT_KEEPS {
        assert!(
            is_this_a(keeps.called, Tag::FoodContainer),
            "{} is priced as somewhere to keep food and is not a food container",
            keeps.called
        );
        assert!(
            keeps.how_fast_food_in_it_goes_off > 0.0,
            "{} would stop time",
            keeps.called
        );
    }
}

/// A vessel slows a thing down; it does not preserve it.
///
/// The ladder has to stay under what drying and salting are worth, or there is
/// no reason to spend three turns preserving anything when a basket is one.
#[test]
fn no_vessel_is_worth_as_much_as_preserving() {
    use crate::world::nutrition::PreparationState;

    let best_vessel = EVERYTHING_THAT_KEEPS
        .iter()
        .map(|keeps| keeps.how_fast_food_in_it_goes_off)
        .fold(f32::INFINITY, f32::min);

    let worst_preserving = [
        PreparationState::Dried,
        PreparationState::Smoked,
        PreparationState::Salted,
        PreparationState::Fermented,
    ]
    .iter()
    .map(|how| how.spoilage_multiplier())
    .fold(0.0_f32, f32::max);

    assert!(
        best_vessel > worst_preserving,
        "the best pot in the world ({best_vessel}) should be worth less than          the feeblest way of preserving a thing ({worst_preserving})"
    );
}

/// Nothing to keep it in is a number, not a special case.
#[test]
fn a_bare_pack_keeps_nothing_and_says_so() {
    let nothing: [(&str, u32); 0] = [];
    assert_eq!(
        the_best_keeping_to_hand(&holding(&nothing)),
        WHAT_A_BARE_PACK_KEEPS
    );
    assert_eq!(how_well_this_keeps("spear"), WHAT_A_BARE_PACK_KEEPS);
    assert_eq!(how_well_this_keeps("gruntlebuck"), WHAT_A_BARE_PACK_KEEPS);
}

/// A person keeps their food in the best thing they are carrying.
#[test]
fn the_pot_goes_in_the_pack_and_the_food_goes_in_the_pot() {
    let with_a_basket = the_best_keeping_to_hand(&holding(&[("basket", 1)]));
    let with_a_pot = the_best_keeping_to_hand(&holding(&[("basket", 1), ("claypot", 1)]));

    assert!(with_a_basket < WHAT_A_BARE_PACK_KEEPS, "a basket is worth something");
    assert!(
        with_a_pot < with_a_basket,
        "the pot is the better of the two and is the one that gets used:          {with_a_pot} against {with_a_basket}"
    );

    // And the ladder is a ladder: firing pots is worth going on with.
    assert!(
        how_well_this_keeps("stoneware") < how_well_this_keeps("claypot"),
        "sealed fired earth should be the best of them"
    );
}
