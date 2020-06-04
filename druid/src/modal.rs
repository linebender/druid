// Copyright 2020 The xi-editor Authors.
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

use std::any::Any;

use crate::lens;
use crate::widget::prelude::*;
use crate::{Data, Point, Selector, SingleUse, WidgetExt, WidgetPod};

/// Describes a modal widget.
///
/// A modal widget is a widget that can be displayed over all the other widgets in a window. It
/// consists of a widget (which must take the same data type as the [`Window`]) and some settings
/// describing how the widget will be presented.
///
/// You can display a modal widget by sending a [`SHOW_MODAL`] command to a window.
///
/// [`Window`]: struct.Window.html
/// [`SHOW_MODAL`]: struct.Modal.html#associatedconstant.SHOW_MODAL
pub struct ModalDesc<T> {
    widget: Box<dyn Widget<T>>,
    /// If false, only the modal will get user input events.
    pass_through_events: bool,
    /// If set, the origin of the modal widget. If unset, the modal widget is centered in the
    /// `ModalHost`.
    position: Option<Point>,
}

// The same as ModalDesc, but with the widget wrapped in a WidgetPod.
pub(crate) struct Modal<T> {
    pub(crate) widget: WidgetPod<T, Box<dyn Widget<T>>>,
    pub(crate) pass_through_events: bool,
    pub(crate) position: Option<Point>,
}

impl<T> From<ModalDesc<T>> for Modal<T> {
    fn from(desc: ModalDesc<T>) -> Modal<T> {
        Modal {
            widget: WidgetPod::new(desc.widget),
            pass_through_events: desc.pass_through_events,
            position: desc.position,
        }
    }
}

impl ModalDesc<()> {
    /// Command to dismiss the modal.
    pub(crate) const DISMISS_MODAL: Selector<()> = Selector::new("druid.dismiss-modal-widget");

    /// Shows a modal widget that doesn't take any data.
    ///
    /// This is less flexible than `SHOW_MODAL`, but it has one big advantage: it makes it possible
    /// for druid to provide nice interfaces to show simple modals. The issue with `SHOW_MODAL` is
    /// that from within druid we have no idea what the right `Data` is, and so druid can't create
    /// its own `ModalDesc<T>`s. See `WidgetExt::tooltip` for an example of the kind of API that we
    /// can provide for `ModalDesc<()>`.
    pub(crate) const SHOW_MODAL_NO_DATA: Selector<SingleUse<ModalDesc<()>>> =
        Selector::new("druid.show-modal-widget-no-data");

    pub(crate) fn lensed<T: Data>(self) -> ModalDesc<T> {
        ModalDesc {
            widget: Box::new(self.widget.lens(lens::Map::new(|_| (), |_, _| {}))),
            pass_through_events: self.pass_through_events,
            position: self.position,
        }
    }
}

impl<T> ModalDesc<T> {
    /// Command to display a modal in this host.
    ///
    /// Note: this is a bit of a footgun, because the typed selectors don't know about generics. In
    /// particular, this means that if you submit a SHOW_MODAL command with the wrong `T`, it will
    /// type-check but panic at run-time.
    pub(crate) const SHOW_MODAL: Selector<SingleUse<Box<dyn Any>>> =
        Selector::new("druid.show-modal-widget");

    /// Creates a new modal for displaying the widget `widget`.
    pub fn new(widget: impl Widget<T> + 'static) -> ModalDesc<T> {
        ModalDesc {
            widget: Box::new(widget),
            pass_through_events: false,
            position: None,
        }
    }

    /// Determines whether to pass through events from the modal to the rest of the window.
    ///
    /// The default value of `pass_through` is `false`, meaning that the user can only interact
    /// with the modal widget.
    pub fn pass_through_events(mut self, pass_through: bool) -> Self {
        self.pass_through_events = pass_through;
        self
    }

    /// Sets the origin of the modal widget, relative to the window.
    ///
    /// By default, the modal widget is centered in the window.
    pub fn position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }
}
