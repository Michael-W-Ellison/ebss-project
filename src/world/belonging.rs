// src/world/belonging.rs
//! Who a thing belongs to, and what that lets somebody else do with it.
//!
//! This is the fourth of the six questions an agent is supposed to be
//! answering - *what do I have access to?* - and until now it was the one with
//! no answer anywhere in the model. There were two owner fields. `Territory`
//! has one and nothing outside its own file has ever read it. `Building` has
//! one, `owns_house` in the goal world-state reads it, and **nothing has ever
//! written it**, so every agent in every world has always been told it owns no
//! house. Neither was a claim anybody could act on.
//!
//! What is here is deliberately small. Two enums and one question:
//!
//! - `Belongs` is a fact about a thing, and it lives on the thing. A pit
//!   belongs to whoever dug it; a hut to whoever put it up; a berry bush to
//!   nobody.
//! - `Access` is what a *particular person* may do with it, which is not a fact
//!   about the thing at all - it depends who is asking. `Agent::may_i_use`
//!   answers it, because the asker is the only one who knows who his kin are.
//!
//! **Nothing here refuses anybody shelter.** A claim decides whose roof it is,
//! not whether a freezing man may stand under one; weather is a fifth of all
//! the deaths in this model and a rule that let somebody die outside a hut
//! would be a worse fault than the one it fixes. What the claim buys is that
//! `UseOwnedShelter`, `UseHouseholdShelter` and `ShareCommunalShelter` are
//! three different ways rather than three names for one - each was declared
//! `NotYet("shelter has no owner, so somebody else's is not a different thing
//! from one's own")`, and that is now untrue.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Who a thing belongs to.
///
/// Three cases, and the model has no fourth: it either has no claim on it, or
/// one person's, or the settlement's. There is no settlement *object* to hold
/// the last of those - see ISSUES_FOUND #11 - so `ToUsAll` is what a thing put
/// up in common is marked, and it means "anybody here may use it" rather than
/// naming a body that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Belongs {
    /// Nobody's. It grew there, or it fell there, or it was always there.
    ///
    /// The default, and the honest one: most of what a stone-age settlement
    /// uses is not owned by anybody, and pretending otherwise would put a
    /// claim on every berry bush.
    ToNobody,

    /// One person's, because they dug it, put it up, or made it.
    To(Uuid),

    /// Everybody here, because it was put up in common or given over to it.
    ToUsAll,
}

impl Default for Belongs {
    fn default() -> Self {
        Belongs::ToNobody
    }
}

impl Belongs {
    /// Every case, so a match over them can be held exhaustive by a test.
    ///
    /// `To` needs a body to name, so it is given one; the guard cares that
    /// there are three shapes, not whose the third is.
    pub fn all(somebody: Uuid) -> Vec<Belongs> {
        vec![Belongs::ToNobody, Belongs::To(somebody), Belongs::ToUsAll]
    }

    /// Whose it is, where it is anybody's in particular.
    pub fn whose(&self) -> Option<Uuid> {
        match self {
            Belongs::To(who) => Some(*who),
            Belongs::ToNobody | Belongs::ToUsAll => None,
        }
    }

    /// Whether this is a claim by one person, which is what makes it possible
    /// for the answer to depend on who is asking.
    pub fn is_somebodys(&self) -> bool {
        matches!(self, Belongs::To(_))
    }
}

/// What one particular person may do with one particular thing.
///
/// Ordered from the freest to the least free, so "at least this free" is a
/// comparison rather than a match - which is what the decision layer wants
/// nearly everywhere it asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Access {
    /// Mine, or nobody's. Use it and think no more about it.
    Freely,

    /// The settlement's. Use it, and so may everybody else, which is what
    /// makes running it down a thing that can happen.
    InCommon,

    /// A kinsman's. Yours to use; you would mention it afterwards.
    ///
    /// This is the case the whole layer is for. A household is not a legal
    /// arrangement in this model - there is no household object - it is the
    /// parents, children, siblings and partner an agent already has in its
    /// `RelationshipMap`, which is a thing the model has kept all along and
    /// never once asked a question of.
    ByKinship(Uuid),

    /// Somebody else's, and they are not kin. Using it is taking it.
    ///
    /// Nothing in the decision layer refuses on this today. It is what
    /// `StealFood` is for, and what makes it a different question from eating;
    /// see ISSUES_FOUND #226 for why that way is still out of reach.
    NotMine(Uuid),
}

impl Access {
    /// Every case, so a match over them can be held exhaustive by a test.
    pub fn all(somebody: Uuid) -> Vec<Access> {
        vec![
            Access::Freely,
            Access::InCommon,
            Access::ByKinship(somebody),
            Access::NotMine(somebody),
        ]
    }

    /// Whether using it is taking it from somebody.
    ///
    /// The one predicate the decision layer needs most of the time: everything
    /// short of `NotMine` is a thing you may put your hand on.
    pub fn is_mine_to_use(&self) -> bool {
        !matches!(self, Access::NotMine(_))
    }

    /// Whose claim this is, where somebody has one.
    pub fn whose(&self) -> Option<Uuid> {
        match self {
            Access::ByKinship(who) | Access::NotMine(who) => Some(*who),
            Access::Freely | Access::InCommon => None,
        }
    }
}
