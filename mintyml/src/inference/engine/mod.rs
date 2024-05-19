pub mod when;

use core::{
    borrow::{Borrow, BorrowMut},
    fmt, mem,
};

use alloc::{borrow::Cow, rc::Rc, string::String, vec::Vec};

use derive_more::Add;
use either::IntoEither;

use crate::{
    document::{Content, Element, Node, NodeType, Selector, TextSlice},
    utils::default,
};

use super::definitions::StandardInfer;

mod sealed {
    use super::*;
    pub trait InferTest<'cfg, T: ?Sized> {
        fn test(
            &mut self,
            cx: &InferencePredicateContext<'cfg, '_>,
            index: usize,
            start_index: usize,
        ) -> Result<Option<&mut T>, usize>;

        fn direction_precedence(&self) -> DirectionPrecedence;
    }
}
use sealed::InferTest;

pub trait InferenceDefinition<'cfg, T: ?Sized>: InferTest<'cfg, T> {
    type Append<Next: InferenceDefinition<'cfg, T>>: InferenceDefinition<'cfg, T>;
    fn append<Next: InferenceDefinition<'cfg, T>>(self, other: Next) -> Self::Append<Next>
    where
        Self: Sized;
}

struct TagFn<F>(F);

impl<'cfg, F: FnMut(&mut Element<'cfg>) + 'cfg> Borrow<TagDefinitionItem<'cfg>> for TagFn<F> {
    fn borrow(&self) -> &TagDefinitionItem<'cfg> {
        &self.0
    }
}

impl<'cfg, F: FnMut(&mut Element<'cfg>) + 'cfg> BorrowMut<TagDefinitionItem<'cfg>> for TagFn<F> {
    fn borrow_mut(&mut self) -> &mut TagDefinitionItem<'cfg> {
        &mut self.0
    }
}

pub struct InferenceDefinitionPair<Pred, Next, T> {
    pred: Pred,
    value: T,
    next: Next,
}

type TagDefinitionItem<'cfg> = dyn FnMut(&mut Element<'cfg>) + 'cfg;

pub trait TagDefinition<'cfg, _Bound = &'cfg Self>:
    'cfg + InferenceDefinition<'cfg, TagDefinitionItem<'cfg>> + Sized
{
    fn apply<Next: TagDefinition<'cfg>>(self, other: Next) -> Self::Append<Next> {
        self.append(other)
    }
    fn apply_from(self, other: &impl Infer<'cfg>) -> Self::Append<impl TagDefinition<'cfg>> {
        self.apply(other.define_tags())
    }

    fn when<M>(
        self,
        pred: impl InferencePredicate,
        value: impl IntoTags<'cfg, M>,
    ) -> Self::Append<impl TagDefinition<'cfg>> {
        let tags = value.tags();
        self.append(InferenceDefinitionPair {
            pred,
            value: TagFn(move |element: &mut Element<'cfg>| element.apply_tags(tags.clone())),
            next: EmptyInferenceDefinition {},
        })
    }
    fn default<M>(self, value: impl IntoTags<'cfg, M>) -> Self::Append<impl TagDefinition<'cfg>> {
        self.when(when::any(), value)
    }
}
pub trait MethodDefinition<'cfg, _Bound = &'cfg Self>:
    'cfg + InferenceDefinition<'cfg, InferenceMethod<'cfg>> + Sized
{
    fn apply<Next: MethodDefinition<'cfg>>(self, other: Next) -> Self::Append<Next> {
        self.append(other)
    }

    fn apply_from(self, other: &impl Infer<'cfg>) -> Self::Append<impl MethodDefinition<'cfg>> {
        self.apply(other.define_methods())
    }

    fn when<Pred: InferencePredicate, M>(
        self,
        pred: Pred,
        value: impl IntoInferenceMethod<'cfg, M>,
    ) -> Self::Append<InferenceDefinitionPair<Pred, EmptyInferenceDefinition, InferenceMethod<'cfg>>>
    {
        self.append(InferenceDefinitionPair {
            pred,
            value: value.into_inference_method(),
            next: EmptyInferenceDefinition {},
        })
    }

    fn default<M>(self, value: impl IntoInferenceMethod<'cfg, M>) -> impl MethodDefinition<'cfg> {
        self.when(when::any(), value)
    }
}

impl<'cfg, D> TagDefinition<'cfg> for D where D: InferenceDefinition<'cfg, TagDefinitionItem<'cfg>> {}

impl<'cfg, D> MethodDefinition<'cfg> for D where D: InferenceDefinition<'cfg, InferenceMethod<'cfg>> {}

#[non_exhaustive]
pub struct EmptyInferenceDefinition {}

pub fn define_tags<'cfg>() -> impl TagDefinition<'cfg> {
    EmptyInferenceDefinition {}
}

pub fn define_methods<'cfg>() -> impl MethodDefinition<'cfg> {
    EmptyInferenceDefinition {}
}

impl<'cfg, T: ?Sized> InferTest<'cfg, T> for EmptyInferenceDefinition {
    fn test(
        &mut self,
        _: &InferencePredicateContext<'cfg, '_>,
        _: usize,
        _: usize,
    ) -> Result<Option<&mut T>, usize> {
        Ok(None)
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        default()
    }
}

impl<'cfg, T: ?Sized> InferenceDefinition<'cfg, T> for EmptyInferenceDefinition {
    type Append<Next: InferenceDefinition<'cfg, T>> = Next;
    fn append<Next>(self, other: Next) -> Next {
        other
    }
}

impl<'cfg, Pred, Next, T, B> InferTest<'cfg, T> for InferenceDefinitionPair<Pred, Next, B>
where
    T: ?Sized,
    B: BorrowMut<T>,
    Pred: InferencePredicate,
    Next: InferenceDefinition<'cfg, T>,
{
    fn test(
        &mut self,
        cx: &InferencePredicateContext<'cfg, '_>,
        index: usize,
        start_index: usize,
    ) -> Result<Option<&mut T>, usize> {
        if index >= start_index {
            if self.pred.test(cx).map_err(|_| index)? {
                return Ok(Some(self.value.borrow_mut()));
            }
        }

        self.next.test(cx, index + 1, start_index)
    }

    fn direction_precedence(&self) -> DirectionPrecedence {
        self.pred.direction_precedence() + self.next.direction_precedence()
    }
}

impl<'cfg, Pred, Next, T, B> InferenceDefinition<'cfg, T> for InferenceDefinitionPair<Pred, Next, B>
where
    T: ?Sized,
    B: BorrowMut<T>,
    Pred: InferencePredicate,
    Next: InferenceDefinition<'cfg, T>,
{
    type Append<Next2: InferenceDefinition<'cfg, T>> =
        InferenceDefinitionPair<Pred, Next::Append<Next2>, B>;

    fn append<Next2: InferenceDefinition<'cfg, T>>(
        self,
        other: Next2,
    ) -> InferenceDefinitionPair<Pred, Next::Append<Next2>, B> {
        let Self { pred, value, next } = self;
        InferenceDefinitionPair {
            pred,
            value,
            next: next.append(other),
        }
    }
}

pub trait Infer<'cfg, _Bound = &'cfg Self>: 'cfg + fmt::Debug {
    fn define_tags(&self) -> impl TagDefinition<'cfg>;
    fn define_methods(&self) -> impl MethodDefinition<'cfg> {
        StandardInfer {}.define_methods()
    }
}

trait DynInfer<'cfg>: fmt::Debug {
    fn infer(&self, inferrer: &mut Inferrer<'cfg, '_>);
}

impl<'cfg, I: Infer<'cfg>> DynInfer<'cfg> for I {
    fn infer(&self, inferrer: &mut Inferrer<'cfg, '_>) {
        inferrer.use_method(self.define_tags(), self.define_methods());
    }
}

pub trait IntoInferenceMethod<'cfg, M = Self> {
    fn into_inference_method(self) -> InferenceMethod<'cfg>;
}

impl<'cfg> IntoInferenceMethod<'cfg> for InferenceMethod<'cfg> {
    fn into_inference_method(self) -> InferenceMethod<'cfg> {
        self
    }
}

impl<'cfg, T: Infer<'cfg> + 'cfg> IntoInferenceMethod<'cfg> for Rc<T> {
    fn into_inference_method(self) -> InferenceMethod<'cfg> {
        InferenceMethod {
            inner: InferenceMethodInner::Rc(self),
        }
    }
}

impl<'cfg, T: Infer<'cfg> + 'cfg> IntoInferenceMethod<'cfg, (T,)> for T {
    fn into_inference_method(self) -> InferenceMethod<'cfg> {
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

#[derive(Debug, Clone)]
enum InferenceMethodInner<'cfg> {
    Rc(Rc<dyn DynInfer<'cfg> + 'cfg>),
    Ref(&'cfg (dyn DynInfer<'cfg> + 'cfg)),
}

#[derive(Debug, Clone)]
pub struct InferenceMethod<'cfg> {
    inner: InferenceMethodInner<'cfg>,
}

impl<'cfg, I: Infer<'cfg>> From<&'cfg I> for InferenceMethod<'cfg> {
    fn from(value: &'cfg I) -> Self {
        InferenceMethod {
            inner: InferenceMethodInner::Ref(value),
        }
    }
}

impl<'cfg> Default for InferenceMethod<'cfg> {
    fn default() -> Self {
        Self {
            inner: InferenceMethodInner::Ref(&StandardInfer {}),
        }
    }
}

impl<'cfg> Default for &InferenceMethod<'cfg> {
    fn default() -> Self {
        &InferenceMethod {
            inner: InferenceMethodInner::Ref(&StandardInfer {}),
        }
    }
}

impl<'cfg> InferenceMethod<'cfg> {
    fn new<M>(value: impl IntoInferenceMethod<'cfg, M> + 'cfg) -> Self {
        value.into_inference_method()
    }
    fn get_inner(&self) -> &dyn DynInfer<'cfg> {
        match self.inner {
            InferenceMethodInner::Rc(ref inner) => &**inner,
            InferenceMethodInner::Ref(inner) => inner,
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

pub trait IntoTags<'cfg, M = (), _Bound = &'cfg Self>: 'cfg {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg;
}

impl<'cfg, It: IntoIterator + 'cfg, M> IntoTags<'cfg, &'cfg [M]> for It
where
    It::Item: IntoTags<'cfg, M>,
    It::IntoIter: Clone + 'cfg,
{
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        self.into_iter().flat_map(IntoTags::tags)
    }
}

impl<'cfg> IntoTags<'cfg> for TextSlice<'cfg> {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self)
    }
}

impl<'cfg> IntoTags<'cfg> for Cow<'cfg, str> {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self.into())
    }
}

impl<'cfg> IntoTags<'cfg> for String {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self.into())
    }
}

impl<'cfg> IntoTags<'cfg> for &'cfg str {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self.into())
    }
}

pub struct InferencePredicateContext<'cfg, 'infer> {
    src: &'cfg str,
    nodes: &'infer [Node<'cfg>],
    parent: Option<&'infer InferencePredicateContext<'cfg, 'infer>>,
    index: usize,
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

struct InferenceState {
    result: Result<(), usize>,
}

struct Inferrer<'cfg, 'infer> {
    src: &'cfg str,
    nodes: &'infer mut [Node<'cfg>],
    parent_context: Option<&'infer InferencePredicateContext<'cfg, 'infer>>,
    states: &'infer mut Vec<InferenceState>,
}

impl<'cfg, 'infer> Inferrer<'cfg, 'infer> {
    fn infer_tag_node(&mut self, define_tags: &mut impl TagDefinition<'cfg>, index: usize) -> bool {
        let InferenceState {
            result: Err(start_index),
        } = self.states[index]
        else {
            // already resolved
            return false;
        };

        {
            // Skip if this element already has a tag
            let tag = self.nodes[index]
                .as_element()
                .and_then(|e| e.selectors.first())
                .and_then(|s| s.tag.name());

            if tag.is_some() {
                self.states[index].result = Ok(());
                return true;
            };
        }

        let result = match define_tags.test(
            &InferencePredicateContext {
                src: self.src,
                nodes: &*self.nodes,
                parent: self.parent_context,
                index,
            },
            0,
            start_index,
        ) {
            Ok(Some(tag_fn)) => {
                if let Some(element) = self.nodes[index].as_element_mut() {
                    tag_fn(element);
                }
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(i) => Err(i),
        };

        self.states[index].result = result;
        result.is_ok()
    }

    fn infer_tag_nodes(&mut self, mut define_tags: impl TagDefinition<'cfg>) {
        let mut reverse = define_tags.direction_precedence().should_reverse();
        let mut total_resolved = 0;

        loop {
            let indices = 0..self.nodes.len();

            let resolved_count = indices
                .into_either_with(|_| reverse)
                .map_left(Iterator::rev)
                .map(|i| self.infer_tag_node(&mut define_tags, i) as usize)
                .sum::<usize>();

            total_resolved += resolved_count;

            if total_resolved == self.nodes.len() || resolved_count == 0 {
                // Either we're done or hit a fixed point
                break;
            }

            reverse = !reverse;
        }
    }

    pub fn use_method(
        &mut self,
        define_tags: impl TagDefinition<'cfg>,
        mut define_methods: impl MethodDefinition<'cfg>,
    ) {
        self.states
            .resize_with(self.nodes.len(), || InferenceState { result: Err(0) });

        self.infer_tag_nodes(define_tags);
        self.states.clear();

        for i in 0..self.nodes.len() {
            self.inference_level(&mut define_methods, i);
        }
    }

    fn inference_level(&mut self, define_methods: &mut impl MethodDefinition<'cfg>, index: usize) {
        let mut nodes = {
            let Some(element) = self.nodes[index].as_element_mut() else {
                return;
            };
            element.split_uninferred();
            mem::take(&mut element.content.nodes)
        };

        let predicate_context = InferencePredicateContext {
            src: self.src,
            nodes: &*self.nodes,
            parent: self.parent_context,
            index,
        };
        let method = {
            let mut index = 0;
            loop {
                match define_methods.test(&predicate_context, 0, index) {
                    Ok(method) => break method,
                    Err(i) => {
                        index = i + 1;
                    }
                }
            }
        }
        .map(|m| &*m)
        .unwrap_or_default();

        {
            method.get_inner().infer(&mut Inferrer {
                src: self.src,
                nodes: &mut nodes,
                parent_context: Some(&predicate_context),
                states: self.states,
            });
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
    StandardInfer {}.infer(&mut Inferrer {
        src,
        nodes: &mut content.nodes,
        parent_context: None,
        states: &mut default(),
    });
}
