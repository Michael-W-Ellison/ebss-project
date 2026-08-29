// src/analytics/wanting/store.rs
//! Putting by, and taking out again.
//!
//! Whether a load is worth carrying home, whether the store still wants
//! filling, and whether this ground will take a pit.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::environment::Action;

impl Simulation {
    /// Whether what this agent is carrying is a harvest rather than supper.
    ///
    /// True only in autumn, only when there is somewhere within reach to put
    /// it, only while the load is still short of a proper one, and only for
    /// somebody who is not in real trouble. A man who will be dead by morning
    /// eats what is in his hand and the store can wait - that is the same
    /// line `would_i_take_it` uses, and for the same reason.
    pub(in crate::analytics) fn is_this_lot_for_the_store(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> bool {
        use crate::world::Position;

        if !matches!(
            self.world.climate.current_season(),
            crate::environment::seasons::Season::Fall
        ) {
            return false;
        }

        if agent.state.is_starving() || agent.nutrition.is_starving() {
            return false;
        }

        // And a man who is properly hungry eats what is in his hand.
        //
        // The first cut used the desperation line - the same 0.85 that decides
        // whether somebody will rob a neighbour - and that is far too late.
        // It had agents carrying food past their own mouths until they were
        // nearly done for, and burials went from 13.8 a world to 17.9 for it.
        // Being a bit peckish is the price of eating in February. Being
        // hungry is not.
        if Self::how_hungry_is_this_one(agent) > Self::WHAT_HUNGER_STOPS_A_HARVEST {
            return false;
        }

        // Once the load is big enough it stops being worth adding to and
        // starts being worth carrying home
        if Self::how_much_food_is_in_the_pack(agent) >= Self::WHAT_A_HARVEST_TRIP_IS {
            return false;
        }

        let here = Position::new(agent_position.0, agent_position.1);

        // And a store that already holds a lean season's eating does not want
        // any more put in it
        if !self.does_the_store_still_want_filling(here) {
            return false;
        }

        self.world
            .nearest_pit_with_room(here, Self::WORTH_WALKING_TO_THE_STORE)
            .is_some()
    }

    /// How many days running the hedgerows give nothing at all.
    ///
    /// Read off the bearing year rather than named, so that retuning the year
    /// retunes the store with it: this is the longest run of days on which not
    /// one growing thing a person can eat is carrying anything. Fish and meat
    /// are left out on purpose - they never stop, and sizing a winter store on
    /// the assumption that everybody will be fishing is the optimism that
    /// produced the number this replaces. As the year stands it is the
    /// seventy-five days from the last root out of the cold ground to the
    /// first leaf.
    pub(in crate::analytics) fn how_long_the_hedgerows_give_nothing() -> u32 {
        use crate::environment::seasons::DAYS_PER_YEAR;
        static ANSWER: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

        *ANSWER.get_or_init(|| {
            // Round the turn of the year, so a run that straddles new year is
            // one run and not two. Twice through the year, and only count the
            // second lap.
            let mut longest = 0;
            let mut running = 0;
            for day in 0..(DAYS_PER_YEAR * 2) {
                running = if Self::are_the_hedgerows_bearing_on(day % DAYS_PER_YEAR) {
                    0
                } else {
                    running + 1
                };
                if day >= DAYS_PER_YEAR {
                    longest = longest.max(running.min(DAYS_PER_YEAR));
                }
            }
            longest
        })
    }

    /// Whether anything a person can eat is growing anywhere on this day.
    ///
    /// The hedgerows only. Fish and meat never stop, and a store sized or
    /// opened on the assumption that everybody will be fishing is the optimism
    /// this whole entry is about - `Fish` is refused ninety-three times in a
    /// hundred.
    pub(in crate::analytics) fn are_the_hedgerows_bearing_on(day_of_year: u32) -> bool {
        Self::edible_resources()
            .into_iter()
            .map(|(what, _)| what)
            .filter(|what| what.is_it_grown())
            .any(|what| what.is_it_bearing(day_of_year))
    }

    /// The same question about today.
    pub(in crate::analytics) fn are_the_hedgerows_bearing(&self) -> bool {
        Self::are_the_hedgerows_bearing_on(self.world.climate.calendar.day_of_year)
    }

    /// How much one mouth wants put by to see it through the lean season.
    ///
    /// The store exists to cover the stretch the hedgerows give nothing. Past
    /// that it is not a store, it is a hole that food is lost in: measured at
    /// thirty-two worlds, a settlement once kept **479 units** in the ground
    /// and **rotted 520 more**, and what was in there was almost all dried
    /// food in lined pits - the very best the model can do. It was not the
    /// wrong food. It was four years of it.
    ///
    /// That entry is why this was a small number, and the small number was
    /// then sized off "a person gets through about a hundred units in ten
    /// thousand ticks" - a figure from the body this model had before the
    /// starvation clock was corrected, and out by something over two orders of
    /// magnitude against the body it has now. It came to **seven items a mouth
    /// for a winter**, which is half a day's food. Twelve people wanted
    /// eighty-four items put by, a settlement reached that in its first
    /// autumn, and every branch behind this gate - burying, walking to a pit,
    /// digging another, and going out to gather for the store at all - shut
    /// down for the rest of the year. Measured over sixteen worlds: the pits
    /// never held more than **fourteen items** at any point in a year, and
    /// **7,794 items were dropped back on the bush for want of room in a
    /// pack**, against 1,472 that were carried home. Five thrown away for
    /// every one kept.
    ///
    /// Derived now, from the two things it is actually about: what a body eats
    /// in a day, and how many days the land gives it nothing.
    pub(in crate::analytics) fn what_one_mouth_wants_put_by() -> u32 {
        (crate::agents::provision::WHAT_A_BODY_EATS_IN_A_DAY
            * Self::how_long_the_hedgerows_give_nothing() as f32)
            .ceil() as u32
    }

    /// Whether the larder round here still wants filling.
    ///
    /// Not "is there room in the hole" - a hole takes three hundred and a
    /// whole settlement eats about a hundred in a winter, so room was never
    /// once the binding question and a people went on burying until the
    /// ground was full of food nobody would live long enough to eat.
    ///
    /// What is asked instead is whether there is already a lean season's
    /// eating in the ground for the people about, which is a thing somebody
    /// standing in their own camp can see.
    pub(in crate::analytics) fn does_the_store_still_want_filling(&self, here: crate::world::Position) -> bool {
        let mouths = self.how_many_mouths_about(here).max(1);
        let put_by = self
            .world
            .how_much_is_in_the_ground_near(here, Self::WORTH_WALKING_TO_THE_STORE);

        put_by < mouths * Self::what_one_mouth_wants_put_by()
    }

    /// How many living people this store has to see through the winter.
    pub(in crate::analytics) fn how_many_mouths_about(&self, here: crate::world::Position) -> u32 {
        self.population
            .agents
            .iter()
            .filter(|other| other.state.is_alive)
            .filter(|other| {
                let there =
                    crate::world::Position::new(other.state.position.0, other.state.position.1);
                here.distance_to(&there) <= Self::WORTH_WALKING_TO_THE_STORE
            })
            .count() as u32
    }

    /// How much food a person will get through before it goes off on them.
    ///
    /// Not how much they can carry — that is a question about weight and it is
    /// the wrong one. A load of berries a fortnight's eating deep is not a
    /// fortnight's eating, it is a few days' eating and a fortnight's rot.
    ///
    /// Sized well above what anybody needs for a day and well below what a
    /// pack holds, so it bites on somebody standing at a full river and not on
    /// somebody with supper about them.
    /// Three days, and it was **eight items** - two thirds of a day, which is
    /// not "well above what anybody needs for a day", it is under it. The
    /// number was right for the body this model had before the starvation
    /// clock was corrected, and has since been an anti-hoarding cap that fires
    /// on a man with supper in his bag.
    ///
    /// Built off the store gate rather than beside it, so that the ordering
    /// the two have to keep - somebody who would not open the store must not
    /// also be barred from foraging - holds by construction rather than being
    /// a relation between two picked numbers that has to be tested for. It was
    /// tested for, and that test is what caught this.
    pub(in crate::analytics) fn what_a_person_gets_through() -> u32 {
        Self::enough_not_to_open_the_store()
            + crate::agents::provision::WHAT_A_BODY_EATS_IN_A_DAY.ceil() as u32
    }

    /// Whether there is already more food about this person than they will eat
    /// before it spoils.
    ///
    /// The demand half of ISSUES_FOUND #43. That entry stopped a settlement
    /// burying four years of food into a hole by asking what the camp would
    /// eat before winter; nothing asked the same question of a pack. Putting
    /// the world's resources back to what the config actually specified then
    /// cost **eight points of efficiency** — doubling what there is to gather
    /// does not double what anybody eats, it doubles what rots in a pack and
    /// on the grass. See ISSUES_FOUND #49 and #57.
    ///
    /// It counts food rather than meals on purpose. A pack of whole fish
    /// nobody has taken a knife to is the single largest thing that rots on
    /// anybody in this model — 1,398 units in a world against 2,250 of
    /// everything foraged put together — and it is food by weight, by bulk and
    /// by the smell it gives off. What it is not is supper, and going back to
    /// the river for more of it is the mistake this stops.
    pub(in crate::analytics) fn more_food_than_he_will_get_through(agent: &crate::agents::Agent) -> bool {
        agent.how_much_good_food_i_have() >= Self::what_a_person_gets_through()
    }

    /// Whether this agent is carrying a load worth taking to the store.
    pub(in crate::analytics) fn is_the_load_worth_carrying_home(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::Position;

        if !matches!(
            self.world.climate.current_season(),
            crate::environment::seasons::Season::Fall
        ) {
            return None;
        }

        if agent.state.is_starving() || agent.nutrition.is_starving() {
            return None;
        }

        // A hungry man eats the load rather than carrying it home
        if Self::how_hungry_is_this_one(agent) > Self::WHAT_HUNGER_STOPS_A_HARVEST {
            return None;
        }

        if Self::how_much_food_is_in_the_pack(agent) < Self::WHAT_A_HARVEST_TRIP_IS {
            return None;
        }

        let here = Position::new(agent_position.0, agent_position.1);

        if !self.does_the_store_still_want_filling(here) {
            return None;
        }

        let (pit, paces) = self
            .world
            .nearest_pit_with_room(here, Self::WORTH_WALKING_TO_THE_STORE)?;

        if paces > 0 {
            return Some(Action::Move {
                target: (pit.where_it_is.x, pit.where_it_is.y, agent_position.2),
            });
        }

        // Standing on it. Dry it first if it is worth drying, because a hole
        // makes a thing keep four times as long and drying makes it keep
        // twenty.
        let what = agent.what_food_i_can_spare().map(|(what, _)| what)?;

        if agent.is_it_worth_drying(&what) {
            return Some(Action::Dry { what });
        }

        Some(Action::Cover { what })
    }

    /// Digging a store, filling it, or going out for something to fill it
    /// with.
    ///
    /// Three steps and the drive picks whichever it is up to. A person with
    /// more food than they can eat and a pit to hand buries it; a person with
    /// more food than they can eat and no pit digs one; a person with a pit
    /// that has room and nothing to put in it goes and gets something. That
    /// last is the one that matters, and it is the only reason anybody in this
    /// model ever gathers food they are not about to eat.
    pub(in crate::analytics) fn putting_food_by(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::Position;

        let here = Position::new(agent_position.0, agent_position.1);

        let spare = agent.what_food_i_can_spare();

        // Cut it down small before you do anything else with it.
        //
        // This is the difference between a joint and a strip and it is the
        // only reason anybody would bother: a joint takes most of a week in
        // the sun and a strip is dry in two days. Somebody putting food by
        // for a winter has the time and a reason to spend it, where somebody
        // who is simply hungry does not - which is why this sits here and not
        // in `food_action`.
        //
        // The first cut of the portioning work left this out and made strips
        // a second step off a joint with nobody to take it, so a settlement
        // cut three hundred carcasses up a world and made almost no strips at
        // all - the preservation chain built the batch before had quietly
        // lost its way in.
        if let Some((verb, to)) = agent.what_i_would_cut_down_for_keeping() {
            return Some(Action::Work { verb, to });
        }

        // If there is a hole right here with room in it, use it.
        //
        // This goes first, ahead of every way of preserving a thing, and the
        // ordering is the whole lesson of this batch. Burying is one turn and
        // it is what actually gets food through to February. Preserving is
        // several turns and only pays if the food is somewhere it will keep.
        // With the preservation branches in front, a settlement spent some
        // two thousand turns a world cutting, boiling, salting and drying,
        // and put a third as much in the ground as it had before any of it
        // existed - all the machinery working, and the settlement worse off.
        if let Some((what, _)) = spare.clone() {
            if self.world.pit_at(here).is_some_and(|pit| pit.has_room()) {
                return Some(Action::Cover { what });
            }
        }

        // Salt it, if there is salt for it.
        //
        // Salting is the third way of keeping a thing and the only one that
        // needs neither a week of sun nor a fire kept going - which makes it
        // the answer in a wet autumn, when the drying branch below simply
        // never comes good. It sits above drying because salt in the pack is
        // salt already paid for: leaving it there while food turns is waste.
        if agent.how_many_i_have("salt") >= Self::WHAT_IT_TAKES_TO_SALT_A_LOT {
            if let Some((what, _)) = agent.what_i_could_salt() {
                return Some(Action::Salt { what });
            }
        }

        // And go and make some, if there is a sea to make it out of and a
        // fire to make it over. Only in autumn and only with food worth
        // keeping: boiling the sea in June is a way of spending a day.
        //
        // And only when the sky will not do it for nothing. Drying costs a
        // turn and a fortnight of weather; boiling costs a turn, a fire and
        // the wood to keep it going. Nobody boils the sea on a clear day in
        // October - the first cut left this out and a settlement boiled the
        // sea three hundred and eighty times a world.
        if agent.how_many_i_have("salt") < Self::WHAT_IT_TAKES_TO_SALT_A_LOT
            && spare.is_some()
            && !self.is_the_sky_clear()
            && matches!(
                self.world.climate.current_season(),
                crate::environment::seasons::Season::Fall
            )
            && self.salt_water_within_reach(agent_position).is_some()
            && self
                .nearest_fire_from(agent_position, Self::WITHIN_REACH_OF_THE_HEARTH, true)
                .is_some()
        {
            return Some(Action::Boil);
        }

        // Dry it before you bury it. A hole in the ground makes a thing keep
        // four times as long; drying makes it keep twenty. Doing both is what
        // a store is actually for, and doing neither is why nothing anybody
        // put by ever lasted the winter.
        //
        // Asked of everything in the pack rather than of the one biggest
        // stack. The first cut of this used `what_food_i_can_spare`, which
        // picks by size, and a settlement carries more whole fish than
        // anything else - so the drying branch spent its life being handed a
        // whole fish. That worked, and it should not have: laying a whole
        // fish in the sun turns it. Once whole flesh was correctly refused,
        // the branch went quiet altogether and the winter store fell by a
        // quarter, because a settlement's entire preservation output had been
        // built on drying fish that ought to have rotted.
        if let Some((what, _)) = agent.what_i_could_dry() {
            return Some(Action::Dry { what });
        }

        // Lay it out where you stand, if you do not yet know what that does.
        //
        // This is how anybody ever finds out, and it has to come before
        // burying or nobody ever does. Nobody here is born knowing that cut
        // flesh laid in the sun keeps and a whole fish laid in the sun turns;
        // somebody has to put something down and come back to it.
        //
        // It sat after the pit branches and after the autumn gate to begin
        // with, so it effectively never ran - a fourth circular precondition
        // of the same family as the three the provisioning work turned up.
        // You had to have seen drying to choose to dry, and the only route to
        // seeing it was behind two conditions that were almost never both
        // true. What a settlement actually did instead was lay whole fish out
        // to dry, which worked and should not have.
        //
        // Once somebody knows, the drying branch above catches it first and
        // this goes quiet - which is right. Laying food on the ground is what
        // you do before you know better; drying it deliberately is what you
        // do after.
        if !agent
            .what_i_found_out()
            .contains(Self::THAT_LAYING_IT_OUT_KEEPS_IT)
        {
            if let Some((what, _)) = spare.clone() {
                if crate::world::World::will_this_dry(&what) && self.is_the_sky_clear() {
                    return Some(Action::PutDown { what });
                }
            }
        }

        // Something to bury, and somewhere to bury it - and a store that is
        // not already full of a winter nobody will get to
        if let Some((what, _)) = spare.clone().filter(|_| self.does_the_store_still_want_filling(here))
        {
            if self.world.pit_at(here).is_some_and(|pit| pit.has_room()) {
                return Some(Action::Cover { what });
            }

            if let Some((pit, _)) = self
                .world
                .nearest_pit_with_room(here, Self::WORTH_WALKING_TO_THE_STORE)
            {
                return Some(Action::Move {
                    target: (pit.where_it_is.x, pit.where_it_is.y, agent_position.2),
                });
            }

            // Nowhere to put it. Dig - but only where a hole will go. The
            // first cut asked for one wherever somebody happened to be
            // standing, and the executor refused most of them: measured at
            // 100 attempts a world for 1.7 pits, which is ninety-eight turns
            // spent trying to dig a hole in a lake.
            if self.is_ground_a_pit_will_go_in(here) {
                return Some(Action::Excavate);
            }
        }

        // A store with room in it, and nothing to fill it with. This is the
        // one that makes a settlement gather more than it eats - and it is
        // the one that has to be kept to its season.
        //
        // The first cut of this ran all year. A settlement dug and foraged
        // for a larder in the middle of summer with berries on every bush,
        // spent 351 trips a world on it, and came out ten people smaller for
        // the effort. Nobody puts food by in June. What a person does in
        // autumn, with the year turning and the harvest in, is exactly this.
        if !matches!(
            self.world.climate.current_season(),
            crate::environment::seasons::Season::Fall
        ) {
            return None;
        }

        if self
            .world
            .nearest_pit_with_room(here, Self::WORTH_WALKING_TO_THE_STORE)
            .is_some()
        {
            return Some(Action::Gather {
                resource_type: "food".to_string(),
            });
        }

        // No store anywhere, and the year turning. Dig one.
        //
        // This is what was missing, and it was a circle: digging a pit wanted
        // a surplus in hand, and gathering a surplus for the store wanted a
        // pit to put it in. Neither could happen first. So the moment the
        // land actually went bare in winter the larder stopped being used
        // altogether - burials fell from 10.8 a world to 1.8 exactly when a
        // store was worth most - because the only thing that had ever filled
        // it was food somebody happened to be carrying.
        //
        // Autumn with nowhere to put anything is reason enough to dig.
        if self.is_ground_a_pit_will_go_in(here) {
            return Some(Action::Excavate);
        }

        // And with nothing at all to put by, the trip is still worth making
        // pay. Both of these sit at the *bottom* of this branch on purpose.
        //
        // The first cut of them sat at the top, and it cost a settlement half
        // its winter store and tripled its refused turns: an agent that wanted
        // a bowl and had nothing to carve with returned a refused `Work` every
        // turn instead of burying, drying or storing anything at all. A branch
        // that can refuse must never stand in front of the branches that
        // cannot.
        //
        // Taking what can be carried while standing here anyway. "I am
        // going here or doing this action anyway - is there anything I can do
        // which decreases the time to satisfy a drive without detracting from
        // the current one?" The trip out is the expensive part and the load is
        // nearly free, so somebody standing on a salt flat takes what they can
        // carry rather than what they need today.
        if let Some(what) = self.what_i_should_take_while_i_am_here(agent, agent_position) {
            return Some(Action::Gather {
                resource_type: what,
            });
        }

        None
    }

    /// Whether this agent should go and draw on the store rather than the
    /// land.
    ///
    /// A pit within reach with something in it beats walking out to a berry
    /// bush, which is the whole of what a larder buys.
    pub(in crate::analytics) fn something_out_of_the_store(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::Position;

        // Somebody with a proper meal about them does not open the store.
        //
        // *A* meal is not a proper meal: this asked for an empty pack, and
        // `Cover` hands a person one unit back on its way past, so the
        // condition was never once met by anybody who had just filled a pit.
        //
        // And it counts meals rather than food, which is not the same thing.
        // `is_food` answers yes to an uncut haunch, a stack that has gone
        // over, and raw flesh this one has been ill off - none of which is
        // supper. Counting those shuts the store on exactly the people who
        // most need it open: a man carrying a rotten carcass reads as
        // provisioned.
        if agent.how_many_meals_i_have() >= Self::enough_not_to_open_the_store() {
            return None;
        }

        // And a store is for the stretch when the land gives nothing.
        //
        // Nothing asked this, so a pit within reach was simply the nearest
        // food and a settlement drew on its winter store in July. That is this
        // entry's title: laid down and eaten at the same rate, so it is never
        // a winter store. Measured over a year, the pits held between seven
        // and fourteen items from one end of it to the other and never
        // accumulated - a settlement's whole larder was under one person-day
        // of food, in a model where a body eats fifteen items a day.
        //
        // Somebody genuinely in trouble still opens it, in any month. A man
        // three days into his reserve does not keep larder discipline, and a
        // rule that let him starve beside a full pit would be a worse fault
        // than the one it fixed.
        if self.are_the_hedgerows_bearing()
            && !agent.state.is_starving()
            && !agent.nutrition.is_starving()
        {
            return None;
        }

        let here = Position::new(agent_position.0, agent_position.1);
        let (pit, paces) = self
            .world
            .nearest_full_pit(here, Self::WORTH_WALKING_TO_THE_STORE)?;

        let what = pit.something_to_eat()?.to_string();

        if paces == 0 {
            return Some(Action::PickUp { what });
        }

        Some(Action::Move {
            target: (pit.where_it_is.x, pit.where_it_is.y, agent_position.2),
        })
    }

    /// How much food in the pack is enough that a person leaves the store
    /// shut.
    ///
    /// Two days' worth, which is what it always said it was and is now what it
    /// is: at four items it was under a third of a day, so anybody with a
    /// morning's food about them still opened the pit, and the store was eaten
    /// at very nearly the rate it was laid down. That is the other half of
    /// this entry's title.
    ///
    /// It has to be more than `WHAT_A_PERSON_KEEPS_ON_THEM`, or somebody who
    /// has just filled a pit is locked out of it by the one meal burying
    /// handed them back - which is still true, by a wide margin, now that it
    /// is counted in days.
    pub(in crate::analytics) fn enough_not_to_open_the_store() -> u32 {
        (crate::agents::provision::WHAT_A_BODY_EATS_IN_A_DAY * 2.0).ceil() as u32
    }

    /// Whether a hole will go in here.
    ///
    /// The same question a field asks, and for the same reason: you cannot dig
    /// a pit in a lake or in bare rock. Asked in the decision as well as in
    /// the executor, because an agent standing on the wrong ground would
    /// otherwise ask for a pit every turn for the rest of its life.
    pub(in crate::analytics) fn is_ground_a_pit_will_go_in(&self, here: crate::world::Position) -> bool {
        if self.world.pit_at(here).is_some() {
            return false;
        }

        self.world
            .grid
            .get_tile(&here)
            .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
            .unwrap_or(false)
    }

    /// Salt water close enough to dip a pot in.
    pub(in crate::analytics) fn salt_water_within_reach(&self, from: (i32, i32, i32)) -> Option<crate::world::Position> {
        use crate::world::Position;

        for dy in -Self::AS_FAR_AS_A_POT_GETS_CARRIED..=Self::AS_FAR_AS_A_POT_GETS_CARRIED {
            for dx in -Self::AS_FAR_AS_A_POT_GETS_CARRIED..=Self::AS_FAR_AS_A_POT_GETS_CARRIED {
                let there = Position::new(from.0 + dx, from.1 + dy);
                if self
                    .world
                    .grid
                    .get_tile(&there)
                    .is_some_and(|tile| tile.terrain.is_the_water_salt())
                {
                    return Some(there);
                }
            }
        }

        None
    }

    /// How far somebody will carry a pot of sea water to a fire.
    ///
    /// Not far. Water is heavy, and the point of boiling the sea is that you
    /// are standing beside it.
    pub(in crate::analytics) const AS_FAR_AS_A_POT_GETS_CARRIED: i32 = 4;

    /// What one pot of the sea leaves behind when the water has gone.
    ///
    /// Two. This is why salt is dear and why a settlement that has a flat or
    /// a seam within reach uses that instead: boiling is the answer for
    /// people who have neither.
    pub(in crate::analytics) const WHAT_A_POT_OF_THE_SEA_LEAVES: u32 = 2;

    /// How much salt it takes to keep one lot of food.
    pub(in crate::analytics) const WHAT_IT_TAKES_TO_SALT_A_LOT: u32 = 1;
}
