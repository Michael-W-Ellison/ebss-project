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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A way of working that has to be discovered rather than known
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    confidence: HashMap<Practice, f32>,
    /// How many times it has tried it
    attempts: HashMap<Practice, u32>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Undertaking {
    /// Going after an animal
    Hunting,
    /// Standing in a river after fish
    Fishing,
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
    belief: HashMap<Undertaking, f32>,
    /// How many times it has been tried
    attempts: HashMap<Undertaking, u32>,
    /// How many of those went well
    successes: HashMap<Undertaking, u32>,
    /// And the same record kept on each particular thing attempted, which is
    /// what an agent actually decides on - see `record_particular`
    #[serde(default)]
    particular: HashMap<String, f32>,
    #[serde(default)]
    particular_attempts: HashMap<String, u32>,
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

    pub fn how_likely_to_try(&self, undertaking: Undertaking) -> f32 {
        // The benefit of the doubt, until there is a record worth reading
        if self.attempts(undertaking) < Self::ENOUGH_TO_JUDGE {
            return Self::NEVER_QUITE_CERTAIN;
        }

        self.belief(undertaking)
            .clamp(Self::NEVER_QUITE_GIVES_UP, Self::NEVER_QUITE_CERTAIN)
    }

    /// Whether this agent will try it this time.
    pub fn will_try_again(&self, undertaking: Undertaking) -> bool {
        use rand::Rng;
        rand::thread_rng().gen_bool(self.how_likely_to_try(undertaking) as f64)
    }


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
        *self.particular_attempts.entry(what).or_insert(0) += 1;
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
        rand::thread_rng().gen_bool(self.how_likely_to_try_this(what) as f64)
    }

    /// How many times this particular thing has been tried.
    pub fn tried_this(&self, what: &str) -> u32 {
        self.particular_attempts.get(what).copied().unwrap_or(0)
    }

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
