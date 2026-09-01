//! A spear tells when you stand your ground, not only when you go looking.
//!
//! `Action::Fight` took a `weapon` and never read it, so a man standing over
//! his child with a flint spear in his hand fought the wolf exactly as he
//! would have fought it empty-handed. `hunting`, two modules away, is careful
//! about precisely this - the specification says so twice - and the two sides
//! of one problem disagreed.
//!
//! The cause was a vocabulary, not an oversight. Three places read the weapon
//! out of `Agent::equipment`, which nothing in this model has ever put a
//! weapon into: the action's own field, `own_strength`, and by omission the
//! fight itself. The live vocabulary is `environment::making`, reached through
//! `how_much_my_tools_help`. See ISSUES_FOUND.md #100.

use crate::agents::{AgentConfig, InventoryItem, LifeStage, Population, SkillType};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

/// A world with nothing else wandering about in it.
fn an_empty_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

/// One adult, and a wolf close enough to be a problem.
fn a_man_and_a_wolf() -> (Simulation, uuid::Uuid) {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (30, 31))
        .expect("a wolf should spawn");
    let which = world.animals.get_all()[0].id;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = 100.0;
    (simulation, which)
}

fn give_him_a_spear(simulation: &mut Simulation) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("spear".to_string(), 1, 2.0));
}

/// How many blows it takes to put the wolf down, averaged over forty fights.
///
/// This is the question the specification asks - *"a wooden spear... should
/// take several attacks to kill the animal... a flint spear should reduce the
/// number of attacks"* - and it is the one a health-taken-off measure cannot
/// answer, because a wolf that dies to the first blow absorbs no more than a
/// wolf's worth of it however hard the blow was.
fn blows_to_put_it_down(with_a_spear: bool) -> f32 {
    const HOW_MANY_FIGHTS: u64 = 40;
    const BEFORE_GIVING_UP: u32 = 60;

    let mut blows = 0u32;
    for seed in 0..HOW_MANY_FIGHTS {
        // Seeded before the fixture as well as after it. Building the man
        // draws a random weight per drive, so the *number of drives* decides
        // what man this is; seeding only the fight left the fighter himself
        // riding on whatever the stream happened to hold.
        crate::core::dice::seed(8_000 + seed);

        let (mut simulation, which) = a_man_and_a_wolf();
        if with_a_spear {
            give_him_a_spear(&mut simulation);
        }

        // Seeded *after* the world is built, not before it. What is being
        // measured is the fight, and building a world draws an unknown number
        // of times before the first blow is struck - so a seed set beforehand
        // pins the world and leaves the fight wherever the world happened to
        // leave the stream. Any change at all to world generation then moves
        // the fights, and this read 1.4 blows bare-handed against 1.3 with a
        // spear on one such change: near enough every fight over in one blow,
        // with no room left for a spear to tell. See ISSUES_FOUND.md #132.
        crate::core::dice::seed(9_000 + seed);

        for swung in 1..=BEFORE_GIVING_UP {
            let mut rng = crate::core::dice::roll();
            let _ = simulation.fighting_a_beast(&which, &None, 0, &mut rng, 10);

            // The man has to survive to swing again, and the wolf has to still
            // be there to be swung at.
            let done = simulation
                .world
                .animals
                .get(&which)
                .map(|beast| !beast.is_alive())
                .unwrap_or(true);
            if done || !simulation.population.agents[0].state.is_alive {
                blows += swung;
                break;
            }
            if swung == BEFORE_GIVING_UP {
                blows += swung;
            }
        }
    }
    blows as f32 / HOW_MANY_FIGHTS as f32
}

/// The whole of it: a spear puts the wolf down in fewer blows than bare hands.
#[test]
fn a_spear_tells_when_you_stand_your_ground() {
    let bare = blows_to_put_it_down(false);
    let armed = blows_to_put_it_down(true);

    assert!(
        armed < bare * 0.85,
        "a spear should cut the number of blows appreciably: bare hands took \
         {bare:.1} blows on average, a spear {armed:.1}"
    );
}

/// And the action carries the truth about what is in the hand.
///
/// The field used to be filled from `Agent::equipment`, so it was `None` in
/// every fight this model has ever run, whatever the man was carrying.
#[test]
fn the_action_says_what_is_in_the_hand() {
    let (mut simulation, _) = a_man_and_a_wolf();

    assert_eq!(
        Simulation::what_is_in_hand_for_this(&simulation.population.agents[0]),
        None,
        "empty-handed is empty-handed"
    );

    give_him_a_spear(&mut simulation);
    assert_eq!(
        Simulation::what_is_in_hand_for_this(&simulation.population.agents[0]),
        Some("spear".to_string()),
        "a spear in the pack should reach the action that uses it"
    );
}

/// Bare hands are exactly what they were: standing your ground unarmed is a
/// poor idea, not an impossible one. Refusing to hunt an ox empty-handed sends
/// somebody home hungry; refusing to fight a wolf already on them sends them
/// home dead, so there is no size gate here of the kind `hunting` has.
#[test]
fn bare_hands_still_fight() {
    let bare = blows_to_put_it_down(false);
    assert!(
        bare < 60.0,
        "an unarmed man should still be able to put a wolf down eventually, \
         and took {bare:.1} blows"
    );
}

/// The Hunting tool is what a fight reads, which is the same thing hunting
/// reads. One vocabulary, not two.
#[test]
fn a_fight_and_a_hunt_read_the_same_hand() {
    let (mut simulation, _) = a_man_and_a_wolf();
    let bare = simulation.population.agents[0].how_much_my_tools_help(SkillType::Hunting);

    give_him_a_spear(&mut simulation);
    let armed = simulation.population.agents[0].how_much_my_tools_help(SkillType::Hunting);

    assert!(
        armed > bare,
        "a spear should be worth more than a bare hand to the Hunting trade: \
         {bare:.2} to {armed:.2}"
    );
}


