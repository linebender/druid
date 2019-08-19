// Copyright 2019 The xi-editor Authors.
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

use std::collections::HashMap;
use std::sync::Arc;

use crate::data::Data;
use crate::env::Env;
use fluent::{FluentArgs, FluentValue};

//NOTE: instead of a closure, at some point we can use something like a lens for this.

//TODO: this is an Arc so that it can be clone, which is a bound on things like `Menu`.
/// A closure that generates a localization value.
type ArgClosure<T> = Arc<dyn Fn(&Env, &T) -> FluentValue<'static> + 'static>;

type Args<T> = HashMap<&'static str, ArgSource<T>>;

/// Wraps a closure that generates an argument for localization.
#[derive(Clone)]
struct ArgSource<T>(ArgClosure<T>);

/// A string that can be localized based on the current locale.
///
/// At it's simplest, a `LocalizedString` is a key that can be resolved
/// against a map of localized strings for a given locale.
#[derive(Debug, Clone)]
pub struct LocalizedString<T> {
    key: &'static str,
    placeholder: Option<&'static str>,
    args: Option<Args<T>>,
    resolved: Option<String>,
}

impl<T> LocalizedString<T> {
    /// Create a new `LocalizedString` with the given key.
    pub const fn new(key: &'static str) -> Self {
        LocalizedString {
            key,
            args: None,
            placeholder: None,
            resolved: None,
        }
    }

    /// Add a placeholder value. This will be used if localization fails.
    ///
    /// This is intended for use during prototyping.
    pub const fn with_placeholder(mut self, placeholder: &'static str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// Return the localized value for this string, or the placeholder, if
    /// the localization is missing, or the key if there is no placeholder.
    pub fn localized_str(&self) -> &str {
        self.resolved
            .as_ref()
            .map(|s| s.as_str())
            .or(self.placeholder)
            .unwrap_or(self.key)
    }
}

impl<T: Data> LocalizedString<T> {
    /// Add a named argument and a cooresponding [`ArgClosure`]. This closure
    /// is a function that will return a value for the given key from the current
    /// environment and data.
    pub fn with_arg(
        mut self,
        key: &'static str,
        f: impl Fn(&Env, &T) -> FluentValue<'static> + 'static,
    ) -> Self {
        self.args
            .get_or_insert(HashMap::new())
            .insert(key, ArgSource(Arc::new(f)));
        self
    }

    /// Recompute the localized value for this string based on the provided
    /// environment and data.
    pub fn resolve<'a>(&'a mut self, env: &Env, data: &T) -> &'a str {
        let args: Option<FluentArgs> = self
            .args
            .as_ref()
            .map(|a| a.iter().map(|(k, v)| (*k, (v.0)(env, data))).collect());
        self.resolved = env.localize(&self.key, args.as_ref());
        self.localized_str()
    }
}

impl<T> std::fmt::Debug for ArgSource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Arg Resolver {:p}", self.0)
    }
}
