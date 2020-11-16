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

use druid::im::Vector;
use druid::widget::{
    Axis, Button, CrossAxisAlignment, Flex, Label, MainAxisAlignment, RadioGroup, Split, TabInfo,
    Tabs, TabsEdge, TabsPolicy, TabsTransition, TextBox, ViewSwitcher,
};
use druid::{theme, AppLauncher, Color, Data, Env, Lens, Widget, WidgetExt, WindowDesc};
use instant::Duration;

#[derive(Data, Clone, Lens)]
struct DynamicTabData {
    highest_tab: usize,
    removed_tabs: usize,
    tab_labels: Vector<usize>,
}

impl DynamicTabData {
    fn new(highest_tab: usize) -> Self {
        DynamicTabData {
            highest_tab,
            removed_tabs: 0,
            tab_labels: (1..=highest_tab).collect(),
        }
    }

    fn add_tab(&mut self) {
        self.highest_tab += 1;
        self.tab_labels.push_back(self.highest_tab);
    }

    fn remove_tab(&mut self, idx: usize) {
        if idx >= self.tab_labels.len() {
            log::warn!("Attempt to remove non existent tab at index {}", idx)
        } else {
            self.removed_tabs += 1;
            self.tab_labels.remove(idx);
        }
    }

    // This provides a key that will monotonically increase as interactions occur.
    fn tabs_key(&self) -> (usize, usize) {
        (self.highest_tab, self.removed_tabs)
    }
}

#[derive(Data, Clone, Lens)]
struct TabConfig {
    axis: Axis,
    edge: TabsEdge,
    transition: TabsTransition,
}

#[derive(Data, Clone, Lens)]
struct AppState {
    tab_config: TabConfig,
    advanced: DynamicTabData,
    first_tab_name: String,
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title("Tabs")
        .window_size((700.0, 400.0));

    // create the initial app state
    let initial_state = AppState {
        tab_config: TabConfig {
            axis: Axis::Horizontal,
            edge: TabsEdge::Leading,
            transition: Default::default(),
        },
        first_tab_name: "First tab".into(),
        advanced: DynamicTabData::new(2),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    fn group<T: Data, W: Widget<T> + 'static>(text: &str, w: W) -> impl Widget<T> {
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Label::new(text)
                    .background(theme::PLACEHOLDER_COLOR)
                    .expand_width(),
            )
            .with_default_spacer()
            .with_child(w)
            .with_default_spacer()
            .border(Color::WHITE, 0.5)
    }

    let axis_picker = group(
        "Tab bar axis",
        RadioGroup::new(vec![
            ("Horizontal", Axis::Horizontal),
            ("Vertical", Axis::Vertical),
        ])
        .lens(TabConfig::axis),
    );

    let cross_picker = group(
        "Tab bar edge",
        RadioGroup::new(vec![
            ("Leading", TabsEdge::Leading),
            ("Trailing", TabsEdge::Trailing),
        ])
        .lens(TabConfig::edge),
    );

    let transit_picker = group(
        "Transition",
        RadioGroup::new(vec![
            ("Instant", TabsTransition::Instant),
            (
                "Slide",
                TabsTransition::Slide(Duration::from_millis(250).as_nanos() as u64),
            ),
        ])
        .lens(TabConfig::transition),
    );

    let sidebar = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Start)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(axis_picker)
        .with_default_spacer()
        .with_child(cross_picker)
        .with_default_spacer()
        .with_child(transit_picker)
        .with_flex_spacer(1.)
        .fix_width(200.0)
        .lens(AppState::tab_config);

    let vs = ViewSwitcher::new(
        |app_s: &AppState, _| app_s.tab_config.clone(),
        |tc: &TabConfig, _, _| Box::new(build_tab_widget(tc)),
    );
    Flex::row().with_child(sidebar).with_flex_child(vs, 1.0)
}

#[derive(Clone, Data)]
struct NumberedTabs;

impl TabsPolicy for NumberedTabs {
    type Key = usize;
    type Build = ();
    type Input = DynamicTabData;
    type LabelWidget = Label<DynamicTabData>;
    type BodyWidget = Label<DynamicTabData>;

    fn tabs_changed(&self, old_data: &DynamicTabData, data: &DynamicTabData) -> bool {
        old_data.tabs_key() != data.tabs_key()
    }

    fn tabs(&self, data: &DynamicTabData) -> Vec<Self::Key> {
        data.tab_labels.iter().copied().collect()
    }

    fn tab_info(&self, key: Self::Key, _data: &DynamicTabData) -> TabInfo<DynamicTabData> {
        TabInfo::new(format!("Tab {:?}", key), true)
    }

    fn tab_body(&self, key: Self::Key, _data: &DynamicTabData) -> Label<DynamicTabData> {
        Label::new(format!("Dynamic tab body {:?}", key))
    }

    fn close_tab(&self, key: Self::Key, data: &mut DynamicTabData) {
        if let Some(idx) = data.tab_labels.index_of(&key) {
            data.remove_tab(idx)
        }
    }

    fn tab_label(
        &self,
        _key: Self::Key,
        info: TabInfo<Self::Input>,
        _data: &Self::Input,
    ) -> Self::LabelWidget {
        Self::default_make_label(info)
    }
}

fn build_tab_widget(tab_config: &TabConfig) -> impl Widget<AppState> {
    let dyn_tabs = Tabs::for_policy(NumberedTabs)
        .with_axis(tab_config.axis)
        .with_edge(tab_config.edge)
        .with_transition(tab_config.transition)
        .lens(AppState::advanced);

    let control_dynamic = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Control dynamic tabs"))
        .with_child(Button::new("Add a tab").on_click(|_c, d: &mut DynamicTabData, _e| d.add_tab()))
        .with_child(Label::new(|adv: &DynamicTabData, _e: &Env| {
            format!("Highest tab number is {}", adv.highest_tab)
        }))
        .with_spacer(20.)
        .lens(AppState::advanced);

    let first_static_tab = Flex::row()
        .with_child(Label::new("Rename tab:"))
        .with_child(TextBox::new().lens(AppState::first_tab_name));

    let main_tabs = Tabs::new()
        .with_axis(tab_config.axis)
        .with_edge(tab_config.edge)
        .with_transition(tab_config.transition)
        .with_tab(
            |app_state: &AppState, _: &Env| app_state.first_tab_name.to_string(),
            first_static_tab,
        )
        .with_tab("Dynamic", control_dynamic)
        .with_tab("Page 3", Label::new("Page 3 content"))
        .with_tab("Page 4", Label::new("Page 4 content"))
        .with_tab("Page 5", Label::new("Page 5 content"))
        .with_tab("Page 6", Label::new("Page 6 content"));

    Split::rows(main_tabs, dyn_tabs).draggable(true)
}
