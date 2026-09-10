// src/environment/remedies.rs
//! What a herbal is good for, and how little that is.
//!
//! The specification asks for ten medicinal plants and is unusually careful
//! about them: aloe is "topical gel for minor skin irritation; **not a
//! replacement for burn or wound care**"; echinacea is "widely used in herbal
//! products, **clinical benefits remain uncertain**"; garlic has "historical
//! medicinal use; **avoid treating it as an antibiotic substitute**";
//! turmeric's "bioavailability and clinical effects vary". Only ginger gets a
//! plain claim, and it is for nausea.
//!
//! That care is the model. A remedy here **eases an illness and does not cure
//! one**: it takes something off how badly a person is laid up, it never
//! shortens the illness by a day, and the best of them at a practised hand
//! takes off about a third. A settlement can have the whole pharmacopoeia and
//! still bury people, which is what actually happened.
//!
//! Before this, `ResourceType::Herbs` spawned, gathered, turned into
//! `ItemType::Herbs` and taught Herbalism, and then **nothing**. There was no
//! treatment of any kind in the model - see ISSUES_FOUND.md #202 - and the
//! chamomile, mint, sage, aloe, lavender and ginseng in the flora table were
//! scenery.

use serde::{Deserialize, Serialize};

/// What sort of trouble a remedy is any use against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WhatARemedyEases {
    /// A bad gut. **Every illness in this model is one of these** - raw
    /// flesh, food on the turn, foul ground - which is not a simplification
    /// so much as a fact about what laid people up before anybody boiled
    /// water.
    TheGut,
    /// Skin, and small burns, and no more than that. What a wound gone bad
    /// wants, and the one place a topical is the right answer.
    TheSkin,
    /// The chest: a cough, a soaking that turned into something. A steam and
    /// a gargle, which is what people had.
    TheChest,
    /// Sleep, worry, and being uncomfortable. It does not touch what is
    /// wrong; it makes the week easier to sit through.
    TheNerves,
    /// Nothing anybody has been able to show.
    ///
    /// Kept as a case of its own rather than left out of the table, because
    /// a people who believe in a thing that does nothing is a real and
    /// common state of affairs, and the model should be able to be in it.
    NothingAnybodyCanShow,
}

/// One remedy: what it is for, and how much good it does.
#[derive(Debug, Clone, Copy)]
pub struct ARemedy {
    /// The item as it sits in a pack - what a plant drops, or what comes off
    /// a patch of herbs.
    pub id: &'static str,
    /// What it is any use against.
    pub eases: WhatARemedyEases,
    /// How much it takes off an illness, in a practised hand and against the
    /// thing it is for. Nought to one, and none of them is near one.
    pub takes_off: f32,
    /// What the specification says about it, kept so that a number in this
    /// table can be argued with.
    pub because: &'static str,
}

/// The most any herbal can ever take off an illness, however much of it a
/// person swallows and however good a herbalist they are.
///
/// A third. This is the line between easing and curing, and it is the whole
/// point of the exercise: a remedy buys a person some of their week back and
/// does not save their life. Without a cap, a settlement with a herb patch
/// would simply stop being ill.
pub const THE_MOST_A_HERBAL_CAN_DO: f32 = 0.35;

/// What the wrong remedy is still worth.
///
/// Something rather than nothing: a person given aloe for a bad gut has been
/// looked after, warmed, and given something to do, and that is worth a
/// little even when the aloe is not. It is small enough that a herbalist who
/// knows which is which is plainly better off.
pub const WHAT_THE_WRONG_REMEDY_IS_STILL_WORTH: f32 = 0.25;

/// Everything this model knows how to treat anybody with.
///
/// Keyed on the id the thing actually carries in a pack - see the `drops` on
/// each plant in `flora.rs` - so a remedy that no plant yields cannot get
/// into the table by accident.
pub const EVERY_REMEDY: &[ARemedy] = &[
    // --- the gut, which is what people are actually ill with ---------------
    ARemedy {
        id: "mint_leaves",
        eases: WhatARemedyEases::TheGut,
        takes_off: 0.20,
        because: "peppermint and spearmint: flavouring and digestive comfort",
    },
    ARemedy {
        id: "chamomile_flowers",
        eases: WhatARemedyEases::TheGut,
        takes_off: 0.15,
        because: "chamomile: tea and topical preparations",
    },

    // The generic bundle that comes off a patch of herbs. It is a mixed
    // handful and it is treated as one: something for the gut, and not much.
    ARemedy {
        id: "herbs",
        eases: WhatARemedyEases::TheGut,
        takes_off: 0.12,
        because: "a mixed handful off the hedgerow, which is what most of it was",
    },
    ARemedy {
        id: "medicinal_herbs",
        eases: WhatARemedyEases::TheGut,
        takes_off: 0.18,
        because: "gathered as medicine rather than stumbled on",
    },

    // --- the skin, which is no use against a bad gut ------------------------
    ARemedy {
        id: "aloe_gel",
        eases: WhatARemedyEases::TheSkin,
        takes_off: 0.15,
        because: "topical gel for minor skin irritation; not a replacement for \
                  burn or wound care",
    },
    ARemedy {
        id: "rose_hips",
        eases: WhatARemedyEases::TheSkin,
        takes_off: 0.08,
        because: "a hedgerow standby, and mostly a food",
    },

    // --- the chest ----------------------------------------------------------
    ARemedy {
        id: "sage_leaves",
        eases: WhatARemedyEases::TheChest,
        takes_off: 0.12,
        because: "a gargle, which is what a sore throat had before it had \
                  anything else",
    },

    // --- and the nerves, which is comfort and is not medicine ---------------
    //
    // Nothing in this model is ill in the nerves, so a nerve remedy is always
    // the wrong remedy and always worth the quarter that being looked after
    // is worth. That is the honest place for lavender and it is where the
    // specification puts it: "fragrance and topical/aromatic products".
    ARemedy {
        id: "lavender",
        eases: WhatARemedyEases::TheNerves,
        takes_off: 0.10,
        because: "fragrance and topical or aromatic products",
    },

    // --- and what does nothing ----------------------------------------------
    ARemedy {
        id: "ginseng_root",
        eases: WhatARemedyEases::NothingAnybodyCanShow,
        takes_off: 0.05,
        because: "prized, rare, expensive, and never shown to do this",
    },
    ARemedy {
        id: "mandrake_root",
        eases: WhatARemedyEases::NothingAnybodyCanShow,
        takes_off: 0.05,
        because: "powerful in every account and in no measurement",
    },
];

/// What this thing in a pack is good for, if anything.
pub fn what_this_is_good_for(item_id: &str) -> Option<&'static ARemedy> {
    let id = item_id.trim().to_lowercase();
    EVERY_REMEDY.iter().find(|remedy| remedy.id == id)
}

/// Whether this is worth carrying home for somebody who is ill.
pub fn is_a_remedy(item_id: &str) -> bool {
    what_this_is_good_for(item_id).is_some()
}
