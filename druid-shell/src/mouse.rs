// Copyright 2019 The Druid Authors.
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

//! Common types for representing mouse events and state

use crate::backend;
use crate::kurbo::Point;
use crate::piet::ImageBuf;

//NOTE: this currently only contains cursors that are included by default on
//both Windows and macOS. We may want to provide polyfills for various additional cursors.
/// Mouse cursors.
#[derive(Clone, PartialEq)]
pub enum Cursor {
    /// The default arrow cursor.
    Arrow,
    /// A vertical I-beam, for indicating insertion points in text.
    IBeam,
    Pointer,
    Crosshair,

    #[deprecated(note = "this will be removed in future because it is not available on windows")]
    OpenHand,
    NotAllowed,
    ResizeLeftRight,
    ResizeUpDown,
    // The platform cursor should be small. Any image data that it uses should be shared (i.e.
    // behind an `Arc` or using a platform API that does the sharing).
    Custom(backend::window::CustomCursor),
}

/// A platform-independent description of a custom cursor.
#[derive(Clone)]
pub struct CursorDesc {
    pub(crate) image: ImageBuf,
    pub(crate) hot: Point,
}

impl CursorDesc {
    /// Creates a new `CursorDesc`.
    ///
    /// `hot` is the "hot spot" of the cursor, measured in terms of the pixels in `image` with
    /// `(0, 0)` at the top left. The hot spot is the logical position of the mouse cursor within
    /// the image. For example, if the image is a picture of a arrow, the hot spot might be the
    /// coordinates of the arrow's tip.
    pub fn new(image: ImageBuf, hot: impl Into<Point>) -> CursorDesc {
        CursorDesc {
            image,
            hot: hot.into(),
        }
    }
}

impl std::fmt::Debug for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        #[allow(deprecated)]
        match self {
            Cursor::Arrow => write!(f, "Cursor::Arrow"),
            Cursor::IBeam => write!(f, "Cursor::IBeam"),
            Cursor::Pointer => write!(f, "Cursor::Pointer"),
            Cursor::Crosshair => write!(f, "Cursor::Crosshair"),
            Cursor::OpenHand => write!(f, "Cursor::OpenHand"),
            Cursor::NotAllowed => write!(f, "Cursor::NotAllowed"),
            Cursor::ResizeLeftRight => write!(f, "Cursor::ResizeLeftRight"),
            Cursor::ResizeUpDown => write!(f, "Cursor::ResizeUpDown"),
            Cursor::Custom(_) => write!(f, "Cursor::Custom"),
        }
    }
}
