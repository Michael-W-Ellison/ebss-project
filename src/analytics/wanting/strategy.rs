// src/analytics/wanting/strategy.rs
//! Layer 3: the ways of answering a drive, what each costs, and when each is
//! worth taking.
//!
//! Between the drive and the action there is a choice nobody in this model was
//! making. A thirsty man can drink what he is carrying, drink from the water in
//! front of him, walk to water he remembers, or dig a well - four bets with
//! quite different costs and quite different horizons - and until now the answer
//! was whichever of them appeared first in a `.or_else()` chain somebody typed.
//!
//! Three things here:
//!
//! - **The ways are declared**, all of them, including the ones this world
//!   cannot yet carry out. A way that is named and unreachable is a gap anybody
//!   can count; a way that was never named is a gap nobody can see. See `Reach`.
//! - **What each is worth is computed**, by the formula in `Utility`, in one
//!   currency.
//! - **When each is worth taking** is a separate question from what it is
//!   worth. A man dying of thirst does not dig a well, however good a well is.
//!   See `Horizon`.
//!
//! See `SATISFACTION.md` for the five layers this is the third of.

use crate::agents::patterns::Element;
use crate::core::DriveType;
use crate::environment::Action;

/// A named way of answering a drive.
///
/// One variant per distinct bet, not per action. `ConsumeCarriedWater` and
/// `DrinkFromLocalSource` both come out as `Action::Gather { water }`, and they
/// are still two ways, because a waterskin runs out and a river does not. The
/// whole point of the layer is that those two can be told apart, and so can be
/// costed and learned about separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Strategy {
    // ---- hydration ------------------------------------------------------
    ConsumeCarriedWater,
    DrinkFromLocalSource,
    FetchFromKnownSource,
    AskOrFollowAnotherToWater,
    ExploitRainCatchment,
    DigOrRepairWell,
    RelocateTowardWater,

    // ---- hunger ---------------------------------------------------------
    EatCarriedFood,
    EatStoredFood,
    GatherWildFood,
    ScavengeWhatIsLyingAbout,
    WalkTheTrapline,
    FishLocalWaters,
    HuntLocalAnimals,
    TradeForFood,
    StealFood,
    RequestCommunalAllocation,
    ProcessStoredRawFood,

    // ---- shelter --------------------------------------------------------
    UseOwnedShelter,
    UseHouseholdShelter,
    ShareCommunalShelter,
    RepairDamagedShelter,
    BuildTemporaryShelter,
    BuildDurableShelter,
    RelocateToNaturalShelter,
}

/// Whether this world can actually carry a way out yet.
///
/// The specification names ways this model has no machinery for - a rain
/// catchment, a well, asking the settlement for a share. They are declared
/// anyway, with the reason they cannot fire, because a named gap is one
/// somebody can count and go and fill, and an unnamed one is one nobody knows
/// is there. `NotYet` ways are never candidates; they cost nothing at runtime
/// beyond a match arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The world supports it and it can be chosen.
    Now,
    /// Declared, and what is missing before it could fire.
    NotYet(&'static str),
}

/// How far ahead a way pays off.
///
/// **A man dying of thirst does not dig a well, however good a well is.** That
/// is not a matter of the well being worth less - over a season it is worth far
/// more than a mouthful - it is that the need is now and the well is not. So
/// the horizon gates the choice before utility decides within it, rather than
/// being another term subtracted from the score. A cost that is subtracted can
/// always be outweighed by a big enough number; this cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Horizon {
    /// Answers the need this turn or next: drink, eat, lie down, go inside.
    Immediate,
    /// Answers it over the next few days: fill the skin, gather, mend the
    /// tent, fetch fuel.
    ShortTerm,
    /// Answers it for a season: dig a well, sink a storage pit, sow a crop,
    /// make a net, put up a permanent roof.
    LongTerm,
}

impl Strategy {
    /// What it is called, which is what goes in the record.
    ///
    /// Short, lower case and stable: it is written into `Element::By`, so
    /// changing one of these forgets what every agent knew about it.
    pub fn called(&self) -> &'static str {
        match self {
            Strategy::ConsumeCarriedWater => "drink-carried",
            Strategy::DrinkFromLocalSource => "drink-here",
            Strategy::FetchFromKnownSource => "fetch-water",
            Strategy::AskOrFollowAnotherToWater => "follow-to-water",
            Strategy::ExploitRainCatchment => "catch-rain",
            Strategy::DigOrRepairWell => "dig-well",
            Strategy::RelocateTowardWater => "move-to-water",

            Strategy::EatCarriedFood => "eat-carried",
            Strategy::EatStoredFood => "eat-stored",
            Strategy::GatherWildFood => "gather-wild",
            Strategy::ScavengeWhatIsLyingAbout => "scavenge",
            Strategy::WalkTheTrapline => "walk-the-line",
            Strategy::FishLocalWaters => "fish",
            Strategy::HuntLocalAnimals => "hunt",
            Strategy::TradeForFood => "trade-for-food",
            Strategy::StealFood => "steal-food",
            Strategy::RequestCommunalAllocation => "ask-for-a-share",
            Strategy::ProcessStoredRawFood => "process-raw",

            Strategy::UseOwnedShelter => "own-shelter",
            Strategy::UseHouseholdShelter => "household-shelter",
            Strategy::ShareCommunalShelter => "share-shelter",
            Strategy::RepairDamagedShelter => "mend-shelter",
            Strategy::BuildTemporaryShelter => "build-temporary",
            Strategy::BuildDurableShelter => "build-durable",
            Strategy::RelocateToNaturalShelter => "natural-shelter",
        }
    }

    /// The element the pattern layer learns this by.
    pub fn as_element(&self) -> Element {
        Element::By(self.called().to_string())
    }

    /// How far ahead this one pays off.
    pub fn horizon(&self) -> Horizon {
        match self {
            // Drink now, eat now, get inside now.
            Strategy::ConsumeCarriedWater
            | Strategy::DrinkFromLocalSource
            | Strategy::EatCarriedFood
            | Strategy::EatStoredFood
            // Gathering is not preparation here, whatever it is elsewhere.
            // `food_action` walks to a bush within reach and eats what it
            // picks, so the payoff is this turn - and calling it short-term
            // cost a settlement three quarters of its person-days, because a
            // starving man was left with nothing but his own empty pack.
            | Strategy::GatherWildFood
            | Strategy::ScavengeWhatIsLyingAbout
            | Strategy::UseOwnedShelter
            | Strategy::UseHouseholdShelter
            | Strategy::ShareCommunalShelter
            | Strategy::RelocateToNaturalShelter => Horizon::Immediate,

            // A trip, a cast, a stalk, a bargain: this afternoon or tomorrow.
            Strategy::FetchFromKnownSource
            | Strategy::AskOrFollowAnotherToWater
            | Strategy::ExploitRainCatchment
            | Strategy::WalkTheTrapline
            | Strategy::FishLocalWaters
            | Strategy::HuntLocalAnimals
            | Strategy::TradeForFood
            | Strategy::StealFood
            | Strategy::RequestCommunalAllocation
            | Strategy::ProcessStoredRawFood
            | Strategy::RepairDamagedShelter
            | Strategy::BuildTemporaryShelter => Horizon::ShortTerm,

            // Work that answers the need for a season.
            Strategy::DigOrRepairWell
            | Strategy::RelocateTowardWater
            | Strategy::BuildDurableShelter => Horizon::LongTerm,
        }
    }

    /// Whether this world can carry it out yet, and what is missing if not.
    pub fn reach(&self) -> Reach {
        match self {
            Strategy::ConsumeCarriedWater
            | Strategy::DrinkFromLocalSource
            | Strategy::FetchFromKnownSource
            | Strategy::EatCarriedFood
            | Strategy::EatStoredFood
            | Strategy::GatherWildFood
            | Strategy::ScavengeWhatIsLyingAbout
            | Strategy::WalkTheTrapline
            | Strategy::FishLocalWaters
            | Strategy::HuntLocalAnimals
            | Strategy::TradeForFood
            | Strategy::UseOwnedShelter
            | Strategy::UseHouseholdShelter
            | Strategy::ShareCommunalShelter
            | Strategy::RelocateToNaturalShelter => Reach::Now,

            Strategy::AskOrFollowAnotherToWater => {
                Reach::NotYet("nobody can be followed: knowing where somebody \
                    is going is not something an agent can ask about")
            }
            Strategy::ExploitRainCatchment => {
                Reach::NotYet("rain falls but nothing catches it - there is no \
                    vessel left out and no roof that runs off")
            }
            Strategy::DigOrRepairWell => {
                Reach::NotYet("there are no wells: the water table is not \
                    modelled, so there is nowhere to sink one")
            }
            Strategy::RelocateTowardWater => {
                Reach::NotYet("moving camp never fires - see ISSUES_FOUND #237")
            }
            Strategy::StealFood => {
                Reach::NotYet("theft exists but sits at the tail of a chain \
                    almost nobody reaches - see ISSUES_FOUND #226")
            }
            Strategy::RequestCommunalAllocation => {
                Reach::NotYet("there is no settlement object to ask - see \
                    ISSUES_FOUND #11")
            }
            Strategy::ProcessStoredRawFood => {
                Reach::NotYet("portioning is a making, and is chosen by the \
                    Utility arm rather than by hunger")
            }
            Strategy::RepairDamagedShelter => {
                Reach::NotYet("a building has a condition and nothing mends \
                    one, so there is no repair to choose")
            }
            Strategy::BuildTemporaryShelter | Strategy::BuildDurableShelter => {
                Reach::NotYet("building is answered by Construction rather than \
                    by Shelter, and the two do not yet meet")
            }
        }
    }

    /// The ways of answering a need, in the order somebody wrote them.
    ///
    /// The order is no longer the ranking - `Utility` is - but it still breaks
    /// ties, and ties are most comparisons on a body that has learned nothing
    /// and can see nothing to tell two ways apart.
    pub fn all_for(need: DriveType) -> &'static [Strategy] {
        match need {
            DriveType::Thirst => &[
                Strategy::ConsumeCarriedWater,
                Strategy::DrinkFromLocalSource,
                Strategy::FetchFromKnownSource,
                Strategy::AskOrFollowAnotherToWater,
                Strategy::ExploitRainCatchment,
                Strategy::DigOrRepairWell,
                Strategy::RelocateTowardWater,
            ],
            // The order is the one the old arm was measured into, not a
            // tidier one. **The store sits behind the ordinary food branch**,
            // and that placement was measured both ways: in front, the store
            // is drawn on five times as often and the rot in the pits halves -
            // and it costs a fifth of all the food anybody eats and six of the
            // people in a settlement, because a meal out of a hole costs two
            // turns where a berry costs one, and because everything taken out
            // was put back in by somebody a day earlier. See ISSUES_FOUND #43.
            //
            // Writing it second here, which is what I did first, cost the
            // month-nine population 0.4 and 0.7 of a body across two blocks.
            DriveType::Hunger => &[
                Strategy::EatCarriedFood,
                Strategy::GatherWildFood,
                Strategy::WalkTheTrapline,
                Strategy::EatStoredFood,
                Strategy::ScavengeWhatIsLyingAbout,
                Strategy::FishLocalWaters,
                Strategy::HuntLocalAnimals,
                Strategy::TradeForFood,
                Strategy::StealFood,
                Strategy::RequestCommunalAllocation,
                Strategy::ProcessStoredRawFood,
            ],
            DriveType::Shelter => &[
                Strategy::UseOwnedShelter,
                Strategy::UseHouseholdShelter,
                Strategy::ShareCommunalShelter,
                Strategy::RepairDamagedShelter,
                Strategy::BuildTemporaryShelter,
                Strategy::BuildDurableShelter,
                Strategy::RelocateToNaturalShelter,
            ],
            _ => &[],
        }
    }

    /// Every way declared, for counting what is reachable and what is not.
    pub fn every_one() -> impl Iterator<Item = Strategy> {
        [DriveType::Thirst, DriveType::Hunger, DriveType::Shelter]
            .into_iter()
            .flat_map(|need| Strategy::all_for(need).iter().copied())
    }
}

/// What a way is worth, and what it costs to take.
///
/// ```text
/// utility = relief - time - effort - danger - wear - uncertainty
/// ```
///
/// **All six terms are in one currency: drive demand.** A formula that
/// subtracts turns from demand and energy from both is arithmetic on three
/// different things and means nothing, so every cost is converted before it is
/// taken off - see the `WHAT_*_COSTS` constants. That the currency is drive
/// demand rather than anything else is not arbitrary either: it is what the
/// pattern layer is already denominated in, and #188 is a long note about what
/// happens when one part of this model quietly starts keeping a second set of
/// books.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Utility {
    /// What answering it would take off the drive, if it comes off at all.
    pub relief: f32,
    /// Turns it takes, walking included.
    pub turns: f32,
    /// Energy it burns.
    pub effort: f32,
    /// What it is expected to cost, on this or any other drive: the sum of
    /// what these elements have cost this agent before. See
    /// `Patterns::what_i_dread`.
    pub danger: f32,
    /// How much of a tool's life it spends.
    pub wear: f32,
    /// How sure he is it will work at all, from nought to one. Not a cost in
    /// itself - it discounts the relief, which is the honest way to price not
    /// knowing: a half-believed mouthful is worth half a mouthful.
    pub confidence: f32,
}

impl Utility {
    /// What a turn costs, in the demand it could have taken off something else.
    ///
    /// A turn is the scarcest thing anybody in this world has - #182's whole
    /// finding was that an errand costs the walk as well as the work, and
    /// nothing was charging for the walk.
    pub const WHAT_A_TURN_COSTS: f32 = 0.05;

    /// What a point of energy costs, in the same money.
    ///
    /// Low, because energy comes back with a meal and a night's sleep, and
    /// pricing it near a turn had agents refusing to work.
    pub const WHAT_EFFORT_COSTS: f32 = 0.004;

    /// What spending a unit of a tool's life costs.
    ///
    /// A tool is several turns of making, so wearing one out is those turns
    /// deferred rather than avoided.
    pub const WHAT_WEAR_COSTS: f32 = 0.02;

    /// The score, as the specification writes it.
    pub fn score(&self) -> f32 {
        let uncertainty = self.relief * (1.0 - self.confidence.clamp(0.0, 1.0));

        self.relief
            - self.turns * Self::WHAT_A_TURN_COSTS
            - self.effort * Self::WHAT_EFFORT_COSTS
            - self.danger
            - self.wear * Self::WHAT_WEAR_COSTS
            - uncertainty
    }

    /// A way that costs nothing but a turn and is certain to work.
    ///
    /// The starting point every particular way adjusts from, so that adding a
    /// way means saying how it *differs* rather than restating six numbers.
    pub fn a_sure_thing(relief: f32) -> Self {
        Utility {
            relief,
            turns: 1.0,
            effort: 0.0,
            danger: 0.0,
            wear: 0.0,
            confidence: 1.0,
        }
    }
}

/// What a strategy came back with: the action, the way, and what it was worth.
pub struct TheWayItWasDone {
    pub doing: Action,
    pub by: Strategy,
    pub worth: Utility,
}

impl crate::analytics::Simulation {
    /// How hard a need has to press before nothing but an immediate answer
    /// will do.
    ///
    /// Above this a man drinks; below it he may go and fill the skin, or mend
    /// the roof against next week. This is the whole of "satisfy thirst
    /// immediately, then improve future water security" - and it is a gate
    /// rather than a term in the score, because a cost that is subtracted can
    /// always be outweighed and this must not be.
    pub(in crate::analytics) const WHEN_ONLY_NOW_WILL_DO: f32 = 0.6;

    /// And how hard it has to press before the long work is set aside too.
    pub(in crate::analytics) const WHEN_A_SEASON_IS_TOO_FAR_OFF: f32 = 0.3;

    /// Every way that is open to this agent now, dearest first.
    ///
    /// Reachable, on a horizon he can afford, preconditions met, and costed.
    /// The list rather than the winner, because three different rules want to
    /// pick out of it - see `the_way_to_answer`.
    pub(in crate::analytics) fn the_ways_open(
        &self,
        need: DriveType,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Vec<(Strategy, Action, Utility)> {
        let ways = Strategy::all_for(need);
        if ways.is_empty() {
            return Vec::new();
        }

        let pressing = agent.how_hard_it_presses(need);
        let as_far_ahead_as_he_can_afford = if pressing >= Self::WHEN_ONLY_NOW_WILL_DO {
            Horizon::Immediate
        } else if pressing >= Self::WHEN_A_SEASON_IS_TOO_FAR_OFF {
            Horizon::ShortTerm
        } else {
            Horizon::LongTerm
        };

        // **The horizon is a preference, not a prohibition, and getting that
        // wrong cost three quarters of everything.** Built as a hard filter it
        // could empty the list: a starving man with an empty pack and no store
        // had no immediate way of eating, so hunger answered nothing at all and
        // he stood beside a berry bush until he died. Measured, 32 worlds:
        // person-days 105,429 to 24,846, population at month three 11.0 to 1.5,
        // every world emptied.
        //
        // "A man dying of thirst does not dig a well" is a thing to say about a
        // man who has water to drink. With nothing nearer, digging is what
        // there is. So the near horizon is tried first and the reach widens
        // until something is open.
        let mut open = Vec::new();
        for horizon in [Horizon::Immediate, Horizon::ShortTerm, Horizon::LongTerm] {
            if horizon > as_far_ahead_as_he_can_afford && !open.is_empty() {
                break;
            }
            open = ways
                .iter()
                .filter(|way| matches!(way.reach(), Reach::Now))
                .filter(|way| way.horizon() <= horizon)
                .filter_map(|way| {
                    self.can_this_way_be_taken(*way, agent, agent_position)
                        .map(|doing| {
                            let worth = self.what_this_way_is_worth(*way, need, agent, &doing);
                            (*way, doing, worth)
                        })
                })
                .collect();

            if !open.is_empty() && horizon >= as_far_ahead_as_he_can_afford {
                break;
            }
        }

        // Best score first. `sort_by` is stable, so the written order breaks
        // ties - which is what decides between two ways nothing can tell apart.
        //
        // **What the sort is worth was measured, and the answer is: nothing
        // either way.** Ranking against taking the ways in written order, two
        // blocks of 32 worlds over two years, against the hand-ordered ladder
        // this replaced:
        //
        // |                | person-days       | emptied | month nine |
        // |----------------|-------------------|---------|------------|
        // | the old arm    | 105,429 / 106,431 | 28 / 28 | 8.2 / 8.5  |
        // | ranked         | 105,746 / 103,600 | 25 / 27 | 7.8 / 7.8  |
        // | written order  | 108,396 / 102,780 | 24 / 28 | 8.2 / 7.6  |
        //
        // Written order won the first block and lost the second by as much. Over
        // all 64 worlds the two come to 209,346 and 211,176 person-days, which
        // is a spread of under one per cent on a measure whose block-to-block
        // noise is about ten. So the sort is kept, because the specification
        // asks for the ways to be priced and chosen on the price, and because
        // nothing measured argues against it.
        //
        // It is kept knowing what it currently does, which is less than it
        // looks. Within one need every way relieves the same need by the same
        // amount, so `relief` is common to all of them and cancels; what is
        // left to tell two ways apart is the cost spread, and the whole spread
        // from reaching into your own pack to going hunting is about a fifth of
        // a point. The uncertainty term is `relief * (1 - confidence)` and runs
        // to a full one. **The price of everything the ways differ in is a
        // fifth of the price of the one thing they do not**, so this sort is
        // very nearly a sort on `Lessons`' confidence with the walk and the
        // work as rounding error. Dropping the doubt term and ranking on cost
        // alone was measured too - 107,982 / 102,270 - and is the same nothing.
        //
        // Making the price mean something is denominating a walk and a doubt in
        // one currency, and that is its own piece of work with its own
        // measurement. `a_doubt_outweighs_every_cost_put_together` holds the
        // arithmetic still until it is done.
        open.sort_by(|(_, _, left), (_, _, right)| {
            right
                .score()
                .partial_cmp(&left.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        open
    }

    /// Which way this agent answers this need, this turn.
    ///
    /// Four rules over the open ways, in this order, and each one is here
    /// because something was measured:
    ///
    /// 1. **Anything already in his hand.** A search is for finding out whether
    ///    the walk you keep taking is worth taking; the supper in your own pack
    ///    is not a hypothesis. Without this a frightened man with food on him
    ///    walked past it - see
    ///    `fear_of_running_short_comes_out_as_answering_the_need`.
    /// 2. **The plan**, where there is one for this need. A learned run is held
    ///    across turns, so it can carry an agent through a step that is worth
    ///    nothing on its own - walking to the store buys nothing until you take
    ///    something out of it. See #187.
    /// 3. **The run**, where one is worn deep enough. This is the reader for
    ///    `Element::Then` and the whole use of keeping runs. See #188.
    /// 4. **Utility**, and a share of turns on the next one down.
    ///
    /// The first three used to live inside the hunger arm and nowhere else,
    /// which meant thirst and shelter had no way of holding a plan or following
    /// a run. They are not about hunger; they are about choosing among things
    /// you could do, which is what this layer is. What changed when they moved
    /// is that they now pick out of a list ranked by cost rather than one
    /// ranked by where a line sits in a file.
    ///
    /// `None` when the drive has no ways declared, or none of them can be
    /// taken - and the drive's own arm answers as it always did.
    pub(in crate::analytics) fn the_way_to_answer(
        &self,
        need: DriveType,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<TheWayItWasDone> {
        let mut open = self.the_ways_open(need, agent, agent_position);
        if open.is_empty() {
            return None;
        }

        let take = |open: &mut Vec<(Strategy, Action, Utility)>, at: usize| {
            let (by, doing, worth) = open.swap_remove(at);
            Some(TheWayItWasDone { doing, by, worth })
        };

        // 1. What is already in his hand.
        if let Some(at) = open
            .iter()
            .position(|(_, doing, _)| Self::is_it_already_in_his_hand(doing))
        {
            return take(&mut open, at);
        }

        // 2. The plan, where one is running for this need.
        if agent.is_the_plan_for(need) && agent.should_execute_plan() {
            if let Some(step) = agent.what_the_plan_wants_next() {
                if let Some(at) = open
                    .iter()
                    .position(|(_, doing, _)| Self::is_that_the_verb(doing, step))
                {
                    return take(&mut open, at);
                }
            }
        }

        // 3. What he has found usually comes next after what he has just done.
        if let Some(next) = agent.what_usually_comes_next(need) {
            if let Some(at) = open
                .iter()
                .position(|(_, doing, _)| Self::is_that_the_verb(doing, next))
            {
                return take(&mut open, at);
            }
            // Nothing the run names is open now, so he falls back to the best
            // of what is - not to the tail. The extra ways were costed to give
            // the run a choice, not to change what he does when it has none.
            return take(&mut open, 0);
        }

        // 4. The best of them, and a share of turns on the next one down.
        //
        // In the hunger arm this was a *truncation* - gather only as many
        // candidates as you mean to consider - which worked because the list
        // was in a fixed order. Ranked by cost there is nothing to truncate, so
        // the same rule reads as taking the second best instead of the best,
        // which is what it always meant.
        let past_the_habit = Self::how_far_down_the_list_to_look(agent, need);
        let taken = past_the_habit.min(open.len() - 1);
        take(&mut open, taken)
    }

    /// What this way is worth to this agent, here, now.
    ///
    /// The six terms, each from the thing in the model that already knows it,
    /// rather than from a table of guesses: dread from the trails, confidence
    /// from the lessons, turns from the distance actually being walked.
    pub(in crate::analytics) fn what_this_way_is_worth(
        &self,
        way: Strategy,
        need: DriveType,
        agent: &crate::agents::Agent,
        doing: &Action,
    ) -> Utility {
        // What it would take off the drive if it works. A mouthful is a
        // mouthful whichever way it was got, so this is about the need rather
        // than the way - the ways differ in what they cost, not in what water
        // does once it is drunk.
        let mut worth = Utility::a_sure_thing(agent.how_hard_it_presses(need).min(1.0));

        // The walk. This is the term #193 is about: an errand costs the walk as
        // well as the work, and until it was priced a trip across the map
        // looked exactly as cheap as reaching into the pack.
        if let Action::Move { target } = doing {
            let here = agent.state.position;
            let steps = (target.0 - here.0).abs().max((target.1 - here.1).abs());
            worth.turns += steps as f32;
        }

        // What it is expected to cost, on any drive, going by what it has cost
        // before. Nought for a way nobody has been hurt by.
        worth.danger = agent
            .patterns
            .what_i_dread(need, std::slice::from_ref(&way.as_element()));

        // And how sure he is of it. `Lessons` is keyed on what was tried, so a
        // way that has been refused over and over is discounted here rather
        // than being struck off - `NEVER_QUITE_GIVES_UP` is why a man tries
        // again on the afternoon the world has changed.
        let tried = crate::agents::Agent::what_was_tried(doing);
        worth.confidence = agent.lessons.how_likely_to_try_this(&tried);

        // The particular costs of the particular way.
        match way {
            // Reaching into your own pack. No walk and nothing spent.
            Strategy::ConsumeCarriedWater | Strategy::EatCarriedFood => {}

            // Bending to the river. A turn, and no more.
            Strategy::DrinkFromLocalSource => {}

            // Hunting is the dear one and the model already says so: slow,
            // effortful, and refused far more often than it succeeds.
            Strategy::HuntLocalAnimals => {
                worth.effort += 12.0;
                worth.wear += 1.0;
            }
            Strategy::FishLocalWaters => {
                worth.effort += 6.0;
                worth.wear += 1.0;
            }
            Strategy::GatherWildFood | Strategy::ScavengeWhatIsLyingAbout => {
                worth.effort += 2.0;
            }
            // The round is a walk with several stops on it, and it is charged
            // for as one - the turns come from the `Move` above.
            Strategy::WalkTheTrapline => {
                worth.effort += 3.0;
            }
            Strategy::FetchFromKnownSource => {
                worth.effort += 2.0;
            }
            // Taking something out of a hole and putting the lid back.
            Strategy::EatStoredFood => {
                worth.turns += 1.0;
            }
            _ => {}
        }

        worth
    }

    /// Whether this way can be taken at all, and what it comes to if so.
    ///
    /// The preconditions and the action in one place, because they are the same
    /// question asked twice: a way that cannot name what to do now has not met
    /// its preconditions, whatever else is true of it.
    ///
    /// Every arm here delegates to the helper the old ladder already used, so
    /// that naming the ways did not fork the model into two opinions about what
    /// a drink or a meal is. What is new is that they are asked *separately*
    /// and costed, rather than being tried in order until one answers.
    pub(in crate::analytics) fn can_this_way_be_taken(
        &self,
        way: Strategy,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        match way {
            // ---- hydration --------------------------------------------
            //
            // Whole units only. `Gather` drinks from a waterskin when there is
            // no source about, but not in dribbles, so an agent with half a
            // mouthful left kept choosing to drink and being told there was no
            // water anywhere - which was the largest single failure in the
            // simulation.
            Strategy::ConsumeCarriedWater => (agent.inventory.available_water() >= 1.0)
                .then(|| Action::Gather {
                    resource_type: "water".to_string(),
                }),

            Strategy::DrinkFromLocalSource => self
                .drinkable_water_within_reach(agent, agent_position)
                .then(|| Action::Gather {
                    resource_type: "water".to_string(),
                }),

            Strategy::FetchFromKnownSource => {
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

            // ---- hunger -----------------------------------------------
            Strategy::EatCarriedFood => self.a_catch_at_my_feet(agent, agent_position),
            Strategy::EatStoredFood => self.something_out_of_the_store(agent, agent_position),
            // Whether he is desperate is a fact about the man, so it is asked
            // here rather than passed in from four levels up.
            Strategy::GatherWildFood => {
                let starving = agent.state.is_starving() || agent.nutrition.is_starving();
                self.food_action(agent, agent_position, starving)
            }
            Strategy::WalkTheTrapline => self.going_round_is_due(agent, agent_position),
            Strategy::ScavengeWhatIsLyingAbout => {
                self.walking_to_a_catch(agent, agent_position)
            }
            Strategy::FishLocalWaters => self.fishing_action(agent, agent_position),
            Strategy::HuntLocalAnimals => self.hunting_action(agent, agent_position),
            // **Never once fires.** Dropping this arm entirely and running 32
            // worlds for two years gave a byte-identical result - the same
            // person-days, the same worlds emptied on the same days - so
            // `somebody_to_trade_with` never answers for a hungry man in a live
            // settlement. It is left here rather than marked `NotYet`, because
            // the machinery is all present and something upstream of it is not;
            // finding out what is the work, and a way that is named and silent
            // is a thing somebody can go and count. Compare #226, where theft
            // sits at the tail of a chain almost nobody reaches.
            Strategy::TradeForFood => self
                .somebody_to_trade_with(agent, agent_position)
                .map(|with| Action::Trade { with }),

            // ---- shelter ----------------------------------------------
            //
            // Four ways of getting under a roof, told apart by whose it is.
            // They all come out as `Action::SeekShelter`, which walks its own
            // way to cover, so **which one fires does not change where he
            // goes** - it changes what goes in the record. That is the point:
            // `Element::By` can now hold "own-shelter" against
            // "household-shelter", and the pattern layer can find out that one
            // of them keeps working and the other stops when a brother dies.
            // Before this, all three were one arm and there was nothing to
            // tell apart.
            //
            // The union of the four is exactly what the single arm accepted -
            // a completed building or a wood - so no cover is taken from
            // anybody by naming it. See `world::belonging`.
            Strategy::UseOwnedShelter
            | Strategy::UseHouseholdShelter
            | Strategy::ShareCommunalShelter
            | Strategy::RelocateToNaturalShelter => {
                use crate::analytics::wanting::shelter::WhoseRoof;

                let worth_going_in = agent.needs_shelter()
                    || agent.body_temperature.is_too_cold()
                    || agent.surroundings.foul_weather;

                if !worth_going_in || agent.surroundings.under_shelter {
                    return None;
                }

                let which = match way {
                    Strategy::UseOwnedShelter => WhoseRoof::HisOwn,
                    Strategy::UseHouseholdShelter => WhoseRoof::AKinsmans,
                    Strategy::ShareCommunalShelter => WhoseRoof::TheSettlements,
                    _ => WhoseRoof::Natural,
                };

                self.nearest_roof_of_this_kind(agent, agent_position, which)
                    .map(|_| Action::SeekShelter)
            }

            // Declared and out of reach. `the_way_to_answer` filters these out
            // before asking, so this arm is the belt to that braces.
            _ => None,
        }
    }
}
