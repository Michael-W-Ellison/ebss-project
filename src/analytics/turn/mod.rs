// src/analytics/turn/mod.rs
//! A turn of the world, in the order it happens.
//!
//! `tick` was 852 lines, and its actual shape - a run of world phases, then
//! everybody taking a turn, then a second run of world phases - was buried
//! under six hundred and seventy lines of per-agent decision code sitting in
//! the middle of it. The order of the phases is argued over in the comments
//! below, and several of those arguments were bought with a measurement: the
//! beasts look before they move rather than after, the world is ticked once
//! rather than twice, what a body has to pass goes back on the ground before
//! anybody smells it. None of that could be read while the middle of the
//! function was longer than the whole of anything else in the file.
//!
//! - [`each_one`] holds one person's turn: the goals, the choosing, what the
//!   choice takes, and what came of it.
//! - The world phases stay named methods on `Simulation`, called from here in
//!   the order the model needs them.
//!
//! Nothing about what the model does changed in the move: three seeds run six
//! hundred ticks give byte-identical worlds either side of it.

pub mod each_one;

use super::Simulation;
use log::{debug, warn};

impl Simulation {
    /// Execute one simulation tick
    pub fn tick(&mut self) {
        // Food does not sit on a fire forever: it is taken off, or it burns
        // away. Either way the smell of cooking is a passing thing, so old
        // contents are cleared before scents are worked out.
        self.clear_finished_cooking();

        // Let agents smell nearby food and water before they perceive and act,
        // so world resources reach the percept/memory pipeline this tick.
        self.emit_scents();

        // Process population lifecycle (aging, starvation, deaths, reproduction)
        // This also increments the tick counter and updates all agents
        self.population.tick();

        // Sync simulation tick with population tick
        self.current_tick = self.population.current_tick;

        // Let agents look around them. Sight needs both the population and the
        // world, which only exist together here, so this is the one place it
        // can happen - and until it did, agents found food by smell alone.
        self.population.process_exploration_with_world(&mut self.world);

        // Looking around fills a head faster than talking does, so what
        // nobody has a use for goes out of it again after the looking rather
        // than before
        {
            let now = self.current_tick;
            for agent in self.population.agents.iter_mut() {
                if agent.state.is_alive {
                    agent.forget_what_does_not_matter(now);
                }
            }
        }

        // World systems - climate, fauna, flora - are ticked by World::tick
        // further down this function. Ticking them here as well ran the whole
        // living world at double speed: animals aged, starved, bred and grazed
        // twice for every tick an agent lived through.

        // A man sitting at a fire with a bright stone in his hand may notice
        // what the fire does to it
        self.somebody_notices_something();

        // And the ground they fouled last season comes up in berries
        self.what_was_dropped_comes_up();

        // Grain carried through a wet season starts growing in the pack, and
        // what is dropped out of a pack takes root where it falls
        self.what_got_wet_sprouts();
        self.what_was_dropped_takes_root();

        // Let hungry predators try their luck with the people
        self.process_predator_attacks();

        // Update exposure damage for all agents
        self.update_agent_exposure();

        // Tell each agent what the world around it is doing, so that next
        // tick its drives rise on the conditions the design document gives
        // them rather than on a clock
        self.read_the_situation();

        // Put back on the ground what came off it
        self.return_what_the_living_and_the_dead_leave();

        // And feel about whatever is standing in the way
        self.feel_about_what_stands_in_the_way();
        self.square_up_to_the_people_i_resent();

        // The same question from the other side. A beast has two drives worth
        // the name - eat, and do not be eaten - and until now it had neither
        // opinion about us: a deer stood placidly in a field while somebody
        // walked up to it with a spear.
        // Everybody takes in what is round them first: what would eat them,
        // and who else is about.
        //
        // Before the beasts move, not after. The first cut ran this at the
        // end of the tick and it saw almost nothing: a wolf pack that has
        // just been frightened off by the man it walked up to is nine paces
        // away by the time anybody looks, so the man never learned there were
        // wolves there at all.
        self.what_everybody_saw_that_frightened_them();
        self.who_everybody_saw();

        self.what_the_beasts_make_of_us();
        self.the_beasts_act_on_it();

        // And whoever was standing near enough to watch a thing dry out in
        // the sun now knows something they did not
        self.who_saw_that_dry();
        self.what_the_fire_hardened();
        self.who_came_back_to_look();

        // And whoever has been living on a midden or beside a body may be
        // about to find out what that costs
        self.what_the_ground_underfoot_does();

        // And whoever has been sitting at a fire with clay in their pack may
        // be about to find out what a fire does to clay
        self.what_the_embers_did();

        // And whoever has weakened under a load they could carry last month
        // sets down what they can no longer hold. Before the turn, so that
        // somebody who wakes overloaded has room to gather with by the time
        // they choose to.
        self.what_nobody_can_carry_any_more();



        debug!("=== Tick {} ===", self.current_tick);

        // And everybody in it takes a turn - see `turn::each_one`.
        self.everybody_takes_a_turn();

        // What everybody makes of their provisions, and the winter coming
        self.reckon_what_is_put_by();

        // Process environmental damage (exposure, falling, disease)
        self.process_environmental_damage();

        // Process building production collection (every 50 ticks)
        // Agents near production buildings automatically collect resources
        if self.current_tick % crate::environment::seasons::ONCE_EVERY_FEW_DAYS == 0 {
            self.process_building_production_collection();
        }

        // Process building maintenance (every 100 ticks)
        // Generate maintenance tasks for buildings in poor condition
        if self.current_tick % crate::environment::seasons::ONCE_A_WEEK == 0 {
            self.process_building_maintenance();
        }

        // Lies are found out by walking to the place - see the sight pass in
        // `Population::process_exploration_with_world`.
        //
        // There used to be a second path here: a sweep every hundred ticks
        // over remembered claims, checking each with `verify_resource_claim`.
        // That reads the agent's own map as though it were ground truth, and
        // an agent's map holds what it has been told as well as what it has
        // seen, so the check confirmed hearsay against itself. Measured with
        // lying switched off entirely, it still accused every agent of being
        // a proven liar to twenty-seven others - every one of those
        // accusations false, and none of them from the sight pass, which
        // fired not once. It is retired rather than repaired: standing on the
        // spot is the honest test and the sweep cannot be made into one.
        // Process pregnancies and births
        self.process_pregnancies_and_births();

        // Process nursing for infants
        // The small children are fed out of their parents before anybody
        // nurses, because for a child of five and under that is where the food
        // comes from: "child agents automatically receive their food/water
        // from their parent agent's internal food energy and water". This also
        // fills the hands of whoever is carrying somebody under two.
        self.feed_the_small_children();
        self.process_nursing();

        // Tick world (building construction progress, etc.)
        self.world.tick();

        // And what the wild things do about the people in it. Nothing in the
        // fauna module knew agents existed except the predator pass, so a deer
        // stood where it stood while a settlement walked up to it - which is
        // most of why a hunt was a matter of finding an animal rather than of
        // stalking one. See ISSUES_FOUND #57.
        {
            let people: Vec<(i32, i32)> = self
                .population
                .agents
                .iter()
                .filter(|agent| agent.state.is_alive)
                .map(|agent| (agent.state.position.0, agent.state.position.1))
                .collect();

            self.world.animals.shy_away_from(&people);
        }

        // Apply religious building effects to agent happiness
        self.apply_religious_effects();

        // Log statistics every 10 ticks
        if self.current_tick % crate::environment::seasons::ONCE_A_DAY == 0 {
            self.log_statistics();
        }

        // Check if autosave should trigger
        if let Err(e) = self.check_autosave() {
            warn!("Auto-save failed: {}", e);
        }
    }
}
