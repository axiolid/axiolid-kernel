//! Certified signs: the bridge between a float computation and a decision.
//!
//! No topology-changing decision may depend on an uncertified floating-point
//! sign. A bare `f64` cannot express "I computed this and the sign is proven",
//! so this module makes the distinction a type the compiler enforces.

use crate::Precision;

/// The sign of a geometric predicate, with its trustworthiness attached.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign {
    /// Certified strictly negative.
    Negative,
    /// Certified exactly zero (degenerate configuration).
    Zero,
    /// Certified strictly positive.
    Positive,
}

impl Sign {
    /// The sign of the negated value.
    ///
    /// Orientation predicates are antisymmetric: swapping two arguments must
    /// flip the result. Expressing that as one operation keeps callers from
    /// open-coding a match that silently mishandles `Zero`.
    #[must_use]
    pub const fn flip(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
            Self::Zero => Self::Zero,
        }
    }
}

/// A predicate evaluation that may or may not be trustworthy.
///
/// `Uncertain` is deliberately not a sign: it carries no `Sign` payload, so a
/// caller cannot accidentally read a value out of it. The only way to obtain a
/// `Sign` is to handle the uncertain case explicitly.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Certified {
    /// The sign is proven at the reported precision.
    Certain {
        /// The proven sign.
        sign: Sign,
        /// Precision tier at which the proof was obtained.
        precision: Precision,
    },
    /// The computed value was within its own error bound of zero, so the sign
    /// could not be determined at this precision. Escalate.
    Uncertain {
        /// Precision tier that failed to decide.
        attempted: Precision,
    },
}

impl Certified {
    /// Certify a value against a filter's absolute error bound.
    ///
    /// The bound is the maximum absolute error the computation may carry. If
    /// zero lies inside `value +- bound` the sign is not decidable here, which
    /// is the whole point of a filtered predicate: it reports failure instead
    /// of guessing. A non-finite value or bound is never certifiable.
    pub fn from_filter(value: f64, error_bound: f64, precision: Precision) -> Self {
        if !value.is_finite() || !error_bound.is_finite() || error_bound < 0.0 {
            return Self::Uncertain {
                attempted: precision,
            };
        }
        if value > error_bound {
            Self::Certain {
                sign: Sign::Positive,
                precision,
            }
        } else if value < -error_bound {
            Self::Certain {
                sign: Sign::Negative,
                precision,
            }
        } else {
            Self::Uncertain {
                attempted: precision,
            }
        }
    }

    /// Certify an exact computation. Only valid where no rounding occurred.
    ///
    /// An exact zero is a *certain* answer -- the configuration is genuinely
    /// degenerate -- which is different from being unable to decide.
    /// A sign established by exact arithmetic, where no integer value exists.
    ///
    /// An exact predicate cascade produces a proven sign without producing a
    /// representable magnitude: the determinant lives in a multi-term
    /// expansion, not one `i64`. Without this constructor such a result could
    /// only be reported as `Uncertain`, which would discard the very proof the
    /// exact path was paid for.
    pub const fn exact_sign(sign: Sign) -> Self {
        Self::Certain {
            sign,
            precision: Precision::Exact,
        }
    }

    pub const fn exact(value: i64) -> Self {
        let sign = if value > 0 {
            Sign::Positive
        } else if value < 0 {
            Sign::Negative
        } else {
            Sign::Zero
        };
        Self::Certain {
            sign,
            precision: Precision::Exact,
        }
    }

    /// The proven sign, or `None` when escalation is required.
    pub const fn sign(self) -> Option<Sign> {
        match self {
            Self::Certain { sign, .. } => Some(sign),
            Self::Uncertain { .. } => None,
        }
    }

    /// Whether this result is safe to drive a topology decision.
    pub const fn is_certain(self) -> bool {
        matches!(self, Self::Certain { .. })
    }
}

/// The precision tiers a filtered predicate steps through, weakest first.
///
/// This is the executable form of the fast-path/escalation design: try a cheap
/// filter, and only pay for stronger arithmetic on the cases that need it. A
/// backend advertises how far it can escalate; a caller learns whether the
/// answer it got was cheap or expensive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EscalationLadder {
    ceiling: Precision,
}

impl EscalationLadder {
    /// Tiers in escalation order. `Mixed` is not a rung: it describes an
    /// operation's internal strategy, not a step of increasing certainty.
    const RUNGS: [Precision; 3] = [Precision::F32, Precision::F64, Precision::Exact];

    /// A ladder that may escalate up to and including `ceiling`.
    pub const fn new(ceiling: Precision) -> Self {
        Self { ceiling }
    }

    /// A ladder ending in exact arithmetic: every sign is decidable.
    pub const fn exact() -> Self {
        Self::new(Precision::Exact)
    }

    /// Highest precision this ladder may reach.
    pub const fn ceiling(self) -> Precision {
        self.ceiling
    }

    /// Whether reaching `ceiling` guarantees every sign becomes decidable.
    ///
    /// Only exact arithmetic does. A ladder topping out at `F64` can still
    /// return `Uncertain`, and a caller that needs a decision must know that.
    pub const fn is_total(self) -> bool {
        matches!(self.ceiling, Precision::Exact)
    }

    /// The next tier to try after `current`, or `None` at the ceiling.
    pub fn next_after(self, current: Precision) -> Option<Precision> {
        let index = Self::RUNGS.iter().position(|&rung| rung == current)?;
        Self::RUNGS
            .iter()
            .skip(index + 1)
            .copied()
            .find(|&rung| self.permits(rung))
    }

    /// Whether this ladder may use `precision`.
    pub fn permits(self, precision: Precision) -> bool {
        let Some(rung) = Self::rung_index(precision) else {
            return false;
        };
        match Self::rung_index(self.ceiling) {
            Some(ceiling) => rung <= ceiling,
            None => false,
        }
    }

    /// Tiers this ladder will actually attempt, in order.
    pub fn rungs(self) -> impl Iterator<Item = Precision> {
        Self::RUNGS
            .into_iter()
            .filter(move |&rung| self.permits(rung))
    }

    fn rung_index(precision: Precision) -> Option<usize> {
        Self::RUNGS.iter().position(|&rung| rung == precision)
    }
}
