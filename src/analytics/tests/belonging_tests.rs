// src/analytics/tests/belonging_tests.rs
//! **What do I have access to?** - the fourth of the six questions.
//!
//! What is asserted here is that a claim exists, that it depends on who is
//! asking, that it orders what an agent reaches for - and, twice over and at
//! some length, that it never once refuses anybody anything. A claim in this
//! model is an *order*, not a gate; the two tests that say so are the ones
//! worth keeping if the rest were ever thrown away.

use crate::agents::{AgentConfig, Population, Relationship, RelationshipType};
use crate::analytics::wanting::shelter::WhoseRoof;
use crate::analytics::wanting::strategy::{Reach, Strategy};
use crate::analytics::Simulation;
use crate::world::belonging::{Access, Belongs};
use crate::world::{Building, BuildingType, Pit, Position, World, WorldConfig};

fn two_people() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[1].state.position = (25, 25, 0);
    simulation
}

/// A claim has three shapes and no fourth, and the reading of one has four.
///
/// The guard `OWNERSHIP.md` asks for: an exhaustive match per variant, so
/// adding a case without saying what it means fails to compile rather than
/// falling through to whatever the last arm was.
#[test]
fn every_kind_of_claim_is_accounted_for() {
    let somebody = crate::core::dice::name();

    for claim in Belongs::all(somebody) {
        let named = match claim {
            Belongs::ToNobody => false,
            Belongs::To(_) => true,
            Belongs::ToUsAll => false,
        };
        assert_eq!(named, claim.is_somebodys());
        assert_eq!(claim.whose().is_some(), named);
    }

    for reading in Access::all(somebody) {
        let free = match reading {
            Access::Freely => true,
            Access::InCommon => true,
            Access::ByKinship(_) => true,
            Access::NotMine(_) => false,
        };
        assert_eq!(free, reading.is_mine_to_use());
    }
}

/// The same hole reads differently depending who is looking at it.
///
/// This is the whole reason `Access` is not a field on the thing. A claim is a
/// fact about a pit; what it permits is a fact about a pit *and a person*.
#[test]
fn whose_it_is_depends_on_who_is_asking() {
    let simulation = two_people();
    let mine = simulation.population.agents[0].id;
    let his = simulation.population.agents[1].id;

    let me = &simulation.population.agents[0];

    assert_eq!(me.may_i_use(&Belongs::To(mine)), Access::Freely);
    assert_eq!(me.may_i_use(&Belongs::ToNobody), Access::Freely);
    assert_eq!(me.may_i_use(&Belongs::ToUsAll), Access::InCommon);
    assert_eq!(me.may_i_use(&Belongs::To(his)), Access::NotMine(his));
}

/// A brother's store is not a stranger's store.
///
/// There is no household object in this model and there does not need to be
/// one: a household is who you are kin to, and the `RelationshipMap` has held
/// that since the relationship graph was built without anything ever asking it
/// a question.
#[test]
fn a_brothers_hole_is_as_good_as_ones_own() {
    let mut simulation = two_people();
    let his = simulation.population.agents[1].id;

    simulation.population.agents[0]
        .relationships
        .add_relationship(Relationship::new(his, RelationshipType::Sibling));

    let me = &simulation.population.agents[0];
    assert_eq!(me.may_i_use(&Belongs::To(his)), Access::ByKinship(his));
    assert!(me.may_i_use(&Belongs::To(his)).is_mine_to_use());
}

/// And a friend is not kin, which is the line this draws on purpose.
///
/// The model has a `Friend` bond and it is the wrong one for this. Friendship
/// is who you would help; kinship is whose store you would open without
/// asking. Widening it to friends would make the whole distinction a
/// formality in a settlement of twelve, where everybody knows everybody.
#[test]
fn a_friend_is_not_kin() {
    let mut simulation = two_people();
    let his = simulation.population.agents[1].id;

    simulation.population.agents[0]
        .relationships
        .add_relationship(Relationship::new(his, RelationshipType::Friend));

    let me = &simulation.population.agents[0];
    assert_eq!(me.may_i_use(&Belongs::To(his)), Access::NotMine(his));
}

/// **A claim orders what a man walks to. It never stops him walking.**
///
/// One of the two tests worth keeping if all the rest went. A stranger's pit
/// is still the answer when it is the only one he remembers: a rule that let a
/// man starve beside a full larder over whose hole it was would cost more than
/// it bought, and the weather and hunger between them take four deaths in five
/// in this model.
#[test]
fn a_strangers_hole_is_still_an_answer_when_it_is_the_only_one() {
    use crate::core::memory::SpatialMemoryType;

    let mut simulation = two_people();
    let his = simulation.population.agents[1].id;

    simulation.world.pits.push(Pit {
        where_it_is: Position::new(30, 25),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: Belongs::To(his),
    });

    simulation.population.agents[0]
        .memory
        .remember_location(SpatialMemoryType::Storage, (30, 25, 0));

    let agent = &simulation.population.agents[0];
    assert!(
        simulation
            .nearest_pit_i_remember(agent, agent.state.position)
            .is_some(),
        "the only pit he knows of is somebody else's, and he was refused it"
    );
}

/// And his own comes first when there are two.
///
/// The order, which is the part that is load-bearing: a pit belongs to
/// whoever dug it, and a man goes to his own before he goes to one somebody
/// else sank - even when the other is nearer.
#[test]
fn a_hole_of_ones_own_is_walked_to_before_a_strangers() {
    use crate::core::memory::SpatialMemoryType;

    let mut simulation = two_people();
    let mine = simulation.population.agents[0].id;
    let his = simulation.population.agents[1].id;

    // His is three paces off; mine is eight.
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(28, 25),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: Belongs::To(his),
    });
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(33, 25),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: Belongs::To(mine),
    });

    for at in [(28, 25, 0), (33, 25, 0)] {
        simulation.population.agents[0]
            .memory
            .remember_location(SpatialMemoryType::Storage, at);
    }

    let agent = &simulation.population.agents[0];
    let (where_it_is, _) = simulation
        .nearest_pit_i_remember(agent, agent.state.position)
        .expect("he remembers two holes and found neither");

    assert_eq!(
        where_it_is,
        Position::new(33, 25),
        "he walked past his own hole to open somebody else's, which is the \
         one thing this ordering is for"
    );
}

/// The three ways of getting under somebody's roof are three ways now.
///
/// Each of `UseOwnedShelter`, `UseHouseholdShelter` and `ShareCommunalShelter`
/// was declared `NotYet("shelter has no owner, so somebody else's is not a
/// different thing from one's own")`. That sentence is what this layer makes
/// untrue, so it is what this test watches.
#[test]
fn a_roof_now_has_somebody_to_belong_to() {
    for way in [
        Strategy::UseOwnedShelter,
        Strategy::UseHouseholdShelter,
        Strategy::ShareCommunalShelter,
        Strategy::RelocateToNaturalShelter,
    ] {
        assert_eq!(
            way.reach(),
            Reach::Now,
            "{way:?} is still out of reach, and shelter has an owner now"
        );
    }
}

/// **No claim takes cover away from anybody.**
///
/// The second of the two that matter. The four kinds of roof between them are
/// exactly what the single shelter test accepted before the split - a
/// completed building or a wood - so wherever `is_shelter_tile` said yes, at
/// least one of the four kinds must still say yes. If that ever stops being
/// true, somebody is standing outside in the weather over whose hut it is, and
/// the weather is a fifth of every death in this model.
#[test]
fn every_tile_that_was_cover_is_still_cover_to_somebody() {
    let mut simulation = two_people();
    let mine = simulation.population.agents[0].id;
    let his = simulation.population.agents[1].id;

    // One of each claim, so no kind of roof goes unexamined.
    for (at, belongs) in [
        (Position::new(20, 20), Belongs::To(mine)),
        (Position::new(21, 20), Belongs::To(his)),
        (Position::new(22, 20), Belongs::ToUsAll),
        (Position::new(23, 20), Belongs::ToNobody),
    ] {
        let mut roof = Building::new(BuildingType::SmallHouse, at);
        roof.now_belongs_to(belongs);
        simulation.world.add_building(roof);
    }

    // And the stranger's is the awkward one, so he is made kin for half of it.
    simulation.population.agents[0]
        .relationships
        .add_relationship(Relationship::new(his, RelationshipType::Sibling));

    let agent = &simulation.population.agents[0];

    for x in 18..26 {
        for y in 18..26 {
            let here = Position::new(x, y);
            if !simulation.is_shelter_tile(&here) {
                continue;
            }

            let cover_to_somebody = [
                WhoseRoof::HisOwn,
                WhoseRoof::AKinsmans,
                WhoseRoof::TheSettlements,
                WhoseRoof::Natural,
            ]
            .into_iter()
            .any(|which| simulation.is_this_roof_mine_to_use(agent, &here, which));

            assert!(
                cover_to_somebody,
                "({x}, {y}) was cover before the claim and is cover to nobody now"
            );
        }
    }
}

/// And a stranger's roof is a stranger's, which is the one case the split
/// leaves out on purpose.
///
/// A hut a man is not kin to the builder of is not `HisOwn`, not
/// `AKinsmans` and not `TheSettlements`. It is still cover - `is_shelter_tile`
/// says so and `SeekShelter` walks to it - but no *way* names it, so nothing
/// in the record ever claims he had a right to be there. That is the honest
/// shape of it until there is a settlement to ask.
#[test]
fn a_strangers_roof_is_nobodys_way_in() {
    let mut simulation = two_people();
    let his = simulation.population.agents[1].id;

    let at = Position::new(20, 20);
    let mut roof = Building::new(BuildingType::SmallHouse, at);
    roof.now_belongs_to(Belongs::To(his));
    simulation.world.add_building(roof);

    let agent = &simulation.population.agents[0];

    for which in [
        WhoseRoof::HisOwn,
        WhoseRoof::AKinsmans,
        WhoseRoof::TheSettlements,
    ] {
        assert!(
            !simulation.is_this_roof_mine_to_use(agent, &at, which),
            "{which:?} let him into a hut belonging to somebody he is no kin to"
        );
    }
}
