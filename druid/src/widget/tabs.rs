// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A widget that can switch between one of many views, hiding the inactive ones.
//!

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::kurbo::Line;
use crate::piet::RenderContext;

use crate::widget::{Axis, CrossAxisAlignment, Flex, Label, LensScopeTransfer, Scope, ScopePolicy};
use crate::{
    theme, Affine, BoxConstraints, Data, Env, Event, EventCtx, Insets, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, PaintCtx, Point, Rect, SingleUse, Size, UpdateCtx, Widget, WidgetExt, WidgetPod,
};
use instant::Duration;

type TabsScope<TP> = Scope<TabsScopePolicy<TP>, Box<dyn Widget<TabsState<TP>>>>;
type TabBodyPod<TP> = WidgetPod<<TP as TabsPolicy>::Input, <TP as TabsPolicy>::BodyWidget>;
type TabBarPod<TP> = WidgetPod<TabsState<TP>, Box<dyn Widget<TabsState<TP>>>>;
type TabIndex = usize;
type Nanos = u64; // TODO: Make Duration Data?

pub struct TabInfo {
    pub name: String,
    pub can_close: bool,
}

impl TabInfo {
    pub fn new(name: String, can_close: bool) -> Self {
        TabInfo { name, can_close }
    }
}

/// A policy that determines how a Tabs instance derives its tabs from its app data
pub trait TabsPolicy: Data {
    /// The identity of a tab.
    type Key: Hash + Eq + Clone + Debug;

    /// The input data that will a) be used to derive the tab and b) also be the input data of all the child widgets.
    type Input: Data;

    /// The common type for all body widgets in this set of tabs.
    /// A flexible default is Box<dyn Widget<Self::Input>>
    type BodyWidget: Widget<Self::Input>;

    /// The common type for all label widgets in this set of tabs
    /// Usually this would be Label<Self::Input>
    type LabelWidget: Widget<Self::Input>;

    /// This policy whilst it is being built.
    /// This is only be useful for implementations supporting AddTab, such as StaticTabs.
    /// It can be filled in with () by other implementations until associated type defaults are stable
    type Build;

    /// Have the tabs changed. Expected to be cheap, eg pointer or numeric comparison.
    fn tabs_changed(&self, old_data: &Self::Input, data: &Self::Input) -> bool;

    /// What are the current tabs set in order.
    fn tabs(&self, data: &Self::Input) -> Vec<Self::Key>;

    /// Presentation information for the tab
    fn tab_info(&self, key: Self::Key, data: &Self::Input) -> TabInfo;

    /// Body widget for the tab
    fn tab_body(&self, key: Self::Key, data: &Self::Input) -> Option<Self::BodyWidget>;

    /// Label widget for the tab.
    /// Usually implemented with a call to default_make_label ( can't default here because Self::LabelWidget isn't determined)
    fn tab_label(&self, key: Self::Key, info: &TabInfo, data: &Self::Input) -> Self::LabelWidget;

    /// Change the data to reflect the user requesting to close a tab.
    #[allow(unused_variables)]
    fn close_tab(&self, key: Self::Key, data: &mut Self::Input) {}

    #[allow(unused_variables)]
    /// Construct an instance of this TabsFromData from its Build type.
    /// This should only be implemented if supporting AddTab - possibly only StaticTabs needs to.
    fn build(build: Self::Build) -> Self {
        unimplemented!()
    }

    fn default_make_label(info: &TabInfo) -> Label<Self::Input> {
        Label::new(info.name.clone()).with_text_color(theme::FOREGROUND_LIGHT)
    }
}

#[derive(Clone)]
pub struct StaticTabs<T> {
    // This needs be able to avoid cloning the widgets we are given -
    // as such it is Rc
    tabs: Rc<Vec<InitialTab<T>>>,
}

impl<T> StaticTabs<T> {
    pub fn new() -> Self {
        StaticTabs {
            tabs: Rc::new(Vec::new()),
        }
    }
}

impl<T> Default for StaticTabs<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Data> Data for StaticTabs<T> {
    fn same(&self, _other: &Self) -> bool {
        // Changing the tabs after construction shouldn't be possible for static tabs
        // It seems pointless to compare them
        true
    }
}

impl<T: Data> TabsPolicy for StaticTabs<T> {
    type Key = usize;
    type Input = T;
    type BodyWidget = Box<dyn Widget<T>>;
    type LabelWidget = Label<T>;
    type Build = Vec<InitialTab<T>>;

    fn tabs_changed(&self, _old_data: &T, _data: &T) -> bool {
        false
    }

    fn tabs(&self, _data: &T) -> Vec<Self::Key> {
        (0..self.tabs.len()).collect()
    }

    fn tab_info(&self, key: Self::Key, _data: &T) -> TabInfo {
        TabInfo::new(self.tabs[key].name.clone(), false)
    }

    fn tab_body(&self, key: Self::Key, _data: &T) -> Option<Self::BodyWidget> {
        self.tabs[key].child.take()
    }

    fn tab_label(&self, _key: Self::Key, info: &TabInfo, _data: &Self::Input) -> Self::LabelWidget {
        Self::default_make_label(info)
    }

    fn build(build: Self::Build) -> Self {
        StaticTabs {
            tabs: Rc::new(build),
        }
    }
}

pub trait AddTab: TabsPolicy {
    fn add_tab(
        tabs: &mut Self::Build,
        name: impl Into<String>,
        child: impl Widget<Self::Input> + 'static,
    );
}

impl<T: Data> AddTab for StaticTabs<T> {
    fn add_tab(tabs: &mut Self::Build, name: impl Into<String>, child: impl Widget<T> + 'static) {
        tabs.push(InitialTab::new(name, child))
    }
}

#[derive(Clone, Lens, Data)]
pub struct TabsState<TP: TabsPolicy> {
    pub inner: TP::Input,
    pub selected: TabIndex,
    pub policy: TP,
}

impl<TP: TabsPolicy> TabsState<TP> {
    pub fn new(inner: TP::Input, selected: usize, policy: TP) -> Self {
        TabsState {
            inner,
            selected,
            policy,
        }
    }
}

pub struct TabBar<TP: TabsPolicy> {
    axis: Axis,
    cross: CrossAxisAlignment,
    orientation: TabsOrientation,
    tabs: Vec<(TP::Key, TabBarPod<TP>)>,
    hot: Option<TabIndex>,
    phantom_tp: PhantomData<TP>,
}

impl<TP: TabsPolicy> TabBar<TP> {
    pub fn new(axis: Axis, cross: CrossAxisAlignment, orientation: TabsOrientation) -> Self {
        TabBar {
            axis,
            cross,
            orientation,
            tabs: vec![],
            hot: None,
            phantom_tp: Default::default(),
        }
    }

    fn find_idx(&self, pos: Point) -> Option<TabIndex> {
        let major_pix = self.axis.major_pos(pos);
        let axis = self.axis;
        let res = self
            .tabs
            .binary_search_by_key(&((major_pix * 10.) as i64), |(_, tab)| {
                let rect = tab.layout_rect();
                let far_pix = axis.major_pos(rect.origin()) + axis.major(rect.size());
                (far_pix * 10.) as i64
            });
        match res {
            Ok(idx) => Some(idx),
            Err(idx) if idx < self.tabs.len() => Some(idx),
            _ => None,
        }
    }

    fn ensure_tabs(&mut self, data: &TabsState<TP>) {
        // Borrow checker/ type inference fun
        let (orientation, axis, cross) = (self.orientation, self.axis, self.cross);
        let finish_row = |w| WidgetPod::new(orientation.rotate_and_box(w, axis, cross));
        let finish_label = |w| WidgetPod::new(orientation.rotate_and_box(w, axis, cross));

        ensure_for_tabs(&mut self.tabs, &data.policy, &data.inner, |policy, key| {
            let info = policy.tab_info(key.clone(), &data.inner);

            let label = data
                .policy
                .tab_label(key.clone(), &info, &data.inner)
                // TODO: Type inference fails here because both sides of the lens are dependent on
                // associated types of the policy. Needs changes to lens derivation to embed PhantomData of the (relevant?) type params)
                // of the lensed types into the lens, to type inference has something to grab hold of
                .lens::<TabsState<TP>, tabs_state_derived_lenses::inner>(TabsState::<TP>::inner)
                .padding(Insets::uniform_xy(9., 5.));

            if info.can_close {
                let row = Flex::row()
                    .with_child(label)
                    .with_child(Label::new("ⓧ").on_click(
                        move |_ctx, data: &mut TabsState<TP>, _env| {
                            data.policy.close_tab(key.clone(), &mut data.inner);
                        },
                    ));
                finish_row(row)
            } else {
                finish_label(label)
            }
        });
    }
}

impl<TP: TabsPolicy> Widget<TabsState<TP>> for TabBar<TP> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut TabsState<TP>, env: &Env) {
        match event {
            Event::MouseDown(e) => {
                if let Some(idx) = self.find_idx(e.pos) {
                    data.selected = idx;
                }
            }
            Event::MouseMove(e) => {
                let new_hot = if ctx.is_hot() {
                    self.find_idx(e.pos)
                } else {
                    None
                };
                if new_hot != self.hot {
                    self.hot = new_hot;
                    ctx.request_paint();
                }
            }
            _ => {}
        }

        for (_, tab) in self.tabs.iter_mut() {
            tab.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &TabsState<TP>,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.ensure_tabs(data);
            ctx.children_changed();
            ctx.request_layout();
        }

        for (_, tab) in self.tabs.iter_mut() {
            tab.lifecycle(ctx, event, data, env);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &TabsState<TP>,
        data: &TabsState<TP>,
        _env: &Env,
    ) {
        if data.policy.tabs_changed(&old_data.inner, &data.inner) {
            self.ensure_tabs(data);
            ctx.children_changed();
            ctx.request_layout();
        } else if old_data.selected != data.selected {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &TabsState<TP>,
        env: &Env,
    ) -> Size {
        let (mut major, mut minor) = (0., 0.);
        for (_, tab) in self.tabs.iter_mut() {
            let size = tab.layout(ctx, bc, data, env);
            tab.set_layout_rect(
                ctx,
                data,
                env,
                Rect::from_origin_size(self.axis.pack(major, 0.), size),
            );
            major += self.axis.major(size);
            minor = f64::max(minor, self.axis.minor(size));
        }
        // Now go back through to reset the minors
        for (_, tab) in self.tabs.iter_mut() {
            let rect = tab.layout_rect();
            let rect = rect.with_size(self.axis.pack(self.axis.major(rect.size()), minor));
            tab.set_layout_rect(ctx, data, env, rect);
        }

        let wanted = self
            .axis
            .pack(f64::max(major, self.axis.major(bc.max())), minor);
        bc.constrain(wanted)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TabsState<TP>, env: &Env) {
        let hl_thickness = 2.;
        let highlight = env.get(theme::PRIMARY_LIGHT);
        // TODO: allow reversing tab order (makes more sense in some rotations)
        for (idx, (_, tab)) in self.tabs.iter_mut().enumerate() {
            let rect = tab.layout_rect();
            let rect = Rect::from_origin_size(rect.origin(), rect.size());
            let bg = match (idx == data.selected, Some(idx) == self.hot) {
                (_, true) => env.get(theme::BUTTON_DARK),
                (true, false) => env.get(theme::BACKGROUND_LIGHT),
                _ => env.get(theme::BACKGROUND_DARK),
            };
            ctx.fill(rect, &bg);

            tab.paint(ctx, data, env);
            if idx == data.selected {
                let (maj_near, maj_far) = self.axis.major_span(&rect);
                let (min_near, min_far) = self.axis.minor_span(&rect);
                let minor_pos = if let CrossAxisAlignment::End = self.cross {
                    min_near + (hl_thickness / 2.)
                } else {
                    min_far - (hl_thickness / 2.)
                };

                ctx.stroke(
                    Line::new(
                        self.axis.pack(maj_near, minor_pos),
                        self.axis.pack(maj_far, minor_pos),
                    ),
                    &highlight,
                    hl_thickness,
                )
            }
        }
    }
}

pub struct TabsTransitionState {
    previous_idx: TabIndex,
    current_time: u64,
    duration: Nanos,
    increasing: bool,
}

impl TabsTransitionState {
    pub fn new(previous_idx: TabIndex, duration: Nanos, increasing: bool) -> Self {
        TabsTransitionState {
            previous_idx,
            current_time: 0,
            duration,
            increasing,
        }
    }

    pub fn live(&self) -> bool {
        self.current_time < self.duration
    }

    pub fn fraction(&self) -> f64 {
        (self.current_time as f64) / (self.duration as f64)
    }

    pub fn previous_transform(&self, axis: Axis, main: f64) -> Affine {
        let x = if self.increasing {
            -main * self.fraction()
        } else {
            main * self.fraction()
        };
        Affine::translate(axis.pack(x, 0.))
    }

    pub fn selected_transform(&self, axis: Axis, main: f64) -> Affine {
        let x = if self.increasing {
            main * (1.0 - self.fraction())
        } else {
            -main * (1.0 - self.fraction())
        };
        Affine::translate(axis.pack(x, 0.))
    }
}

fn ensure_for_tabs<Content, TP: TabsPolicy + ?Sized>(
    contents: &mut Vec<(TP::Key, Content)>,
    policy: &TP,
    data: &TP::Input,
    f: impl Fn(&TP, TP::Key) -> Content,
) -> Vec<usize> {
    let mut existing_by_key: HashMap<TP::Key, Content> = contents.drain(..).collect();

    let mut existing_idx = Vec::new();
    for key in policy.tabs(data).into_iter() {
        let next = if let Some(child) = existing_by_key.remove(&key) {
            existing_idx.push(contents.len());
            child
        } else {
            f(&policy, key.clone())
        };
        contents.push((key.clone(), next))
    }
    existing_idx
}

pub struct TabsBody<TP: TabsPolicy> {
    children: Vec<(TP::Key, Option<TabBodyPod<TP>>)>,
    axis: Axis,
    transition: TabsTransition,
    transition_state: Option<TabsTransitionState>,
    phantom_tp: PhantomData<TP>,
}

impl<TP: TabsPolicy> TabsBody<TP> {
    pub fn new(axis: Axis, transition: TabsTransition) -> TabsBody<TP> {
        TabsBody {
            children: vec![],
            axis,
            transition,
            transition_state: None,
            phantom_tp: Default::default(),
        }
    }

    fn make_tabs(&mut self, data: &TabsState<TP>) -> Vec<usize> {
        ensure_for_tabs(
            &mut self.children,
            &data.policy,
            &data.inner,
            |policy, key| policy.tab_body(key, &data.inner).map(WidgetPod::new),
        )
    }

    fn active_child(&mut self, state: &TabsState<TP>) -> Option<&mut TabBodyPod<TP>> {
        Self::child(&mut self.children, state.selected)
    }

    // Doesn't take self to allow separate borrowing
    fn child(
        children: &mut Vec<(TP::Key, Option<TabBodyPod<TP>>)>,
        idx: usize,
    ) -> Option<&mut TabBodyPod<TP>> {
        children.get_mut(idx).and_then(|x| x.1.as_mut())
    }

    fn child_pods(&mut self) -> impl Iterator<Item = &mut TabBodyPod<TP>> {
        self.children.iter_mut().flat_map(|x| x.1.as_mut())
    }
}

fn hidden_should_receive_event(evt: &Event) -> bool {
    match evt {
        Event::WindowConnected
        | Event::WindowSize(_)
        | Event::Timer(_)
        | Event::Command(_)
        | Event::Internal(_) => true,
        Event::MouseDown(_)
        | Event::MouseUp(_)
        | Event::MouseMove(_)
        | Event::Wheel(_)
        | Event::KeyDown(_)
        | Event::KeyUp(_)
        | Event::Paste(_)
        | Event::Zoom(_) => false,
    }
}

fn hidden_should_receive_lifecycle(lc: &LifeCycle) -> bool {
    match lc {
        LifeCycle::WidgetAdded | LifeCycle::Internal(_) => true,
        LifeCycle::Size(_)
        | LifeCycle::AnimFrame(_)
        | LifeCycle::HotChanged(_)
        | LifeCycle::FocusChanged(_) => false,
    }
}

impl<TP: TabsPolicy> Widget<TabsState<TP>> for TabsBody<TP> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut TabsState<TP>, env: &Env) {
        if hidden_should_receive_event(event) {
            for child in self.child_pods() {
                child.event(ctx, event, &mut data.inner, env);
            }
        } else if let Some(child) = self.active_child(data) {
            child.event(ctx, event, &mut data.inner, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &TabsState<TP>,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.make_tabs(data);
            ctx.children_changed();
            ctx.request_layout();
        }

        if hidden_should_receive_lifecycle(event) {
            for child in self.child_pods() {
                child.lifecycle(ctx, event, &data.inner, env);
            }
        } else if let Some(child) = self.active_child(data) {
            // Pick which events go to all and which just to active
            child.lifecycle(ctx, event, &data.inner, env);
        }

        if let (Some(t_state), LifeCycle::AnimFrame(interval)) = (&mut self.transition_state, event) {
            t_state.current_time += *interval;
            if t_state.live() {
                ctx.request_anim_frame();
            } else {
                self.transition_state = None;
            }
            ctx.request_paint();
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &TabsState<TP>,
        data: &TabsState<TP>,
        env: &Env,
    ) {
        let init = if data.policy.tabs_changed(&old_data.inner, &data.inner) {
            ctx.children_changed();
            ctx.request_layout();
            Some(self.make_tabs(data))
        } else {
            None
        };

        if old_data.selected != data.selected {
            self.transition_state = self.transition.tab_changed(old_data.selected, data.selected);
            ctx.request_layout();

            if self.transition_state.is_some() {
                ctx.request_anim_frame();
            }
        }

        // Make sure to only pass events to initialised children
        if let Some(init) = init {
            for idx in init {
                if let Some(child) = Self::child(&mut self.children, idx) {
                    child.update(ctx, &data.inner, env)
                }
            }
        } else {
            for child in self.child_pods() {
                child.update(ctx, &data.inner, env);
            }
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &TabsState<TP>,
        env: &Env,
    ) -> Size {
        if let Some(ref mut child) = self.active_child(data) {
            let inner = &data.inner;
            let size = child.layout(ctx, bc, inner, env);
            child.set_layout_rect(ctx, inner, env, Rect::from_origin_size(Point::ORIGIN, size));
            size
        } else {
            bc.max()
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TabsState<TP>, env: &Env) {
        if let Some(trans) = &self.transition_state {
            let axis = self.axis;
            let size = ctx.size();
            let major = axis.major(size);
            ctx.clip(Rect::from_origin_size(Point::ZERO, size));

            let children = &mut self.children;
            if let Some(ref mut prev) = Self::child(children, trans.previous_idx) {
                ctx.with_save(|ctx| {
                    ctx.transform(trans.previous_transform(axis, major));
                    prev.paint_raw(ctx, &data.inner, env);
                })
            }
            if let Some(ref mut child) = Self::child(children, data.selected) {
                ctx.with_save(|ctx| {
                    ctx.transform(trans.selected_transform(axis, major));
                    child.paint_raw(ctx, &data.inner, env);
                })
            }
        } else if let Some(ref mut child) = Self::child(&mut self.children, data.selected) {
            child.paint_raw(ctx, &data.inner, env);
        }
    }
}

// This only needs to exist to be able to give a reasonable type to the TabScope
pub struct TabsScopePolicy<TP> {
    tabs_from_data: TP,
    selected: TabIndex,
}

impl<TP> TabsScopePolicy<TP> {
    pub fn new(tabs_from_data: TP, selected: TabIndex) -> Self {
        Self {
            tabs_from_data,
            selected,
        }
    }
}

impl<TP: TabsPolicy> ScopePolicy for TabsScopePolicy<TP> {
    type In = TP::Input;
    type State = TabsState<TP>;
    type Transfer = LensScopeTransfer<tabs_state_derived_lenses::inner, Self::In, Self::State>;

    fn create(self, inner: &Self::In) -> (Self::State, Self::Transfer) {
        (
            TabsState::new(inner.clone(), self.selected, self.tabs_from_data),
            LensScopeTransfer::new(Self::State::inner),
        )
    }
}

#[derive(Data, Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum TabsOrientation {
    Standard,
    Turns(u8), // These represent 90 degree rotations clockwise.
}

#[derive(Data, Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum TabsTransition {
    Instant,
    Slide(Nanos)
}

impl Default for TabsTransition{
    fn default() -> Self {
        TabsTransition::Slide(Duration::from_millis(250).as_nanos() as Nanos)
    }
}

impl TabsTransition{
    fn tab_changed(&self, old: TabIndex, new: TabIndex)->Option<TabsTransitionState>{
        match self{
            TabsTransition::Instant=>None,
            TabsTransition::Slide(dur)=>Some(TabsTransitionState::new(
                old,
                *dur,
                old < new,
            ))
        }
    }
}

impl TabsOrientation {
    pub fn rotate_and_box<W: Widget<T> + 'static, T: Data>(
        self,
        widget: W,
        axis: Axis,
        cross: CrossAxisAlignment,
    ) -> Box<dyn Widget<T>> {
        let turns = match self {
            Self::Standard => match (axis, cross) {
                (Axis::Horizontal, _) => 0,
                (Axis::Vertical, CrossAxisAlignment::Start) => 3,
                (Axis::Vertical, _) => 1,
            },
            Self::Turns(turns) => turns,
        };

        if turns == 0 {
            Box::new(widget)
        } else {
            Box::new(widget.rotate(turns))
        }
    }
}

pub struct InitialTab<T> {
    name: String,
    child: SingleUse<Box<dyn Widget<T>>>, // This is to avoid cloning provided tabs
}

impl<T: Data> InitialTab<T> {
    pub fn new(name: impl Into<String>, child: impl Widget<T> + 'static) -> Self {
        InitialTab {
            name: name.into(),
            child: SingleUse::new(Box::new(child)),
        }
    }
}

enum TabsContent<TP: TabsPolicy> {
    Building {
        tabs: TP::Build,
    },
    Complete {
        tabs: TP,
    },
    Running {
        scope: WidgetPod<TP::Input, TabsScope<TP>>,
    },
    Swapping,
}

pub struct Tabs<TP: TabsPolicy> {
    axis: Axis,
    cross: CrossAxisAlignment, // Not sure if this should have another enum. Middle means nothing here
    rotation: TabsOrientation,
    transition: TabsTransition,
    content: TabsContent<TP>,
}

impl<T: Data> Tabs<StaticTabs<T>> {
    pub fn new() -> Self {
        Tabs::building(Vec::new())
    }
}

impl<T: Data> Default for Tabs<StaticTabs<T>> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TP: TabsPolicy> Tabs<TP> {
    fn of_content(content: TabsContent<TP>) -> Self {
        Tabs {
            axis: Axis::Horizontal,
            cross: CrossAxisAlignment::Start,
            rotation: TabsOrientation::Standard,
            transition: Default::default(),
            content,
        }
    }

    pub fn for_policy(tabs: TP) -> Self {
        Self::of_content(TabsContent::Complete { tabs })
    }

    pub fn building(tabs_from_data: TP::Build) -> Self
    where
        TP: AddTab,
    {
        Self::of_content(TabsContent::Building {
            tabs: tabs_from_data,
        })
    }

    pub fn with_axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn with_rotation(mut self, rotation: TabsOrientation) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_cross_axis_alignment(mut self, cross: CrossAxisAlignment) -> Self {
        self.cross = cross;
        self
    }

    pub fn with_transition(mut self, transition: TabsTransition) -> Self {
        self.transition = transition;
        self
    }

    pub fn with_tab(
        mut self,
        name: impl Into<String>,
        child: impl Widget<TP::Input> + 'static,
    ) -> Tabs<TP>
    where
        TP: AddTab,
    {
        self.add_tab(name, child);
        self
    }

    pub fn add_tab(&mut self, name: impl Into<String>, child: impl Widget<TP::Input> + 'static)
    where
        TP: AddTab,
    {
        if let TabsContent::Building { tabs } = &mut self.content {
            TP::add_tab(tabs, name, child)
        } else {
            log::warn!("Can't add static tabs to a running or complete tabs instance!")
        }
    }

    pub fn with_tabs<TabsFromD: TabsPolicy>(self, tabs: TabsFromD) -> Tabs<TabsFromD> {
        Tabs {
            axis: self.axis,
            cross: self.cross,
            rotation: self.rotation,
            transition: self.transition,
            content: TabsContent::Complete { tabs },
        }
    }

    pub fn make_scope(&self, tabs_from_data: TP) -> WidgetPod<TP::Input, TabsScope<TP>> {
        let (tabs_bar, tabs_body) = (
            (TabBar::new(self.axis, self.cross, self.rotation), 0.0),
            (
                TabsBody::new(self.axis, self.transition)
                    .padding(5.)
                    .border(theme::BORDER_DARK, 0.5)
                    .expand(),
                1.0,
            ),
        );
        let mut layout: Flex<TabsState<TP>> = Flex::for_axis(self.axis.cross());

        if let CrossAxisAlignment::End = self.cross {
            layout.add_flex_child(tabs_body.0, tabs_body.1);
            layout.add_flex_child(tabs_bar.0, tabs_bar.1);
        } else {
            layout.add_flex_child(tabs_bar.0, tabs_bar.1);
            layout.add_flex_child(tabs_body.0, tabs_body.1);
        };

        WidgetPod::new(Scope::new(
            TabsScopePolicy::new(tabs_from_data, 0),
            Box::new(layout),
        ))
    }
}

impl<TP: TabsPolicy> Widget<TP::Input> for Tabs<TP> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut TP::Input, env: &Env) {
        if let TabsContent::Running { scope } = &mut self.content {
            scope.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &TP::Input,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            let mut temp = TabsContent::Swapping;
            std::mem::swap(&mut self.content, &mut temp);

            self.content = match temp {
                TabsContent::Building { tabs } => {
                    ctx.children_changed();
                    TabsContent::Running {
                        scope: self.make_scope(TP::build(tabs)),
                    }
                }
                TabsContent::Complete { tabs } => {
                    ctx.children_changed();
                    TabsContent::Running {
                        scope: self.make_scope(tabs),
                    }
                }
                _ => temp,
            };
        }
        if let TabsContent::Running { scope } = &mut self.content {
            scope.lifecycle(ctx, event, data, env)
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &TP::Input, data: &TP::Input, env: &Env) {
        if let TabsContent::Running { scope } = &mut self.content {
            scope.update(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &TP::Input,
        env: &Env,
    ) -> Size {
        if let TabsContent::Running { scope } = &mut self.content {
            let size = scope.layout(ctx, bc, data, env);
            scope.set_layout_rect(ctx, data, env, Rect::from_origin_size(Point::ORIGIN, size));
            size
        } else {
            bc.min()
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TP::Input, env: &Env) {
        if let TabsContent::Running { scope } = &mut self.content {
            scope.paint(ctx, data, env)
        }
    }
}
