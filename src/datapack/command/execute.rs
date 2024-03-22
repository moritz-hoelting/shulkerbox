use std::ops::{BitAnd, BitOr, Not};

use serde::{Deserialize, Serialize};

use super::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Execute {
    Align(String, Box<Execute>),
    Anchored(String, Box<Execute>),
    As(String, Box<Execute>),
    At(String, Box<Execute>),
    AsAt(String, Box<Execute>),
    Facing(String, Box<Execute>),
    In(String, Box<Execute>),
    On(String, Box<Execute>),
    Positioned(String, Box<Execute>),
    Rotated(String, Box<Execute>),
    Store(String, Box<Execute>),
    Summon(String, Box<Execute>),
    If(Condition, Box<Execute>),
    Run(String, Box<Command>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    Atom(String),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}
impl Condition {
    pub fn normalize(self) -> Self {
        match self {
            Self::Atom(_) => self,
            Self::Not(c) => match *c {
                Self::Atom(c) => Self::Not(Box::new(Self::Atom(c))),
                Self::Not(c) => c.normalize(),
                Self::And(c1, c2) => ((!*c1).normalize()) | ((!*c2).normalize()),
                Self::Or(c1, c2) => ((!*c1).normalize()) & ((!*c2).normalize()),
            },
            Self::And(c1, c2) => c1.normalize() & c2.normalize(),
            Self::Or(c1, c2) => c1.normalize() | c2.normalize(),
        }
    }
}

impl Not for Condition {
    type Output = Self;

    fn not(self) -> Self {
        Condition::Not(Box::new(self))
    }
}
impl BitAnd for Condition {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Condition::And(Box::new(self), Box::new(rhs))
    }
}
impl BitOr for Condition {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Condition::Or(Box::new(self), Box::new(rhs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition() {
        let c1 = Condition::Atom("foo".to_string());
        let c2 = Condition::Atom("bar".to_string());
        let c3 = Condition::Atom("baz".to_string());

        assert_eq!(
            (c1.clone() & c2.clone()).normalize(),
            c1.clone() & c2.clone()
        );
        assert_eq!(
            (c1.clone() & c2.clone() & c3.clone()).normalize(),
            c1.clone() & c2.clone() & c3.clone()
        );
        assert_eq!(
            (c1.clone() | c2.clone()).normalize(),
            c1.clone() | c2.clone()
        );
        assert_eq!(
            (c1.clone() | c2.clone() | c3.clone()).normalize(),
            c1.clone() | c2.clone() | c3.clone()
        );
        assert_eq!(
            (c1.clone() & c2.clone() | c3.clone()).normalize(),
            c1.clone() & c2.clone() | c3.clone()
        );
        assert_eq!(
            (c1.clone() | c2.clone() & c3.clone()).normalize(),
            c1.clone() | c2.clone() & c3.clone()
        );
        assert_eq!(
            (c1.clone() & c2.clone() | c3.clone() & c1.clone()).normalize(),
            c1.clone() & c2.clone() | c3.clone() & c1.clone()
        );
        assert_eq!(
            (!(c1.clone() | c2.clone())).normalize(),
            !c1.clone() & !c2.clone()
        );
    }
}
