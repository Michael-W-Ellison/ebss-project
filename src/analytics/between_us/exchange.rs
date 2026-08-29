// src/analytics/between_us/exchange.rs
//! Trading, taking, and giving.
//!
//! Who is worth approaching, what the two of them would swap, what somebody
//! would hand over - and the two that are not trades at all: taking from
//! somebody because you are desperate, and giving to somebody who needs it more
//! than you do.
//!
//! Part of how one agent stands towards another - see [`super`].

use super::super::Simulation;

impl Simulation {
    /// Execute an action in the environment and return the result
    /// Which trade a thing taken off the land belongs to.
    ///
    /// The same split the experience grants already used, in one place, so
    /// that what a hand is worth at a job and what the job teaches it are
    /// never allowed to drift apart.
    pub(in crate::analytics) fn trade_for_gathering(
        resource_type: crate::world::ResourceType,
    ) -> crate::agents::skills::SkillType {
        use crate::agents::skills::SkillType;
        use crate::world::ResourceType;

        match resource_type {
            ResourceType::Wood => SkillType::Woodcutting,
            ResourceType::Stone | ResourceType::Iron | ResourceType::Coal
            | ResourceType::Clay | ResourceType::Sand => SkillType::Mining,
            ResourceType::Grain => SkillType::Farming,
            ResourceType::Food | ResourceType::Herbs => SkillType::Herbalism,
            ResourceType::Flax | ResourceType::Cotton => SkillType::Farming,
            ResourceType::Fish => SkillType::Fishing,
            _ => SkillType::Herbalism,
        }
    }

    /// How far an agent will walk to reach a resource while foraging, in
    /// walking (Manhattan) distance
    pub(in crate::analytics) const FORAGE_RADIUS: u32 = 25;

    /// How close an agent has to be to a fire to light it, feed it or cook on
    /// it: near enough to reach into the flames
    pub(in crate::analytics) const FIRE_REACH: i32 = 1;

    /// How much soft litter one unit of spoiled food amounts to
    pub(in crate::analytics) const MUCK_PER_UNIT: f32 = 0.12;

    /// And what a spoiled fish is worth, tipped on the same field.
    ///
    /// Several times a turnip, and the difference is not in the size of it.
    /// Everything else in the pack was grown on the settlement's own ground
    /// and is at best going back where it came from. The fish was grown at
    /// sea. It is the only muck a farming people ever get that leaves the
    /// country better off than it found it.
    pub(in crate::analytics) const MUCK_PER_FISH: f32 = 0.9;

    /// How much a field holds when it is full.
    ///
    /// Wild food regrows about four times slower than a grown settlement eats
    /// it. A handful of fields is what closes that gap: the same patch of
    /// ground yields many times what the hedgerow beside it does.
    pub(in crate::analytics) const FIELD_YIELD: u32 = 80;

    /// Wood a campfire is built from, matching `HeatSourceType::Campfire`
    pub(in crate::analytics) const FIRE_BUILD_WOOD: u32 = 5;

    /// Wood put on to burn, worth about fifty ticks at a campfire's rate
    pub(in crate::analytics) const FIRE_FUEL_WOOD: u32 = 5;

    /// How long food goes on smelling of cooking after it is taken off the
    /// fire, in ticks
    pub(in crate::analytics) const COOKING_SMELL_TICKS: u32 = 60;

    /// How much food fits over a campfire at once
    pub(in crate::analytics) const COOK_BATCH: u32 = 5;

    /// What these two would swap, if anything: what the first has spare and
    /// the second wants, and the other way round.
    ///
    /// "The agents should also use a barter system if they have an abundance
    /// of something another agent wants and that agent has an abundance of
    /// something they want." Both halves, and it returns `None` unless both
    /// hold.
    pub(in crate::analytics) fn what_the_two_of_them_would_swap(
        &self,
        me: usize,
        them: usize,
    ) -> Option<((String, u32), (String, u32))> {
        let mine = self.what_i_would_hand_over(me, them)?;
        let theirs = self.what_i_would_hand_over(them, me)?;

        if mine.0 == theirs.0 {
            return None;
        }

        Some((mine, theirs))
    }

    /// What the first of these would hand the second, if anything.
    ///
    /// One-sided on purpose: it is what a gift is, and it is half of what a
    /// trade is. Abundance is measured against the other pack rather than
    /// against a number — what makes a thing worth handing over is that they
    /// have much less of it than you do, which is a comparison and not a
    /// threshold. The first cut of this asked for six of a thing on one side
    /// and fewer than six on the other, and over eight worlds of ten thousand
    /// ticks a settlement traded once.
    pub(in crate::analytics) fn what_i_would_hand_over(&self, me: usize, them: usize) -> Option<(String, u32)> {
        let mine = self.population.agents[me].what_i_can_spare()?;

        let they_have = self.population.agents[them].how_many_i_have(&mine.0);
        let i_have = self.population.agents[me].how_many_i_have(&mine.0);

        // They want it if they have markedly less of it than I do. A man with
        // forty sticks and a man with thirty-eight are not trading partners.
        if they_have * Self::WHAT_MAKES_IT_WORTH_HAVING >= i_have {
            return None;
        }

        Some(mine)
    }

    /// How many times more of a thing you have to have before it is worth
    /// somebody else's while to take it off you.
    pub(in crate::analytics) const WHAT_MAKES_IT_WORTH_HAVING: u32 = 2;

    /// Somebody within reach worth trading with.
    ///
    /// Trust matters: you do not put a thing in the hands of somebody you
    /// think would take it. What decides that is the same judgement that
    /// decides whether to take their word - see `Agent::would_take_their_word`.
    pub(in crate::analytics) fn somebody_to_trade_with(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        let me = self
            .population
            .agents
            .iter()
            .position(|other| other.id == agent.id)?;

        self.population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, them)| them.id != agent.id && them.state.is_alive)
            .filter(|(_, them)| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER
            })
            .filter(|(_, them)| agent.would_take_their_word(them.id, &them.traits))
            .find(|(them, _)| self.what_the_two_of_them_would_swap(me, *them).is_some())
            .map(|(_, them)| them.id)
    }

    /// How often turning a thing over in your hands tells you what it is for.
    ///
    /// Low, and scaled by the hand doing the turning. It has to be low: this
    /// costs a turn and no materials, so if it were generous it would collapse
    /// the whole chain into an afternoon spent looking at things.
    pub(in crate::analytics) const WHAT_LOOKING_CLOSELY_IS_WORTH: f32 = 0.06;

    /// How far a frightened person gets in one turn.
    ///
    /// Further than a walk, which is the whole difference between running and
    /// going somewhere.
    pub(in crate::analytics) const HOW_FAR_A_FRIGHTENED_PERSON_GETS: i32 = Self::FAR_ENOUGH_AWAY + 4;

    /// And what it takes out of them.
    pub(in crate::analytics) const WHAT_RUNNING_COSTS: f32 = 14.0;

    /// What it costs to get a thing out of the pack, or put it back.
    ///
    /// Nearly nothing, because it is nearly nothing - the point of the action
    /// is the turn it takes rather than the effort, and a turn is what a
    /// person spends by doing this instead of something else.
    pub(in crate::analytics) const WHAT_GETTING_A_THING_OUT_COSTS: f32 = 1.5;

    /// What freezing costs, which is nothing but the turn.
    ///
    /// Deliberately cheap in energy and ruinous in every other way: an agent
    /// that freezes has spent a turn not getting away from the thing that is
    /// about to reach it.
    pub(in crate::analytics) const WHAT_FREEZING_COSTS: f32 = 0.5;

    /// What digging a pit takes out of somebody.
    ///
    /// A real morning's work, and deliberately so: this is the most expensive
    /// single act in the model, because it is the one that buys a settlement a
    /// February.
    pub(in crate::analytics) const WHAT_DIGGING_A_PIT_COSTS: f32 = 22.0;

    /// How near a fire you have to be to hang something in the smoke of it.
    pub(in crate::analytics) const WITHIN_REACH_OF_THE_HEARTH: i32 = 2;

    /// How far gone a thing can be and still be worth preserving.
    ///
    /// Preserving does not undo what has already happened to it: all you get
    /// from drying carrion is dry carrion.
    pub(in crate::analytics) const TOO_FAR_GONE_TO_KEEP: f32 = 0.5;

    /// What laying food out or hanging it in the smoke takes.
    pub(in crate::analytics) const WHAT_DRYING_COSTS: f32 = 3.0;

    /// How much stone comes out of a hole somebody digs.
    pub(in crate::analytics) const WHAT_COMES_OUT_OF_A_HOLE: u32 = 3;

    /// How much a person carries away from a store in one go.
    pub(in crate::analytics) const WHAT_A_PERSON_TAKES_OUT: u32 = 8;

    /// And how much they keep on them when they are standing on it.
    ///
    /// One meal. The store is right there.
    ///
    /// This wants to be *less* than `ENOUGH_NOT_TO_OPEN_THE_STORE`, and the
    /// obvious-looking fix of raising it above so that nobody buries food and
    /// then immediately digs it up again is wrong: at five, a person holding
    /// five or fewer has nothing spare to bury at all, and `Cover` was refused
    /// **3,672 times out of 3,729** with the store left empty. The small
    /// churn is the cheaper of the two failures by a wide margin - it is under
    /// one per cent of the turns in a world, against a store that does not
    /// exist.
    pub(in crate::analytics) const WHAT_A_PERSON_KEEPS_ON_THEM: u32 = 1;

    /// How far somebody will walk for a thing they can see lying on the ground.
    pub(in crate::analytics) const WORTH_WALKING_OVER_FOR: u32 = 12;

    /// Somebody within reach worth taking something off.
    ///
    /// Decided on drive demand, which is what the specification asks for and
    /// what the first cut of this did not do. That one was a temperament roll
    /// - a base chance, nudged by Honest and Greedy and by whether the agent
    /// was starving - and it never looked at what was being taken or what it
    /// was worth. It fired once in eight worlds of ten thousand ticks, and
    /// when it did fire the agent had no idea whether the thing it had just
    /// robbed somebody for was any use to it.
    ///
    /// Now: what would this answer, against what it would cost me later. The
    /// cost runs through the bonds, because in this model everything a person
    /// gets from other people runs through the bonds. And a primary drive
    /// past bearing sets the cost aside, because a man who will be dead by
    /// morning is not weighing his reputation.
    pub(in crate::analytics) fn somebody_to_take_from(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        let me = self
            .population
            .agents
            .iter()
            .position(|other| other.id == agent.id)?;

        // Who would see it. The same reckoning a liar makes about the people
        // in earshot - it is the same kind of decision.
        let watching: Vec<&crate::agents::Agent> = self
            .population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP
            })
            .collect();

        // And what this agent gets from them, which is what it would be
        // spending
        let bonds = if watching.is_empty() {
            0.0
        } else {
            watching
                .iter()
                .map(|them| {
                    agent
                        .relationships
                        .get_relationship(&them.id)
                        .map(|bond| bond.bond_strength)
                        .unwrap_or(0.0)
                })
                .sum::<f32>()
                / watching.len() as f32
        };

        let cost = agent.what_taking_it_would_cost_me(watching.len(), bonds);

        self.population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, them)| them.id != agent.id && them.state.is_alive)
            .filter(|(_, them)| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER
            })
            // The best thing anybody standing here has that this agent wants
            .filter_map(|(them, they)| {
                let (what, how_many) = self.what_i_would_hand_over(them, me)?;
                let gain = agent.what_taking_this_would_answer(&what, how_many);
                Some((they.id, gain))
            })
            .filter(|(_, gain)| agent.would_i_take_it(*gain, cost))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(who, _)| who)
    }

    /// How generous somebody has to feel about a person before handing them
    /// anything for nothing.
    ///
    /// Higher than the bar for trading with them: a trade is square and a gift
    /// is not, so it goes to people you actually think well of.
    pub(in crate::analytics) const WELL_ENOUGH_OF_THEM_TO_GIVE: f32 = 0.4;

    /// Somebody within reach worth giving something to.
    pub(in crate::analytics) fn somebody_to_give_to(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        let me = self
            .population
            .agents
            .iter()
            .position(|other| other.id == agent.id)?;

        let spare = agent.what_i_can_spare()?;

        self.population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, them)| them.id != agent.id && them.state.is_alive)
            .filter(|(_, them)| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER
            })
            .filter(|(_, them)| {
                agent.how_far_i_trust(them.id, &them.traits) >= Self::WELL_ENOUGH_OF_THEM_TO_GIVE
            })
            .filter(|(_, them)| them.what_i_am_short_of().contains(&spare.0.as_str()))
            .find(|(them, _)| self.what_i_would_hand_over(me, *them).is_some())
            .map(|(_, them)| them.id)
    }

    /// Somebody of this agent's own who is worse off than it is, and hungry
    /// enough that the difference matters.
    ///
    /// This is the gift that costs, and it is deliberately kept apart from
    /// `somebody_to_give_to`: that one hands over what is spare, and what is
    /// spare is by definition not a sacrifice. Here an agent hands over food
    /// it is going to want itself, because somebody it loves will not last
    /// the week without it.
    pub(in crate::analytics) fn somebody_of_mine_who_needs_it_more(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        // Nothing to give
        agent.find_best_food_to_eat()?;

        // And an agent already past bearing itself keeps what it has. This is
        // not selfishness so much as arithmetic: two dead people is not
        // better than one.
        if agent.state.is_starving() && agent.nutrition.is_starving() {
            return None;
        }

        self.population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER
            })
            .filter(|them| {
                agent
                    .relationships
                    .get_relationship(&them.id)
                    .is_some_and(|bond| bond.is_loved_one())
            })
            // Worse off than this agent, and badly enough for it to count
            .filter(|them| them.nutrition.is_starving() || them.state.is_starving())
            .filter(|them| them.find_best_food_to_eat().is_none())
            .min_by(|a, b| {
                a.nutrition
                    .energy_reserves
                    .partial_cmp(&b.nutrition.energy_reserves)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|them| them.id)
    }

    /// What going without for somebody is worth to them, against an ordinary
    /// gift.
    ///
    /// More, and it should be: a thing somebody could spare is not the same
    /// as a thing they could not.
    pub(in crate::analytics) const WHAT_GOING_WITHOUT_IS_WORTH: f32 = 0.8;

    /// How near you have to be standing to notice that the midden is growing.
    pub(in crate::analytics) const CLOSE_ENOUGH_TO_SEE_IT_COME_UP: i32 = 6;

    /// What a mouthful of a strange plant that turns out to be food is worth.
    pub(in crate::analytics) const WHAT_ONE_MOUTHFUL_IS_WORTH: f32 = 60.0;

    /// And what one that turns out not to be costs, in health.
    ///
    /// A person carries a hundred. The low end is a bad afternoon; the high
    /// end kills somebody who was not in good condition to start with, which
    /// is what makes tasting a thing a people does carefully and rarely.
    pub(in crate::analytics) const WHAT_A_BAD_PLANT_DOES: (f32, f32) = (12.0, 55.0);
}
