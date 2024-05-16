use crate::utils::default;

use super::{
    when, ChildInference, DirectionPrecedence, InferWhen, InferencePredicate,
    InferencePredicateContext, InferenceTarget, IntoInferenceMethod,
};

pub trait InferenceRule<'cfg> {
    fn test_rule(
        &mut self,
        depth: usize,
        max_depth: usize,
        cx: &InferencePredicateContext<'cfg, '_>,
    ) -> Result<Option<usize>, usize>;

    fn apply_rule(
        &mut self,
        depth: usize,
        target_depth: usize,
        target: &mut InferenceTarget<'cfg, '_>,
    );

    fn direction_precedence(&self) -> DirectionPrecedence;

    fn height(&self) -> usize;
}

#[non_exhaustive]
pub struct EmptyRule {}

impl InferenceRule<'_> for EmptyRule {
    fn test_rule(
        &mut self,
        _: usize,
        _: usize,
        _: &InferencePredicateContext,
    ) -> Result<Option<usize>, usize> {
        Ok(None)
    }

    fn apply_rule(&mut self, _: usize, _: usize, _: &mut InferenceTarget<'_, '_>) {}

    fn direction_precedence(&self) -> DirectionPrecedence {
        default()
    }

    fn height(&self) -> usize {
        0
    }
}

pub struct RuleImpl<P, A> {
    pub(crate) pred: P,
    pub(crate) action: A,
}

pub struct RulePair<R1, R2> {
    prev_rules: R1,
    rule: R2,
    height: usize,
}

impl<'cfg, P, A> InferenceRule<'cfg> for RuleImpl<P, A>
where
    P: InferencePredicate,
    A: for<'a, 'b> FnMut(&'a mut InferenceTarget<'cfg, 'b>) -> &'a mut InferenceTarget<'cfg, 'b>,
{
    fn test_rule(
        &mut self,
        depth: usize,
        _: usize,
        cx: &InferencePredicateContext,
    ) -> Result<Option<usize>, usize> {
        match self.pred.test(cx) {
            Ok(true) => Ok(Some(depth)),
            Ok(false) => Ok(None),
            Err(_) => Err(depth),
        }
    }

    fn apply_rule(&mut self, _: usize, _: usize, target: &mut InferenceTarget<'cfg, '_>) {
        (self.action)(target);
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        self.pred.direction_precedence()
    }

    fn height(&self) -> usize {
        1
    }
}

impl<'cfg, R1, R2> InferenceRule<'cfg> for RulePair<R1, R2>
where
    R1: InferenceRule<'cfg>,
    R2: InferenceRule<'cfg>,
{
    fn test_rule(
        &mut self,
        depth: usize,
        max_depth: usize,
        cx: &InferencePredicateContext<'cfg, '_>,
    ) -> Result<Option<usize>, usize> {
        if depth > max_depth {
            return Ok(None);
        }

        if let out @ (Ok(Some(_)) | Err(_)) =
            self.prev_rules
                .test_rule(depth + self.rule.height(), max_depth, cx)
        {
            return out;
        }

        self.rule.test_rule(depth, max_depth, cx)
    }

    fn apply_rule(
        &mut self,
        depth: usize,
        target_depth: usize,
        target: &mut InferenceTarget<'cfg, '_>,
    ) {
        let next_depth = depth + self.rule.height();

        if target_depth < next_depth {
            self.rule.apply_rule(depth, target_depth, target)
        } else {
            self.prev_rules.apply_rule(next_depth, target_depth, target)
        }
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        self.prev_rules.direction_precedence() + self.rule.direction_precedence()
    }

    fn height(&self) -> usize {
        self.height
    }
}

pub struct RuleSet<'cfg, Rules, Pred> {
    rules: Rules,
    pred: InferWhen<Pred>,
    child_inference: Option<ChildInference<'cfg>>,
}

pub trait DefineRules<'cfg> {
    type Rules: InferenceRule<'cfg>;
    type Pred: InferencePredicate;
    fn into_rules(self) -> RuleSet<'cfg, Self::Rules, Self::Pred>;
}

impl<'cfg, Rules, Pred> DefineRules<'cfg> for RuleSet<'cfg, Rules, Pred>
where
    Rules: InferenceRule<'cfg>,
    Pred: InferencePredicate,
{
    type Rules = Rules;
    type Pred = Pred;
    fn into_rules(self) -> RuleSet<'cfg, Self::Rules, Self::Pred> {
        self
    }
}

impl<'cfg, Rules, Pred> InferenceRule<'cfg> for RuleSet<'cfg, Rules, Pred>
where
    Rules: InferenceRule<'cfg>,
    Pred: InferencePredicate,
{
    fn test_rule(
        &mut self,
        depth: usize,
        max_depth: usize,
        cx: &InferencePredicateContext<'cfg, '_>,
    ) -> Result<Option<usize>, usize> {
        match self.pred.test(cx) {
            Ok(true) => {}
            Ok(false) => return Ok(None),
            Err(_) => return Err(depth),
        }

        let out = self.rules.test_rule(depth, max_depth, cx);
        if let Some(ref value) = self.child_inference {
            cx.set_child_inference(|| value.clone());
        }
        out
    }

    fn apply_rule(
        &mut self,
        depth: usize,
        target_depth: usize,
        target: &mut InferenceTarget<'cfg, '_>,
    ) {
        self.rules.apply_rule(depth, target_depth, target)
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        self.pred.direction_precedence() + self.rules.direction_precedence()
    }

    fn height(&self) -> usize {
        self.rules.height()
    }
}

impl<'cfg, Rules, Pred> RuleSet<'cfg, Rules, Pred>
where
    Rules: InferenceRule<'cfg>,
    Pred: InferencePredicate,
{
    pub fn add<R>(self, rule: R) -> RuleSet<'cfg, RulePair<Rules, R>, Pred>
    where
        R: InferenceRule<'cfg>,
    {
        let height = self.rules.height() + rule.height();
        RuleSet {
            rules: RulePair {
                prev_rules: self.rules,
                rule,
                height,
            },
            pred: self.pred,
            child_inference: self.child_inference,
        }
    }

    pub fn when(
        self,
        pred: InferWhen<impl InferencePredicate>,
    ) -> RuleSet<'cfg, Rules, impl InferencePredicate> {
        RuleSet {
            rules: self.rules,
            pred: self.pred & when(pred),
            child_inference: self.child_inference,
        }
    }

    fn child_inference(mut self, value: ChildInference<'cfg>) -> Self {
        self.child_inference = Some(value);
        self
    }

    pub fn inherit_inference(self) -> Self {
        self.child_inference(ChildInference::Continue)
    }

    pub fn revert_inference(self) -> Self {
        self.child_inference(ChildInference::Revert)
    }

    pub fn inference_method<M>(self, method: impl IntoInferenceMethod<'cfg, M> + 'cfg) -> Self {
        self.child_inference(ChildInference::WithMethod(method.into_inference_method()))
    }
}

pub fn rules<'cfg>() -> RuleSet<'cfg, EmptyRule, impl InferencePredicate> {
    RuleSet {
        rules: EmptyRule {},
        pred: when::any(),
        child_inference: default(),
    }
}

pub fn rule<'cfg, P, A>(pred: InferWhen<P>, action: A) -> RuleImpl<P, A>
where
    P: InferencePredicate,
    A: for<'a, 'b> FnMut(&'a mut InferenceTarget<'cfg, 'b>) -> &'a mut InferenceTarget<'cfg, 'b>,
{
    pred.then_apply(action)
}

impl<'cfg, R: ?Sized> InferenceRule<'cfg> for &mut R
where
    R: InferenceRule<'cfg>,
{
    fn test_rule(
        &mut self,
        depth: usize,
        max_depth: usize,
        cx: &InferencePredicateContext<'cfg, '_>,
    ) -> Result<Option<usize>, usize> {
        R::test_rule(self, depth, max_depth, cx)
    }

    fn apply_rule(
        &mut self,
        depth: usize,
        target_depth: usize,
        target: &mut InferenceTarget<'cfg, '_>,
    ) {
        R::apply_rule(self, depth, target_depth, target)
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        R::direction_precedence(self)
    }

    fn height(&self) -> usize {
        R::height(self)
    }
}
