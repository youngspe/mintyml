mod rules;
pub mod when;

use core::{cell::Cell, fmt, mem};

use alloc::{borrow::Cow, rc::Rc, string::String, vec::Vec};

use derive_more::Add;
use either::{Either, IntoEither};

use crate::{
    document::{Content, Element, Node, NodeType, Selector, TextSlice},
    utils::default,
};

pub use rules::{rule, rules, DefineRules};
pub use when::{when, InferWhen};

use self::rules::InferenceRule;

use super::definitions::StandardInfer;

pub trait Infer<'cfg>: fmt::Debug {
    fn define_rules(&self) -> impl DefineRules<'cfg>;
}

trait DynInfer<'cfg>: fmt::Debug {
    fn infer(&self, inferrer: &mut Inferrer<'cfg, '_, '_>, method: &InferenceMethod<'cfg>);
}

impl<'cfg, I: Infer<'cfg>> DynInfer<'cfg> for I {
    fn infer(&self, inferrer: &mut Inferrer<'cfg, '_, '_>, method: &InferenceMethod<'cfg>) {
        let mut rules = self.define_rules().into_rules();
        inferrer.use_rule(&mut rules, method);
    }
}

pub trait IntoInferenceMethod<'cfg, M = Self> {
    fn into_inference_method(self) -> InferenceMethod<'cfg>
    where
        Self: 'cfg;
}

impl<'cfg> IntoInferenceMethod<'cfg> for InferenceMethod<'cfg> {
    fn into_inference_method(self) -> InferenceMethod<'cfg>
    where
        Self: 'cfg,
    {
        self
    }
}

impl<'cfg, T: Infer<'cfg>> IntoInferenceMethod<'cfg> for Rc<T> {
    fn into_inference_method(self) -> InferenceMethod<'cfg>
    where
        Self: 'cfg,
    {
        InferenceMethod {
            inner: InferenceMethodInner::Rc(self),
        }
    }
}

impl<'cfg, T: Infer<'cfg>> IntoInferenceMethod<'cfg, (T,)> for T {
    fn into_inference_method(self) -> InferenceMethod<'cfg>
    where
        Self: 'cfg,
    {
        InferenceMethod {
            inner: InferenceMethodInner::Rc(Rc::new(self)),
        }
    }
}

impl<'cfg, T: Infer<'cfg>> IntoInferenceMethod<'cfg> for &'cfg T {
    fn into_inference_method(self) -> InferenceMethod<'cfg> {
        InferenceMethod {
            inner: InferenceMethodInner::Ref(self),
        }
    }
}

#[derive(Default, Debug, Clone)]
enum InferenceMethodInner<'cfg> {
    #[default]
    Inherit,
    Rc(Rc<dyn DynInfer<'cfg> + 'cfg>),
    Ref(&'cfg (dyn DynInfer<'cfg> + 'cfg)),
}

#[derive(Default, Debug, Clone)]
pub struct InferenceMethod<'cfg> {
    inner: InferenceMethodInner<'cfg>,
}

impl<'cfg> InferenceMethod<'cfg> {
    fn get_inner(&self) -> Option<&dyn DynInfer<'cfg>> {
        match self.inner {
            InferenceMethodInner::Inherit => None,
            InferenceMethodInner::Rc(ref inner) => Some(&**inner),
            InferenceMethodInner::Ref(inner) => Some(inner),
        }
    }
}

#[derive(Default, Clone)]
pub(crate) enum ChildInference<'cfg> {
    #[default]
    Revert,
    Continue,
    WithMethod(InferenceMethod<'cfg>),
}

#[non_exhaustive]
pub struct InferenceTarget<'cfg, 'infer> {
    src: &'cfg str,
    element: &'infer mut Element<'cfg>,
    child_inference: &'infer mut ChildInference<'cfg>,
    needs_inference: bool,
}

pub trait IntoTags<'cfg, M = ()> {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>>;
}

impl<'cfg, It: IntoIterator, M> IntoTags<'cfg, [M; 1]> for It
where
    It::Item: IntoTags<'cfg, M>,
{
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> {
        self.into_iter().flat_map(IntoTags::tags)
    }
}

impl<'cfg> IntoTags<'cfg> for TextSlice<'cfg> {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> {
        core::iter::once(self)
    }
}

impl<'cfg> IntoTags<'cfg> for Cow<'cfg, str> {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> {
        core::iter::once(self.into())
    }
}

impl<'cfg> IntoTags<'cfg> for String {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> {
        core::iter::once(self.into())
    }
}

impl<'cfg> IntoTags<'cfg> for &'cfg str {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> {
        core::iter::once(self.into())
    }
}

pub struct InferencePredicateContext<'cfg, 'infer> {
    src: &'cfg str,
    nodes: &'infer [Node<'cfg>],
    parent: Option<&'infer InferencePredicateContext<'cfg, 'infer>>,
    index: usize,
    child_inference: &'infer Cell<Option<ChildInference<'cfg>>>,
}

impl<'cfg, 'infer> InferencePredicateContext<'cfg, 'infer> {
    fn match_this_node<T: Default, U: Into<Option<T>>>(
        &self,
        f: impl FnOnce(&'infer Node<'cfg>) -> TestResult<U>,
    ) -> TestResult<T> {
        Ok(self
            .nodes
            .get(self.index)
            .map(f)
            .transpose()?
            .and_then(Into::into)
            .unwrap_or_default())
    }

    fn match_this_element<T: Default, U: Into<Option<T>>>(
        &self,
        f: impl FnOnce(&'infer Element<'cfg>) -> TestResult<U>,
    ) -> TestResult<T> {
        self.match_this_node(|n| match n {
            Node {
                node_type: NodeType::Element { value },
                ..
            } => f(value).map(Into::into),
            _ => Ok(None),
        })
    }

    fn set_child_inference(&self, f: impl FnOnce() -> ChildInference<'cfg>) -> bool {
        let old = self.child_inference.take();

        if old.is_some() {
            self.child_inference.set(old);
            return false;
        }

        self.child_inference.set(Some(f()));
        true
    }
}

#[derive(Default, Clone, Copy, Add)]
pub struct DirectionPrecedence {
    adjacent: i16,
    any: i16,
}

impl DirectionPrecedence {
    pub fn should_reverse(self) -> bool {
        matches!(
            self,
            Self { adjacent: 1.., .. }
                | Self {
                    adjacent: 0,
                    any: 1..
                }
        )
    }
}

pub trait InferencePredicate {
    fn test(&mut self, cx: &InferencePredicateContext) -> TestResult;
    fn direction_precedence(&self) -> DirectionPrecedence {
        default()
    }
}

impl<'cfg, 'infer> InferenceTarget<'cfg, 'infer> {
    pub fn tag<M>(&mut self, tag: impl IntoTags<'cfg, M>) -> &mut Self {
        if self.needs_inference {
            self.needs_inference = false;
            let mut tags = tag.tags();

            let Some(first) = tags.next() else {
                if !self.element.selectors.is_empty() {
                    self.element.selectors.remove(0);
                }
                return self;
            };

            let selector_location;

            if let Some(selector) = self.element.selectors.first_mut() {
                selector_location = selector.range.end;
                selector.tag = first.into();
            } else {
                selector_location = self.element.range.start;
                self.element
                    .selectors
                    .push(Selector::empty(selector_location).with_tag(first))
            }

            self.element.selectors.splice(
                1..1,
                tags.map(|t| Selector::empty(selector_location).with_tag(t)),
            );
        }
        self
    }

    pub fn dissolve(&mut self) -> &mut Self {
        self.tag(core::iter::empty::<TextSlice>())
    }

    pub fn inherit_inference(&mut self) -> &mut Self {
        *self.child_inference = ChildInference::Continue;
        self
    }

    pub fn inference_method<M>(
        &mut self,
        method: impl IntoInferenceMethod<'cfg, M> + 'cfg,
    ) -> &mut Self {
        *self.child_inference = ChildInference::WithMethod(method.into_inference_method());
        self
    }
}

#[non_exhaustive]
#[derive(Default)]
pub struct Incomplete {}
type TestResult<T = bool> = Result<T, Incomplete>;

struct InferenceState<'cfg> {
    result: Result<Option<usize>, usize>,
    child_inference: Option<ChildInference<'cfg>>,
}

struct Inferrer<'cfg, 'infer, 'base> {
    src: &'cfg str,
    nodes: &'infer mut [Node<'cfg>],
    parent_context: Option<&'infer InferencePredicateContext<'cfg, 'infer>>,
    states: &'infer mut Vec<InferenceState<'cfg>>,
    base_rule: &'base mut (dyn InferenceRule<'cfg>),
    base_inference_method: &'base InferenceMethod<'cfg>,
}

fn fallback_rule<'cfg>(
    rule: impl InferenceRule<'cfg>,
    base: impl InferenceRule<'cfg>,
) -> impl InferenceRule<'cfg> {
    rules().add(rule).add(base)
}

impl<'cfg, 'infer, 'base> Inferrer<'cfg, 'infer, 'base> {
    fn test_node<'this>(&mut self, rule: &mut impl InferenceRule<'cfg>, index: usize) -> bool
    where
        'this: 'base,
    {
        let InferenceState {
            result: Err(max_depth),
            ref mut child_inference,
        } = self.states[index]
        else {
            // already resolved
            return false;
        };

        if !matches!(self.nodes[index].node_type, NodeType::Element { .. }) {
            self.states[index].result = Ok(None);
            return true;
        }

        let result = fallback_rule(rule, &mut *self.base_rule).test_rule(
            0,
            max_depth,
            &InferencePredicateContext {
                src: self.src,
                nodes: &*self.nodes,
                parent: self.parent_context,
                index,
                child_inference: Cell::from_mut(child_inference),
            },
        );

        self.states[index].result = result;
        result.is_ok()
    }

    fn test_nodes(&mut self, rule: &mut impl InferenceRule<'cfg>) {
        let mut reverse = rule.direction_precedence().should_reverse();
        let mut total_resolved = 0;

        loop {
            let indices = 0..self.nodes.len();

            let resolved_count = indices
                .into_either_with(|_| reverse)
                .map_left(Iterator::rev)
                .map(|i| self.test_node(rule, i) as usize)
                .sum::<usize>();

            total_resolved += resolved_count;

            if total_resolved == self.nodes.len() || resolved_count == 0 {
                // Either we're done or hit a fixed point
                break;
            }

            reverse = !reverse;
        }
    }

    fn apply_rule_to_nodes(&mut self, rule: &mut impl InferenceRule<'cfg>) {
        for i in 0..self.nodes.len() {
            let (
                &mut InferenceState {
                    result: Ok(Some(depth)),
                    ref mut child_inference,
                },
                &mut Node {
                    node_type:
                        NodeType::Element {
                            value: ref mut element,
                            ..
                        },
                    ..
                },
            ) = (&mut self.states[i], &mut self.nodes[i])
            else {
                continue;
            };

            let mut child_inference = child_inference.take().unwrap_or(ChildInference::Revert);
            let needs_inference = element
                .selectors
                .first()
                .map(|s| s.uninferred())
                .unwrap_or(true);

            let mut inference_target = InferenceTarget {
                src: self.src,
                element,
                needs_inference,
                child_inference: &mut child_inference,
            };

            fallback_rule(&mut *rule, &mut *self.base_rule).apply_rule(
                0,
                depth,
                &mut inference_target,
            );

            element.inference_method = child_inference;
        }
    }

    pub fn use_rule(
        &mut self,
        rule: &mut impl InferenceRule<'cfg>,
        method: &InferenceMethod<'cfg>,
    ) {
        self.states
            .resize_with(self.nodes.len(), || InferenceState {
                result: Err(usize::MAX),
                child_inference: default(),
            });

        self.test_nodes(rule);
        self.apply_rule_to_nodes(rule);
        self.states.clear();

        for i in 0..self.nodes.len() {
            self.inference_level(rule, method, i);
        }
    }

    fn inference_level(
        &mut self,
        rule: &mut impl InferenceRule<'cfg>,
        parent_method: &InferenceMethod<'cfg>,
        index: usize,
    ) {
        let child_inference;
        let mut nodes = {
            let Some(element) = self.nodes[index].as_element_mut() else {
                return;
            };
            child_inference = mem::take(&mut element.inference_method);
            element.split_uninferred();
            mem::take(&mut element.content.nodes)
        };

        {
            let mut continue_inference = false;

            let method = match child_inference {
                ChildInference::Revert => self.base_inference_method,
                ChildInference::Continue => {
                    continue_inference = true;
                    parent_method
                }
                ChildInference::WithMethod(ref m) => m,
            };

            let dyn_infer = method
                .get_inner()
                .or_else(|| self.base_inference_method.get_inner())
                .unwrap_or(&StandardInfer {});

            let mut base_rule = if continue_inference {
                Either::Left(fallback_rule(rule, &mut *self.base_rule))
            } else {
                Either::Right(&mut *self.base_rule)
            };

            dyn_infer.infer(
                &mut Inferrer {
                    src: self.src,
                    nodes: &mut nodes,
                    parent_context: Some(&InferencePredicateContext {
                        src: self.src,
                        nodes: &*self.nodes,
                        parent: self.parent_context,
                        index,
                        child_inference: &default(),
                    }),
                    states: self.states,
                    base_rule: match base_rule {
                        Either::Left(ref mut rule) => rule,
                        Either::Right(rule) => rule,
                    },
                    base_inference_method: if continue_inference {
                        parent_method
                    } else {
                        self.base_inference_method
                    },
                },
                method,
            )
        }

        {
            let Some(element) = self.nodes[index].as_element_mut() else {
                return;
            };
            element.content.nodes = nodes;
        }
    }
}

pub fn infer<'cfg>(src: &'cfg str, content: &mut Content<'cfg>) {
    let method = (&StandardInfer {}).into_inference_method();
    StandardInfer {}.infer(
        &mut Inferrer {
            src,
            nodes: &mut content.nodes,
            parent_context: None,
            states: &mut default(),
            base_rule: &mut StandardInfer {}.define_rules().into_rules(),
            base_inference_method: &method,
        },
        &method,
    );
}
