mod ops;
use crate::{
    document::{Element, ElementDelimiter, ElementType, NodeType, Tag},
    utils::default,
};

use super::{
    rules, DirectionPrecedence, Incomplete, InferencePredicate, InferencePredicateContext,
    InferenceTarget, TestResult,
};

pub struct InferWhen<P>(P);

impl<P: InferencePredicate> InferencePredicate for InferWhen<P> {
    fn test(&mut self, cx: &InferencePredicateContext) -> TestResult {
        self.0.test(cx)
    }
}

pub fn when<P: InferencePredicate>(pred: P) -> InferWhen<P> {
    InferWhen(pred)
}

impl<P> InferWhen<P>
where
    P: InferencePredicate,
{
    pub fn then_apply<'cfg, A>(self, action: A) -> rules::RuleImpl<P, A>
    where
        A: for<'a, 'b> FnMut(
            &'a mut InferenceTarget<'cfg, 'b>,
        ) -> &'a mut InferenceTarget<'cfg, 'b>,
    {
        rules::RuleImpl {
            pred: self.0,
            action,
        }
    }
}

struct PredImpl<Test, DirPrec, Data> {
    matches: Test,
    dir_prec: DirPrec,
    data: Data,
}

impl<Test, DirPrec, Data> InferWhen<PredImpl<Test, DirPrec, Data>> {
    pub fn direction_precedence<F: Fn(&Data) -> DirectionPrecedence>(
        self,
        dir_prec: F,
    ) -> InferWhen<PredImpl<Test, F, Data>> {
        InferWhen(PredImpl {
            matches: self.0.matches,
            dir_prec,
            data: self.0.data,
        })
    }
}

fn pred_impl<Test: FnMut(&InferencePredicateContext) -> TestResult>(
    mut m: Test,
) -> InferWhen<
    PredImpl<
        impl FnMut(&mut (), &InferencePredicateContext) -> TestResult,
        impl Fn(&()) -> DirectionPrecedence,
        (),
    >,
> {
    InferWhen(PredImpl {
        matches: move |_: &mut _, cx: &InferencePredicateContext| m(cx),
        dir_prec: |&_: &_| default(),
        data: (),
    })
}

fn pred_impl_with<Test: FnMut(&mut Data, &InferencePredicateContext) -> TestResult, Data>(
    data: Data,
    matches: Test,
) -> InferWhen<PredImpl<Test, impl Fn(&Data) -> DirectionPrecedence, Data>> {
    InferWhen(PredImpl {
        matches,
        dir_prec: |_: &_| default(),
        data,
    })
}

impl<
        Test: FnMut(&mut Data, &InferencePredicateContext) -> TestResult,
        DirPrec: Fn(&Data) -> DirectionPrecedence,
        Data,
    > InferencePredicate for PredImpl<Test, DirPrec, Data>
{
    fn test(&mut self, cx: &InferencePredicateContext) -> TestResult {
        (self.matches)(&mut self.data, cx)
    }
}

pub fn element_where<F>(mut f: F) -> InferWhen<impl InferencePredicate>
where
    F: FnMut(&Element) -> bool,
{
    pred_impl(move |cx| cx.match_this_element(|e| Ok(f(e))))
}

pub fn element() -> InferWhen<impl InferencePredicate> {
    pred_impl(|cx| cx.match_this_node(|n| Ok(matches!(n.node_type, NodeType::Element { .. }))))
}

pub fn paragraph() -> InferWhen<impl InferencePredicate> {
    element_where(|e| matches!(e.element_type, ElementType::Paragraph {}))
}

pub fn line() -> InferWhen<impl InferencePredicate> {
    element_where(|e| {
        matches!(
            e.element_type,
            ElementType::Standard {
                delimiter: ElementDelimiter::Line { .. } | ElementDelimiter::LineBlock { .. }
            }
        )
    })
}

pub fn block() -> InferWhen<impl InferencePredicate> {
    element_where(|e| {
        matches!(
            e.element_type,
            ElementType::Standard {
                delimiter: ElementDelimiter::Block { .. }
            }
        )
    })
}

pub fn tag(tag: &str) -> InferWhen<impl InferencePredicate + '_> {
    tag_where(move |s| s == tag)
}

pub fn tag_in<'tag>(
    tags: impl IntoIterator<Item = &'tag str> + Clone + 'tag,
) -> InferWhen<impl InferencePredicate + 'tag> {
    tag_where(move |s| tags.clone().into_iter().any(|tag| s == tag))
}

pub fn tag_where<'tag>(
    mut pred: impl FnMut(&str) -> bool + 'tag,
) -> InferWhen<impl InferencePredicate + 'tag> {
    pred_impl(move |cx| {
        cx.match_this_element(|e| {
            Ok(e.selectors
                .first()
                .map(|s| match s.tag {
                    Tag::Explicit { ref value } => Ok(value.as_str(cx.src)),
                    _ => Err(Incomplete {}),
                })
                .transpose()?
                .map(&mut pred))
        })
    })
}

pub fn any() -> InferWhen<impl InferencePredicate> {
    pred_impl(|_| Ok(true))
}

pub fn child_of(pred: impl InferencePredicate) -> InferWhen<impl InferencePredicate> {
    pred_impl_with(pred, |pred, cx| {
        Ok(cx.parent.and_then(|cx| pred.test(cx).ok()).unwrap_or(false))
    })
}

pub fn descendant_of(pred: impl InferencePredicate) -> InferWhen<impl InferencePredicate> {
    pred_impl_with(pred, |pred, mut cx| {
        while let Some(parent) = cx.parent {
            cx = parent;
            if pred.test(cx).unwrap_or(false) {
                return Ok(true);
            }
        }
        Ok(false)
    })
}

mod relative {
    use super::*;
    macro_rules! impl_relative {
        ($func:ident, $Type:ident, |$idx:pat_param, $lo:pat_param, $hi:pat_param| $which:expr, $field:ident, $dir:expr) => {
            pub struct $Type<P>(P);

            impl<P: InferencePredicate> InferencePredicate for $Type<P> {
                fn test(&mut self, cx: &InferencePredicateContext) -> TestResult {
                    let mut out = Ok(false);
                    match (cx.index, 0, cx.nodes.len()) {
                        ($idx, $lo, $hi) => {
                            for index in $which {
                                out.and(
                                    match self.0.test(&InferencePredicateContext { index, ..*cx }) {
                                        Ok(true) => return Ok(true),
                                        res => res,
                                    },
                                );
                            }
                        }
                    }
                    out
                }

                fn direction_precedence(&self) -> DirectionPrecedence {
                    self.0.direction_precedence()
                        + DirectionPrecedence {
                            $field: $dir,
                            ..default()
                        }
                }
            }

            pub fn $func<P: InferencePredicate>(pred: P) -> InferWhen<$Type<P>> {
                when($Type(pred))
            }
        };
    }

    impl_relative!(before, Before, |i, _, hi| (i + 1)..hi, any, -1);
    impl_relative!(after, After, |i, lo, _| lo..i, any, 1);
    impl_relative!(
        just_before,
        JustBefore,
        |i, _, hi| (i + 1 < hi).then_some(i + 1),
        any,
        -1
    );
    impl_relative!(just_after, JustAfter, |i, lo, _| i.checked_sub(1), any, 1);
}
