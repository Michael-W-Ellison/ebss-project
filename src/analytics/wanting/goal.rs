// src/analytics/wanting/goal.rs
//! Layer 2: what answering a drive would look like, and what counts as enough.
//!
//! A drive is a pressure and a strategy is a way. Between them is a question
//! neither answers: **when is this done?** A pressure goes quiet when the body
//! is momentarily comfortable, which is not the same as the need being met - a
//! man with one mouthful in him is not thirsty and is not provisioned.
//!
//! # The layer was not absent. It was distributed.
//!
//! This is the finding, and it is why the layer reads as missing when it is
//! mostly working. Four separate spellings, none of them called a goal:
//!
//! - **The thresholds are Layer 1 drives.** `Preparedness` is
//!   `short_of(food_put_by, ENOUGH_FOOD)` plus the same for materials and
//!   tools; `Sustenance` is the food half again. "Enough put by to see a
//!   winter out" is not a goal under Hunger in this model - it was promoted to
//!   a drive of its own, with the threshold as a constant inside it.
//! - **The commitment is `Errand`.** `stick_to_the_errand` holds an agent to
//!   what it set out to do until a clearly harder drive takes over, which is
//!   the "hold it until it is met" half, and it works.
//! - **The food reckoning is `provision::WhatIsPutBy`**, which asks exactly
//!   "how many days in hand against how long the winter is" and answers on a
//!   named rung. It exists for food and for nothing else.
//! - **And `core::goals` is called Goal and is none of the above.**
//!   `InternalGoal` and `ExternalGoal` are emotions and property -
//!   `ReduceStress`, `OwnHouse` - and #187 measured the branch that carries
//!   them: un-gating it cost five per cent of person-days.
//!
//! So this table does not add a fourth mechanism. It **names** the goals, and
//! for each one says what enough means *and which machinery already asks*, so
//! that the layer can be read and counted. Where a threshold already exists it
//! is read from where it lives rather than restated -
//! `who_already_asks_it` is the honest half, and
//! `the_goal_table_and_the_drives_cannot_drift` holds it to that.
//!
//! One goal is asked by nobody, and that is the one thing here that is new:
//! see `Goal::ObtainPotableWater`.

use crate::core::DriveType;

/// What answering a drive would look like.
///
/// One per distinct state of affairs, not per drive: hunger has two - eat
/// today, and have something by for the winter - because they are answered by
/// different work and met at different times, and conflating them is what #213
/// is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Goal {
    /// Water about me, and not only water in me.
    ///
    /// **The one goal nothing in this model asks.** A container is filled as a
    /// side effect of drinking at a source, and thirst only rises when the
    /// body is already dry, so nobody ever tops up against tomorrow. A dry
    /// spell then empties a world - see ISSUES_FOUND #189.
    ObtainPotableWater,

    /// Something to eat today.
    ObtainEdibleCalories,

    /// And something left over that will still be there in the winter.
    PreserveFood,

    /// Somewhere to be when the weather turns.
    SecureSleepingPlace,

    /// A roof better than the one there is.
    ImproveLocalShelter,

    /// An edge, because nearly every other making wants one.
    AcquireCuttingTool,
}

/// What counts as enough of it.
///
/// Named rather than a bare number so the threshold is a thing somebody can
/// read and argue with. Where the number already lives somewhere it is taken
/// from there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Enough {
    /// This many days of it within reach, in the units a body burns in a day.
    DaysInHand(f32),

    /// This many units put by, on the same count `Preparedness` reads.
    UnitsPutBy(f32),

    /// This many of them to hand, on the same count `Preparedness` reads.
    ToolsToHand(f32),

    /// One, and a second is no better than the first.
    OneOfThem,

    /// Nothing in this world can say whether it is met, and this is why.
    NothingCanSay(&'static str),
}

/// Which machinery already asks whether a goal is met.
///
/// The point of the layer is to be able to see this. A goal nobody asks is a
/// gap; a goal two things ask is a pair of books that will drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhoAsks {
    /// A Layer 1 drive carries the threshold inside it.
    ThisDrive(DriveType),

    /// `provision::WhatIsPutBy`, which is the food reckoning.
    TheLarderReckoning,

    /// The shelter machinery: `needs_shelter` and `nearest_shelter_from`.
    TheShelterCheck,

    /// Nobody. This is the gap.
    Nobody,
}

impl Goal {
    /// Which drive this is a way of answering.
    pub fn answers(&self) -> DriveType {
        match self {
            Goal::ObtainPotableWater => DriveType::Thirst,
            Goal::ObtainEdibleCalories => DriveType::Hunger,
            // **Not Hunger.** Laying food by is answered by Preparedness in
            // this model, and that is not an accident of naming: a full man
            // with an empty pit is not hungry and does need to work. See the
            // note at the head of this file.
            Goal::PreserveFood => DriveType::Preparedness,
            Goal::SecureSleepingPlace | Goal::ImproveLocalShelter => DriveType::Shelter,
            // An edge is not a need in itself. It is wanted because the things
            // that answer needs want one - an enabler rather than a satisfier.
            Goal::AcquireCuttingTool => DriveType::Preparedness,
        }
    }

    /// The goals that answer a need, nearest-term first.
    pub fn all_for(need: DriveType) -> &'static [Goal] {
        match need {
            DriveType::Thirst => &[Goal::ObtainPotableWater],
            DriveType::Hunger => &[Goal::ObtainEdibleCalories],
            DriveType::Preparedness => &[Goal::PreserveFood, Goal::AcquireCuttingTool],
            DriveType::Shelter => &[Goal::SecureSleepingPlace, Goal::ImproveLocalShelter],
            _ => &[],
        }
    }

    /// Every goal declared, for counting what is asked and what is not.
    pub fn every_one() -> &'static [Goal] {
        &[
            Goal::ObtainPotableWater,
            Goal::ObtainEdibleCalories,
            Goal::PreserveFood,
            Goal::SecureSleepingPlace,
            Goal::ImproveLocalShelter,
            Goal::AcquireCuttingTool,
        ]
    }

    /// What counts as enough.
    pub fn enough(&self) -> Enough {
        match self {
            // A day's drink about him. Anything less and he is at the river
            // every day whatever the weather is doing.
            Goal::ObtainPotableWater => Enough::DaysInHand(1.0),

            // Supper, which is the line `WhatIsPutBy` calls `NotTheDay`.
            Goal::ObtainEdibleCalories => Enough::DaysInHand(1.0),

            // Read from where it lives, so there is one number and not two.
            Goal::PreserveFood => Enough::UnitsPutBy(DriveType::ENOUGH_FOOD),
            Goal::AcquireCuttingTool => Enough::ToolsToHand(DriveType::ENOUGH_TOOLS),

            Goal::SecureSleepingPlace => Enough::OneOfThem,

            Goal::ImproveLocalShelter => Enough::NothingCanSay(
                "a building has a condition and nothing mends one, so there is \
                 no better roof to be had - see ISSUES_FOUND #237",
            ),
        }
    }

    /// Which machinery already asks whether this is met.
    pub fn who_already_asks_it(&self) -> WhoAsks {
        match self {
            // The gap, and the reason this table is more than a list.
            Goal::ObtainPotableWater => WhoAsks::Nobody,

            Goal::ObtainEdibleCalories => WhoAsks::TheLarderReckoning,
            Goal::PreserveFood | Goal::AcquireCuttingTool => {
                WhoAsks::ThisDrive(DriveType::Preparedness)
            }
            Goal::SecureSleepingPlace => WhoAsks::TheShelterCheck,
            Goal::ImproveLocalShelter => WhoAsks::Nobody,
        }
    }

    /// Whether the world can say if this one is met at all.
    pub fn can_be_told(&self) -> bool {
        !matches!(self.enough(), Enough::NothingCanSay(_))
    }
}

impl crate::analytics::Simulation {
    /// How far short of a goal this one is: nought met, one nothing done.
    ///
    /// Reads the machinery named in `who_already_asks_it` rather than keeping
    /// its own count, so the layer can be inspected without becoming a second
    /// set of books.
    pub fn how_short_of(
        &self,
        goal: Goal,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> f32 {
        let shortfall = |have: f32, wanted: f32| {
            if wanted <= 0.0 {
                0.0
            } else {
                (1.0 - have / wanted).clamp(0.0, 1.0)
            }
        };

        match goal {
            // **Layer 5 gates Layer 2 here, and it has to.** A man with no
            // vessel is not short of carried water, he is unable to carry
            // any - and a goal that can never be met would keep its drive
            // asking for ever.
            Goal::ObtainPotableWater => {
                if !agent.inventory.has_a_container() {
                    return 0.0;
                }
                shortfall(agent.inventory.available_water(), Self::A_DAYS_DRINK)
            }

            Goal::ObtainEdibleCalories => match agent.state.what_the_larder_says.as_ref() {
                Some(larder) => shortfall(larder.days_in_hand, 1.0),
                // Before the first reckoning of the year has run, which is the
                // only time a live agent has none.
                None => 0.0,
            },

            Goal::PreserveFood => shortfall(
                agent.food_put_by() as f32,
                crate::core::DriveType::ENOUGH_FOOD,
            ),

            // An edge, by the only spelling this model has of one: something
            // in the pack that a making asks to be held while it is done.
            Goal::AcquireCuttingTool => {
                if agent.how_many_i_have("stoneknife") > 0
                    || agent.how_many_i_have("metalblade") > 0
                {
                    0.0
                } else {
                    1.0
                }
            }

            Goal::SecureSleepingPlace => {
                if self.nearest_shelter_from(agent_position).is_some() {
                    0.0
                } else {
                    1.0
                }
            }

            // Cannot be short of a thing nothing can tell you about.
            Goal::ImproveLocalShelter => 0.0,
        }
    }

    /// What a day's drink comes to in carried units.
    ///
    /// Thirst rises 0.012 a tick and a drink takes half the drive off, so a
    /// day of forty-eight ticks is about one drink - and `ConsumeCarriedWater`
    /// deals in whole units. One unit is a day.
    pub const A_DAYS_DRINK: f32 = 1.0;

    /// **Top up before you need it.**
    ///
    /// The one goal in the table nobody asks. A container is filled as a side
    /// effect of drinking at a source, and thirst only rises once the body is
    /// already dry, so nobody ever fills a skin against tomorrow: a settlement
    /// walks to the water every single day, and a dry spell empties a world -
    /// see ISSUES_FOUND #189.
    ///
    /// Deliberately the *last* thing considered, taken only where the turn
    /// would otherwise be spent standing still. A goal that can outrank a
    /// pressing drive is a drive, and this layer is not for making more of
    /// those; what it is for is spending a turn nobody wanted on something
    /// that will be wanted tomorrow.
    pub fn top_up_before_you_need_it(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<crate::environment::Action> {
        if self.how_short_of(Goal::ObtainPotableWater, agent, agent_position) <= 0.0 {
            return None;
        }

        // The same question `DrinkFromLocalSource` asks, so there is one
        // opinion about whether there is water to be had here.
        self.drinkable_water_within_reach(agent, agent_position)
            .then(|| crate::environment::Action::Gather {
                resource_type: "water".to_string(),
            })
    }
}
