//! What a finished run does to the card it ran on.
//!
//! The other half of `steps::sends_back`, which has been the only automatic
//! transition in the product and has only ever gone backwards. This is the
//! forward half, and the two are deliberately not symmetric: a card carries
//! where it came from, so backward needs no declaration; nothing records where
//! it was going, so forward has to be told.
//!
//! The rule is a function of plain values on purpose. It decides whether work
//! somebody has not watched moves their card, and a rule like that should be
//! readable and testable without a window, a database or an agent.

use devpit_rpc::Step;

/// How much a lane decides without being asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Autonomy {
    /// Nothing happens on its own. The default, and what every board has.
    Manual,
    /// The step runs and the verdict is read; the card waits for a person.
    Ask,
    /// The card advances on the verdict.
    Auto,
}

impl Autonomy {
    /// Reads the stored word.
    ///
    /// Anything unrecognised is `Manual`, and that direction is the point: a
    /// value this build does not understand must not be read as permission to
    /// move somebody's card on its own.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "ask" => Self::Ask,
            "auto" => Self::Auto,
            _ => Self::Manual,
        }
    }

    pub fn stored(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Ask => "ask",
            Self::Auto => "auto",
        }
    }
}

/// What the lane knows when a run on it ends.
pub struct Lane<'a> {
    pub autonomy: Autonomy,
    /// Where a pass sends the card, when the lane says.
    pub on_pass: Option<&'a str>,
    /// Whether the destination runs something with no undo.
    pub target_is_irreversible: bool,
}

/// What a run's answer said about itself.
pub enum Verdict {
    /// The step declared a verdict field and the answer said it passed.
    Passed,
    /// The step declared one and the answer asked for the card to go back.
    SendBack(String),
    /// The step declared none, or the answer did not carry it.
    ///
    /// Not a pass. A run that ends `ok` with an answer that has no opinion is
    /// a run with no opinion, and treating silence as approval is how a card
    /// advances on a judgment nobody made.
    NoOpinion,
}

/// What should happen to the card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Move {
    /// Nowhere. The card stays where it is.
    Stay,
    /// Back where it came from, with the reason.
    Back { to: String, why: String },
    /// Forward, because the lane said so and is allowed to.
    Forward { to: String },
    /// Forward is possible and waiting to be accepted by a person.
    Offer { to: String },
}

/// The rule.
///
/// Reads top to bottom, and the order is the argument:
///
///  1. A refusal moves the card back whatever the lane's autonomy is. Sending
///     work back is the one direction that is never a surprise — it is the
///     agent saying its own answer was not good enough.
///  2. Nothing else moves a card in a manual lane. That is what manual means.
///  3. Silence is not approval.
///  4. A destination that runs something irreversible is never entered
///     automatically. The `irreversible` flag already keeps a deploy from
///     being fired by a drag; an automatic advance that walks around it is
///     the same failure through a different door.
pub fn decide(lane: &Lane, verdict: &Verdict, came_from: Option<&str>) -> Move {
    if let Verdict::SendBack(why) = verdict {
        return match came_from {
            Some(to) => Move::Back {
                to: to.to_owned(),
                why: why.clone(),
            },
            // Nowhere to send it. A card that started in this lane has no
            // previous column, and inventing one would move it somewhere
            // nobody chose.
            None => Move::Stay,
        };
    }

    if lane.autonomy == Autonomy::Manual {
        return Move::Stay;
    }
    if !matches!(verdict, Verdict::Passed) {
        return Move::Stay;
    }
    let Some(to) = lane.on_pass else {
        return Move::Stay;
    };
    if lane.target_is_irreversible {
        // Offered rather than refused: the work passed, and the person should
        // be told they can take the next step — just not have it taken for
        // them.
        return Move::Offer { to: to.to_owned() };
    }

    match lane.autonomy {
        Autonomy::Auto => Move::Forward { to: to.to_owned() },
        _ => Move::Offer { to: to.to_owned() },
    }
}

/// Reads an agent step's answer into a verdict.
///
/// `sends_back` already owns the backward half and is left owning it; this
/// only adds the question it never asked — whether the answer said it passed.
pub fn verdict_of(step: &Step, answer: Option<&str>) -> Verdict {
    let Some(answer) = answer else {
        return Verdict::NoOpinion;
    };
    if let Some(why) = crate::steps::verdict::sends_back(&step.config, answer) {
        return Verdict::SendBack(why);
    }
    if crate::steps::verdict::declares_a_verdict(&step.config)
        && crate::steps::verdict::verdict_given(&step.config, answer)
    {
        return Verdict::Passed;
    }
    Verdict::NoOpinion
}

#[cfg(test)]
#[path = "advancing_tests.rs"]
mod tests;
