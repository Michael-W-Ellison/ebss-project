// src/agents/exploration.rs
//! Agent exploration and map discovery system.
//!
//! Tracks what each agent has discovered about the world including:
//! - Explored tiles (fog of war)
//! - Discovered resources
//! - Discovered buildings
//! - Terrain types encountered

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use crate::world::{Position, TerrainType, ResourceType, BuildingType};

/// Types of discoveries agents can make
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryType {
    /// Discovered a new terrain type
    Terrain(TerrainType),
    /// Discovered a resource node
    Resource {
        resource_type: ResourceType,
        position: Position,
    },
    /// Discovered a building
    Building {
        building_type: BuildingType,
        position: Position,
    },
    /// Explored a new area (milestone)
    AreaExplored {
        tiles_count: usize,
    },
    /// Discovered a storage container or stockpile
    Storage {
        storage_type: String,
        position: Position,
        capacity: f32,
    },
}

/// A single discovery event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discovery {
    pub discovery_type: DiscoveryType,
    pub tick: u32,
    pub position: Position,
}

/// Agent's exploration knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationKnowledge {
    /// Set of explored tile positions
    pub explored_tiles: BTreeSet<Position>,
    /// Discovered resource positions (position -> resource type)
    pub known_resources: BTreeMap<Position, ResourceType>,
    /// Which of those places this agent was told about rather than saw, who
    /// said so, and how fresh they said it was.
    ///
    /// What an agent has seen and what it has been told went into the same map
    /// with nothing to tell them apart, so a man who walked to a place he had
    /// been told about and found bare ground read his own hearsay back off the
    /// map as confirmation. Keeping the source is what lets the bare ground be
    /// laid at somebody's door - and keeping the age is what stops it being
    /// laid there unfairly.
    #[serde(default)]
    pub who_told_me: BTreeMap<Position, Hearsay>,
    /// Discovered building positions (position -> building type)
    pub known_buildings: BTreeMap<Position, BuildingType>,
    /// Discovered storage positions (position -> (storage type, capacity))
    pub known_storage: BTreeMap<Position, (String, f32)>,
    /// Terrain types encountered
    pub encountered_terrains: BTreeSet<TerrainType>,
    /// History of discoveries
    pub discoveries: Vec<Discovery>,
    /// Total tiles explored
    pub total_tiles_explored: usize,
    /// Last exploration tick
    pub last_exploration_tick: u32,
    /// Curiosity-driven exploration count
    pub curiosity_driven_explorations: u32,
    /// Total curiosity satisfaction gained from discoveries
    pub total_curiosity_satisfaction: f32,
    /// Resource discovery tick tracking (position -> tick discovered)
    ///
    /// The tick a thing was *first* found, and nothing else. Skill experience
    /// is paid on this being the current tick, so it must never be touched
    /// again afterwards - see `last_seen_ticks` for the other question.
    pub resource_discovery_ticks: BTreeMap<Position, u32>,
    /// When this agent last laid eyes on each place it knows.
    ///
    /// Distinct from the tick of discovery, because they answer different
    /// questions: discovery is what an agent learns from, and last sighting is
    /// what an agent can vouch for. Folding the second into the first paid
    /// somebody Farming experience every tick they stood near a field.
    #[serde(default)]
    pub last_seen_ticks: BTreeMap<Position, u32>,
    /// Building discovery tick tracking (position -> tick discovered)
    pub building_discovery_ticks: BTreeMap<Position, u32>,
    /// Where this one has seen something it would rather not meet again.
    ///
    /// The map held explored tiles, resources with an age and a source,
    /// buildings, storage and terrains - a real picture of the world's
    /// *things* - and nothing whatever about danger. An agent could be
    /// mauled at a ford and walk back to the same ford the next morning with
    /// no more hesitation than the first time, because there was nowhere for
    /// "there are wolves in that wood" to live.
    #[serde(default)]
    pub where_it_went_badly: BTreeMap<Position, Danger>,
    /// And where each person this one knows was last actually seen.
    ///
    /// Everything social in the model reads live positions, which is to say
    /// every agent knows where every other agent is at all times. This is
    /// what somebody would actually know: where they last laid eyes on them,
    /// and when.
    #[serde(default)]
    pub where_i_last_saw: BTreeMap<uuid::Uuid, (Position, u32)>,
    /// And where this one went for something and found the place picked bare.
    ///
    /// The map knew *what* was at a place and never whether there was any of
    /// it left, so an agent would walk back to a patch it had stripped itself
    /// the day before, find nothing, and walk back again the day after. "No
    /// food sources nearby" was ten thousand refused turns a world, which is
    /// half of everything a settlement ever got refused.
    ///
    /// It fades, and it has to: a berry patch picked out in June is bearing
    /// again by September, and a man who writes it off for life is as wrong
    /// as the man who goes back every morning.
    #[serde(default)]
    pub where_it_ran_out: BTreeMap<Position, u32>,
    /// And how much was standing at each place this one last laid eyes on.
    ///
    /// `where_it_ran_out` is the same question asked as a yes or no, and a
    /// yes-or-no cannot be passed on usefully: everything an agent told
    /// anybody else was a place and a date. This is what lets a man say "a
    /// rich seam, last week" rather than "a seam, last week", and lets the
    /// man hearing it tell that from "the last handful, this morning".
    #[serde(default)]
    pub how_much_was_there: BTreeMap<Position, u32>,
}

/// Something met on a particular piece of ground, and how badly it went.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Danger {
    /// What it was, in the agent's own words - "wolves", "a bear".
    pub what: String,
    /// When it was last seen there.
    pub when: u32,
    /// How badly it read at the time, from nought to one.
    pub how_bad: f32,
}

impl Danger {
    /// How much of this is still worth minding, given how long ago it was.
    ///
    /// Fades, and for the same reason a claim about a berry patch fades: a
    /// wolf pack works a wood for a season and then moves on, and a man who
    /// avoids that wood for the rest of his life is not being careful, he is
    /// being wrong. Gone entirely after a season.
    pub fn how_bad_it_still_looks(&self, now: u32) -> f32 {
        let ago = now.saturating_sub(self.when) as f32;
        let over = Self::HOW_LONG_A_FRIGHT_LASTS as f32;

        if ago >= over {
            return 0.0;
        }

        self.how_bad * (1.0 - ago / over)
    }

    /// How long a fright takes to fade to nothing.
    ///
    /// One season. Long enough to keep somebody out of a wood for the summer
    /// the pack is working it, short enough that the country is not
    /// permanently marked by one bad afternoon.
    pub const HOW_LONG_A_FRIGHT_LASTS: u32 =
        crate::environment::seasons::DAYS_PER_SEASON * crate::environment::seasons::TICKS_PER_DAY;
}

/// Something an agent was told rather than saw.
///
/// "An agent saying that they saw a berry patch a week prior should not be
/// seen as a liar if the patch was found empty. Whereas an agent which says
/// that a berry patch they just passed was full and another agent finds it
/// empty should be seen as a liar."
///
/// So a claim carries when the man who made it says he saw the thing. A patch
/// is picked, an animal moves on, a deposit is worked out; none of that makes
/// the man who reported it last season a liar, and all of it makes the man who
/// reported it this morning one.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hearsay {
    /// Who said it
    pub who: uuid::Uuid,
    /// The tick they say they saw it on
    pub they_saw_it_on: u32,
    /// The tick they said so
    pub told_me_on: u32,
    /// And how much they said was there.
    ///
    /// A place-name and a date was the whole of what one agent could tell
    /// another, so a listener could weigh "a seam I passed last week" against
    /// "a seam I passed this morning" and had no way at all to weigh either
    /// against "the last handful of a worked-out one". `None` is somebody who
    /// did not say — old saves, and a speaker who cannot remember.
    #[serde(default)]
    pub how_much_they_said: Option<u32>,
}

impl Hearsay {
    /// How long ago the sighting was, as the speaker told it.
    pub fn how_stale(&self, now: u32) -> u32 {
        now.saturating_sub(self.they_saw_it_on)
    }

    /// How long a sighting stays fresh enough that finding the place empty
    /// means somebody was lying.
    ///
    /// Two days. Long enough that a man is not called a liar for the walk
    /// between telling and being checked, short enough that "I passed it this
    /// morning" is a claim he can be held to.
    pub const STILL_ANSWERABLE_FOR: u32 = 24;

    /// Whether the man who said this can be held to it.
    pub fn was_he_answerable_for_it(&self, now: u32) -> bool {
        self.how_stale(now) <= Self::STILL_ANSWERABLE_FOR
    }

    /// Whether finding this place bare makes the man who named it a liar.
    ///
    /// Two ways it does not, and they are different. His news may simply be
    /// old, which `was_he_answerable_for_it` decides. Or somebody else may
    /// have got there first and stripped the place between the telling and
    /// the walk, which is the world's business and not his.
    ///
    /// That second one mattered far more than it looks. A worked-out mineral
    /// seam is taken off the map, so the ground reads exactly like the
    /// invented spot a liar names - and a settlement of twenty-five people
    /// who would not dream of lying still produced dozens of proven liars.
    /// Both call sites of this ask the same question now, in one place,
    /// because there are two copies of the verification sweep and the first
    /// fix went into one of them. See ISSUES_FOUND #48.
    pub fn does_bare_ground_convict_him(&self, now: u32, somebody_got_here_first: bool) -> bool {
        self.was_he_answerable_for_it(now)
            && !somebody_got_here_first
            && !self.he_did_say_it_was_nearly_gone()
    }

    /// Whether what he said was, in effect, "there is hardly anything there".
    ///
    /// The third way bare ground fails to convict, and the one the other two
    /// could not cover. A man who says he saw the last handful of a seam this
    /// morning, and is found to have told the truth about the last handful of
    /// a seam, is plainly not lying - somebody took the handful. Holding him
    /// to it makes honesty about a poor place more dangerous than silence,
    /// which is the opposite of what the reporting is for.
    ///
    /// It cannot shelter a liar, because a liar claims a place worth walking
    /// to - see `Population::tell_them_where_it_is`. Nobody invents a seam
    /// with nothing in it.
    pub fn he_did_say_it_was_nearly_gone(&self) -> bool {
        self.how_much_they_said
            .is_some_and(|how_much| how_much <= Self::THE_LAST_OF_IT)
    }

    /// What counts as the last of a place.
    ///
    /// A handful. Small enough that one person clears it in a turn or two,
    /// which is exactly the case this is about.
    pub const THE_LAST_OF_IT: u32 = 3;
}

impl ExplorationKnowledge {
    pub fn new() -> Self {
        Self {
            explored_tiles: BTreeSet::new(),
            known_resources: BTreeMap::new(),
            who_told_me: BTreeMap::new(),
            how_much_was_there: BTreeMap::new(),
            where_it_ran_out: BTreeMap::new(),
            known_buildings: BTreeMap::new(),
            known_storage: BTreeMap::new(),
            encountered_terrains: BTreeSet::new(),
            discoveries: Vec::new(),
            total_tiles_explored: 0,
            last_exploration_tick: 0,
            curiosity_driven_explorations: 0,
            total_curiosity_satisfaction: 0.0,
            resource_discovery_ticks: BTreeMap::new(),
            last_seen_ticks: BTreeMap::new(),
            building_discovery_ticks: BTreeMap::new(),
            where_it_went_badly: BTreeMap::new(),
            where_i_last_saw: BTreeMap::new(),
        }
    }

    /// This one saw something on that ground it would rather not meet again.
    ///
    /// Keeps the worse of what it already thought and what it has just seen,
    /// so that one quiet afternoon in a bad wood does not talk somebody into
    /// going back. Ageing does that job instead, and does it slowly.
    pub fn saw_danger(&mut self, where_it_was: Position, what: &str, how_bad: f32, now: u32) {
        let how_bad = how_bad.clamp(0.0, 1.0);
        if how_bad <= 0.0 {
            return;
        }

        let standing = self
            .where_it_went_badly
            .get(&where_it_was)
            .map(|danger| danger.how_bad_it_still_looks(now))
            .unwrap_or(0.0);

        if how_bad < standing {
            // Still worth refreshing when it happened, or a wood somebody
            // walks through weekly would fade while the pack was still in it
            if let Some(danger) = self.where_it_went_badly.get_mut(&where_it_was) {
                danger.when = now;
            }
            return;
        }

        self.where_it_went_badly.insert(
            where_it_was,
            Danger {
                what: what.to_string(),
                when: now,
                how_bad,
            },
        );

        self.forget_the_frights_that_have_faded(now);
    }

    /// How bad this one thinks that piece of ground is.
    ///
    /// Nought for anywhere it has never had trouble, and for anywhere the
    /// trouble is old enough not to matter. Reads the ground *around* the
    /// place as well as the place itself, because "there are wolves in that
    /// wood" is not a fact about one tile.
    pub fn how_bad_is_it_there(&self, where_it_is: Position, now: u32) -> f32 {
        self.where_it_went_badly
            .iter()
            .filter(|(went_badly, _)| {
                (went_badly.x - where_it_is.x).abs() <= Self::HOW_WIDE_A_BAD_PLACE_IS
                    && (went_badly.y - where_it_is.y).abs() <= Self::HOW_WIDE_A_BAD_PLACE_IS
            })
            .map(|(_, danger)| danger.how_bad_it_still_looks(now))
            .fold(0.0f32, f32::max)
    }

    /// And what it thinks is there, if anything.
    pub fn what_is_wrong_with_that_place(&self, where_it_is: Position, now: u32) -> Option<&str> {
        self.where_it_went_badly
            .iter()
            .filter(|(went_badly, _)| {
                (went_badly.x - where_it_is.x).abs() <= Self::HOW_WIDE_A_BAD_PLACE_IS
                    && (went_badly.y - where_it_is.y).abs() <= Self::HOW_WIDE_A_BAD_PLACE_IS
            })
            .filter(|(_, danger)| danger.how_bad_it_still_looks(now) > 0.0)
            .max_by(|(_, one), (_, other)| {
                one.how_bad_it_still_looks(now)
                    .partial_cmp(&other.how_bad_it_still_looks(now))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(_, danger)| danger.what.as_str())
    }

    /// Drop what has faded, and the oldest of what is left if there is too
    /// much of it.
    ///
    /// An agent holds ninety-six places it has been told about; there is no
    /// reason it should hold an unbounded number of frights.
    fn forget_the_frights_that_have_faded(&mut self, now: u32) {
        self.where_it_went_badly
            .retain(|_, danger| danger.how_bad_it_still_looks(now) > 0.0);

        while self.where_it_went_badly.len() > Self::AS_MANY_BAD_PLACES_AS_ANYBODY_HOLDS {
            let Some(oldest) = self
                .where_it_went_badly
                .iter()
                .min_by_key(|(_, danger)| danger.when)
                .map(|(where_it_was, _)| *where_it_was)
            else {
                break;
            };
            self.where_it_went_badly.remove(&oldest);
        }
    }

    /// This one went for something here and there was none of it.
    pub fn found_none_at(&mut self, where_it_is: Position, now: u32) {
        self.where_it_ran_out.insert(where_it_is, now);

        while self.where_it_ran_out.len() > Self::AS_MANY_BARE_PLACES_AS_ANYBODY_HOLDS {
            let Some(oldest) = self
                .where_it_ran_out
                .iter()
                .min_by_key(|(_, when)| **when)
                .map(|(where_it_was, _)| *where_it_was)
            else {
                break;
            };
            self.where_it_ran_out.remove(&oldest);
        }
    }

    /// And this one went for something here and got it, which settles the
    /// question whatever it used to think.
    pub fn found_some_at(&mut self, where_it_is: Position) {
        self.where_it_ran_out.remove(&where_it_is);
    }

    /// Whether this one believes there is anything left here.
    ///
    /// Belief, not fact: the ground may have grown back and this one would not
    /// know until it walked over and looked. That is the point of it.
    pub fn is_it_picked_out(&self, where_it_is: Position, now: u32) -> bool {
        self.where_it_ran_out
            .get(&where_it_is)
            .is_some_and(|when| now.saturating_sub(*when) <= Self::HOW_LONG_A_PLACE_STAYS_PICKED_OUT)
    }

    /// How long a place stays written off.
    ///
    /// Half a season. Long enough that a settlement stops walking back to
    /// ground it stripped last week, short enough that it finds the same
    /// hedgerow bearing again in the autumn.
    pub const HOW_LONG_A_PLACE_STAYS_PICKED_OUT: u32 =
        crate::environment::seasons::DAYS_PER_SEASON * crate::environment::seasons::TICKS_PER_DAY
            / 2;

    /// And how many bare places anybody carries about with them.
    const AS_MANY_BARE_PLACES_AS_ANYBODY_HOLDS: usize = 32;

    /// This one laid eyes on somebody.
    pub fn saw_somebody(&mut self, who: uuid::Uuid, where_they_were: Position, now: u32) {
        self.where_i_last_saw.insert(who, (where_they_were, now));
    }

    /// Where this one last saw somebody, if it has seen them lately enough
    /// for it to be worth walking to.
    pub fn where_did_i_last_see(&self, who: uuid::Uuid, now: u32) -> Option<Position> {
        self.where_i_last_saw
            .get(&who)
            .filter(|(_, when)| now.saturating_sub(*when) <= Self::HOW_LONG_A_SIGHTING_IS_WORTH)
            .map(|(where_they_were, _)| *where_they_were)
    }

    /// How far either side of a bad place the badness reaches.
    ///
    /// "There are wolves in that wood" is not a fact about one tile.
    const HOW_WIDE_A_BAD_PLACE_IS: i32 = 3;

    /// How many bad places anybody carries about with them.
    const AS_MANY_BAD_PLACES_AS_ANYBODY_HOLDS: usize = 32;

    /// How long a sighting of somebody is worth acting on.
    ///
    /// A day. People move.
    const HOW_LONG_A_SIGHTING_IS_WORTH: u32 = crate::environment::seasons::TICKS_PER_DAY;

    /// Mark a tile as explored and return true if it's a new discovery
    pub fn explore_tile(&mut self, position: Position, current_tick: u32) -> bool {
        self.last_exploration_tick = current_tick;
        if self.explored_tiles.insert(position) {
            self.total_tiles_explored += 1;
            true
        } else {
            false
        }
    }

    /// Take somebody's word for it that there is something at a place.
    ///
    /// The same as finding it yourself, except that it is remembered as
    /// hearsay so that being wrong can be laid at the door of whoever said it.
    pub fn take_their_word_for_it(
        &mut self,
        position: Position,
        resource_type: ResourceType,
        who_said_so: uuid::Uuid,
        they_saw_it_on: u32,
        how_much_they_said: Option<u32>,
        current_tick: u32,
    ) -> bool {
        if self.discover_resource(position, resource_type, current_tick) {
            self.who_told_me.insert(
                position,
                Hearsay {
                    who: who_said_so,
                    they_saw_it_on,
                    told_me_on: current_tick,
                    how_much_they_said,
                },
            );

            // What he said is what this one now thinks is there, until it goes
            // and looks. Kept in the same book as a first-hand sighting on
            // purpose: everything downstream that weighs a place - what to
            // keep in mind, where to walk - should weigh a reported seam and a
            // seen one on the same scale, and `who_told_me` is what says which
            // it was when that matters.
            if let Some(how_much) = how_much_they_said {
                self.how_much_was_there.insert(position, how_much);
            }

            true
        } else {
            false
        }
    }

    /// The places this agent has seen with its own eyes.
    ///
    /// An agent tells people about what it knows, and what it knows is a
    /// mixture of what it saw and what it was told. Passing on hearsay as
    /// though it were first hand launders a lie: the man who invented it is
    /// never blamed, because everybody heard it from somebody honest who
    /// heard it from somebody honest. Measured with agents repeating what they
    /// had been told, a hundred and fifty lies produced four thousand
    /// accusations, nearly all of them against people who had told the truth
    /// as they understood it.
    ///
    /// So an agent passes on only what it has been to and looked at.
    pub fn seen_for_myself(&self) -> Vec<(Position, ResourceType)> {
        self.known_resources
            .iter()
            .filter(|(where_it_is, _)| !self.who_told_me.contains_key(*where_it_is))
            .map(|(where_it_is, what)| (*where_it_is, *what))
            .collect()
    }

    /// Everywhere this one remembers something of a kind being, whether it
    /// saw the place itself or was told about it.
    ///
    /// `known_resources` has been a map of every resource an agent has ever
    /// laid eyes on since exploration was written, and until now **nothing
    /// asked it where anything was.** It fed gossip - what one person passes
    /// on to another - and the check on whether somebody's claim was true,
    /// and that was the whole of its use. Meanwhile the only memory the
    /// decision layer consulted was `SpatialMemoryType::Food` and `Water`,
    /// which is written from `Percept::ResourceDetected`, which is raised
    /// only from **smell**. So an agent could remember a thing it had sniffed
    /// and could not remember a thing it had seen, and had no memory
    /// whatever of where the wood or the stone or the clay was.
    ///
    /// Hearsay counts here where it does not count in `seen_for_myself`. The
    /// distinction there is about what you are entitled to *repeat*, which is
    /// a question of honesty; this is about where you are willing to *walk*,
    /// and a place somebody mentioned is worth trying. Being wrong costs a
    /// walk, and the walk teaches: `exploring` drops anything from this map
    /// that turns out not to be there once the agent is standing in view of
    /// it.
    pub fn where_i_remember(&self, wanted: ResourceType) -> Vec<Position> {
        self.known_resources
            .iter()
            .filter(|(_, what)| **what == wanted)
            .map(|(where_it_is, _)| *where_it_is)
            .collect()
    }

    /// The nearest of those, skipping ground this one believes it has
    /// stripped.
    pub fn nearest_i_remember(
        &self,
        wanted: ResourceType,
        from: Position,
        now: u32,
    ) -> Option<Position> {
        self.where_i_remember(wanted)
            .into_iter()
            .filter(|where_it_is| !self.is_it_picked_out(*where_it_is, now))
            .min_by_key(|where_it_is| from.distance_to(where_it_is))
    }

    /// What this agent has been told is here, and by whom, out of everything
    /// it can see from where it is standing.
    ///
    /// Anything on this list that is not really there is a lie somebody told,
    /// found out at the only moment it can be: with the agent standing on the
    /// spot.
    pub fn hearsay_in_view(
        &self,
        centre: Position,
        radius: i32,
        really_here: &std::collections::BTreeSet<Position>,
    ) -> Vec<(Position, Hearsay, ResourceType)> {
        self.who_told_me
            .iter()
            .filter(|(where_it_is, _)| {
                (where_it_is.x - centre.x).abs() <= radius
                    && (where_it_is.y - centre.y).abs() <= radius
            })
            .filter(|(where_it_is, _)| !really_here.contains(*where_it_is))
            .filter_map(|(where_it_is, said)| {
                self.known_resources
                    .get(where_it_is)
                    .map(|what| (*where_it_is, *said, *what))
            })
            .collect()
    }

    /// And what it was told is here, and is.
    ///
    /// The other half of `hearsay_in_view`, which filters to claims that have
    /// *failed* and is therefore structurally incapable of noticing one that
    /// held up. Being right was unrecordable in a running settlement:
    /// `TrustRating::correct_count` was zero across thirty-two worlds while
    /// `wrong_count` ran to 1,646, so trust could only ever fall.
    pub fn hearsay_borne_out(
        &self,
        centre: Position,
        radius: i32,
        really_here: &std::collections::BTreeSet<Position>,
    ) -> Vec<(Position, Hearsay)> {
        self.who_told_me
            .iter()
            .filter(|(where_it_is, _)| {
                (where_it_is.x - centre.x).abs() <= radius
                    && (where_it_is.y - centre.y).abs() <= radius
            })
            .filter(|(where_it_is, _)| really_here.contains(*where_it_is))
            .map(|(where_it_is, said)| (*where_it_is, *said))
            .collect()
    }

    /// When this agent last saw a place for itself, if it ever did.
    ///
    /// What an honest man passes on: not "there is food there" but "there was
    /// food there when I went past".
    pub fn when_i_saw_it(&self, where_it_is: &Position) -> Option<u32> {
        self.last_seen_ticks
            .get(where_it_is)
            .or_else(|| self.resource_discovery_ticks.get(where_it_is))
            .copied()
    }

    /// Note that this agent has just laid eyes on a place again, and what was
    /// standing there when it did.
    pub fn saw_it_again(&mut self, where_it_is: Position, how_much: u32, current_tick: u32) {
        self.last_seen_ticks.insert(where_it_is, current_tick);
        self.how_much_was_there.insert(where_it_is, how_much);
    }

    /// How much was there when this one last looked, if it knows.
    ///
    /// What an honest man can say about a place beyond where it is: not "there
    /// is clay there" but "there was a good seam of it there when I went past".
    pub fn how_much_was_there_then(&self, where_it_is: &Position) -> Option<u32> {
        self.how_much_was_there.get(where_it_is).copied()
    }

    /// Discover a resource at a position
    pub fn discover_resource(
        &mut self,
        position: Position,
        resource_type: ResourceType,
        current_tick: u32,
    ) -> bool {
        if !self.known_resources.contains_key(&position) {
            self.known_resources.insert(position, resource_type);
            self.resource_discovery_ticks.insert(position, current_tick);

            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Resource {
                    resource_type,
                    position,
                },
                tick: current_tick,
                position,
            });

            true
        } else {
            false
        }
    }

    /// Discover a building at a position
    pub fn discover_building(
        &mut self,
        position: Position,
        building_type: BuildingType,
        current_tick: u32,
    ) -> bool {
        if !self.known_buildings.contains_key(&position) {
            self.known_buildings.insert(position, building_type);
            self.building_discovery_ticks.insert(position, current_tick);

            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Building {
                    building_type,
                    position,
                },
                tick: current_tick,
                position,
            });

            true
        } else {
            false
        }
    }

    /// Discover a storage container at a position
    pub fn discover_storage(
        &mut self,
        position: Position,
        storage_type: String,
        capacity: f32,
        current_tick: u32,
    ) -> bool {
        if !self.known_storage.contains_key(&position) {
            self.known_storage.insert(position, (storage_type.clone(), capacity));

            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Storage {
                    storage_type,
                    position,
                    capacity,
                },
                tick: current_tick,
                position,
            });

            true
        } else {
            false
        }
    }

    /// Encounter a new terrain type
    pub fn encounter_terrain(
        &mut self,
        terrain_type: TerrainType,
        position: Position,
        current_tick: u32,
    ) -> bool {
        if self.encountered_terrains.insert(terrain_type) {
            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Terrain(terrain_type),
                tick: current_tick,
                position,
            });

            true
        } else {
            false
        }
    }

    /// Check if a tile has been explored
    pub fn is_explored(&self, position: &Position) -> bool {
        self.explored_tiles.contains(position)
    }

    /// Get number of unexplored neighbors around a position
    pub fn count_unexplored_neighbors(&self, position: &Position) -> usize {
        position.neighbors_8()
            .iter()
            .filter(|p| !self.is_explored(p))
            .count()
    }

    /// Find the nearest unexplored position from a given position
    pub fn find_nearest_unexplored(
        &self,
        from: &Position,
        search_radius: u32,
    ) -> Option<Position> {
        let mut nearest: Option<(Position, u32)> = None;

        // Search in expanding radius
        for radius in 1..=search_radius {
            for dx in -(radius as i32)..=(radius as i32) {
                for dy in -(radius as i32)..=(radius as i32) {
                    if dx.abs() + dy.abs() > radius as i32 {
                        continue;
                    }

                    let pos = Position::new(from.x + dx, from.y + dy);

                    if !self.is_explored(&pos) {
                        let distance = from.distance_to(&pos);

                        match nearest {
                            None => nearest = Some((pos, distance)),
                            Some((_, current_dist)) if distance < current_dist => {
                                nearest = Some((pos, distance));
                            }
                            _ => {}
                        }
                    }
                }
            }

            // If we found something in this radius, return it
            if let Some((pos, _)) = nearest {
                return Some(pos);
            }
        }

        nearest.map(|(pos, _)| pos)
    }

    /// Get exploration percentage (requires world size)
    pub fn exploration_percentage(&self, total_world_tiles: usize) -> f32 {
        if total_world_tiles == 0 {
            return 0.0;
        }
        (self.total_tiles_explored as f32 / total_world_tiles as f32) * 100.0
    }

    /// Get recent discoveries (last N)
    pub fn recent_discoveries(&self, count: usize) -> Vec<&Discovery> {
        let start = if self.discoveries.len() > count {
            self.discoveries.len() - count
        } else {
            0
        };

        self.discoveries[start..].iter().collect()
    }

    /// Record a curiosity-driven exploration action and return satisfaction gained
    pub fn record_curiosity_exploration(&mut self, discovery_type: &DiscoveryType) -> f32 {
        self.curiosity_driven_explorations += 1;
        let satisfaction = calculate_exploration_reward(discovery_type);
        self.total_curiosity_satisfaction += satisfaction;
        satisfaction
    }

    /// Get the average curiosity satisfaction per discovery
    pub fn average_curiosity_satisfaction(&self) -> f32 {
        if self.discoveries.is_empty() {
            0.0
        } else {
            self.total_curiosity_satisfaction / self.discoveries.len() as f32
        }
    }

    /// Get exploration efficiency (satisfaction per exploration action)
    pub fn exploration_efficiency(&self) -> f32 {
        if self.curiosity_driven_explorations == 0 {
            0.0
        } else {
            self.total_curiosity_satisfaction / self.curiosity_driven_explorations as f32
        }
    }

    /// Get discoveries by type count
    pub fn discoveries_by_type(&self) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();

        for discovery in &self.discoveries {
            let type_name = match &discovery.discovery_type {
                DiscoveryType::Terrain(_) => "Terrain",
                DiscoveryType::Resource { .. } => "Resource",
                DiscoveryType::Building { .. } => "Building",
                DiscoveryType::AreaExplored { .. } => "Area",
                DiscoveryType::Storage { .. } => "Storage",
            };
            *counts.entry(type_name.to_string()).or_insert(0) += 1;
        }

        counts
    }

    // ===== Fog of War Methods =====




}

/// Visibility status for fog of war
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityStatus {
    /// Currently visible (in line of sight)
    Visible,
    /// Previously explored but not currently visible
    Explored,
    /// Never seen (complete fog of war)
    Unexplored,
}

impl Default for ExplorationKnowledge {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate exploration reward (curiosity satisfaction) based on discovery
pub fn calculate_exploration_reward(discovery: &DiscoveryType) -> f32 {
    match discovery {
        DiscoveryType::Terrain(_) => 0.1,  // New terrain type
        DiscoveryType::Resource { .. } => 0.3,  // Resource discovery is very rewarding
        DiscoveryType::Building { .. } => 0.2,  // Building discovery
        DiscoveryType::AreaExplored { tiles_count } => {
            // Scale reward with area size
            (*tiles_count as f32 * 0.01).min(0.5)
        }
        DiscoveryType::Storage { capacity, .. } => {
            // Storage discovery reward scales with capacity
            // Full storage is more interesting (0.2 base + 0.15 capacity bonus)
            0.2 + (capacity * 0.15)
        }
    }
}

/// Determine if an agent should explore based on their state
pub fn should_explore(
    curiosity_drive: f32,
    unexplored_nearby: usize,
    last_exploration_ticks_ago: u32,
) -> bool {
    // High curiosity drive makes exploration more likely
    if curiosity_drive > 0.6 {
        return true;
    }

    // Many unexplored tiles nearby and moderate curiosity
    if unexplored_nearby > 5 && curiosity_drive > 0.3 {
        return true;
    }

    // Haven't explored in a while and some curiosity
    if last_exploration_ticks_ago > 1000 && curiosity_drive > 0.2 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exploration_knowledge_creation() {
        let knowledge = ExplorationKnowledge::new();
        assert_eq!(knowledge.total_tiles_explored, 0);
        assert_eq!(knowledge.explored_tiles.len(), 0);
    }

    #[test]
    fn test_explore_tile() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(5, 5);

        // First exploration should be new
        assert!(knowledge.explore_tile(pos, 0));
        assert_eq!(knowledge.total_tiles_explored, 1);

        // Second exploration of same tile should not be new
        assert!(!knowledge.explore_tile(pos, 1));
        assert_eq!(knowledge.total_tiles_explored, 1);
    }

    #[test]
    fn test_discover_resource() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(10, 10);

        // First discovery should be new
        assert!(knowledge.discover_resource(pos, ResourceType::Wood, 0));
        assert_eq!(knowledge.known_resources.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);

        // Second discovery at same position should not be new
        assert!(!knowledge.discover_resource(pos, ResourceType::Wood, 1));
        assert_eq!(knowledge.known_resources.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);
    }

    #[test]
    fn test_terrain_encounter() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(0, 0);

        // First encounter should be new
        assert!(knowledge.encounter_terrain(TerrainType::Forest, pos, 0));
        assert_eq!(knowledge.encountered_terrains.len(), 1);

        // Second encounter of same terrain should not be new
        assert!(!knowledge.encounter_terrain(TerrainType::Forest, pos, 1));
        assert_eq!(knowledge.encountered_terrains.len(), 1);

        // Different terrain should be new
        assert!(knowledge.encounter_terrain(TerrainType::Mountain, pos, 2));
        assert_eq!(knowledge.encountered_terrains.len(), 2);
    }

    #[test]
    fn test_is_explored() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(3, 7);

        assert!(!knowledge.is_explored(&pos));
        knowledge.explore_tile(pos, 0);
        assert!(knowledge.is_explored(&pos));
    }

    #[test]
    fn test_count_unexplored_neighbors() {
        let mut knowledge = ExplorationKnowledge::new();
        let center = Position::new(5, 5);

        // All 8 neighbors should be unexplored initially
        assert_eq!(knowledge.count_unexplored_neighbors(&center), 8);

        // Explore one neighbor
        knowledge.explore_tile(Position::new(6, 5), 0);
        assert_eq!(knowledge.count_unexplored_neighbors(&center), 7);

        // Explore all neighbors
        for neighbor in center.neighbors_8() {
            knowledge.explore_tile(neighbor, 0);
        }
        assert_eq!(knowledge.count_unexplored_neighbors(&center), 0);
    }

    #[test]
    fn test_exploration_percentage() {
        let mut knowledge = ExplorationKnowledge::new();

        // Explore 50 tiles in a 100-tile world
        for i in 0..50 {
            knowledge.explore_tile(Position::new(i, 0), 0);
        }

        assert_eq!(knowledge.exploration_percentage(100), 50.0);
        assert_eq!(knowledge.exploration_percentage(200), 25.0);
    }

    #[test]
    fn test_should_explore() {
        // High curiosity should trigger exploration
        assert!(should_explore(0.7, 0, 0));

        // Moderate curiosity with many unexplored tiles
        assert!(should_explore(0.4, 10, 0));

        // Low curiosity but haven't explored in a while
        assert!(should_explore(0.3, 0, 1500));

        // Low curiosity, few unexplored, recent exploration
        assert!(!should_explore(0.1, 2, 50));
    }

    #[test]
    fn test_discover_storage() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(15, 15);

        // First discovery should be new
        assert!(knowledge.discover_storage(pos, "Chest".to_string(), 0.8, 100));
        assert_eq!(knowledge.known_storage.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);

        // Verify the storage was recorded correctly
        let (storage_type, capacity) = knowledge.known_storage.get(&pos).unwrap();
        assert_eq!(storage_type, "Chest");
        assert_eq!(*capacity, 0.8);

        // Second discovery at same position should not be new
        assert!(!knowledge.discover_storage(pos, "Chest".to_string(), 0.8, 101));
        assert_eq!(knowledge.known_storage.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);
    }

    #[test]
    fn test_storage_exploration_reward() {
        // Empty storage
        let empty_storage = DiscoveryType::Storage {
            storage_type: "Box".to_string(),
            position: Position::new(0, 0),
            capacity: 0.0,
        };
        let reward_empty = calculate_exploration_reward(&empty_storage);
        assert_eq!(reward_empty, 0.2); // Base reward only

        // Half-full storage
        let half_storage = DiscoveryType::Storage {
            storage_type: "Barrel".to_string(),
            position: Position::new(0, 0),
            capacity: 0.5,
        };
        let reward_half = calculate_exploration_reward(&half_storage);
        assert!(reward_half > reward_empty);
        assert_eq!(reward_half, 0.275); // 0.2 + 0.5 * 0.15

        // Full storage
        let full_storage = DiscoveryType::Storage {
            storage_type: "Warehouse".to_string(),
            position: Position::new(0, 0),
            capacity: 1.0,
        };
        let reward_full = calculate_exploration_reward(&full_storage);
        assert!(reward_full > reward_half);
        assert!((reward_full - 0.35).abs() < 0.001); // 0.2 + 1.0 * 0.15
    }

    #[test]
    fn test_curiosity_driven_exploration_tracking() {
        let mut knowledge = ExplorationKnowledge::new();

        // Make a resource discovery
        let discovery = DiscoveryType::Resource {
            resource_type: ResourceType::Wood,
            position: Position::new(5, 5),
        };

        let satisfaction = knowledge.record_curiosity_exploration(&discovery);

        assert_eq!(knowledge.curiosity_driven_explorations, 1);
        assert_eq!(satisfaction, 0.3); // Resource reward
        assert_eq!(knowledge.total_curiosity_satisfaction, 0.3);
    }

    #[test]
    fn test_exploration_efficiency() {
        let mut knowledge = ExplorationKnowledge::new();

        // Record multiple explorations with varying rewards
        let discovery1 = DiscoveryType::Terrain(TerrainType::Forest);
        let discovery2 = DiscoveryType::Resource {
            resource_type: ResourceType::Stone,
            position: Position::new(3, 3),
        };

        knowledge.record_curiosity_exploration(&discovery1);
        knowledge.record_curiosity_exploration(&discovery2);

        // Efficiency should be total satisfaction / explorations
        let expected_efficiency = (0.1 + 0.3) / 2.0;
        assert_eq!(knowledge.exploration_efficiency(), expected_efficiency);
    }

    #[test]
    fn test_discoveries_by_type() {
        let mut knowledge = ExplorationKnowledge::new();

        // Add various discoveries
        knowledge.discover_resource(Position::new(1, 1), ResourceType::Wood, 0);
        knowledge.discover_resource(Position::new(2, 2), ResourceType::Stone, 1);
        knowledge.discover_building(Position::new(3, 3), BuildingType::SmallHouse, 2);
        knowledge.discover_storage(Position::new(4, 4), "Chest".to_string(), 0.5, 3);
        knowledge.encounter_terrain(TerrainType::Forest, Position::new(5, 5), 4);

        let counts = knowledge.discoveries_by_type();

        assert_eq!(*counts.get("Resource").unwrap(), 2);
        assert_eq!(*counts.get("Building").unwrap(), 1);
        assert_eq!(*counts.get("Storage").unwrap(), 1);
        assert_eq!(*counts.get("Terrain").unwrap(), 1);
    }

    #[test]
    fn test_average_curiosity_satisfaction() {
        let mut knowledge = ExplorationKnowledge::new();

        // Use record_curiosity_exploration to properly track satisfaction
        let discovery1 = DiscoveryType::Resource {
            resource_type: ResourceType::Wood,
            position: Position::new(1, 1),
        };
        let discovery2 = DiscoveryType::Building {
            building_type: BuildingType::SmallHouse,
            position: Position::new(2, 2),
        };

        knowledge.record_curiosity_exploration(&discovery1); // Adds 0.3 to total_curiosity_satisfaction
        knowledge.record_curiosity_exploration(&discovery2); // Adds 0.2 to total_curiosity_satisfaction

        // Record the actual discoveries (this adds to discoveries vec)
        knowledge.discover_resource(Position::new(1, 1), ResourceType::Wood, 0);
        knowledge.discover_building(Position::new(2, 2), BuildingType::SmallHouse, 1);

        // Average is total_satisfaction (0.5) / discoveries.len() (2) = 0.25
        let avg_satisfaction = knowledge.average_curiosity_satisfaction();
        assert!((avg_satisfaction - 0.25).abs() < 0.001);
    }
}
