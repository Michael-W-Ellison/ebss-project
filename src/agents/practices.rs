// src/agents/practices.rs
//! Ways of working that an agent has come to believe in.
//!
//! Not everything an agent does should be something it was born knowing. Some
//! things are worked out: somebody tips a basket of spoiled food onto a field
//! because they were curious and it was in the way, notices the following
//! season that the ground there is darker and the crop heavier, does it again,
//! and the people who watched them do it start doing it too.
//!
//! This is the record of that. A practice starts unproven; an agent will try an
//! unproven practice occasionally, out of curiosity, and what happens next
//! decides whether it does it again. Watching somebody else do it counts for
//! something too, though less than doing it yourself - which is the difference
//! between being told a thing works and finding out.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A way of working that has to be discovered rather than known
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Practice {
    /// Carrying spoiled food, bones and refuse onto a field, on the theory
    /// that it does the ground good
    SpreadingMuck,
    /// Breaking ground and putting seed in it on purpose, rather than walking
    /// to wherever food happens to be growing.
    ///
    /// Nobody starts out knowing this. What teaches it is the midden: a people
    /// that eats fruit and voids the pips in one place walks past, one season
    /// on, the same plants coming up out of their own refuse. Seeing that is
    /// what connects "seed in ground" to "food later", and until somebody has
    /// seen it, breaking ground is a strange thing to do with a day.
    Farming,
}

/// What an agent believes about the practices it knows of
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Practices {
    /// How sure the agent is that a practice is worth the trouble, 0.0 to 1.0
    confidence: BTreeMap<Practice, f32>,
    /// How many times it has tried it
    attempts: BTreeMap<Practice, u32>,
}

impl Practices {
    /// Confidence above which an agent stops experimenting and simply does the
    /// thing as a matter of course
    pub const ESTABLISHED: f32 = 0.5;

    /// How often an agent tries something it has no opinion about, per
    /// opportunity, before curiosity is taken into account
    const BASE_CURIOSITY: f32 = 0.05;

    pub fn new() -> Self {
        Self::default()
    }

    /// How sure the agent is about this practice
    pub fn confidence(&self, practice: Practice) -> f32 {
        self.confidence.get(&practice).copied().unwrap_or(0.0)
    }

    /// How many times it has tried it
    pub fn attempts(&self, practice: Practice) -> u32 {
        self.attempts.get(&practice).copied().unwrap_or(0)
    }

    /// Whether this is settled practice for this agent
    pub fn is_established(&self, practice: Practice) -> bool {
        self.confidence(practice) >= Self::ESTABLISHED
    }

    /// Whether the agent gives this a go on an opportunity it has now.
    ///
    /// A settled practice is simply done. An unproven one is tried now and
    /// again, more often by a curious agent and by one that has already had it
    /// half work. Something tried repeatedly and found useless is dropped.
    pub fn would_try(&self, practice: Practice, curiosity: f32, roll: f32) -> bool {
        if self.is_established(practice) {
            return true;
        }

        let belief = self.confidence(practice);

        // Given up on: tried a good few times and it never came to anything
        if self.attempts(practice) >= 6 && belief <= 0.05 {
            return false;
        }

        let appetite = Self::BASE_CURIOSITY * (0.5 + curiosity.clamp(0.0, 1.0)) + belief * 0.4;

        roll < appetite
    }

    /// Record how a try turned out.
    ///
    /// Trial and error, and error counts: something that does nothing loses
    /// ground faster than something that works gains it, which is why a
    /// practice has to earn its place several times over.
    pub fn record_outcome(&mut self, practice: Practice, worked: bool) {
        *self.attempts.entry(practice).or_insert(0) += 1;

        let belief = self.confidence.entry(practice).or_insert(0.0);

        if worked {
            *belief = (*belief + 0.2).min(1.0);
        } else {
            *belief = (*belief - 0.1).max(0.0);
        }
    }

    /// Watching somebody else do it.
    ///
    /// Worth something, and less than doing it: seeing a thing done tells you
    /// it is done, not that it works.
    pub fn learn_from_watching(&mut self, practice: Practice) {
        let belief = self.confidence.entry(practice).or_insert(0.0);
        *belief = (*belief + 0.06).min(1.0);
    }

    /// Seeing the thing itself happen, rather than seeing somebody do it.
    ///
    /// There is a difference between watching a man tip a basket of refuse on
    /// a field and walking past that field a season later to find it thick
    /// with what he threw away. The second is the outcome, not the gesture,
    /// and it moves an agent a great deal further than hearsay does: two such
    /// sights and the thing is settled practice.
    pub fn saw_it_work(&mut self, practice: Practice) {
        let belief = self.confidence.entry(practice).or_insert(0.0);
        *belief = (*belief + Self::WHAT_SEEING_IT_IS_WORTH).min(1.0);
    }

    /// How far one sighting of the outcome moves an agent
    const WHAT_SEEING_IT_IS_WORTH: f32 = 0.3;

    /// Every practice this agent has an opinion about
    pub fn known(&self) -> impl Iterator<Item = (&Practice, &f32)> {
        self.confidence.iter()
    }
}

/// A kind of undertaking an agent can get better or worse at judging.
///
/// Coarser than an `Action`: what an agent learns is not "that particular
/// rabbit got away" but "going after animals does not work out for me". The
/// grain has to be coarse or nothing ever accumulates enough instances to be
/// a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Undertaking {
    /// Going after an animal
    Hunting,
    /// Standing in a river after fish
    Fishing,
    /// Setting a line of snares and going round it.
    ///
    /// Kept apart from `Hunting` on purpose, and it is the same argument that
    /// keeps `Fleeing` apart from `Fighting`: they are not the same lesson. A
    /// man who cannot get within a spear's throw of a deer may still be the
    /// best trapper in the settlement, and folding the two together would
    /// teach him he cannot feed himself.
    Trapping,
    /// Standing your ground against something
    Fighting,
    /// Getting away from something. Not the same thing as fighting it, and
    /// deliberately kept apart from it: a man who has run from four wolves
    /// and lived has learnt that running works, not that he can beat a wolf.
    Fleeing,
    /// Putting food over a fire
    Cooking,
    /// Breaking ground, sowing, spreading muck
    Farming,
    /// Making or wearing something
    Clothing,
    /// Going and getting something off the land
    Foraging,
    /// Building
    Building,
    /// Making a thing out of other things
    Crafting,
    /// Dealing with other people
    Dealing,
    /// Giving somebody something for what ails them.
    ///
    /// Its own undertaking rather than a corner of `Foraging`, and for the
    /// reason `Trapping` is kept out of `Hunting`: picking a herb and dosing
    /// a sick man are two different things to be good at, and a settlement's
    /// herbalist is usually not its best forager.
    Healing,
}

/// What an agent has found out about the things it does.
///
/// The behaviour-tree weights were meant to be this and never had a caller, so
/// nothing an agent did ever changed what it did next. This is the record that
/// does: every attempt is counted, and what came of it moves a running belief
/// about whether that kind of undertaking pays for this particular agent.
///
/// Deliberately slow to move on any single result - one lucky hunt should not
/// make somebody a hunter - and asymmetric: a failure counts for rather more
/// than a success, because the cost of persisting with something that does not
/// work is paid in a currency an agent has little of.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lessons {
    /// Running belief that this kind of thing works out, 0.0 to 1.0
    belief: BTreeMap<Undertaking, f32>,
    /// How many times it has been tried
    attempts: BTreeMap<Undertaking, u32>,
    /// How many of those went well
    successes: BTreeMap<Undertaking, u32>,
    /// And the same record kept on each particular thing attempted, which is
    /// what an agent actually decides on - see `record_particular`
    #[serde(default)]
    particular: BTreeMap<String, f32>,
    #[serde(default)]
    particular_attempts: BTreeMap<String, u32>,
    /// How many of those went well. The running belief above is an opinion,
    /// slow to move and asymmetric on purpose; this is the plain count, and a
    /// plain count is what one circumstance has to be compared against
    /// another with.
    #[serde(default)]
    particular_successes: BTreeMap<String, u32>,
    /// And the same count kept separately for each circumstance the thing was
    /// attempted under - see `Circumstance`. Nested rather than keyed by a
    /// pair so that it survives a round trip through a format whose map keys
    /// are strings.
    #[serde(default)]
    under: BTreeMap<String, BTreeMap<Circumstance, Tally>>,
}

impl Lessons {
    /// What an agent assumes about something it has never tried: enough to be
    /// worth one attempt, not enough to build a life around
    pub const UNTRIED: f32 = 0.5;

    /// How far one good outcome moves the belief
    const LEARNED_FROM_SUCCESS: f32 = 0.06;

    /// And one bad one, which counts for more
    const LEARNED_FROM_FAILURE: f32 = 0.10;

    /// How many attempts before an agent trusts its own record over the
    /// benefit of the doubt
    const ENOUGH_TO_JUDGE: u32 = 5;

    pub fn new() -> Self {
        Self::default()
    }

    /// How well this agent thinks this kind of thing goes for it
    pub fn belief(&self, undertaking: Undertaking) -> f32 {
        self.belief.get(&undertaking).copied().unwrap_or(Self::UNTRIED)
    }

    /// How many times it has tried
    pub fn attempts(&self, undertaking: Undertaking) -> u32 {
        self.attempts.get(&undertaking).copied().unwrap_or(0)
    }

    /// How many of those went well
    pub fn successes(&self, undertaking: Undertaking) -> u32 {
        self.successes.get(&undertaking).copied().unwrap_or(0)
    }

    /// What actually happened, out of how many tries
    pub fn success_rate(&self, undertaking: Undertaking) -> f32 {
        let tries = self.attempts(undertaking);
        if tries == 0 {
            return Self::UNTRIED;
        }
        self.successes(undertaking) as f32 / tries as f32
    }

    /// Record how one attempt turned out.
    pub fn record(&mut self, undertaking: Undertaking, worked: bool) {
        *self.attempts.entry(undertaking).or_insert(0) += 1;
        if worked {
            *self.successes.entry(undertaking).or_insert(0) += 1;
        }

        let belief = self.belief.entry(undertaking).or_insert(Self::UNTRIED);
        *belief = if worked {
            (*belief + Self::LEARNED_FROM_SUCCESS).min(1.0)
        } else {
            (*belief - Self::LEARNED_FROM_FAILURE).max(0.0)
        };
    }

    /// Whether this agent still thinks this is worth its time.
    ///
    /// Untried things get the benefit of the doubt, so nothing is written off
    /// before it has been attempted. Past a handful of attempts the agent goes
    /// on its own record, and something that has gone badly nearly every time
    /// is dropped - which is the difference between an agent that learns and
    /// one that walks into the same wall for its whole life.
    pub fn worth_trying(&self, undertaking: Undertaking) -> bool {
        if self.attempts(undertaking) < Self::ENOUGH_TO_JUDGE {
            return true;
        }

        self.belief(undertaking) > 0.2
    }


    /// How willing this agent is to try something again.
    ///
    /// "When an action fails to satisfy a drive, its odds of repeating should
    /// decrease. Inversely, when an action satisfies a drive, its odds of
    /// repeating should increase."
    ///
    /// `worth_trying` is the same idea as a cliff: everything is worth trying
    /// until the belief crosses a line, and then nothing is. That is the right
    /// shape for "do I set out after a deer" and the wrong one for the general
    /// case, where an agent should keep half an eye on a thing that mostly
    /// fails rather than swearing off it for life.
    ///
    /// Never quite nought and never quite one: a man who has failed at
    /// something forty times still tries it now and again, which is what lets
    /// him find out the world has changed.
    /// A man who has failed at something forty times still tries it now and
    /// again, which is what lets him find out the world has changed. Set to
    /// 0.05 this was not a slackening but a ban: an action whose success needs
    /// the agent to be somewhere particular - cooking needs a fire, and
    /// getting to the fire takes a few turns - was written off before it had
    /// ever had a fair go at working.
    pub const NEVER_QUITE_GIVES_UP: f32 = 0.2;
    pub const NEVER_QUITE_CERTAIN: f32 = 0.95;

    /// And how much of a run of them it takes before an agent will hear it.
    ///
    /// Five is enough to judge whether you are a hunter, which is what the
    /// coarse record is for. It is not enough to judge whether a thing works,
    /// because the first few goes at anything are spent getting into position.
    const A_FAIR_GO: u32 = 12;




    /// What an agent has found out about one particular thing it does.
    ///
    /// The `Undertaking` record is deliberately coarse - it answers "am I a
    /// hunter" - and that is too coarse to act on. Going for water and going
    /// for wood are both `Foraging`, so a settlement whose river had dried up
    /// would learn that foraging does not work and stop gathering food as
    /// well.
    ///
    /// This is the same arithmetic keyed on the thing actually attempted:
    /// `gather:water` is one lesson and `gather:wood` another.
    fn note(&mut self, what: String, worked: bool) {
        let belief = self.particular.entry(what.clone()).or_insert(Self::UNTRIED);
        if worked {
            *belief = (*belief + Self::LEARNED_FROM_SUCCESS).min(1.0);
        } else {
            *belief = (*belief - Self::LEARNED_FROM_FAILURE).max(0.0);
        }
        *self.particular_attempts.entry(what.clone()).or_insert(0) += 1;
        if worked {
            *self.particular_successes.entry(what).or_insert(0) += 1;
        }
    }

    /// Note how one particular attempt turned out.
    pub fn record_particular(&mut self, what: &str, worked: bool) {
        self.note(what.to_string(), worked);
    }

    /// How willing this agent is to try this particular thing again.
    pub fn how_likely_to_try_this(&self, what: &str) -> f32 {
        let tried = self.particular_attempts.get(what).copied().unwrap_or(0);
        if tried < Self::A_FAIR_GO {
            return Self::NEVER_QUITE_CERTAIN;
        }
        self.particular
            .get(what)
            .copied()
            .unwrap_or(Self::UNTRIED)
            .clamp(Self::NEVER_QUITE_GIVES_UP, Self::NEVER_QUITE_CERTAIN)
    }

    /// Whether it will bother this time.
    pub fn will_try_this_again(&self, what: &str) -> bool {
        use rand::Rng;
        crate::core::dice::roll().gen_bool(self.how_likely_to_try_this(what) as f64)
    }

    /// How many times this particular thing has been tried.
    pub fn tried_this(&self, what: &str) -> u32 {
        self.particular_attempts.get(what).copied().unwrap_or(0)
    }

    /// Note how one particular attempt turned out, and what the world was
    /// doing at the time.
    ///
    /// The circumstances are not a description of the attempt: nobody decides
    /// to dry fish *in the sun*, they decide to dry fish, and the sun is
    /// simply what the sky happened to be doing. Writing them down against the
    /// attempt anyway is the whole of this: it is what lets an agent find out
    /// afterwards that the sky was the part that mattered.
    pub fn record_particular_here(&mut self, what: &str, worked: bool, here: &[Circumstance]) {
        self.note(what.to_string(), worked);

        if here.is_empty() {
            return;
        }

        // A head is not a filing cabinet, and `what` is open-ended: every
        // resource and every made thing in the world has its own key. Keep the
        // circumstances only for the things this agent actually does, and let
        // the ones it did once and never again fall out.
        if !self.under.contains_key(what) && self.under.len() >= Self::AS_MANY_THINGS_AS_ANYBODY_HOLDS
        {
            self.forget_the_thing_i_have_done_least();
        }

        let against = self.under.entry(what.to_string()).or_default();

        for circumstance in here {
            let tally = against.entry(*circumstance).or_default();
            tally.tried = tally.tried.saturating_add(1);
            if worked {
                tally.worked = tally.worked.saturating_add(1);
            }
        }
    }

    /// Drop the circumstances of whatever this agent has done least of.
    fn forget_the_thing_i_have_done_least(&mut self) {
        let least = self
            .under
            .iter()
            .min_by_key(|(what, _)| self.particular_attempts.get(*what).copied().unwrap_or(0))
            .map(|(what, _)| what.clone());

        if let Some(what) = least {
            self.under.remove(&what);
        }
    }

    /// How much better or worse this thing goes under this circumstance than
    /// it goes in general, or `None` where there is not yet a record worth
    /// reading.
    ///
    /// The comparison is against the agent's own overall record of the same
    /// thing, which is what makes it a lesson about the circumstance rather
    /// than about the thing. A man who has only ever dried fish in the sun
    /// learns nothing from having done it forty times: the sun is all he has
    /// to compare with, and this returns nought for him, correctly. It takes
    /// one wet afternoon to teach him anything at all.
    pub fn what_this_changes(&self, what: &str, here: Circumstance) -> Option<f32> {
        let tried = self.particular_attempts.get(what).copied().unwrap_or(0);
        if tried < Self::ENOUGH_TO_SEE_A_PATTERN {
            return None;
        }

        let tally = self.under.get(what)?.get(&here)?;
        if tally.tried < Self::ENOUGH_TO_SEE_A_PATTERN {
            return None;
        }

        let overall = self.particular_successes.get(what).copied().unwrap_or(0) as f32 / tried as f32;

        Some(tally.rate()? - overall)
    }

    /// How willing this agent is to try this particular thing, here and now.
    ///
    /// What it thinks of the thing in general, moved by whatever it has worked
    /// out about the circumstances it finds itself in. Where it has worked out
    /// nothing this is exactly `how_likely_to_try_this`, which is what every
    /// caller had before there were circumstances at all.
    pub fn how_likely_to_try_this_here(&self, what: &str, here: &[Circumstance]) -> f32 {
        let base = self.how_likely_to_try_this(what);

        let moved: f32 = here
            .iter()
            .filter_map(|circumstance| self.what_this_changes(what, *circumstance))
            .sum();

        (base + moved).clamp(Self::NEVER_QUITE_GIVES_UP, Self::NEVER_QUITE_CERTAIN)
    }

    /// Whether it will bother, here and now.
    pub fn will_try_this_here(&self, what: &str, here: &[Circumstance]) -> bool {
        use rand::Rng;
        crate::core::dice::roll().gen_bool(self.how_likely_to_try_this_here(what, here) as f64)
    }

    /// How many times this thing has been tried under this circumstance.
    pub fn tried_this_here(&self, what: &str, here: Circumstance) -> u32 {
        self.under
            .get(what)
            .and_then(|against| against.get(&here))
            .map(|tally| tally.tried)
            .unwrap_or(0)
    }

    /// Everything this agent has worked out about when things work, strongest
    /// first.
    ///
    /// Nobody wrote any of these down. They are whatever fell out of what this
    /// particular agent happened to do and what happened to come of it, which
    /// is the point: an agent can arrive at "laying fish out works in the sun"
    /// without anybody having thought of fish, or of the sun.
    pub fn what_i_have_worked_out(&self) -> Vec<(&str, Circumstance, f32)> {
        let mut worked_out: Vec<(&str, Circumstance, f32)> = self
            .under
            .keys()
            .flat_map(|what| {
                Circumstance::EVERY_CIRCUMSTANCE
                    .iter()
                    .filter_map(move |circumstance| {
                        let changes = self.what_this_changes(what, *circumstance)?;
                        (changes.abs() >= Self::WORTH_KNOWING)
                            .then_some((what.as_str(), *circumstance, changes))
                    })
            })
            .collect();

        worked_out.sort_by(|a, b| {
            b.2.abs()
                .partial_cmp(&a.2.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        worked_out
    }

    /// How many such things this agent has worked out.
    pub fn how_much_i_have_worked_out(&self) -> usize {
        self.what_i_have_worked_out().len()
    }

    /// How many instances of a circumstance it takes before a difference is a
    /// pattern rather than a run of luck.
    ///
    /// Lower than `A_FAIR_GO`, because this is a comparison between two
    /// records the agent already holds rather than a judgement built from
    /// nothing, and higher than the two or three that would let any pair of
    /// coincidences turn into a rule.
    const ENOUGH_TO_SEE_A_PATTERN: u32 = 8;

    /// How large a difference has to be before it is worth calling a thing
    /// somebody has worked out.
    const WORTH_KNOWING: f32 = 0.15;

    /// How many different things one agent keeps the circumstances of.
    const AS_MANY_THINGS_AS_ANYBODY_HOLDS: usize = 48;

    /// The thing this agent has found works best for it, of those it has tried
    /// enough times to have an opinion about.
    ///
    /// This is the pattern-spotting: not a rule anybody wrote down, just the
    /// undertaking with the best record behind it.
    pub fn what_works_best(&self) -> Option<Undertaking> {
        self.belief
            .iter()
            .filter(|(undertaking, _)| self.attempts(**undertaking) >= Self::ENOUGH_TO_JUDGE)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(undertaking, _)| *undertaking)
    }
}

/// Something that was true of the world at the moment an attempt was made.
///
/// A lesson keyed on the thing attempted - `dry`, `gather:greens`, `hunt` -
/// says whether a thing works. It cannot say *when* a thing works, and for a
/// good half of what this people does that is the only question worth asking.
/// Laying fish out works in the sun and ruins them in the rain; greens are
/// there in the spring and gone by the autumn; firing clay works at a fire and
/// nowhere else. Every one of those had to be written into the code by hand as
/// a rule or a discovery flag, which means an agent can only ever learn about
/// a situation somebody already thought of.
///
/// These are the circumstances instead. They are attached to every attempt
/// automatically, without anybody naming the situation, and what an agent
/// works out is which of them go with a thing working and which go with it
/// failing. Deliberately few and deliberately coarse: a finer set would be a
/// truer description of the afternoon and no agent would ever gather enough
/// instances of any one of them to notice anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Circumstance {
    /// Nothing coming out of the sky and the sun on it
    ClearSky,
    /// Something coming out of the sky
    Raining,
    /// A lit fire close enough to work at
    AFireToHand,
    /// Standing under a roof
    UnderARoof,
    /// Somebody else within sight
    OtherPeopleAbout,
    /// Water within a few paces
    ByWater,
    InSpring,
    InSummer,
    InAutumn,
    InWinter,
}

impl Circumstance {
    pub const EVERY_CIRCUMSTANCE: [Circumstance; 10] = [
        Circumstance::ClearSky,
        Circumstance::Raining,
        Circumstance::AFireToHand,
        Circumstance::UnderARoof,
        Circumstance::OtherPeopleAbout,
        Circumstance::ByWater,
        Circumstance::InSpring,
        Circumstance::InSummer,
        Circumstance::InAutumn,
        Circumstance::InWinter,
    ];

    /// How an agent would put it, if an agent could put things.
    pub fn describe(&self) -> &'static str {
        match self {
            Circumstance::ClearSky => "in the sun",
            Circumstance::Raining => "in the rain",
            Circumstance::AFireToHand => "at a fire",
            Circumstance::UnderARoof => "under a roof",
            Circumstance::OtherPeopleAbout => "with others about",
            Circumstance::ByWater => "by water",
            Circumstance::InSpring => "in spring",
            Circumstance::InSummer => "in summer",
            Circumstance::InAutumn => "in autumn",
            Circumstance::InWinter => "in winter",
        }
    }
}

/// How one thing has gone under one circumstance.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Tally {
    pub tried: u32,
    pub worked: u32,
}

impl Tally {
    fn rate(&self) -> Option<f32> {
        if self.tried == 0 {
            None
        } else {
            Some(self.worked as f32 / self.tried as f32)
        }
    }
}
