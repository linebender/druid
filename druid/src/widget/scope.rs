use crate::kurbo::{Point, Rect};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, PaintCtx,
    Size, UpdateCtx, Widget, WidgetPod,
};
use std::marker::PhantomData;

/// A policy that controls how a [`Scope`] will interact with its surrounding
/// application data. Specifically, how to create an initial State from the
/// input, and how to synchronise the two using a [`ScopeTransfer`].
///
/// [`Scope`]: struct.Scope.html
/// [`ScopeTransfer`]: trait.ScopeTransfer.html
pub trait ScopePolicy {
    /// The type of data that comes in from the surrounding application or scope.
    type In: Data;
    /// The type of data that the `Scope` will maintain internally.
    /// This will usually be larger than the input data, and will embed the input data.
    type State: Data;
    /// The type of transfer that will be used to synchronise internal and application state
    type Transfer: ScopeTransfer<In = Self::In, State = Self::State>;
    /// Make a new state and transfer from the input.
    ///
    /// This consumes the policy, so non-cloneable items can make their way
    /// into the state this way.
    fn create(self, inner: &Self::In) -> (Self::State, Self::Transfer);
}

/// A `ScopeTransfer` knows how to synchronise input data with its counterpart
/// within a [`Scope`].
///
/// It is separate from the policy mainly to allow easy use of lenses to do
/// synchronisation, with a custom [`ScopePolicy`].
///
/// [`Scope`]: struct.Scope.html
/// [`ScopePolicy`]: trait.ScopePolicy.html
pub trait ScopeTransfer {
    /// The type of data that comes in from the surrounding application or scope.
    type In: Data;
    /// The type of data that the Scope will maintain internally.
    type State: Data;

    /// Replace the input we have within our State with a new one from outside
    fn read_input(&self, state: &mut Self::State, inner: &Self::In);
    /// Take the modifications we have made and write them back
    /// to our input.
    fn write_back_input(&self, state: &Self::State, inner: &mut Self::In);
}

/// A default implementation of [`ScopePolicy`] that takes a function and a transfer.
///
/// [`ScopePolicy`]: trait.ScopePolicy.html
pub struct DefaultScopePolicy<F: FnOnce(Transfer::In) -> Transfer::State, Transfer: ScopeTransfer> {
    make_state: F,
    transfer: Transfer,
}

impl<F: FnOnce(Transfer::In) -> Transfer::State, Transfer: ScopeTransfer>
    DefaultScopePolicy<F, Transfer>
{
    /// Create a `ScopePolicy` from a factory function and a `ScopeTransfer`.
    pub fn new(make_state: F, transfer: Transfer) -> Self {
        DefaultScopePolicy {
            make_state,
            transfer,
        }
    }
}

impl<F: FnOnce(In) -> State, L: Lens<State, In>, In: Data, State: Data>
    DefaultScopePolicy<F, LensScopeTransfer<L, In, State>>
{
    /// Create a `ScopePolicy` from a factory function and a lens onto that
    /// `Scope`'s state.
    pub fn from_lens(make_state: F, lens: L) -> Self {
        Self::new(make_state, LensScopeTransfer::new(lens))
    }
}

impl<F: Fn(Transfer::In) -> Transfer::State, Transfer: ScopeTransfer> ScopePolicy
    for DefaultScopePolicy<F, Transfer>
{
    type In = Transfer::In;
    type State = Transfer::State;
    type Transfer = Transfer;

    fn create(self, inner: &Self::In) -> (Self::State, Self::Transfer) {
        let state = (self.make_state)(inner.clone());
        (state, self.transfer)
    }
}

/// A `ScopeTransfer` that uses a Lens to synchronise between a large internal
/// state and a small input.
pub struct LensScopeTransfer<L: Lens<State, In>, In, State> {
    lens: L,
    phantom_in: PhantomData<In>,
    phantom_state: PhantomData<State>,
}

impl<L: Lens<State, In>, In, State> LensScopeTransfer<L, In, State> {
    /// Create a `ScopeTransfer` from a Lens onto a portion of the `Scope`'s state.
    pub fn new(lens: L) -> Self {
        LensScopeTransfer {
            lens,
            phantom_in: PhantomData::default(),
            phantom_state: PhantomData::default(),
        }
    }
}

impl<L: Lens<State, In>, In: Data, State: Data> ScopeTransfer for LensScopeTransfer<L, In, State> {
    type In = In;
    type State = State;

    fn read_input(&self, state: &mut State, data: &In) {
        self.lens.with_mut(state, |inner| {
            if !inner.same(&data) {
                *inner = data.clone()
            }
        });
    }

    fn write_back_input(&self, state: &State, data: &mut In) {
        self.lens.with(state, |inner| {
            if !inner.same(&data) {
                *data = inner.clone();
            }
        });
    }
}

enum ScopeContent<SP: ScopePolicy> {
    Policy {
        policy: Option<SP>,
    },
    Transfer {
        state: SP::State,
        transfer: SP::Transfer,
    },
}

/// A widget that allows encapsulation of application state.
///
/// This is useful in circumstances where
/// * A (potentially reusable) widget is composed of a tree of multiple cooperating child widgets
/// * Those widgets communicate amongst themselves using Druid's reactive data mechanisms
/// * It is undesirable to complicate the surrounding application state with the internal details
///   of the widget.
///
///
/// Examples include:
/// * In a tabs widget composed of a tab bar, and a widget switching body, those widgets need to
///   cooperate on which tab is selected. However not every user of a tabs widget wishes to
///   encumber their application state with this internal detail - especially as many tabs widgets may
///   reasonably exist in an involved application.
/// * In a table/grid widget composed of various internal widgets, many things need to be synchronised.
///   Scroll position, heading moves, drag operations, sort/filter operations. For many applications
///   access to this internal data outside of the table widget isn't needed.
///   For this reason it may be useful to use a Scope to establish private state.
///
/// A scope embeds some input state (from its surrounding application or parent scope)
/// into a larger piece of internal state. This is controlled by a user provided policy.
///
/// The ScopePolicy needs to do two things
/// a) Create a new scope from the initial value of its input,
/// b) Provide two way synchronisation between the input and the state via a ScopeTransfer
///
/// Convenience methods are provided to make a policy from a function and a lens.
/// It may sometimes be advisable to implement ScopePolicy directly if you need to
/// mention the type of a Scope.
///
/// # Examples
/// ```
/// use druid::{Data, Lens, WidgetExt};
/// use druid::widget::{TextBox, Scope};
/// #[derive(Clone, Data, Lens)]
/// struct AppState {
///     name: String,
/// }
///
/// #[derive(Clone, Data, Lens)]
/// struct PrivateState {
///     text: String,
///     other: u32,
/// }
///
/// impl PrivateState {
///     pub fn new(text: String) -> Self {
///         PrivateState { text, other: 0 }
///     }
/// }
///
/// fn main() {
///     let scope = Scope::from_lens(
///         PrivateState::new,
///         PrivateState::text,
///         TextBox::new().lens(PrivateState::text),
///     );
/// }
/// ```
pub struct Scope<SP: ScopePolicy, W: Widget<SP::State>> {
    content: ScopeContent<SP>,
    inner: WidgetPod<SP::State, W>,
}

impl<SP: ScopePolicy, W: Widget<SP::State>> Scope<SP, W> {
    /// Create a new scope from a policy and an inner widget
    pub fn new(policy: SP, inner: W) -> Self {
        Scope {
            content: ScopeContent::Policy {
                policy: Some(policy),
            },
            inner: WidgetPod::new(inner),
        }
    }

    fn with_state<V>(
        &mut self,
        data: &SP::In,
        mut f: impl FnMut(&mut SP::State, &mut WidgetPod<SP::State, W>) -> V,
    ) -> V {
        match &mut self.content {
            ScopeContent::Policy { policy } => {
                // We know that the policy is a Some - it is an option to allow
                // us to take ownership before replacing the content.
                let (mut state, policy) = policy.take().unwrap().create(data);
                let v = f(&mut state, &mut self.inner);
                self.content = ScopeContent::Transfer {
                    state,
                    transfer: policy,
                };
                v
            }
            ScopeContent::Transfer {
                ref mut state,
                transfer,
            } => {
                transfer.read_input(state, data);
                f(state, &mut self.inner)
            }
        }
    }

    fn write_back_input(&mut self, data: &mut SP::In) {
        if let ScopeContent::Transfer { state, transfer } = &mut self.content {
            transfer.write_back_input(state, data)
        }
    }
}

impl<
        F: Fn(Transfer::In) -> Transfer::State,
        Transfer: ScopeTransfer,
        W: Widget<Transfer::State>,
    > Scope<DefaultScopePolicy<F, Transfer>, W>
{
    /// Create a new policy from a function creating the state, and a ScopeTransfer synchronising it
    pub fn from_function(make_state: F, transfer: Transfer, inner: W) -> Self {
        Self::new(DefaultScopePolicy::new(make_state, transfer), inner)
    }
}

impl<In: Data, State: Data, F: Fn(In) -> State, L: Lens<State, In>, W: Widget<State>>
    Scope<DefaultScopePolicy<F, LensScopeTransfer<L, In, State>>, W>
{
    /// Create a new policy from a function creating the state, and a Lens synchronising it
    pub fn from_lens(make_state: F, lens: L, inner: W) -> Self {
        Self::new(DefaultScopePolicy::from_lens(make_state, lens), inner)
    }
}

impl<SP: ScopePolicy, W: Widget<SP::State>> Widget<SP::In> for Scope<SP, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut SP::In, env: &Env) {
        self.with_state(data, |state, inner| inner.event(ctx, event, state, env));
        self.write_back_input(data);
        ctx.request_update()
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &SP::In, env: &Env) {
        self.with_state(data, |state, inner| inner.lifecycle(ctx, event, state, env));
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &SP::In, data: &SP::In, env: &Env) {
        self.with_state(data, |state, inner| inner.update(ctx, state, env));
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &SP::In,
        env: &Env,
    ) -> Size {
        self.with_state(data, |state, inner| {
            let size = inner.layout(ctx, bc, state, env);
            inner.set_layout_rect(ctx, state, env, Rect::from_origin_size(Point::ORIGIN, size));
            size
        })
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &SP::In, env: &Env) {
        self.with_state(data, |state, inner| inner.paint_raw(ctx, state, env));
    }
}
