use core::ops::{BitAnd, BitOr, Not};

use super::{
    super::{DirectionPrecedence, InferencePredicate, InferencePredicateContext, TestResult},
    when, InferWhen,
};

pub struct AndPredicate<P1, P2> {
    pred1: P1,
    pred2: P2,
}

pub struct OrPredicate<P1, P2> {
    pred1: P1,
    pred2: P2,
}

pub struct NotPredicate<P> {
    pred: P,
}

impl<P1, P2> BitAnd<InferWhen<P2>> for InferWhen<P1>
where
    P1: InferencePredicate,
    P2: InferencePredicate,
{
    type Output = InferWhen<AndPredicate<P1, P2>>;

    fn bitand(self, rhs: InferWhen<P2>) -> Self::Output {
        when(AndPredicate {
            pred1: self.0,
            pred2: rhs.0,
        })
    }
}

impl<P1, P2> BitOr<InferWhen<P2>> for InferWhen<P1>
where
    P1: InferencePredicate,
    P2: InferencePredicate,
{
    type Output = InferWhen<OrPredicate<P1, P2>>;

    fn bitor(self, rhs: InferWhen<P2>) -> Self::Output {
        when(OrPredicate {
            pred1: self.0,
            pred2: rhs.0,
        })
    }
}

impl<P> Not for InferWhen<P>
where
    P: InferencePredicate,
{
    type Output = InferWhen<NotPredicate<P>>;

    fn not(self) -> Self::Output {
        when(NotPredicate { pred: self.0 })
    }
}

impl<P1, P2> InferencePredicate for AndPredicate<P1, P2>
where
    P1: InferencePredicate,
    P2: InferencePredicate,
{
    fn test(&mut self, cx: &InferencePredicateContext) -> TestResult {
        match self.pred1.test(cx) {
            Ok(false) => return Ok(false),
            res => res,
        }
        .and(match self.pred2.test(cx) {
            Ok(false) => return Ok(false),
            res => res,
        })
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        self.pred1.direction_precedence() + self.pred2.direction_precedence()
    }
}

impl<P1, P2> InferencePredicate for OrPredicate<P1, P2>
where
    P1: InferencePredicate,
    P2: InferencePredicate,
{
    fn test(&mut self, cx: &InferencePredicateContext) -> TestResult {
        match self.pred1.test(cx) {
            Ok(true) => return Ok(true),
            res => res,
        }
        .and(match self.pred2.test(cx) {
            Ok(true) => return Ok(true),
            res => res,
        })
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        self.pred1.direction_precedence() + self.pred2.direction_precedence()
    }
}

impl<P> InferencePredicate for NotPredicate<P>
where
    P: InferencePredicate,
{
    fn test(&mut self, cx: &InferencePredicateContext) -> TestResult {
        self.pred.test(cx).map(Not::not)
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        self.pred.direction_precedence()
    }
}
