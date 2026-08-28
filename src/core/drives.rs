// src/core/drives.rs
//! Drive system for agent motivation.
//!
//! Drives represent internal motivations that accumulate over time and
//! trigger goal-seeking behavior. Each drive has:
//! - A current value (0.0 to 1.0)
//! - A threshold for activation
//! - A weight (agent personality)
//! - Increase/decrease conditions

use serde::{Deserialize, Serialize};

/// The 14 core drives that motivate agent behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveType {
    /// Need for food
    Hunger,
    /// Need for water
    Thirst,
    /// Need for sleep
    Rest,
    /// Need for protective structure
    Shelter,
    /// Need for safety from threats
    Safety,
    /// Need for resource stockpiles
    Preparedness,
    /// Need to gather and process materials
    Industry,
    /// Need to produce food
    Sustenance,
    /// Need to explore and learn
    Curiosity,
    /// Need for proximity to others
    Social,
    /// Need to produce offspring
    Reproduction,
    /// Need for rare or decorative items
    Luxury,
    /// Need for tools and equipment
    Utility,
    /// Need to build structures
    Construction,
    /// Need to keep one's children safe and close
    Protection,
}

impl DriveType {
    /// Get all drive types
    pub fn all() -> [DriveType; 15] {
        [
            DriveType::Hunger,
            DriveType::Thirst,
            DriveType::Rest,
            DriveType::Shelter,
            DriveType::Safety,
            DriveType::Preparedness,
            DriveType::Industry,
            DriveType::Sustenance,
            DriveType::Curiosity,
            DriveType::Social,
            DriveType::Reproduction,
            DriveType::Luxury,
            DriveType::Utility,
            DriveType::Construction,
            DriveType::Protection,
        ]
    }

    /// Get the default threshold for this drive type
    pub fn default_threshold(&self) -> f32 {
        match self {
            DriveType::Hunger => 0.7,
            DriveType::Thirst => 0.75,
            DriveType::Rest => 0.6,
            DriveType::Shelter => 0.5,
            DriveType::Safety => 0.8,
            DriveType::Preparedness => 0.4,
            DriveType::Industry => 0.3,
            DriveType::Sustenance => 0.3,
            DriveType::Curiosity => 0.2,
            DriveType::Social => 0.5,
            DriveType::Reproduction => 0.6,
            DriveType::Luxury => 0.1,
            DriveType::Utility => 0.4,
            DriveType::Construction => 0.3,
            // Low, because it should be easy to trip: a parent does not wait
            // until a child is in real trouble to go and look for it
            DriveType::Protection => 0.3,
        }
    }

    /// Get the base accumulation rate per tick
    pub fn base_accumulation_rate(&self) -> f32 {
        match self {
            // Derived from the stomach rather than chosen.
            //
            // At 0.01 an ordinary body climbed to its threshold in about
            // seventeen turns - a day and a half - so a settlement with the
            // whole of spring standing round it sat down to 2.26 meals a day
            // against the three its body burns, took in nine hundred and
            // seventy units a day against fourteen hundred and forty, and
            // starved over about a hundred days with full bushes and a full
            // pit. Nothing was short of food; the body never asked for it.
            //
            // A meal holds for as long as the stomach takes to empty, which
            // the gastric schedule already states, so that is how long the
            // drive takes to climb from nothing to wanting food again - at the
            // ordinary product of the three tables. A body behind on its
            // reserve or empty in the gut climbs faster, which is the whole
            // point of the tables.
            DriveType::Hunger => {
                self.default_threshold()
                    / (crate::agents::physiology::TURNS_A_MEAL_HOLDS
                        * crate::agents::physiology::AN_ORDINARY_APPETITE)
            }
            DriveType::Thirst => 0.012,  // Slightly faster than hunger
            DriveType::Rest => 0.008,
            DriveType::Shelter => 0.005,
            DriveType::Safety => 0.02,  // Spikes with threats
            DriveType::Preparedness => 0.002,
            DriveType::Industry => 0.003,
            DriveType::Sustenance => 0.003,
            DriveType::Curiosity => 0.004,
            DriveType::Social => 0.006,
            DriveType::Reproduction => 0.001,
            DriveType::Luxury => 0.001,
            DriveType::Utility => 0.002,
            DriveType::Construction => 0.002,
            // Driven by where the children are rather than by the clock, so
            // this only ticks over slowly on its own
            DriveType::Protection => 0.001,
        }
    }

    /// How much food, materials, tools and finery an agent counts as "enough".
    ///
    /// The specification's decrease conditions are worded as sufficiency -
    /// "sufficient stockpiled food, tools, materials", "sufficient tool variety
    /// stored" - so each of them needs a number for what sufficient means.
    const ENOUGH_FOOD: f32 = 20.0;
    const ENOUGH_MATERIALS: f32 = 30.0;
    const ENOUGH_TOOLS: f32 = 3.0;
    const ENOUGH_FINERY: f32 = 2.0;

    /// How much the agent's situation is asking for this drive right now, from
    /// 0.0 (nothing about the situation calls for it) to 1.0 (everything does).
    ///
    /// `None` means this drive does not read the world at all: hunger, thirst
    /// and tiredness build with time whatever is going on, which is what the
    /// specification says of them ("time passage", "time since sleep") and what
    /// the rest of the survival loop is built on.
    ///
    /// The nine that do read the world used to build on a clock like the
    /// others, and because their satisfying actions are chosen rarely they sat
    /// pinned at their ceiling for whole runs - nine of fifteen drives at 1.00
    /// and active every tick, which left the per-agent weight as the only thing
    /// telling them apart. Reading the conditions the specification gives them
    /// is what unpins them: a drive with nothing asking for it now falls away
    /// instead of waiting at the top.
    pub fn demand(&self, ctx: &DriveContext) -> Option<f32> {
        let short_of = |have: u32, enough: f32| (1.0 - have as f32 / enough).clamp(0.0, 1.0);
        let yes = |condition: bool, weight: f32| if condition { weight } else { 0.0 };

        let demand = match self {
            // "Environmental exposure, weather, nightfall, monster proximity",
            // answered by being inside something.
            DriveType::Shelter => {
                if ctx.around.under_shelter {
                    0.0
                } else {
                    // Weighted so that no single ordinary condition carries it
                    // over the threshold on its own, but any two do: a cold
                    // night, or a wet one, is a reason to be indoors. Reading
                    // only damage already taken - which is what a first cut at
                    // this did - left the drive quiet through every night of a
                    // run, and agents who never wanted to be indoors spent
                    // their lives walking about in the open.
                    yes(ctx.exposed, 0.55)
                        + yes(ctx.chilly, 0.3)
                        + yes(ctx.around.night, 0.35)
                        + yes(ctx.around.foul_weather, 0.3)
                        + yes(ctx.around.predator_near, 0.2)
                }
            }

            // "Hostile entity proximity, recent injury, darkness", answered by
            // "being in shelter, possessing weapons or armor". This is the one
            // the old code claimed in a comment - `Safety => 0.02, // Spikes
            // with threats` - and did not do.
            DriveType::Safety => {
                let threat = yes(ctx.around.predator_near, 0.7)
                    + yes(ctx.around.recently_hurt, 0.5)
                    + yes(ctx.around.night, 0.35);
                let cover = yes(ctx.around.under_shelter, 0.5) + yes(ctx.armed, 0.5);
                threat * (1.0 - cover.min(0.9))
            }

            // "Tool count zero, missing materials in storage", answered by
            // "sufficient stockpiled food, tools, materials".
            DriveType::Preparedness => {
                let food = short_of(ctx.food_put_by, Self::ENOUGH_FOOD);
                let materials = short_of(ctx.materials_put_by, Self::ENOUGH_MATERIALS);
                let tools = short_of(ctx.tools_to_hand, Self::ENOUGH_TOOLS);
                (food + materials + tools) / 3.0
            }

            // "Low material stockpiles, high tool durability available",
            // answered by delivering and storing what was won.
            DriveType::Industry => {
                let short = short_of(ctx.materials_put_by, Self::ENOUGH_MATERIALS);
                // Something to work with, or there is no point going out
                let able = if ctx.tools_to_hand > 0 { 1.0 } else { 0.45 };
                short * able
            }

            // "Low food stockpile, crop depletion, available farming tools",
            // answered by planting, harvesting and storing.
            DriveType::Sustenance => {
                let short = short_of(ctx.food_put_by, Self::ENOUGH_FOOD);
                let ground_failing = 1.0 - ctx.around.crop_near.clamp(0.0, 1.0);
                (short * 0.6 + ground_failing * 0.4).clamp(0.0, 1.0)
            }

            // "Idle time, lack of rare items, unfulfilled crafting goals",
            // answered by possessing something fine.
            //
            // The idleness matters as much as the lack. Read on the lack
            // alone this sits at its ceiling for every agent in the world -
            // nothing here makes jewellery - and being both unsatisfiable and
            // permanently maximal it takes over the drive fallback for half
            // the population. A person with work to do is not thinking about
            // ornaments.
            DriveType::Luxury => {
                let wanting = short_of(ctx.fine_things, Self::ENOUGH_FINERY);
                let idle = if ctx.at_leisure { 1.0 } else { 0.15 };
                wanting * idle
            }

            // "Task interruptions, tool unavailability, broken tools",
            // answered by "sufficient tool variety stored, maintained gear".
            DriveType::Utility => {
                let short = short_of(ctx.tools_to_hand, Self::ENOUGH_TOOLS);
                let broken = (ctx.broken_tools as f32 / Self::ENOUGH_TOOLS).clamp(0.0, 1.0);
                (short * 0.6 + broken * 0.6).clamp(0.0, 1.0)
            }

            // "Buildable templates seen, others building, drive synergy". The
            // last of those is why `shelter_pressing` is in the context: an
            // agent that badly wants to be out of the weather is an agent that
            // wants to build something.
            DriveType::Construction => {
                let room = yes(ctx.around.somewhere_to_build, 0.4);
                let neighbours = yes(ctx.around.neighbours_building, 0.25);
                let synergy = ctx.shelter_pressing.clamp(0.0, 1.0) * 0.45;
                let means = if ctx.materials_put_by > 0 { 1.0 } else { 0.35 };
                (room + neighbours + synergy) * means
            }

            // Not in the specification - added when parents were given a
            // reason to keep their children close. It is answered by being
            // where the children are, so it asks for nothing when there are
            // none.
            DriveType::Protection => {
                if ctx.around.children_to_mind == 0 {
                    0.0
                } else if ctx.around.child_astray {
                    1.0
                } else {
                    0.25
                }
            }

            // Hunger, Thirst, Rest, Curiosity, Social and Reproduction build
            // with time rather than with the situation
            _ => return None,
        };

        Some(demand.clamp(0.0, 1.0))
    }

    /// How hard this drive is allowed to press on an agent's attention.
    ///
    /// Three bands, and the band is about *interruption* rather than about how
    /// much anybody wants the thing. A primary need can take an agent off
    /// whatever else it is doing; a secondary one waits for a lull; a tertiary
    /// one waits for a good year. What unlocks what is a separate matter and
    /// lives in [`Self::unlocked_by`].
    pub fn rank(&self) -> DriveRank {
        match self {
            // These kill you, and the only question is which kills you first
            DriveType::Hunger
            | DriveType::Thirst
            | DriveType::Rest
            | DriveType::Safety => DriveRank::Primary,

            // These decide whether there is anybody here in ten years
            DriveType::Sustenance
            | DriveType::Preparedness
            | DriveType::Shelter
            | DriveType::Social
            | DriveType::Reproduction
            | DriveType::Curiosity => DriveRank::Secondary,

            // These decide what sort of place it is
            DriveType::Luxury
            | DriveType::Utility
            | DriveType::Construction
            | DriveType::Industry
            | DriveType::Protection => DriveRank::Tertiary,
        }
    }

    /// What has to be answered before this drive is worth anything.
    ///
    /// A hungry person is not thinking about saving food for later, and
    /// somebody who cannot keep the rain off is not thinking about whether
    /// their coat is a fine one. Each drive names what stands before it; a
    /// drive that names nothing is always free to build.
    ///
    /// The chains, in full:
    ///
    /// - Hunger, then Sustenance, then Luxury
    /// - Hunger and Thirst, then Preparedness
    /// - Rest, then Shelter
    /// - Safety, then Shelter, then Protection
    /// - Social, then Construction and Industry, then Utility
    /// - every primary answered, then Reproduction, then Protection
    ///
    /// Where two chains meet on one drive - Preparedness stands after both
    /// Sustenance and Thirst, Shelter after both Rest and Safety, Protection
    /// after both Safety and Reproduction - all of them have to be answered.
    pub fn unlocked_by(&self) -> &'static [DriveType] {
        match self {
            // Nothing stands before a thing that kills you, and nothing stands
            // before wanting to know or wanting company
            DriveType::Hunger
            | DriveType::Thirst
            | DriveType::Rest
            | DriveType::Safety
            | DriveType::Curiosity
            | DriveType::Social => &[],

            // Next winter's grain waits on tonight's dinner
            DriveType::Sustenance => &[DriveType::Hunger],

            // And putting something by waits on the same thing, and on
            // nothing else.
            //
            // This used to stand behind Sustenance, on the reasoning that a
            // people puts by what it grows. It does not: a people puts by
            // what it *finds*, and it has been doing that for a great deal
            // longer than it has been growing anything. Standing behind
            // Sustenance meant Preparedness could not build until food
            // production was answered, and in a foraging settlement food
            // production is never answered - so a forager could never store
            // anything, which is precisely backwards. Probed directly:
            // Preparedness sat below its threshold in eight agents out of
            // eight, at values of 0.00 to 0.14 against thresholds of 0.26 to
            // 0.40, for the whole of a settlement's life.
            //
            // What it waits on is being neither hungry nor parched today.
            DriveType::Preparedness => &[DriveType::Hunger, DriveType::Thirst],
            DriveType::Luxury => &[DriveType::Preparedness],

            // A roof is what you want once you are rested and safe
            DriveType::Shelter => &[DriveType::Rest, DriveType::Safety],

            // Building and working are things people do together
            DriveType::Construction => &[DriveType::Social],
            DriveType::Industry => &[DriveType::Social],
            DriveType::Utility => &[DriveType::Construction, DriveType::Industry],

            // Nobody has children while something is still trying to kill them
            DriveType::Reproduction => &[
                DriveType::Hunger,
                DriveType::Thirst,
                DriveType::Rest,
                DriveType::Safety,
            ],

            // And looking after somebody else waits on being safe yourself,
            // and on there being somebody of your own to look after
            DriveType::Protection => &[DriveType::Safety, DriveType::Reproduction],
        }
    }

    /// Whether this drive is about the season after next rather than the next
    /// few hours.
    ///
    /// A person with nothing to eat is not thinking about next winter's grain,
    /// and a person who has eaten is. These rise when the immediate needs are
    /// answered and fall quiet when they are not, which is what turns a
    /// settlement that survives into one that provides for itself.
    pub fn is_long_term(&self) -> bool {
        matches!(
            self,
            DriveType::Preparedness
                | DriveType::Sustenance
                | DriveType::Industry
                | DriveType::Construction
                | DriveType::Utility
                | DriveType::Luxury
        )
    }

    /// Get a description of what satisfies this drive
    pub fn satisfaction_description(&self) -> &'static str {
        match self {
            DriveType::Hunger => "Consuming food",
            DriveType::Thirst => "Drinking water",
            DriveType::Rest => "Sleeping in bed",
            DriveType::Shelter => "Being inside shelter structure",
            DriveType::Safety => "Being in shelter, possessing weapons",
            DriveType::Preparedness => "Stockpiling resources and tools",
            DriveType::Industry => "Mining, smelting, processing materials",
            DriveType::Sustenance => "Farming, harvesting, producing food",
            DriveType::Curiosity => "Exploring, learning, discovering recipes",
            DriveType::Social => "Being near other agents",
            DriveType::Reproduction => "Producing offspring",
            DriveType::Luxury => "Acquiring rare or decorative items",
            DriveType::Utility => "Crafting and maintaining tools",
            DriveType::Construction => "Building structures",
            DriveType::Protection => "Keeping one's children close and safe",
        }
    }
}

/// What the world around an agent is doing to it, as far as its drives are
/// concerned.
///
/// The design document specifies each drive by the conditions that raise it -
/// Safety by "hostile entity proximity, recent injury, darkness", Construction
/// by "buildable templates seen, others building, drive synergy". Some of those
/// conditions are things an agent knows about itself and some are things only
/// the world knows. This is the second kind: the simulation fills it in once
/// per agent per tick, and the agent folds in what it knows about itself when
/// its drives are ticked.
///
/// A default one describes an agent standing in open country in daylight with
/// nothing happening: no threat, no neighbours at work, no children, and no
/// ground worth breaking. Agents ticked without a world - a bare `Population`
/// in a test - get that, which is the right answer for a world that is not
/// there.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Surroundings {
    /// Something that would eat the agent is close by
    pub predator_near: bool,
    /// It is dark
    pub night: bool,
    /// Rain, snow, wind - weather worth being out of
    pub foul_weather: bool,
    /// The agent is under a roof or in deep cover
    pub under_shelter: bool,
    /// Something has attacked the agent recently
    pub recently_hurt: bool,
    /// How well the ground within reach is bearing, 0.0 to 1.0
    pub crop_near: f32,
    /// There is ground here worth breaking, and room to build on
    pub somewhere_to_build: bool,
    /// Other people nearby are building
    pub neighbours_building: bool,
    /// Small children of this agent's own
    pub children_to_mind: u32,
    /// One of them has strayed, or something is stalking it
    pub child_astray: bool,
    /// Anybody else within talking distance
    pub company: bool,
}

/// Everything a drive can consult when working out how much the situation
/// calls for it: the world around the agent, and the agent's own means.
#[derive(Debug, Clone, Default)]
pub struct DriveContext {
    /// What the world is doing - see [`Surroundings`]
    pub around: Surroundings,
    /// Food the agent has put by, in units
    pub food_put_by: u32,
    /// Wood, stone, ore and the like
    pub materials_put_by: u32,
    /// Tools in working order
    pub tools_to_hand: u32,
    /// Tools worn out or broken
    pub broken_tools: u32,
    /// Rare or decorative things
    pub fine_things: u32,
    /// A weapon or armour to hand
    pub armed: bool,
    /// Out in the weather with nothing between the agent and it
    pub exposed: bool,
    /// Cold, though not yet dangerously so
    pub chilly: bool,
    /// How hard the agent's need for shelter is already pressing, 0.0 to 1.0.
    /// This is the specification's "drive synergy": what one drive wants can
    /// raise another.
    pub shelter_pressing: f32,
    /// Nothing more pressing on. Several drives are specified to rise on
    /// "idle time", which is this.
    pub at_leisure: bool,
}

/// The state of a single drive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drive {
    pub drive_type: DriveType,
    /// Current value (0.0 to 1.0)
    pub value: f32,
    /// Threshold for activation
    pub threshold: f32,
    /// Personal weight/importance for this agent
    pub weight: f32,
    /// How long this drive has been asking without being answered
    #[serde(default)]
    pub denied_ticks: u32,
    /// And how long it has gone without needing to ask at all
    #[serde(default)]
    pub answered_ticks: u32,
    /// How much this particular person cares about this particular drive,
    /// over and above what anybody would.
    ///
    /// Kept apart from `weight` because the two come from different places and
    /// are inherited differently: `weight` is the individual variation a
    /// person is born with and passes to their children, and this is what
    /// their personality does to it. Holding them separately means a
    /// personality can be applied and re-applied without compounding, which
    /// matters because a child's traits are settled after its drives are.
    #[serde(default = "no_leaning")]
    pub lean: f32,
}

/// A drive nobody has an opinion about
fn no_leaning() -> f32 {
    1.0
}

impl Drive {
    /// How much faster the long view builds in an agent whose immediate needs
    /// are met
    const SECURE_LONG_TERM_RATE: f32 = 5.0;

    /// And how much slower in one whose are not
    const PRESSED_LONG_TERM_RATE: f32 = 0.25;

    /// How long a drive has to go unanswered to press twice as hard.
    ///
    /// A day and a half on the world's calendar. A person who missed a meal
    /// this morning is a little distracted; one who has not eaten in three
    /// days is not thinking about anything else, and that difference is what
    /// makes a settlement abandon its fields rather than starve politely
    /// beside them.
    const PRESSURE_SPAN: f32 = 18.0;

    /// The most a drive can be magnified by having been denied.
    ///
    /// Bounded, because an unbounded one would make a single old grievance
    /// outrank an immediate threat for ever.
    const MAX_PRESSURE: f32 = 4.0;

    /// Create a new drive with default values
    pub fn new(drive_type: DriveType) -> Self {
        Self {
            drive_type,
            value: 0.0,
            threshold: drive_type.default_threshold(),
            weight: 1.0,
            denied_ticks: 0,
            answered_ticks: 0,
            lean: 1.0,
        }
    }

    /// Create a new drive with custom weight
    pub fn with_weight(drive_type: DriveType, weight: f32) -> Self {
        Self {
            drive_type,
            value: 0.0,
            threshold: drive_type.default_threshold(),
            weight,
            denied_ticks: 0,
            answered_ticks: 0,
            lean: 1.0,
        }
    }

    /// How hard this drive is pressing, over and above how high it stands.
    ///
    /// One while the drive is being answered often enough, climbing towards
    /// [`Self::MAX_PRESSURE`] the longer it is left asking. This multiplies
    /// both how fast the drive builds and how loudly it argues for the agent's
    /// attention, so a need that keeps being deferred does not sit politely at
    /// its threshold - it takes the agent over.
    pub fn pressure(&self) -> f32 {
        1.0 + (self.denied_ticks as f32 / Self::PRESSURE_SPAN).min(Self::MAX_PRESSURE - 1.0)
    }

    /// How long this drive has gone unanswered while asking
    pub fn denied_ticks(&self) -> u32 {
        self.denied_ticks
    }

    /// How long this drive has gone without having to ask at all.
    ///
    /// The other side of the same coin, and the only forward-looking thing an
    /// agent has: a need that has not been a problem for a long stretch is
    /// evidence that it is not about to become one. This is what a settlement
    /// uses to decide it can afford a child, in place of "I had a meal this
    /// morning", which says nothing about next week.
    pub fn answered_ticks(&self) -> u32 {
        self.answered_ticks
    }

    /// Increase the drive value
    pub fn increase(&mut self, amount: f32) {
        self.value = (self.value + amount).min(1.0);
    }

    /// Decrease the drive value.
    ///
    /// Answering a drive enough to stop it asking also takes the weight of
    /// having been ignored off it, though not all at once: an agent that has
    /// been starving stays wary for a while after its first meal.
    pub fn decrease(&mut self, amount: f32) {
        self.value = (self.value - amount).max(0.0);

        if !self.is_active() {
            self.denied_ticks /= 2;
        }
    }

    /// Check if the drive is above threshold
    pub fn is_active(&self) -> bool {
        self.value >= self.threshold
    }

    /// Get the effective urgency: how high the drive stands, how much this
    /// person cares about that sort of thing, what their personality makes of
    /// it, and how long they have been ignoring it.
    pub fn urgency(&self) -> f32 {
        self.bare_urgency() * self.pressure()
    }

    /// What this drive would argue for on its face, before the weight of
    /// having been ignored is added
    pub fn bare_urgency(&self) -> f32 {
        self.value * self.weight * self.lean
    }

    /// How fast a need out of reach stops being felt, against how fast it
    /// would have built if it had been free to.
    ///
    /// It has to be reckoned against the drive's own rate rather than set as
    /// one number for all of them. A flat rate is a different thing to each
    /// drive: at 0.004 a tick it was four times what Reproduction, Luxury and
    /// Protection build at and half what Safety builds at, so the slow drives
    /// were quietly halved. Reproduction is shut out about a tenth of the
    /// time, which under a flat fade left it climbing at 50.5% of its proper
    /// rate - and since conception needs that drive over its threshold in both
    /// parents, that halved the birth rate and with it the population.
    ///
    /// At one, a need fades at the pace it would have grown: a drive shut out
    /// a tenth of the time still climbs at four fifths of its rate, and one
    /// shut out half the time hovers where it is.
    const FADES_AS_FAST_AS_IT_BUILDS: f32 = 1.0;

    /// Let this need go, because nothing before it in the chain is answered.
    ///
    /// Somebody who has gone hungry for a week is not sitting on a banked-up
    /// wish for a finer coat, ready to spend it the moment they eat. The wish
    /// goes while the hunger lasts, and has to build again afterwards.
    pub fn fall_quiet(&mut self) {
        let fades = self.drive_type.base_accumulation_rate()
            * Self::FADES_AS_FAST_AS_IT_BUILDS;

        self.value = (self.value - fades).max(0.0);

        // And it is not being denied while nobody could have answered it: the
        // pressure of going without is for needs an agent could have met
        self.denied_ticks = self.denied_ticks.saturating_sub(1);
    }

    /// Update the drive for one tick
    pub fn tick(&mut self) {
        self.tick_at(self.drive_type.base_accumulation_rate());
    }

    /// Tick, knowing whether the agent's immediate needs are answered.
    ///
    /// Long-term drives run several times faster in an agent that is fed,
    /// watered, rested and warm, and nearly stop in one that is not.
    pub fn tick_with_security(&mut self, secure: bool) {
        let rate = self.drive_type.base_accumulation_rate();

        let rate = if self.drive_type.is_long_term() {
            if secure {
                rate * Self::SECURE_LONG_TERM_RATE
            } else {
                rate * Self::PRESSED_LONG_TERM_RATE
            }
        } else {
            rate
        };

        self.tick_at(rate);
    }

    /// Tick, knowing both whether the agent's immediate needs are answered and
    /// what its situation is asking of it.
    ///
    /// A drive that reads the world moves towards what the situation calls for
    /// rather than climbing a clock, so it settles where the conditions put it
    /// and falls away when they stop. A drive that does not read the world
    /// builds as it always did.
    pub fn tick_in(&mut self, ctx: &DriveContext, secure: bool) {
        let rate = self.drive_type.base_accumulation_rate();

        let rate = if self.drive_type.is_long_term() {
            if secure {
                rate * Self::SECURE_LONG_TERM_RATE
            } else {
                rate * Self::PRESSED_LONG_TERM_RATE
            }
        } else {
            rate
        };

        match self.drive_type.demand(ctx) {
            Some(wanted) => self.approach(wanted, rate),
            None => self.tick_at(rate),
        }
    }

    /// Tick against a situation, without the security modifier
    pub fn tick_in_context(&mut self, ctx: &DriveContext) {
        self.tick_in(ctx, false);
    }

    /// Move towards what the situation calls for.
    ///
    /// The gap closes by a share of itself each tick, so a drive whose base
    /// rate is high answers a change in the situation quickly - Safety, at
    /// 0.02, is most of the way to a new level within a day of a predator
    /// appearing - and one whose rate is low takes seasons. Being denied still
    /// tells: the pressure that builds while a drive waits also makes it close
    /// the gap faster.
    fn approach(&mut self, wanted: f32, rate: f32) {
        self.note_whether_it_had_to_ask();

        let gap = wanted - self.value;
        self.value = (self.value + gap * rate * self.pressure()).clamp(0.0, 1.0);
    }

    /// Keep the tally of how long this drive has been asking, and how long it
    /// has not had to.
    fn note_whether_it_had_to_ask(&mut self) {
        if self.is_active() {
            self.denied_ticks = self.denied_ticks.saturating_add(1);
            self.answered_ticks = 0;
        } else {
            self.answered_ticks = self.answered_ticks.saturating_add(1);

            if self.denied_ticks > 0 {
                // Below the threshold the grievance fades, but not instantly:
                // an agent that has been starving is wary for a while after
                // its first meal.
                self.denied_ticks -= 1;
            }
        }
    }

    /// One tick of a drive building at the given rate, keeping the tally of
    /// how long it has been asking.
    ///
    /// A drive that is over its threshold and still not answered builds faster
    /// every tick it waits. That is what makes hunger escalate from a reason
    /// to go and pick berries into a reason to walk off the map.
    fn tick_at(&mut self, rate: f32) {
        self.note_whether_it_had_to_ask();
        self.increase(rate * self.pressure());
    }

    /// Fully satisfy this drive
    pub fn satisfy(&mut self) {
        self.value = 0.0;
        self.denied_ticks = 0;
    }

    /// Partially satisfy this drive
    pub fn partial_satisfy(&mut self, amount: f32) {
        self.decrease(amount);
    }

    /// Get priority score (alias for urgency, used in TDD tests)
    pub fn priority(&self) -> f32 {
        self.urgency()
    }
}

/// How hard a drive is allowed to press on an agent's attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DriveRank {
    /// Goes unanswered long enough and the agent dies of it
    Primary,
    /// Decides whether the agent and its people are here in ten years
    Secondary,
    /// Decides what sort of place they are living in
    Tertiary,
}

impl DriveRank {
    /// What a drive in this band is worth against one in another.
    ///
    /// Wide enough that no amount of wanting a fine coat outweighs being
    /// thirsty, and narrow enough that a tertiary drive nobody has answered
    /// for years can still eventually get a turn.
    pub fn precedence(&self) -> f32 {
        match self {
            DriveRank::Primary => 100.0,
            DriveRank::Secondary => 10.0,
            DriveRank::Tertiary => 1.0,
        }
    }
}

/// Complete drive state for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveState {
    pub drives: Vec<Drive>,
}

impl DriveState {
    /// Create a new drive state with default values
    pub fn new() -> Self {
        Self {
            drives: DriveType::all()
                .iter()
                .map(|&dt| Drive::new(dt))
                .collect(),
        }
    }

    /// Create a new drive state with randomized weights
    /// Ensures survival drives (Hunger, Rest, Safety, Shelter) have higher minimum weights
    pub fn with_random_weights() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        Self {
            drives: DriveType::all()
                .iter()
                .map(|&dt| {
                    // Survival-critical drives get higher base weights
                    let weight = match dt {
                        DriveType::Hunger | DriveType::Rest => {
                            // Tier 1 survival: 1.5-2.5 weight range
                            rng.gen_range(1.5..2.5)
                        }
                        DriveType::Safety | DriveType::Shelter => {
                            // Tier 2 survival: 1.0-2.0 weight range
                            rng.gen_range(1.0..2.0)
                        }
                        _ => {
                            // Other drives: 0.5-1.5 weight range (lower than survival)
                            rng.gen_range(0.5..1.5)
                        }
                    };
                    Drive::with_weight(dt, weight)
                })
                .collect(),
        }
    }

    /// The narrowest a personality will let a drive get, and the widest.
    ///
    /// Somebody who cares little about a thing still eventually cares; nobody
    /// is so keen on shelter that they will not eat. Traits compound - a
    /// person can be Lazy and nothing else that touches Industry, or Handy and
    /// Diligent and Ambitious together - so without a floor and a ceiling an
    /// unlucky draw could quiet a drive to nothing or drown out every other.
    pub const LEAST_ANYBODY_CARES: f32 = 0.35;
    pub const MOST_ANYBODY_CARES: f32 = 2.5;

    /// And how far a personality can move the point at which somebody acts.
    ///
    /// The ceiling has to be above 1.0 or every trait that raises a threshold -
    /// Lazy needing more pushing before it starts work, Ascetic barely
    /// registering a want for anything fine - would have its whole effect
    /// clamped away. The absolute cap below keeps a drive from becoming one
    /// that never fires at all.
    pub const SOONEST_ANYBODY_ACTS: f32 = 0.4;
    pub const LATEST_ANYBODY_ACTS: f32 = 1.7;

    /// No personality makes a need invisible: past this a drive would sit
    /// under its threshold for a whole life.
    pub const ALWAYS_EVENTUALLY: f32 = 0.95;

    /// Bend these drives to a personality.
    ///
    /// Both what it changes are recomputed from the drive type's own defaults
    /// rather than from whatever they hold now, so this can be applied as many
    /// times as you like and the answer is the same. That matters: a founder's
    /// personality is drawn after its drives exist, and a child's traits are
    /// settled after it has inherited its parents' drive weights, so this gets
    /// called at two different points in two different lives.
    ///
    /// What it does not touch is `weight`, which is the individual variation
    /// somebody is born with and hands on. Two equally lazy people can still
    /// differ in how much work matters to them; being lazy is a thing on top
    /// of that, not instead of it.
    pub fn lean_towards(&mut self, traits: &crate::core::traits::TraitSet) {
        for drive in &mut self.drives {
            drive.lean = 1.0;
            drive.threshold = drive.drive_type.default_threshold();
        }

        for held in traits.get_traits() {
            for &(drive_type, cares, acts) in held.leanings() {
                let Some(drive) = self.get_mut(drive_type) else {
                    continue;
                };
                drive.lean *= cares;
                drive.threshold *= acts;
            }
        }

        for drive in &mut self.drives {
            drive.lean = drive
                .lean
                .clamp(Self::LEAST_ANYBODY_CARES, Self::MOST_ANYBODY_CARES);

            let ordinary = drive.drive_type.default_threshold();
            drive.threshold = drive.threshold.clamp(
                ordinary * Self::SOONEST_ANYBODY_ACTS,
                (ordinary * Self::LATEST_ANYBODY_ACTS).min(Self::ALWAYS_EVENTUALLY),
            );
        }
    }

    /// How long a drive can have been going short and still count as answered.
    ///
    /// A meal missed this morning is not a food problem; three days of missed
    /// meals is. "Answered" has to mean answered *reliably*, or a settlement
    /// would start laying in stores on the strength of one good dinner and
    /// stop again on the next empty afternoon.
    pub const RELIABLY: u32 = 24;

    /// Whether this need is answered, and looks like staying answered.
    ///
    /// A drive nobody has is answered by default, which matters for the
    /// chains: a drive whose predecessor does not exist should not be locked
    /// out for ever.
    pub fn is_answered(&self, drive_type: DriveType) -> bool {
        // A need that is quiet only because it is itself shut out does not
        // count as answered for whatever stands after it. Otherwise a chain
        // unlocks itself from the far end: Preparedness falls quiet while
        // Sustenance goes unmet, reads as satisfied because it is low, and
        // opens Luxury on the strength of it.
        if !self.is_unlocked(drive_type) {
            return false;
        }

        self.get(drive_type)
            .map(|drive| drive.value < drive.threshold && drive.denied_ticks < Self::RELIABLY)
            .unwrap_or(true)
    }

    /// Whether everything standing before this drive has been answered.
    ///
    /// See [`DriveType::unlocked_by`]. A hungry agent should not be thinking
    /// about saving food for later, so Preparedness stays shut while Hunger
    /// and Sustenance are unmet, and opens when they are.
    pub fn is_unlocked(&self, drive_type: DriveType) -> bool {
        drive_type
            .unlocked_by()
            .iter()
            .all(|before| self.is_answered(*before))
    }

    /// What is still standing between this drive and being worth anything.
    ///
    /// For explaining an agent to somebody, and for tests.
    pub fn what_is_still_wanted_before(&self, drive_type: DriveType) -> Vec<DriveType> {
        drive_type
            .unlocked_by()
            .iter()
            .copied()
            .filter(|before| !self.is_answered(*before))
            .collect()
    }

    /// Get a drive by type
    pub fn get(&self, drive_type: DriveType) -> Option<&Drive> {
        self.drives.iter().find(|d| d.drive_type == drive_type)
    }

    /// Get a mutable drive by type
    pub fn get_mut(&mut self, drive_type: DriveType) -> Option<&mut Drive> {
        self.drives.iter_mut().find(|d| d.drive_type == drive_type)
    }

    /// Get the most urgent active drive
    pub fn most_urgent(&self) -> Option<&Drive> {
        self.drives
            .iter()
            .filter(|d| d.is_active())
            .max_by(|a, b| a.urgency().partial_cmp(&b.urgency()).unwrap())
    }

    /// Alias for most_urgent (used in TDD tests)
    pub fn get_most_urgent(&self) -> Option<&Drive> {
        self.most_urgent()
    }

    /// Update all drives for one tick
    pub fn tick(&mut self) {
        for drive in &mut self.drives {
            drive.tick();
        }
    }

    /// Tick every drive, knowing whether the agent's immediate needs are met
    pub fn tick_with_security(&mut self, secure: bool) {
        for drive in &mut self.drives {
            drive.tick_with_security(secure);
        }
    }

    /// Tick every drive against the agent's situation.
    ///
    /// The shelter drive is read before the rest are ticked and handed to them
    /// in the context, because Construction is specified to rise partly on
    /// "drive synergy" and this is what that means: wanting to be out of the
    /// weather is a reason to build something.
    pub fn tick_in(&mut self, ctx: &DriveContext, secure: bool) {
        let mut ctx = ctx.clone();
        ctx.at_leisure = secure;
        ctx.shelter_pressing = self
            .get(DriveType::Shelter)
            .map(|drive| drive.value)
            .unwrap_or(0.0);

        // Which chains are open has to be settled before anything moves, or
        // a drive would be judged against predecessors that had already
        // shifted under it this same tick
        let open: Vec<bool> = self
            .drives
            .iter()
            .map(|drive| self.is_unlocked(drive.drive_type))
            .collect();

        for (drive, open) in self.drives.iter_mut().zip(open) {
            if open {
                drive.tick_in(&ctx, secure);
            } else {
                // Not merely held where it stands: a need that is out of reach
                // stops being felt. Somebody who has gone hungry for a week is
                // not sitting on a banked-up wish for a finer coat, waiting to
                // spend it the moment they eat.
                drive.fall_quiet();
            }
        }
    }

    /// Get all active drives sorted by urgency
    pub fn active_drives(&self) -> Vec<&Drive> {
        let mut active: Vec<&Drive> = self.drives
            .iter()
            .filter(|d| d.is_active())
            .collect();

        active.sort_by(|a, b| b.urgency().partial_cmp(&a.urgency()).unwrap());
        active
    }
}

impl Default for DriveState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_creation() {
        let drive = Drive::new(DriveType::Hunger);
        assert_eq!(drive.value, 0.0);
        assert_eq!(drive.weight, 1.0);
        assert!(!drive.is_active());
    }

    #[test]
    fn test_drive_increase_decrease() {
        let mut drive = Drive::new(DriveType::Hunger);
        
        drive.increase(0.5);
        assert_eq!(drive.value, 0.5);
        
        drive.decrease(0.2);
        assert_eq!(drive.value, 0.3);
    }

    #[test]
    fn test_drive_clamping() {
        let mut drive = Drive::new(DriveType::Hunger);
        
        drive.increase(2.0);
        assert_eq!(drive.value, 1.0);
        
        drive.decrease(2.0);
        assert_eq!(drive.value, 0.0);
    }

    #[test]
    fn test_drive_activation() {
        let mut drive = Drive::new(DriveType::Hunger);
        assert!(!drive.is_active());
        
        drive.value = 0.8;
        assert!(drive.is_active());
    }

    #[test]
    fn test_drive_state_creation() {
        let state = DriveState::new();
        assert_eq!(state.drives.len(), 15);
    }

    #[test]
    fn test_drive_state_get() {
        let state = DriveState::new();
        let hunger = state.get(DriveType::Hunger).unwrap();
        assert_eq!(hunger.drive_type, DriveType::Hunger);
    }

    #[test]
    fn test_most_urgent() {
        let mut state = DriveState::new();
        
        state.get_mut(DriveType::Hunger).unwrap().value = 0.8;
        state.get_mut(DriveType::Safety).unwrap().value = 0.9;
        
        let most_urgent = state.most_urgent().unwrap();
        assert_eq!(most_urgent.drive_type, DriveType::Safety);
    }

    #[test]
    fn test_tick_accumulation() {
        let mut drive = Drive::new(DriveType::Hunger);
        let initial = drive.value;
        
        drive.tick();
        
        assert!(drive.value > initial);
    }
}
