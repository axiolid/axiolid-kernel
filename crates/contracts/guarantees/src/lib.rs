#![forbid(unsafe_code)]
//! Provider-neutral proof, refusal, and escalation vocabulary.

mod certainty;
mod precision;

pub use certainty::{Certified, EscalationLadder, Sign};
pub use precision::Precision;
