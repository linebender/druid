use crate::kurbo::{Point, Rect};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, PaintCtx,
    Size, UpdateCtx, Widget, WidgetPod,
};
use std::marker::PhantomData;

pub trait ScopePolicy {
    type In: Data;
    type State: Data;
    type Transfer: ScopeTransfer<In = Self::In, State = Self::State>;
    // Make a new state and transfer from the input.
    // This consumes the policy, so non cloneable items can make their way into the state this way.
    fn create(self, inner: &Self::In) -> (Self::State, Self::Transfer);
}

pub trait ScopeTransfer {
    type In: Data;
    type State: Data;

    // Replace the input we have with a new one from outside
    fn read_input(&self, state: &mut Self::State, inner: &Self::In);
    // Take the modifications we have made and write them back
    // to our input.
    fn write_back_input(&self, state: &Self::State, inner: &mut Self::In);
}

pub struct DefaultScopePolicy<F: Fn(Transfer::In) -> Transfer::State, Transfer: ScopeTransfer> {
    make_state: F,
    transfer: Transfer,
}

impl<F: Fn(Transfer::In) -> Transfer::State, Transfer: ScopeTransfer>
    DefaultScopePolicy<F, Transfer>
{
    pub fn new(make_state: F, transfer: Transfer) -> Self {
        DefaultScopePolicy {
            make_state,
            transfer,
        }
    }
}

impl<F: Fn(In) -> State, L: Lens<State, In>, In: Data, State: Data>
    DefaultScopePolicy<F, LensScopeTransfer<L, In, State>>
{
    pub fn for_lens(make_state: F, lens: L) -> Self {
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

pub struct LensScopeTransfer<L: Lens<State, In>, In, State> {
    lens: L,
    phantom_in: PhantomData<In>,
    phantom_state: PhantomData<State>,
}

impl<L: Lens<State, In>, In, State> LensScopeTransfer<L, In, State> {
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

pub struct Scope<SP: ScopePolicy, W: Widget<SP::State>> {
    content: ScopeContent<SP>,
    inner: WidgetPod<SP::State, W>,
    widget_added: bool,
}

impl<SP: ScopePolicy, W: Widget<SP::State>> Scope<SP, W> {
    pub fn new(policy: SP, inner: W) -> Self {
        Scope {
            content: ScopeContent::Policy {
                policy: Some(policy),
            },
            inner: WidgetPod::new(inner),
            widget_added: false,
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
                transfer: policy,
            } => {
                policy.read_input(state, data);
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

impl<SP: ScopePolicy, W: Widget<SP::State>> Widget<SP::In> for Scope<SP, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut SP::In, env: &Env) {
        if self.widget_added {
            self.with_state(data, |state, inner| inner.event(ctx, event, state, env));
            self.write_back_input(data);
            ctx.request_update()
        } else {
            log::warn!("Scope dropping event (widget not added) {:?}", event);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &SP::In, env: &Env) {
        self.with_state(data, |state, inner| inner.lifecycle(ctx, event, state, env));
        if let LifeCycle::WidgetAdded = event {
            self.widget_added = true;
        }
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
