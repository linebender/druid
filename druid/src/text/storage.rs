// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Storing text.

use std::sync::Arc;

use crate::env::KeyLike;
use crate::piet::{PietTextLayoutBuilder, TextStorage as PietTextStorage};
use crate::{Data, Env};

use super::attribute::Link;
use crate::UpdateCtx;

/// A type that represents text that can be displayed.
pub trait TextStorage: PietTextStorage + Data {
    /// If this TextStorage object manages style spans, it should implement
    /// this method and update the provided builder with its spans, as required.
    #[allow(unused_variables)]
    fn add_attributes(&self, builder: PietTextLayoutBuilder, env: &Env) -> PietTextLayoutBuilder {
        builder
    }

    /// This is called whenever the Env changes and should return true
    /// if the layout should be rebuilt.
    #[allow(unused_variables)]
    fn env_update(&self, ctx: &EnvUpdateCtx) -> bool {
        false
    }

    /// Any additional [`Link`] attributes on this text.
    ///
    /// If this `TextStorage` object manages link attributes, it should implement this
    /// method and return any attached [`Link`]s.
    ///
    /// Unlike other attributes, links are managed in Druid, not in [`piet`]; as such they
    /// require a separate API.
    ///
    /// [`Link`]: super::attribute::Link
    /// [`piet`]: crate::piet
    fn links(&self) -> &[Link] {
        &[]
    }
}

/// Provides information about keys change for more fine grained invalidation
pub struct EnvUpdateCtx<'a, 'b>(&'a UpdateCtx<'a, 'b>);

impl<'a, 'b> EnvUpdateCtx<'a, 'b> {
    /// Create an [`EnvUpdateCtx`] for [`Widget::update`].
    ///
    /// [`Widget::update`]: crate::Widget::update
    pub(crate) fn for_update(ctx: &'a UpdateCtx<'a, 'b>) -> Self {
        Self(ctx)
    }

    /// Returns `true` if the given key has changed since the last [`env_update`]
    /// call.
    ///
    /// See [`UpdateCtx::env_key_changed`] for more details.
    ///
    /// [`env_update`]: TextStorage::env_update
    pub fn env_key_changed<T>(&self, key: &impl KeyLike<T>) -> bool {
        self.0.env_key_changed(key)
    }
}

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in Druid. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = Arc<str>;

impl TextStorage for ArcStr {}

impl TextStorage for String {}

impl TextStorage for Arc<String> {}
