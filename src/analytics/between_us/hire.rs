// src/analytics/between_us/hire.rs
//! Taking somebody on: work done for a share of what it moves.
//!
//! The reckoning on both sides is in `agents::hire`; this is where it meets
//! the world - who has a kill they cannot get home in time, who is about to be
//! asked, what the one asked would do instead, and what the carrying itself
//! looks like turn by turn (#374).

use crate::agents::hire::{self, Hire, Work};
use crate::analytics::Action;
use crate::world::Position;

use super::super::Simulation;

impl Simulation {
    /// What a fair day's work done and paid does for what the two of them
    /// think of each other: the same step a shared kill is worth.
    pub(in crate::analytics) const WHAT_A_FAIR_DEAL_IS_WORTH: i8 = 3;

    /// Whoever has a kill lying out that they cannot get home before it goes
    /// asks whoever is near enough to hear for a hand with it, for a share,
    /// while one more pair of hands still brings more home to them after the
    /// pay than without; and the one asked agrees if the share is worth more
    /// to them than what they would otherwise go and get. And work that is
    /// done, or can no longer be done, is let go.
    pub(in crate::analytics) fn hands_for_the_carrying(&mut self) {
        self.the_carrying_that_is_over();

        let now = self.current_turn;
        let kills: Vec<(Position, u32, u32, uuid::Uuid)> = self
            .world
            .resources
            .iter()
            .filter(|node| node.is_a_carcass() && node.anything_to_take())
            .filter_map(|node| Some((node.position, node.amount, node.killed_on?, node.killed_by?)))
            .collect();

        for (at, on_it, killed_on, whose) in kills {
            let ticks_left = (killed_on + crate::world::World::HOW_LONG_A_CARCASS_LIES).saturating_sub(now);
            if ticks_left < crate::environment::seasons::TICKS_PER_DAY / 2 {
                continue;
            }
            let Some(owner) = self
                .population
                .agents
                .iter()
                .position(|agent| agent.id == whose && agent.state.is_alive)
            else {
                continue;
            };

            let Some(store) = self.a_store_this_kill_could_go_in(owner, at) else {
                continue;
            };

            let mine = self.what_these_hands_bring_home_off(owner, at, ticks_left);
            let mut theirs: Vec<u32> = self
                .population
                .agents
                .iter()
                .filter_map(|agent| agent.working_for.as_ref())
                .filter(|hire| hire.by == whose && hire.from_and_to().0 == (at.x, at.y))
                .map(|hire| hire.reckoned)
                .collect();

            let within_earshot = self.population.agents[owner].state.position;
            let asked: Vec<usize> = self
                .population
                .agents
                .iter()
                .enumerate()
                .filter(|(i, agent)| {
                    *i != owner
                        && agent.state.is_alive
                        && agent.working_for.is_none()
                        && agent.state.years_old() >= crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
                        && !agent.state.is_starving()
                        && Self::near_enough_to_talk(within_earshot, agent.state.position)
                })
                .map(|(i, _)| i)
                .collect();

            for carrier in asked {
                let his = self.what_these_hands_bring_home_off(carrier, at, ticks_left);
                let Some(pay) =
                    hire::is_another_pair_of_hands_worth_it(on_it, mine, &theirs, his, Hire::A_CARRIERS_SHARE)
                else {
                    continue;
                };
                if !self.would_this_one_carry_for_a_share(carrier, at, store, his, pay) {
                    continue;
                }

                theirs.push(his);
                self.world.carriers_taken_on += 1;
                let agent = &mut self.population.agents[carrier];
                agent.working_for = Some(Hire {
                    by: whose,
                    work: Work::CarryHome { from: (at.x, at.y), to: (store.x, store.y) },
                    share: Hire::A_CARRIERS_SHARE,
                    reckoned: his,
                    taken: 0,
                    handed_over: 0,
                    until: killed_on + crate::world::World::HOW_LONG_A_CARCASS_LIES,
                });
                let knows = &mut agent.exploration_knowledge;
                knows.discover_resource(at, crate::world::ResourceType::Meat, now);
                knows.saw_it_again(at, on_it, now);
            }
        }
    }

    /// A store the meat off this kill could go into for its owner: a pit with
    /// room that is theirs to use - their own, their kin's or the camp's -
    /// nearest the carcass and a walk away from it.
    fn a_store_this_kill_could_go_in(&self, owner: usize, at: Position) -> Option<Position> {
        let agent = &self.population.agents[owner];
        self.world
            .pits
            .iter()
            .filter(|pit| pit.has_room() && agent.may_i_use(&pit.belongs).is_mine_to_use())
            .filter(|pit| self.world.can_walk_between((at.x, at.y), (pit.where_it_is.x, pit.where_it_is.y)))
            .min_by_key(|pit| at.distance_to(&pit.where_it_is))
            .map(|pit| pit.where_it_is)
    }

    /// What this one would bring home off a carcass at `at` with `ticks_left`
    /// before it goes: what there is room for in the pack, a load a day, once
    /// they are there.
    ///
    /// The room there is, and not what the pack would hold empty. Reckoned on
    /// an empty pack a day, every hunter could carry every kill home alone and
    /// nobody was ever asked - while ninety joints in a hundred and sixty went
    /// to the crows on one world in a year. Packs run full (#331's own
    /// finding), and whoever has no room for a joint does not go back for one
    /// (`a_kill_worth_fetching`).
    fn what_these_hands_bring_home_off(&self, who: usize, at: Position, ticks_left: u32) -> u32 {
        let agent = &self.population.agents[who];
        let here = Position::new(agent.state.position.0, agent.state.position.1);
        let getting_there = (crate::world::pace::turns_to_walk(here.distance_to(&at))
            * crate::environment::seasons::TICKS_BETWEEN_PLANS as f32) as u32;
        let a_load = Self::a_load_for(agent);
        hire::what_one_pair_of_hands_brings_home(ticks_left, getting_there, a_load)
    }

    /// How many joints this one has room for in one load.
    fn a_load_for(agent: &crate::agents::Agent) -> u32 {
        hire::a_load(agent.inventory.weight_capacity_remaining(), Self::WHAT_A_JOINT_WEIGHS)
    }

    /// Whether carrying `his` joints home from `at` to `store`, for `pay`
    /// joints of them, is worth this one's while: not hungry, and the pay
    /// worth more than the walking and the cutting cost.
    ///
    /// Priced the way a patch of food is (`what_this_patch_is_worth`), with
    /// every turn of the job counted: what the pay is worth to eat, less what
    /// the walking and cutting cost, over the turns it all takes. The bar is
    /// the one a kill one fetches for oneself has to clear
    /// (`a_kill_worth_fetching`).
    ///
    /// **Not** the best food anywhere. That was the first bar, and nobody
    /// cleared it: 818 askings on a world in a year, all refused, because a
    /// berry patch is a basket a turn and a carcass two joints, so carrying
    /// meat loses to picking even for the whole of the load (measured: the
    /// share came to 0-250 a turn against 1,000-4,600 for the patch). A fed
    /// man putting by is not choosing between this and his supper, and a
    /// hungry one is not asked.
    pub(in crate::analytics) fn would_this_one_carry_for_a_share(
        &self,
        who: usize,
        at: Position,
        store: Position,
        his: u32,
        pay: f32,
    ) -> bool {
        use crate::agents::physiology;

        let agent = &self.population.agents[who];
        if agent.drives.get(crate::core::DriveType::Hunger).is_some_and(|hunger| hunger.is_active()) {
            return false;
        }
        let here = Position::new(agent.state.position.0, agent.state.position.1);
        let energy = self
            .food_database
            .get(&crate::world::ItemType::Meat)
            .map(|template| template.base_nutrition.energy)
            .unwrap_or(physiology::ENERGY_OF_ORDINARY_FOOD)
            * Self::portions_in_one_off(crate::world::ResourceType::Meat);

        let a_load = Self::a_load_for(agent).max(1);
        let loads = his.div_ceil(a_load).max(1);
        let home = at.distance_to(&store);

        let worth = pay * physiology::UNITS_IN_ONE_ITEM * physiology::what_a_unit_of_this_is_worth(energy);
        let costs = crate::agents::provision::what_foraging_costs(here.distance_to(&at), 0.0)
            + loads as f32
                * crate::agents::provision::what_foraging_costs(
                    home,
                    physiology::how_much_work_this_food_is(energy),
                );
        let cutting = his as f32 / Self::JOINTS_A_TURN_OF_CUTTING_TAKES as f32;
        let walking = crate::world::pace::turns_to_walk(here.distance_to(&at))
            + loads as f32 * 2.0 * crate::world::pace::turns_to_walk(home);
        let turns = 1.0 + cutting + walking * Self::TURNS_A_PACE_TAKES;
        let a_turn_of_it = (worth - costs) / turns;

        a_turn_of_it > 0.0
    }

    /// Carrying that is finished, or can no longer be done, is let go: the
    /// kill gone and nothing owed in the pack, the store gone or full, the
    /// one it was for dead, or the time up. A job seen through leaves the two
    /// of them thinking better of each other.
    fn the_carrying_that_is_over(&mut self) {
        let now = self.current_turn;
        for i in 0..self.population.agents.len() {
            let Some(hire) = self.population.agents[i].working_for.clone() else {
                continue;
            };
            let (from, to) = hire.from_and_to();
            let alive = self.population.agents[i].state.is_alive;
            let employer = self.population.agents.iter().position(|agent| agent.id == hire.by);
            let employer_alive = employer.is_some_and(|e| self.population.agents[e].state.is_alive);
            let store = self.world.pit_at(Position::new(to.0, to.1)).is_some_and(|pit| pit.has_room());
            let still_on_it = self
                .world
                .resources
                .iter()
                .any(|node| node.is_a_carcass() && node.anything_to_take() && (node.position.x, node.position.y) == from);
            let in_hand = self.population.agents[i].how_many_i_have("meat");
            let owed = hire.still_owed().min(in_hand);

            let over = !alive
                || !employer_alive
                || !store
                || now > hire.until + crate::environment::seasons::TICKS_PER_DAY
                || (!still_on_it && owed == 0);
            if !over {
                continue;
            }

            self.population.agents[i].working_for = None;
            self.world.joints_kept_as_pay += u64::from(hire.taken.saturating_sub(hire.handed_over));
            if hire.handed_over > 0 && alive && employer_alive {
                let worker_id = self.population.agents[i].id;
                if let Some(e) = employer {
                    self.population.agents[e]
                        .relationships
                        .get_or_create_relationship(worker_id, now)
                        .positive_interaction(Self::WHAT_A_FAIR_DEAL_IS_WORTH, now);
                }
                self.population.agents[i]
                    .relationships
                    .get_or_create_relationship(hire.by, now)
                    .positive_interaction(Self::WHAT_A_FAIR_DEAL_IS_WORTH, now);
            }
        }
    }

    /// The next step of the work this one took on, if there is any left:
    /// what is owed taken to the store once the hands are full or the kill
    /// is done, and otherwise back to the kill to cut another load.
    pub(in crate::analytics) fn the_carrying_i_took_on(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let hire = agent.working_for.as_ref()?;
        let (from, to) = hire.from_and_to();
        let here = (agent_position.0, agent_position.1);

        let owed = hire.still_owed().min(agent.how_many_i_have("meat"));
        let still_on_it = self.current_turn < hire.until
            && self
                .world
                .resources
                .iter()
                .any(|node| node.is_a_carcass() && node.anything_to_take() && (node.position.x, node.position.y) == from);
        let room = agent.inventory.weight_capacity_remaining() >= Self::WHAT_A_JOINT_WEIGHS;

        if owed > 0 && (!room || !still_on_it) {
            if here == to {
                return Some(Action::Cover { what: "meat".to_string() });
            }
            return Some(Action::Move { target: (to.0, to.1, agent_position.2) });
        }

        if still_on_it && room {
            if here == from {
                return Some(Action::Gather { resource_type: "meat".to_string() });
            }
            return Some(Action::Move { target: (from.0, from.1, agent_position.2) });
        }

        None
    }

    /// Joints just cut off the carcass this one was taken on to carry.
    pub(in crate::analytics) fn cut_for_somebody_else(&mut self, agent_index: usize, at: Position, took: u32) {
        if let Some(hire) = self.population.agents[agent_index].working_for.as_mut() {
            if hire.from_and_to().0 == (at.x, at.y) {
                hire.taken += took;
            }
        }
    }

    /// How much of the meat in this one's hands is owed to the store they
    /// are standing on, if it is the store they are carrying to.
    pub(in crate::analytics) fn owed_to_this_store(&self, agent_index: usize, here: Position) -> Option<u32> {
        let hire = self.population.agents[agent_index].working_for.as_ref()?;
        (hire.from_and_to().1 == (here.x, here.y)).then(|| hire.still_owed())
    }
}
