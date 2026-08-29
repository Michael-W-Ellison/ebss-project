// src/core/traits.rs
//! Personality trait system for agents.
//!
//! Traits modify how agents experience emotions and interact with the world.
//! Each trait provides specific modifiers to emotional responses and behaviors.

use serde::{Deserialize, Serialize};

/// All personality traits that can modify agent behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Trait {
    // Emotional Response Modifiers
    Anxious,        // Doubles fear increase, halves fear reduction
    Brave,          // Halves fear gain, doubles fear loss
    HotHeaded,      // Doubles anger gain, halves anger loss
    Calm,           // Halves anger gain, doubles anger loss
    Pacifist,       // Prevents anger gain
    Empathic,       // Doubles sadness loss and gain
    ColdHearted,    // Halves sadness gain, doubles loss
    Resilient,      // Doubles sadness loss
    Clown,          // Doubles happiness gain, halves happiness loss
    Goth,           // Doubles happiness loss, halves happiness gain
    Melancholic,    // Passive slow gain of sadness
    Stoic,          // Decreases emotional reactions by 50%
    Repressed,      // Decreases average emotions, increases extreme

    // Social Traits
    Extrovert,      // Doubles happiness from socializing, double agent count
    Introvert,      // Happiness when alone, decreases with agents nearby
    KindHearted,    // Doubles happiness loss when seeing agent hurt
    Cruel,          // Reverses happiness loss when seeing pain
    Charismatic,    // Increases socialization benefit, doubles nearby emotional affect
    Mute,           // Cannot use word of mouth
    Gossip,         // Halves happiness except from word of mouth
    Intolerant,     // -3 affection modifier on disagreeing traits
    Mediator,       // Decreases nearby negative emotions
    Romantic,       // Additional happiness from relationships
    Insecure,       // Decreased happiness if partner talks to opposite sex
    Manipulator,    // Happiness from convincing agents to act contrary
    Copycat,        // Happiness from copying nearby agent tasks
    Imaginative,    // Increases nearby agent happiness when sharing

    // Work and Achievement Traits
    Handy,          // Doubles happiness from completing jobs
    Lazy,           // Constant happiness decrease when working
    Proud,          // Happiness from accomplishing goals
    Ambitious,      // Increased happiness from completing external goals
    Pragmatist,     // Happiness from survival-increasing goals
    Stubborn,       // Happiness from consistency, sadness from change
    Traditionalist, // Happiness/speed boost from primitive tools
    Rebel,          // Happiness from unpopular jobs/goals
    Builder,        // Happiness from building structures
    CraftObsessed,  // Happiness from crafted items
    Archivist,      // Happiness from keeping records

    // Resource and Material Traits
    Greedy,         // Happiness bonus from supplies in home
    Ascetic,        // No favorite food, decreases happiness with luxury
    Envious,        // Decreases happiness if others have better items
    Frugal,         // Happiness from stored goods, sadness if used by non-family
    Survivalist,    // Happiness when basic needs met from own stores
    Altruist,       // Happiness from helping others

    // Food and Consumption Traits
    Glutton,        // Increases happiness from favorite food

    // Belief and Knowledge Traits
    Believer,       // Boosts happiness from religious buildings
    Atheist,        // Small decrease at religious buildings, twice happiness at museums
    Ignorant,       // Rare, only occurs with Believer trait
    Zealot,         // Additional happiness in religious buildings
    Skeptic,        // Prevents religious traits, only trusts multiple sources
    Bookworm,       // Doubles curiosity decrease at library, happiness gain
    Curious,        // Happiness from refreshing knowledge and learning

    // Investigation and Awareness Traits
    Suspicious,     // Noise curiosity increases at twice rate
    Deaf,           // Immune to noise events
    Blind,          // Cannot see: no sight-based discovery of the world
    Uncaring,       // Reduces noise curiosity by half
    Paranoid,       // Doesn't trust word, assumes malice

    // Conflict and Response Traits
    Vengeful,       // Prevents anger loss unless action taken
    Forgiving,      // Anger decreases at twice speed
    Coward,         // Increased fear in danger, happiness escaping
    Protector,      // Happiness from killing dangerous creatures
    Aggressive,     // Aggressive in conflicts, quick to fight
    Peaceful,       // Avoids confrontation at all costs

    // Interaction with Environment Traits
    AnimalLover,    // Happiness near animals
    Allergic,       // Decreases happiness near animals
    Masochist,      // Happiness from damage until 50% health
    Explorer,       // Happiness from exploring new areas
    Caretaker,      // Happiness from helping sick/injured/elderly

    // Trust and Honesty Traits
    Trusting,       // Trusts others easily, believes information
    Honest,         // Always tells the truth, others trust them
    Dishonest,      // Frequently lies, manipulates information
    Callous,        // Doesn't care about others' feelings

    // Work Ethic (additional)
    Diligent,       // Hard worker, gains satisfaction from labor

    // Compatibility aliases for agents system
    Hottempered,    // Alias for HotHeaded
    Sociable,       // Alias for Extrovert
    Introverted,    // Alias for Introvert
    Empathetic,     // Alias for Empathic
    Manipulative,   // Alias for Manipulator

    // Obsession Trait
    Obsessive,      // Gains/loses happiness based on proximity to obsession

    // Reproductive Traits
    Infertile,      // Cannot reproduce (rare, ~1-2% at birth or from severe malnutrition)

    // Sleep Traits
    Narcoleptic,    // Sleep is less restful (reduced recovery rate)
    SoundSleeper,   // Needs 2 hours less sleep than normal
}

impl Trait {
    /// Get description of what this trait does
    pub fn description(&self) -> &'static str {
        match self {
            Trait::Anxious => "Doubles fear increase, halves fear reduction",
            Trait::Brave => "Halves fear gain, doubles fear loss",
            Trait::HotHeaded => "Doubles anger gain, halves anger loss",
            Trait::Calm => "Halves anger gain, doubles anger loss",
            Trait::Pacifist => "Prevents anger gain",
            Trait::Empathic => "Doubles sadness loss and gain",
            Trait::ColdHearted => "Halves sadness gain, doubles sadness loss",
            Trait::Resilient => "Doubles sadness loss",
            Trait::Clown => "Doubles happiness gain, halves happiness loss",
            Trait::Goth => "Doubles happiness loss, halves happiness gain",
            Trait::Melancholic => "Passive slow gain of sadness",
            Trait::Stoic => "Decreases all emotional reactions by 50%",
            Trait::Repressed => "Decreases average emotions, increases extreme emotions",
            Trait::Extrovert => "Doubles happiness from socializing",
            Trait::Introvert => "Gains happiness when alone, loses it in crowds",
            Trait::KindHearted => "Doubles happiness loss when seeing others hurt",
            Trait::Cruel => "Gains happiness from seeing others in pain",
            Trait::Charismatic => "Increases socialization benefits and emotional influence",
            Trait::Mute => "Cannot use word of mouth",
            Trait::Gossip => "Halves happiness except from word of mouth",
            Trait::Intolerant => "Decreased affection toward different traits",
            Trait::Mediator => "Decreases nearby negative emotions",
            Trait::Romantic => "Additional happiness from relationships",
            Trait::Insecure => "Decreased happiness if partner talks to others",
            Trait::Manipulator => "Happiness from convincing others to act contrary",
            Trait::Copycat => "Happiness from copying nearby agent tasks",
            Trait::Imaginative => "Increases nearby agent happiness when sharing",
            Trait::Handy => "Doubles happiness from completing jobs",
            Trait::Lazy => "Constant happiness decrease when working",
            Trait::Proud => "Happiness from accomplishing goals",
            Trait::Ambitious => "Increased happiness from completing external goals",
            Trait::Pragmatist => "Happiness from survival-increasing goals",
            Trait::Stubborn => "Happiness from consistency, sadness from change",
            Trait::Traditionalist => "Happiness/speed boost from primitive tools",
            Trait::Rebel => "Happiness from unpopular jobs/goals",
            Trait::Builder => "Happiness from building structures",
            Trait::CraftObsessed => "Happiness from crafted items",
            Trait::Archivist => "Happiness from keeping records",
            Trait::Greedy => "Happiness from supplies stored in home",
            Trait::Ascetic => "No favorite food, dislikes luxury",
            Trait::Envious => "Decreased happiness if others have better items",
            Trait::Frugal => "Happiness from stored goods, sadness if used by non-family",
            Trait::Survivalist => "Happiness when basic needs met from own stores",
            Trait::Altruist => "Happiness from helping others",
            Trait::Glutton => "Increased happiness from favorite food",
            Trait::Believer => "Boosts happiness from religious buildings",
            Trait::Atheist => "Small decrease at religious buildings, happiness at museums",
            Trait::Ignorant => "Limited knowledge acceptance",
            Trait::Zealot => "Additional happiness in religious buildings near believers",
            Trait::Skeptic => "Only trusts multiple sources, no religious traits",
            Trait::Bookworm => "Doubles curiosity decrease at library, happiness gain",
            Trait::Curious => "Happiness from learning and discovering",
            Trait::Suspicious => "Noise curiosity increases at twice rate",
            Trait::Deaf => "Immune to noise events",
            Trait::Blind => "Cannot see; finds the world by smell and memory alone",
            Trait::Uncaring => "Reduces noise curiosity by half",
            Trait::Paranoid => "Doesn't trust others, assumes malice",
            Trait::Vengeful => "Prevents anger loss unless action taken against target",
            Trait::Forgiving => "Anger decreases at twice normal speed",
            Trait::Coward => "Increased fear in danger, happiness when escaping",
            Trait::Protector => "Happiness from protecting others and fighting dangers",
            Trait::AnimalLover => "Happiness near animals",
            Trait::Allergic => "Decreases happiness near animals",
            Trait::Masochist => "Happiness from damage until 50% health",
            Trait::Explorer => "Happiness from exploring new areas",
            Trait::Caretaker => "Happiness from helping sick/injured/elderly",
            Trait::Aggressive => "Aggressive in conflicts, quick to fight",
            Trait::Peaceful => "Avoids confrontation at all costs",
            Trait::Trusting => "Trusts others easily, believes information readily",
            Trait::Honest => "Always tells the truth, others trust them",
            Trait::Dishonest => "Frequently lies and manipulates information",
            Trait::Callous => "Doesn't care about others' feelings",
            Trait::Diligent => "Hard worker, gains satisfaction from labor",
            Trait::Hottempered => "Quick to anger (alias for HotHeaded)",
            Trait::Sociable => "Enjoys social interaction (alias for Extrovert)",
            Trait::Introverted => "Prefers solitude (alias for Introvert)",
            Trait::Empathetic => "Feels others' emotions strongly (alias for Empathic)",
            Trait::Manipulative => "Lies and manipulates for personal gain (alias for Manipulator)",
            Trait::Obsessive => "Strong focus on specific obsession target",
            Trait::Infertile => "Cannot reproduce due to biological condition",
            Trait::Narcoleptic => "Sleep is less restful, recovers fatigue slower",
            Trait::SoundSleeper => "Needs 2 hours less sleep than normal",
        }
    }

    /// Get trait name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Trait::Anxious => "Anxious",
            Trait::Brave => "Brave",
            Trait::HotHeaded => "Hot-Headed",
            Trait::Calm => "Calm",
            Trait::Pacifist => "Pacifist",
            Trait::Empathic => "Empathic",
            Trait::ColdHearted => "Cold-Hearted",
            Trait::Resilient => "Resilient",
            Trait::Clown => "Clown",
            Trait::Goth => "Goth",
            Trait::Melancholic => "Melancholic",
            Trait::Stoic => "Stoic",
            Trait::Repressed => "Repressed",
            Trait::Extrovert => "Extrovert",
            Trait::Introvert => "Introvert",
            Trait::KindHearted => "Kind-Hearted",
            Trait::Cruel => "Cruel",
            Trait::Charismatic => "Charismatic",
            Trait::Mute => "Mute",
            Trait::Gossip => "Gossip",
            Trait::Intolerant => "Intolerant",
            Trait::Mediator => "Mediator",
            Trait::Romantic => "Romantic",
            Trait::Insecure => "Insecure",
            Trait::Manipulator => "Manipulator",
            Trait::Copycat => "Copycat",
            Trait::Imaginative => "Imaginative",
            Trait::Handy => "Handy",
            Trait::Lazy => "Lazy",
            Trait::Proud => "Proud",
            Trait::Ambitious => "Ambitious",
            Trait::Pragmatist => "Pragmatist",
            Trait::Stubborn => "Stubborn",
            Trait::Traditionalist => "Traditionalist",
            Trait::Rebel => "Rebel",
            Trait::Builder => "Builder",
            Trait::CraftObsessed => "Craft-Obsessed",
            Trait::Archivist => "Archivist",
            Trait::Greedy => "Greedy",
            Trait::Ascetic => "Ascetic",
            Trait::Envious => "Envious",
            Trait::Frugal => "Frugal",
            Trait::Survivalist => "Survivalist",
            Trait::Altruist => "Altruist",
            Trait::Glutton => "Glutton",
            Trait::Believer => "Believer",
            Trait::Atheist => "Atheist",
            Trait::Ignorant => "Ignorant",
            Trait::Zealot => "Zealot",
            Trait::Skeptic => "Skeptic",
            Trait::Bookworm => "Bookworm",
            Trait::Curious => "Curious",
            Trait::Suspicious => "Suspicious",
            Trait::Deaf => "Deaf",
            Trait::Blind => "Blind",
            Trait::Uncaring => "Uncaring",
            Trait::Paranoid => "Paranoid",
            Trait::Vengeful => "Vengeful",
            Trait::Forgiving => "Forgiving",
            Trait::Coward => "Coward",
            Trait::Protector => "Protector",
            Trait::Aggressive => "Aggressive",
            Trait::Peaceful => "Peaceful",
            Trait::AnimalLover => "Animal Lover",
            Trait::Allergic => "Allergic",
            Trait::Masochist => "Masochist",
            Trait::Explorer => "Explorer",
            Trait::Caretaker => "Caretaker",
            Trait::Trusting => "Trusting",
            Trait::Honest => "Honest",
            Trait::Dishonest => "Dishonest",
            Trait::Callous => "Callous",
            Trait::Diligent => "Diligent",
            Trait::Hottempered => "Hottempered",
            Trait::Sociable => "Sociable",
            Trait::Introverted => "Introverted",
            Trait::Empathetic => "Empathetic",
            Trait::Manipulative => "Manipulative",
            Trait::Obsessive => "Obsessive",
            Trait::Infertile => "Infertile",
            Trait::Narcoleptic => "Narcoleptic",
            Trait::SoundSleeper => "Sound Sleeper",
        }
    }

    /// What this trait argues for, and what it argues against.
    ///
    /// Each entry is a drive, what this trait does to how loudly that drive
    /// argues for the agent's attention, and what it does to how much of the
    /// need it takes before the agent will act on it at all. Both are
    /// multipliers on the drive's ordinary values, so 1.0 is no opinion.
    ///
    /// The two do different work and a trait usually wants both. Weight is how
    /// much somebody cares once they have noticed; threshold is how long they
    /// go before noticing. A lazy person and a diligent one both eventually
    /// get up and work - the lazy one needs more pushing to start (higher
    /// threshold) and drops it sooner for anything else (lower weight). A
    /// coward is not more frightened of a given wolf than a brave person; the
    /// coward starts running at a smaller wolf.
    ///
    /// This is the table that was missing. Sixty traits were defined almost
    /// entirely as modifiers on how an agent *feels* about what happened -
    /// "Lazy: constant happiness decrease when working", "Builder: happiness
    /// from building structures" - and `core/drives.rs` did not mention traits
    /// at all. With personalities assigned but nothing reading them, agents
    /// holding Handy spent 83% of their attempts foraging, Builder 81% and
    /// Greedy 84%: a Builder did not build. Feeling differently about the same
    /// life is not having a different one.
    ///
    /// Traits absent from this table have no view on what to do, which is
    /// right for most of them - Goth, Clown, Melancholic and the rest are
    /// about mood, and mood is somewhere else's business.
    pub fn leanings(&self) -> &'static [(crate::core::DriveType, f32, f32)] {
        use crate::core::DriveType as D;

        match self {
            // Work, and what somebody will put their hands to
            Trait::Lazy => &[(D::Industry, 0.5, 1.4), (D::Construction, 0.6, 1.3)],
            Trait::Diligent => &[(D::Industry, 1.4, 0.75), (D::Sustenance, 1.15, 1.0)],
            Trait::Handy => &[(D::Industry, 1.25, 0.9), (D::Utility, 1.3, 0.85)],
            Trait::Builder => &[(D::Construction, 1.7, 0.7)],
            Trait::CraftObsessed => &[(D::Utility, 1.6, 0.7), (D::Industry, 1.2, 0.9)],
            Trait::Ambitious => &[(D::Construction, 1.25, 0.9), (D::Industry, 1.2, 0.9)],
            Trait::Proud => &[(D::Construction, 1.15, 0.95), (D::Luxury, 1.2, 0.9)],

            // What somebody keeps, and how much of it they want about them
            Trait::Greedy => &[(D::Preparedness, 1.5, 0.7), (D::Luxury, 1.4, 0.8)],
            Trait::Frugal => &[(D::Preparedness, 1.45, 0.75), (D::Luxury, 0.6, 1.3)],
            Trait::Survivalist => &[(D::Preparedness, 1.4, 0.65), (D::Sustenance, 1.3, 0.8)],
            Trait::Ascetic => &[(D::Luxury, 0.3, 1.9), (D::Preparedness, 0.8, 1.15)],
            Trait::Envious => &[(D::Luxury, 1.45, 0.75)],

            // Other people
            Trait::Extrovert | Trait::Sociable => &[(D::Social, 1.7, 0.65)],
            Trait::Introvert | Trait::Introverted => &[(D::Social, 0.45, 1.5)],
            Trait::Charismatic => &[(D::Social, 1.3, 0.85)],
            Trait::Gossip => &[(D::Social, 1.4, 0.8)],
            Trait::Romantic => &[(D::Reproduction, 1.35, 0.8), (D::Social, 1.2, 0.9)],
            Trait::Mute => &[(D::Social, 0.6, 1.3)],

            // Looking after people who are not oneself
            Trait::Caretaker => &[(D::Protection, 1.5, 0.65)],
            Trait::Altruist => &[(D::Protection, 1.4, 0.7), (D::Social, 1.2, 0.9)],
            Trait::KindHearted => &[(D::Protection, 1.3, 0.8)],
            Trait::Protector => &[(D::Protection, 1.4, 0.7), (D::Safety, 1.2, 0.9)],
            Trait::Callous => &[(D::Protection, 0.55, 1.4), (D::Social, 0.8, 1.15)],
            Trait::Cruel => &[(D::Protection, 0.5, 1.5)],

            // Danger, and how big a thing has to be before it is one
            Trait::Coward => &[(D::Safety, 1.6, 0.6)],
            Trait::Brave => &[(D::Safety, 0.6, 1.45)],
            Trait::Anxious => &[(D::Safety, 1.4, 0.7), (D::Preparedness, 1.2, 0.85)],
            Trait::Paranoid => &[(D::Safety, 1.5, 0.65), (D::Preparedness, 1.15, 0.9)],
            Trait::Suspicious => &[(D::Safety, 1.2, 0.85)],
            Trait::Aggressive => &[(D::Safety, 0.7, 1.3)],
            Trait::Peaceful | Trait::Pacifist => &[(D::Safety, 1.15, 0.9)],

            // Wanting to know
            Trait::Curious => &[(D::Curiosity, 1.6, 0.65)],
            Trait::Explorer => &[(D::Curiosity, 1.5, 0.7)],
            Trait::Bookworm => &[(D::Curiosity, 1.4, 0.75)],
            Trait::Imaginative => &[(D::Curiosity, 1.25, 0.85)],
            Trait::Stubborn => &[(D::Curiosity, 0.7, 1.3)],
            Trait::Traditionalist => &[(D::Curiosity, 0.75, 1.25), (D::Utility, 1.15, 0.9)],

            // The body
            Trait::Glutton => &[(D::Hunger, 1.3, 0.8), (D::Sustenance, 1.3, 0.8)],
            Trait::Narcoleptic => &[(D::Rest, 1.3, 0.8)],
            Trait::SoundSleeper => &[(D::Rest, 0.75, 1.25)],
            Trait::Resilient => &[(D::Rest, 0.85, 1.15), (D::Shelter, 0.85, 1.15)],

            // Everything else is about how a life feels rather than what is
            // done with it, which is somewhere else's business
            _ => &[],
        }
    }

    /// Check if two traits are incompatible (alias for is_incompatible_with)
    pub fn incompatible_with(&self, other: &Trait) -> bool {
        self.is_incompatible_with(other)
    }

    /// Check if two traits are incompatible
    pub fn is_incompatible_with(&self, other: &Trait) -> bool {
        matches!(
            (self, other),
            (Trait::Anxious, Trait::Brave) | (Trait::Brave, Trait::Anxious) |
            (Trait::HotHeaded, Trait::Calm) | (Trait::Calm, Trait::HotHeaded) |
            (Trait::HotHeaded, Trait::Pacifist) | (Trait::Pacifist, Trait::HotHeaded) |
            (Trait::Empathic, Trait::ColdHearted) | (Trait::ColdHearted, Trait::Empathic) |
            (Trait::Extrovert, Trait::Introvert) | (Trait::Introvert, Trait::Extrovert) |
            (Trait::KindHearted, Trait::Cruel) | (Trait::Cruel, Trait::KindHearted) |
            (Trait::Clown, Trait::Goth) | (Trait::Goth, Trait::Clown) |
            (Trait::Believer, Trait::Atheist) | (Trait::Atheist, Trait::Believer) |
            (Trait::Believer, Trait::Skeptic) | (Trait::Skeptic, Trait::Believer) |
            (Trait::Zealot, Trait::Atheist) | (Trait::Atheist, Trait::Zealot) |
            (Trait::Zealot, Trait::Skeptic) | (Trait::Skeptic, Trait::Zealot) |
            (Trait::Greedy, Trait::Ascetic) | (Trait::Ascetic, Trait::Greedy) |
            (Trait::Greedy, Trait::Altruist) | (Trait::Altruist, Trait::Greedy) |
            (Trait::Greedy, Trait::Frugal) | (Trait::Frugal, Trait::Greedy) |
            (Trait::AnimalLover, Trait::Allergic) | (Trait::Allergic, Trait::AnimalLover) |
            (Trait::Vengeful, Trait::Forgiving) | (Trait::Forgiving, Trait::Vengeful) |
            (Trait::Brave, Trait::Coward) | (Trait::Coward, Trait::Brave) |
            (Trait::Handy, Trait::Lazy) | (Trait::Lazy, Trait::Handy) |
            (Trait::Stoic, Trait::Repressed) | (Trait::Repressed, Trait::Stoic) |
            (Trait::Charismatic, Trait::Mute) | (Trait::Mute, Trait::Charismatic) |
            (Trait::Aggressive, Trait::Peaceful) | (Trait::Peaceful, Trait::Aggressive) |
            (Trait::Trusting, Trait::Suspicious) | (Trait::Suspicious, Trait::Trusting) |
            (Trait::Trusting, Trait::Paranoid) | (Trait::Paranoid, Trait::Trusting) |
            (Trait::Honest, Trait::Dishonest) | (Trait::Dishonest, Trait::Honest) |
            (Trait::Honest, Trait::Manipulator) | (Trait::Manipulator, Trait::Honest) |
            (Trait::Honest, Trait::Manipulative) | (Trait::Manipulative, Trait::Honest) |
            (Trait::Empathic, Trait::Callous) | (Trait::Callous, Trait::Empathic) |
            (Trait::KindHearted, Trait::Callous) | (Trait::Callous, Trait::KindHearted) |
            (Trait::Diligent, Trait::Lazy) | (Trait::Lazy, Trait::Diligent) |
            // Compatibility aliases
            (Trait::Hottempered, Trait::Calm) | (Trait::Calm, Trait::Hottempered) |
            (Trait::Hottempered, Trait::Pacifist) | (Trait::Pacifist, Trait::Hottempered) |
            (Trait::Sociable, Trait::Introvert) | (Trait::Introvert, Trait::Sociable) |
            (Trait::Sociable, Trait::Introverted) | (Trait::Introverted, Trait::Sociable) |
            (Trait::Extrovert, Trait::Introverted) | (Trait::Introverted, Trait::Extrovert) |
            (Trait::Empathetic, Trait::ColdHearted) | (Trait::ColdHearted, Trait::Empathetic) |
            (Trait::Empathetic, Trait::Callous) | (Trait::Callous, Trait::Empathetic)
        )
    }

    /// Get emotion modifier for this trait
    /// Returns (emotion_type, increase_multiplier, decrease_multiplier)
    pub fn emotion_modifiers(&self) -> Vec<(crate::core::EmotionType, f32, f32)> {
        use crate::core::EmotionType;

        match self {
            Trait::Anxious => vec![(EmotionType::Fear, 2.0, 0.5)],
            Trait::Brave => vec![(EmotionType::Fear, 0.5, 2.0)],
            Trait::HotHeaded => vec![(EmotionType::Anger, 2.0, 0.5)],
            Trait::Calm => vec![(EmotionType::Anger, 0.5, 2.0)],
            Trait::Pacifist => vec![(EmotionType::Anger, 0.0, 1.0)],
            Trait::Empathic => vec![(EmotionType::Sadness, 2.0, 2.0)],
            Trait::ColdHearted => vec![(EmotionType::Sadness, 0.5, 2.0)],
            Trait::Resilient => vec![(EmotionType::Sadness, 1.0, 2.0)],
            Trait::Clown => vec![(EmotionType::Happiness, 2.0, 0.5)],
            Trait::Goth => vec![(EmotionType::Happiness, 0.5, 2.0)],
            Trait::Stoic => vec![
                (EmotionType::Fear, 0.5, 0.5),
                (EmotionType::Anger, 0.5, 0.5),
                (EmotionType::Sadness, 0.5, 0.5),
                (EmotionType::Happiness, 0.5, 0.5),
                (EmotionType::Curiosity, 0.5, 0.5),
            ],
            Trait::Forgiving => vec![(EmotionType::Anger, 1.0, 2.0)],
            // Compatibility aliases - map to same as their base traits
            Trait::Hottempered => vec![(EmotionType::Anger, 2.0, 0.5)], // Same as HotHeaded
            Trait::Empathetic => vec![(EmotionType::Sadness, 2.0, 2.0)], // Same as Empathic
            _ => vec![],
        }
    }

    /// Get happiness gain from expressing this trait
    pub fn happiness_gain(&self) -> f32 {
        match self {
            Trait::Imaginative => 5.0,    // From embellishing stories
            Trait::Manipulator => 10.0,   // From successful manipulation
            Trait::Manipulative => 10.0,  // From successful manipulation (alias)
            Trait::Forgiving => 3.0,      // From forgiving others
            Trait::Extrovert => 5.0,      // From social interaction
            Trait::Diligent => 2.0,       // From hard work
            Trait::Altruist => 5.0,       // From helping others
            Trait::Handy => 5.0,          // From completing tasks
            Trait::Proud => 4.0,          // From accomplishments
            Trait::Builder => 6.0,        // From building
            Trait::Explorer => 4.0,       // From exploring
            Trait::Caretaker => 3.0,      // From helping others
            // Compatibility aliases
            Trait::Sociable => 5.0,       // From social interaction (same as Extrovert)
            _ => 0.0,
        }
    }

    /// Get trust modifier for evaluating or being evaluated by others
    /// Positive values increase trust, negative values decrease trust
    pub fn trust_modifier(&self) -> f32 {
        match self {
            Trait::Trusting => 0.3,        // +30% trust in others
            Trait::Suspicious => -0.3,     // -30% trust in others
            Trait::Paranoid => -0.5,       // -50% trust in others
            Trait::Honest => 0.2,          // Others trust you +20%
            Trait::Dishonest => -0.2,      // Others trust you -20%
            Trait::Manipulator => -0.3,    // Others distrust you -30%
            Trait::Manipulative => -0.3,   // Others distrust you -30% (alias)
            Trait::KindHearted => 0.1,     // Others trust you slightly more
            Trait::Cruel => -0.2,          // Others distrust you
            Trait::Charismatic => 0.15,    // Others trust you more
            Trait::Skeptic => -0.2,        // Distrusts most information
            _ => 0.0,
        }
    }
}

/// Agent's trait collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitSet {
    pub traits: Vec<Trait>,
}

impl TraitSet {
    pub fn new() -> Self {
        Self {
            traits: Vec::new(),
        }
    }

    /// Add a trait if compatible with existing traits
    pub fn add_trait(&mut self, trait_to_add: Trait) -> bool {
        // Check if already has this trait
        if self.traits.contains(&trait_to_add) {
            return false;
        }

        // Check incompatibilities
        for existing_trait in &self.traits {
            if trait_to_add.is_incompatible_with(existing_trait) {
                return false;
            }
        }

        self.traits.push(trait_to_add);
        true
    }

    /// Check if agent has a specific trait
    pub fn has(&self, trait_check: Trait) -> bool {
        self.traits.contains(&trait_check)
    }

    /// Check if agent has a specific trait (alias for has)
    pub fn has_trait(&self, trait_check: &Trait) -> bool {
        self.traits.contains(trait_check)
    }


    /// Get all traits as a slice
    pub fn get_traits(&self) -> &[Trait] {
        &self.traits
    }

    /// Iterate over all traits
    pub fn iter(&self) -> impl Iterator<Item = &Trait> {
        self.traits.iter()
    }

    /// Calculate combined trust modifier from all traits
    pub fn combined_trust_modifier(&self) -> f32 {
        self.traits
            .iter()
            .map(|t| t.trust_modifier())
            .sum()
    }

    /// Check if agent would distort information (lie, exaggerate, manipulate)
    /// Returns the trait that causes distortion if any
    pub fn would_distort_info(&self) -> Option<Trait> {
        if self.has(Trait::Manipulator) {
            Some(Trait::Manipulator)
        } else if self.has(Trait::Manipulative) {
            Some(Trait::Manipulative)
        } else if self.has(Trait::Dishonest) {
            Some(Trait::Dishonest)
        } else if self.has(Trait::Imaginative) {
            Some(Trait::Imaginative)
        } else {
            None
        }
    }

    /// Generate random traits for a new agent
    pub fn generate_random(count: usize) -> Self {
        use rand::seq::SliceRandom;

        let all_traits = [
            Trait::Anxious, Trait::Brave, Trait::HotHeaded, Trait::Calm,
            Trait::Pacifist, Trait::Empathic, Trait::ColdHearted, Trait::Resilient,
            Trait::Clown, Trait::Goth, Trait::Melancholic, Trait::Stoic,
            Trait::Extrovert, Trait::Introvert, Trait::KindHearted, Trait::Cruel,
            Trait::Charismatic, Trait::Gossip, Trait::Intolerant, Trait::Mediator,
            Trait::Romantic, Trait::Insecure, Trait::Handy, Trait::Lazy,
            Trait::Proud, Trait::Ambitious, Trait::Pragmatist, Trait::Stubborn,
            Trait::Traditionalist, Trait::Rebel, Trait::Builder, Trait::CraftObsessed,
            Trait::Greedy, Trait::Ascetic, Trait::Envious, Trait::Frugal,
            Trait::Survivalist, Trait::Altruist, Trait::Glutton, Trait::Believer,
            Trait::Atheist, Trait::Zealot, Trait::Skeptic, Trait::Bookworm,
            Trait::Curious, Trait::Suspicious, Trait::Uncaring, Trait::Vengeful,
            Trait::Forgiving, Trait::Coward, Trait::Protector, Trait::AnimalLover,
            Trait::Allergic, Trait::Explorer, Trait::Caretaker, Trait::Aggressive,
            Trait::Peaceful, Trait::Trusting, Trait::Honest, Trait::Dishonest,
            Trait::Callous, Trait::Diligent, Trait::Manipulator, Trait::Imaginative,
        ];

        let mut rng = crate::core::dice::roll();

        // Walk the whole pool in a random order rather than drawing a fixed
        // handful. Drawing twice the wanted number and stopping was near
        // enough while nobody used this, but it hands back short sets whenever
        // the draw happens to contain a pair that cannot both be true of one
        // person - and a settlement where some people have four traits and
        // others one for no reason is a settlement of accidents.
        let mut pool = all_traits;
        pool.shuffle(&mut rng);

        let mut trait_set = TraitSet::new();
        for trait_candidate in pool {
            if trait_set.traits.len() >= count {
                break;
            }
            trait_set.add_trait(trait_candidate);
        }

        trait_set
    }

    /// How many traits a person is drawn with.
    ///
    /// Enough that no two people in a settlement are quite alike, few enough
    /// that each one still tells: at three to five out of sixty-odd, two
    /// agents sharing even one trait is uncommon, and nobody is a bundle of
    /// every tendency at once.
    pub const TRAITS_AT_BIRTH: std::ops::RangeInclusive<usize> = 3..=5;

    /// Draw a personality for somebody nobody was born to.
    ///
    /// The founding generation of a world has no parents to take after, so
    /// they are drawn from the pool; everybody afterwards inherits from the
    /// two people who made them, with a chance of mutation, which is what
    /// `inherit_traits` does.
    pub fn a_person() -> Self {
        use rand::Rng;

        let count = crate::core::dice::roll().gen_range(Self::TRAITS_AT_BIRTH);

        // Ordinary tendencies only. Blindness, deafness and muteness are in
        // the pool `inherit_traits` mutates from but not in this one, so they
        // arise in a people over generations rather than in the handful who
        // founded the place - which is where congenital infertility already
        // sat, and is the same reasoning: these are things somebody is born
        // with, and a founding twelve who walked into a country are the one
        // group in the model nobody was born into.
        Self::generate_random(count)
    }
}

impl Default for TraitSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_incompatibility() {
        assert!(Trait::Anxious.is_incompatible_with(&Trait::Brave));
        assert!(Trait::Brave.is_incompatible_with(&Trait::Anxious));
        assert!(!Trait::Anxious.is_incompatible_with(&Trait::Calm));
    }

    #[test]
    fn test_trait_set_creation() {
        let trait_set = TraitSet::new();
        assert_eq!(trait_set.traits.len(), 0);
    }

    #[test]
    fn test_add_compatible_trait() {
        let mut trait_set = TraitSet::new();
        assert!(trait_set.add_trait(Trait::Brave));
        assert!(trait_set.add_trait(Trait::Calm));
        assert_eq!(trait_set.traits.len(), 2);
    }

    #[test]
    fn test_reject_incompatible_trait() {
        let mut trait_set = TraitSet::new();
        assert!(trait_set.add_trait(Trait::Brave));
        assert!(!trait_set.add_trait(Trait::Anxious)); // Incompatible
        assert_eq!(trait_set.traits.len(), 1);
    }

    #[test]
    fn test_has_trait() {
        let mut trait_set = TraitSet::new();
        trait_set.add_trait(Trait::Brave);

        assert!(trait_set.has(Trait::Brave));
        assert!(!trait_set.has(Trait::Anxious));
    }

    #[test]
    fn test_emotion_modifiers() {
        let modifiers = Trait::Anxious.emotion_modifiers();
        assert!(!modifiers.is_empty());
    }

    #[test]
    fn test_generate_random_traits() {
        let trait_set = TraitSet::generate_random(3);
        assert!(trait_set.traits.len() >= 1 && trait_set.traits.len() <= 3);
    }
}
