use druid::im::Vector;
use druid::widget::{
    Axis, Button, CrossAxisAlignment, Flex, Label, MainAxisAlignment, Padding, RadioGroup,
    SizedBox, Split, TabInfo, Tabs, TabsOrientation, TabsPolicy, TabsTransition, TextBox,
    ViewSwitcher,
};
use druid::{theme, AppLauncher, Color, Data, Env, Lens, LensExt, Widget, WidgetExt, WindowDesc};
use instant::Duration;

#[derive(Data, Clone)]
struct Basic {}

#[derive(Data, Clone, Lens)]
struct Advanced {
    highest_tab: usize,
    removed_tabs: usize,
    tab_labels: Vector<usize>,
}

impl Advanced {
    fn new(highest_tab: usize) -> Self {
        Advanced {
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

    fn tabs_key(&self) -> (usize, usize) {
        (self.highest_tab, self.removed_tabs)
    }
}

#[derive(Data, Clone, Lens)]
struct TabConfig {
    axis: Axis,
    cross: CrossAxisAlignment,
    rotation: TabsOrientation,
    transition: TabsTransition,
}

#[derive(Data, Clone, Lens)]
struct AppState {
    tab_config: TabConfig,
    basic: Basic,
    advanced: Advanced,
    text: String,
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
            cross: CrossAxisAlignment::Start,
            rotation: TabsOrientation::Standard,
            transition: Default::default(),
        },
        basic: Basic {},
        advanced: Advanced::new(2),
        text: "Interesting placeholder".into(),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    fn decor<T: Data>(label: Label<T>) -> SizedBox<T> {
        label
            .padding(5.)
            .background(theme::PLACEHOLDER_COLOR)
            .expand_width()
    }

    fn group<T: Data, W: Widget<T> + 'static>(w: W) -> Padding<T> {
        w.border(Color::WHITE, 0.5).padding(5.)
    }

    let axis_picker = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(decor(Label::new("Tab bar axis")))
        .with_child(RadioGroup::new(vec![
            ("Horizontal", Axis::Horizontal),
            ("Vertical", Axis::Vertical),
        ]))
        .lens(AppState::tab_config.then(TabConfig::axis));

    let cross_picker = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(decor(Label::new("Tab bar alignment")))
        .with_child(RadioGroup::new(vec![
            ("Start", CrossAxisAlignment::Start),
            ("End", CrossAxisAlignment::End),
        ]))
        .lens(AppState::tab_config.then(TabConfig::cross));

    let rot_picker = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(decor(Label::new("Tab rotation")))
        .with_child(RadioGroup::new(vec![
            ("Standard", TabsOrientation::Standard),
            ("None", TabsOrientation::Turns(0)),
            ("Up", TabsOrientation::Turns(3)),
            ("Down", TabsOrientation::Turns(1)),
            ("Aussie", TabsOrientation::Turns(2)),
        ]))
        .lens(AppState::tab_config.then(TabConfig::rotation));

    let transit_picker = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(decor(Label::new("Transition")))
        .with_child(RadioGroup::new(vec![
            ("Instant", TabsTransition::Instant),
            (
                "Slide",
                TabsTransition::Slide(Duration::from_millis(250).as_nanos() as u64),
            ),
        ]))
        .lens(AppState::tab_config.then(TabConfig::transition));

    let sidebar = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Start)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(group(axis_picker))
        .with_child(group(cross_picker))
        .with_child(group(rot_picker))
        .with_child(group(transit_picker))
        .with_flex_spacer(1.)
        .fix_width(200.0);

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
    type Input = Advanced;
    type LabelWidget = Label<Advanced>;
    type BodyWidget = Label<Advanced>;

    fn tabs_changed(&self, old_data: &Advanced, data: &Advanced) -> bool {
        old_data.tabs_key() != data.tabs_key()
    }

    fn tabs(&self, data: &Advanced) -> Vec<Self::Key> {
        data.tab_labels.iter().copied().collect()
    }

    fn tab_info(&self, key: Self::Key, _data: &Advanced) -> TabInfo {
        TabInfo::new(format!("Tab {:?}", key), true)
    }

    fn tab_body(&self, key: Self::Key, _data: &Advanced) -> Option<Label<Advanced>> {
        Some(Label::new(format!("Dynamic tab body {:?}", key)))
    }

    fn close_tab(&self, key: Self::Key, data: &mut Advanced) {
        if let Some(idx) = data.tab_labels.index_of(&key) {
            data.remove_tab(idx)
        }
    }

    fn tab_label(&self, _key: Self::Key, info: &TabInfo, _data: &Self::Input) -> Self::LabelWidget {
        Self::default_make_label(info)
    }
}

fn build_tab_widget(tab_config: &TabConfig) -> impl Widget<AppState> {
    let dyn_tabs = Tabs::for_policy(NumberedTabs)
        .with_axis(tab_config.axis)
        .with_cross_axis_alignment(tab_config.cross)
        .with_rotation(tab_config.rotation)
        .with_transition(tab_config.transition)
        .lens(AppState::advanced);

    let adv = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Control dynamic tabs"))
        .with_child(Button::new("Add a tab").on_click(|_c, d: &mut Advanced, _e| d.add_tab()))
        .with_child(Label::new(|adv: &Advanced, _e: &Env| {
            format!("Highest tab number is {}", adv.highest_tab)
        }))
        .with_spacer(20.)
        .lens(AppState::advanced);

    let main_tabs = Tabs::new()
        .with_axis(tab_config.axis)
        .with_cross_axis_alignment(tab_config.cross)
        .with_rotation(tab_config.rotation)
        .with_transition(tab_config.transition)
        .with_tab("Basic", Label::new("Basic kind of stuff"))
        .with_tab("Advanced", adv)
        .with_tab("Page 3", Label::new("Basic kind of stuff"))
        .with_tab("Page 4", Label::new("Basic kind of stuff"))
        .with_tab("Page 5", Label::new("Basic kind of stuff"))
        .with_tab("Page 6", Label::new("Basic kind of stuff"))
        .with_tab("Page 7", TextBox::new().lens(AppState::text));

    Split::rows(main_tabs, dyn_tabs).draggable(true)
}
