// src/analytics/wanting/strategy.rs
//! Layer 3: the distinct ways of answering a drive, and how one is chosen.
//!
//! Between the drive and the action there is a choice nobody in this model was
//! making. A thirsty man can drink what he is carrying, drink from the water in
//! front of him, or walk to water he remembers - three different bets with
//! different costs and different ways of failing - and until now the answer was
//! whichever of them appeared first in a `.or_else()` chain somebody typed.
//!
//! The chains are still where the *candidates* come from. What changes is that
//! each one is now named, asked separately whether it can be taken at all, and
//! ranked by what it has been worth to this agent rather than by where it sits
//! in a source file. The written order survives as the prior: a body that has
//! learned nothing does what the list says, which is what it did before.
//!
//! See `SATISFACTION.md` for the five layers this is the third of.

use crate::agents::patterns::Element;
use crate::core::DriveType;
use crate::environment::Action;

/// A named way of answering a drive.
///
/// One variant per distinct bet, not per action: `DrinkWhatIsCarried` and
/// `DrinkFromWhatIsHere` both come out as `Action::Gather { water }`, and they
/// are still two strategies, because a waterskin runs out and a river does not.
/// The whole point of the layer is that those two can now be told apart, and so
/// can be learned about separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Strategy {
    /// Drink out of what is in the pack.
    DrinkWhatIsCarried,
    /// Drink from open water within reach of where he is standing.
    DrinkFromWhatIsHere,
    /// Walk to water he can smell or remembers.
    WalkToWaterHeKnows,
}

impl Strategy {
    /// What it is called, which is what goes in the record.
    ///
    /// Short, lower case and stable: it is a map key and it is written into
    /// `Element::By`, so changing one of these forgets what every agent knew
    /// about it.
    pub fn called(&self) -> &'static str {
        match self {
            Strategy::DrinkWhatIsCarried => "drink-carried",
            Strategy::DrinkFromWhatIsHere => "drink-here",
            Strategy::WalkToWaterHeKnows => "walk-to-water",
        }
    }

    /// The element the pattern layer learns this by.
    pub fn as_element(&self) -> Element {
        Element::By(self.called().to_string())
    }

    /// The ways of answering a need, in the order somebody wrote them.
    ///
    /// This is the prior, not the answer. It decides what a body does before it
    /// has learned anything, and it breaks ties between ways that have been
    /// worth the same - which, early on, is most of them.
    ///
    /// A drive with no strategies yet returns nothing and its arm answers the
    /// old way. That is how this gets built one drive at a time instead of in
    /// one unmeasurable jump.
    pub fn all_for(need: DriveType) -> &'static [Strategy] {
        match need {
            DriveType::Thirst => &[
                Strategy::DrinkWhatIsCarried,
                Strategy::DrinkFromWhatIsHere,
                Strategy::WalkToWaterHeKnows,
            ],
            _ => &[],
        }
    }
}

/// What a strategy came back with: the action, and which way it was.
///
/// Kept together because the pair is the whole point - an action whose strategy
/// has been forgotten cannot be credited to the way it was chosen, and then the
/// ranking never learns anything.
pub struct TheWayItWasDone {
    pub doing: Action,
    pub by: Strategy,
}

impl crate::analytics::Simulation {
    /// Which way this agent answers this need, this turn.
    ///
    /// Every way that *can* be taken is asked for its action - that is the
    /// precondition check, and a way that cannot produce one is simply not a
    /// candidate. What survives is ranked by what this agent has found the way
    /// to be worth, and the written order breaks ties, which early on is every
    /// comparison there is.
    ///
    /// Returns `None` when this drive has no strategies yet, or when none of
    /// them can be taken - and the drive's own arm answers as it always did.
    /// That is deliberate: the layer goes in one drive at a time, and each one
    /// is measured on its own.
    pub(in crate::analytics) fn the_way_to_answer(
        &self,
        need: DriveType,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<TheWayItWasDone> {
        let ways = Strategy::all_for(need);
        if ways.is_empty() {
            return None;
        }

        // Everything that can be done at all, keeping the written order.
        let mut open: Vec<(Strategy, Action)> = ways
            .iter()
            .filter_map(|way| {
                self.can_this_way_be_taken(*way, agent, agent_position)
                    .map(|doing| (*way, doing))
            })
            .collect();

        if open.is_empty() {
            return None;
        }

        // Best first, by what each has been worth to this one. `sort_by_key`
        // on the index keeps the written order as the tie-break, which is what
        // decides for a body that has learned nothing yet - so a new agent
        // behaves exactly as it did before there was a ranking.
        let worth_of = |way: &Strategy| {
            agent
                .patterns
                .trail(need, &way.as_element())
                .map(|trail| trail.worth())
                .unwrap_or(0.0)
        };
        open.sort_by(|(left, _), (right, _)| {
            worth_of(right)
                .partial_cmp(&worth_of(left))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // **No exploration yet, on purpose.** The obvious next thing here is
        // the rule the hunger arm already uses - a share of turns spent on the
        // next way down, so that the first way that ever worked does not
        // become the only way ever tried again. It is deliberately not here,
        // because `Element::By` is not yet written at the end of a turn: the
        // decision layer is `&self` all the way down and the way chosen has no
        // route out of it to the place the episode is recorded.
        //
        // Until that wire exists every worth is zero, the sort is stable, and
        // this returns exactly what the written order returned before - so
        // this step changes no behaviour at all and can be measured to prove
        // it. Exploring between ways nobody can learn about is not a search,
        // it is noise, and it would spend real turns to buy nothing.
        let (by, doing) = open.swap_remove(0);
        Some(TheWayItWasDone { doing, by })
    }

    /// Whether this way can be taken at all, and what it comes to if so.
    ///
    /// The preconditions and the action in one place, because they are the same
    /// question asked twice: a way that cannot name what to do now has not met
    /// its preconditions, whatever else is true of it.
    pub(in crate::analytics) fn can_this_way_be_taken(
        &self,
        way: Strategy,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        match way {
            // Satisfier in the pack. Cheapest thing a thirsty man can do: no
            // walk, no weighing, and no chance of the water having gone.
            //
            // Whole units only. `Gather` drinks from a waterskin when there is
            // no source about, but not in dribbles, so an agent with half a
            // mouthful left kept choosing to drink and being told there was no
            // water anywhere - which was the largest single failure in the
            // simulation.
            Strategy::DrinkWhatIsCarried => (agent.inventory.available_water() >= 1.0)
                .then(|| Action::Gather {
                    resource_type: "water".to_string(),
                }),

            // Satisfier in front of him. The same action and a different bet:
            // a river does not run out in the way a skin does, and it cannot
            // be carried away from.
            Strategy::DrinkFromWhatIsHere => self
                .drinkable_water_within_reach(agent, agent_position)
                .then(|| Action::Gather {
                    resource_type: "water".to_string(),
                }),

            // Neither to hand: go to where the water is. This is the one that
            // costs turns, which is why it should be last for a body that has
            // a full skin and first for one that has not.
            Strategy::WalkToWaterHeKnows => {
                use crate::agents::senses::ScentType;
                use crate::core::memory::SpatialMemoryType;

                let target = self.known_source_position(
                    agent,
                    agent_position,
                    ScentType::Water,
                    SpatialMemoryType::Water,
                )?;

                let distance =
                    (target.0 - agent_position.0).abs() + (target.1 - agent_position.1).abs();

                Some(if distance > 1 {
                    Action::Move { target }
                } else {
                    Action::Gather {
                        resource_type: "water".to_string(),
                    }
                })
            }
        }
    }
}
