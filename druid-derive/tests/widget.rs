//! Test #[derive(Widget)]

use druid::widget::{CompositeMeta, CrossAxisAlignment, Flex, Label, TextBox};
use druid::Widget;
use druid_derive::Widget;

#[derive(Widget)]
pub struct TextBoxWithLabel {
    #[widget(meta)]
    meta: CompositeMeta<String>,
    label: String,
}

mod test {
    use druid::widget::{CompositeMeta, CrossAxisAlignment, Flex, Label, TextBox};
    use druid::Widget;
    use druid_derive::Widget;

    #[derive(Widget)]
    pub struct Test {
        #[widget(meta)]
        meta: CompositeMeta<String>,
        label: String,
    }

    impl Test {
        fn build(&self) -> impl Widget<String> + 'static {
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(Label::new(self.label.clone()))
                .with_spacer(4.)
                .with_child(TextBox::new().with_placeholder(String::from("Test")))
        }
    }
}

impl TextBoxWithLabel {
    fn new(label: impl Into<String>) -> Self {
        TextBoxWithLabel {
            meta: CompositeMeta::default(),
            label: label.into(),
        }
    }

    fn build(&self) -> impl Widget<String> + 'static {
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(Label::new(self.label.clone()))
            .with_spacer(4.)
            .with_child(TextBox::new().with_placeholder(String::from("Test")))
    }
}

#[test]
fn test_widget_derive() {
    TextBoxWithLabel::new("Test");
}
