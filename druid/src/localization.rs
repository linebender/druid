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

//! Localization handling.
//!
//! Localization is backed by [Fluent], via [fluent-rs].
//!
//! In Druid, the main way you will deal with localization is via the
//! [`LocalizedString`] struct.
//!
//! You construct a [`LocalizedString`] with a key, which identifies a 'message'
//! in your `.flt` files. If your string requires arguments, you supply it with
//! closures that can extract those arguments from the current [`Env`] and
//! [`Data`].
//!
//! At runtime, you resolve your [`LocalizedString`] into an actual string,
//! passing it the current [`Env`] and [`Data`].
//!
//!
//! [Fluent]: https://projectfluent.org
//! [fluent-rs]: https://github.com/projectfluent/fluent-rs
//! [`LocalizedString`]: struct.LocalizedString.html
//! [`Env`]: struct.Env.html
//! [`Data`]: trait.Data.html

use std::collections::HashMap;
use std::sync::Arc;
use std::{fs, io};

use log::{debug, error, warn};

use crate::{Application, ArcStr, Env};

use fluent_bundle::{
    FluentArgs, FluentBundle, FluentError, FluentMessage, FluentResource, FluentValue,
};
use fluent_langneg::{negotiate_languages, NegotiationStrategy};
use fluent_syntax::ast::Pattern as FluentPattern;
use unic_langid::LanguageIdentifier;

// Localization looks for string files in druid/resources, but this path is hardcoded;
// it will only work if you're running an example from the druid/ directory.
// At some point we will need to bundle strings with applications, and choose
// the path dynamically.
static FALLBACK_STRINGS: &str = include_str!("../resources/i18n/en-US/builtin.ftl");

/// Provides access to the localization strings for the current locale.
#[allow(dead_code)]
pub(crate) struct L10nManager {
    // these two are not currently used; will be used when we let the user
    // add additional localization files.
    res_mgr: ResourceManager,
    resources: Vec<String>,
    current_bundle: BundleStack,
    current_locale: LanguageIdentifier,
}

/// Manages a collection of localization files.
struct ResourceManager {
    resources: HashMap<String, Arc<FluentResource>>,
    locales: Vec<LanguageIdentifier>,
    default_locale: LanguageIdentifier,
    path_scheme: String,
}

//NOTE: instead of a closure, at some point we can use something like a lens for this.
//TODO: this is an Arc so that it can be clone, which is a bound on things like `Menu`.
/// A closure that generates a localization value.
type ArgClosure<T> = Arc<dyn Fn(&T, &Env) -> FluentValue<'static> + 'static>;

/// Wraps a closure that generates an argument for localization.
#[derive(Clone)]
struct ArgSource<T>(ArgClosure<T>);

/// A string that can be localized based on the current locale.
///
/// At its simplest, a `LocalizedString` is a key that can be resolved
/// against a map of localized strings for a given locale.
#[derive(Debug, Clone)]
pub struct LocalizedString<T> {
    pub(crate) key: &'static str,
    placeholder: Option<ArcStr>,
    args: Option<Vec<(&'static str, ArgSource<T>)>>,
    resolved: Option<ArcStr>,
    resolved_lang: Option<LanguageIdentifier>,
}

/// A stack of localization resources, used for fallback.
struct BundleStack(Vec<FluentBundle<Arc<FluentResource>>>);

impl BundleStack {
    fn get_message(&self, id: &str) -> Option<FluentMessage> {
        self.0.iter().flat_map(|b| b.get_message(id)).next()
    }

    fn format_pattern(
        &self,
        id: &str,
        pattern: &FluentPattern,
        args: Option<&FluentArgs>,
        errors: &mut Vec<FluentError>,
    ) -> String {
        for bundle in self.0.iter() {
            if bundle.has_message(id) {
                return bundle.format_pattern(pattern, args, errors).to_string();
            }
        }
        format!("localization failed for key '{}'", id)
    }
}

//NOTE: much of this is adapted from https://github.com/projectfluent/fluent-rs/blob/master/fluent-resmgr/src/resource_manager.rs
impl ResourceManager {
    /// Loads a new localization resource from disk, as needed.
    fn get_resource(&mut self, res_id: &str, locale: &str) -> Arc<FluentResource> {
        let path = self
            .path_scheme
            .replace("{locale}", locale)
            .replace("{res_id}", res_id);
        if let Some(res) = self.resources.get(&path) {
            res.clone()
        } else {
            let string = fs::read_to_string(&path).unwrap_or_else(|_| {
                if (res_id, locale) == ("builtin.ftl", "en-US") {
                    FALLBACK_STRINGS.to_string()
                } else {
                    error!("missing resouce {}/{}", locale, res_id);
                    String::new()
                }
            });
            let res = match FluentResource::try_new(string) {
                Ok(res) => Arc::new(res),
                Err((res, _err)) => Arc::new(res),
            };
            self.resources.insert(path, res.clone());
            res
        }
    }

    /// Return the best localization bundle for the provided `LanguageIdentifier`.
    fn get_bundle(&mut self, locale: &LanguageIdentifier, resource_ids: &[String]) -> BundleStack {
        let resolved_locales = self.resolve_locales(locale.clone());
        debug!("resolved: {}", PrintLocales(resolved_locales.as_slice()));
        let mut stack = Vec::new();
        for locale in &resolved_locales {
            let mut bundle = FluentBundle::new(&resolved_locales);
            for res_id in resource_ids {
                let res = self.get_resource(&res_id, &locale.to_string());
                bundle.add_resource(res).unwrap();
            }
            stack.push(bundle);
        }
        BundleStack(stack)
    }

    /// Given a locale, returns the best set of available locales.
    pub(crate) fn resolve_locales(&self, locale: LanguageIdentifier) -> Vec<LanguageIdentifier> {
        negotiate_languages(
            &[locale],
            &self.locales,
            Some(&self.default_locale),
            NegotiationStrategy::Filtering,
        )
        .into_iter()
        .map(|l| l.to_owned())
        .collect()
    }
}

impl L10nManager {
    /// Create a new localization manager.
    ///
    /// `resources` is a list of file names that contain strings. `base_dir`
    /// is a path to a directory that includes per-locale subdirectories.
    ///
    /// This directory should be of the structure `base_dir/{locale}/{resource}`,
    /// where '{locale}' is a valid BCP47 language tag, and {resource} is a `.ftl`
    /// included in `resources`.
    pub fn new(resources: Vec<String>, base_dir: &str) -> Self {
        fn get_available_locales(base_dir: &str) -> Result<Vec<LanguageIdentifier>, io::Error> {
            let mut locales = vec![];

            let res_dir = fs::read_dir(base_dir)?;
            for entry in res_dir {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(name) = path.file_name() {
                            if let Some(name) = name.to_str() {
                                let langid: LanguageIdentifier =
                                    name.parse().expect("Parsing failed.");
                                locales.push(langid);
                            }
                        }
                    }
                }
            }
            Ok(locales)
        }

        let default_locale: LanguageIdentifier =
            "en-US".parse().expect("failed to parse default locale");
        let current_locale = Application::get_locale()
            .parse()
            .unwrap_or_else(|_| default_locale.clone());
        let locales = get_available_locales(base_dir).unwrap_or_default();
        debug!(
            "available locales {}, current {}",
            PrintLocales(&locales),
            current_locale,
        );
        let mut path_scheme = base_dir.to_string();
        path_scheme.push_str("/{locale}/{res_id}");

        let mut res_mgr = ResourceManager {
            resources: HashMap::new(),
            path_scheme,
            default_locale,
            locales,
        };

        let current_bundle = res_mgr.get_bundle(&current_locale, &resources);

        L10nManager {
            res_mgr,
            current_bundle,
            resources,
            current_locale,
        }
    }

    /// Fetch a localized string from the current bundle by key.
    ///
    /// In general, this should not be used directly; [`LocalizedString`]
    /// should be used for localization, and you should call
    /// [`LocalizedString::resolve`] to update the string as required.
    ///
    ///[`LocalizedString`]: struct.LocalizedString.html
    ///[`LocalizedString::resolve`]: struct.LocalizedString.html#method.resolve
    pub fn localize<'args>(
        &'args self,
        key: &str,
        args: impl Into<Option<&'args FluentArgs<'args>>>,
    ) -> Option<ArcStr> {
        let args = args.into();
        let value = match self
            .current_bundle
            .get_message(key)
            .and_then(|msg| msg.value)
        {
            Some(v) => v,
            None => return None,
        };
        let mut errs = Vec::new();
        let result = self
            .current_bundle
            .format_pattern(key, value, args, &mut errs);
        for err in errs {
            warn!("localization error {:?}", err);
        }

        // fluent inserts bidi controls when interpolating, and they can
        // cause rendering issues; for now we just strip them.
        // https://www.w3.org/International/questions/qa-bidi-unicode-controls#basedirection
        const START_ISOLATE: char = '\u{2068}';
        const END_ISOLATE: char = '\u{2069}';
        if args.is_some() && result.chars().any(|c| c == START_ISOLATE) {
            Some(
                result
                    .chars()
                    .filter(|c| c != &START_ISOLATE && c != &END_ISOLATE)
                    .collect::<String>()
                    .into(),
            )
        } else {
            Some(result.into())
        }
    }
    //TODO: handle locale change
}

impl<T> LocalizedString<T> {
    /// Create a new `LocalizedString` with the given key.
    pub const fn new(key: &'static str) -> Self {
        LocalizedString {
            key,
            args: None,
            placeholder: None,
            resolved: None,
            resolved_lang: None,
        }
    }

    /// Add a placeholder value. This will be used if localization fails.
    ///
    /// This is intended for use during prototyping.
    pub fn with_placeholder(mut self, placeholder: impl Into<ArcStr>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Return the localized value for this string, or the placeholder, if
    /// the localization is missing, or the key if there is no placeholder.
    pub fn localized_str(&self) -> ArcStr {
        self.resolved
            .clone()
            .or_else(|| self.placeholder.clone())
            .unwrap_or_else(|| self.key.into())
    }

    /// Add a named argument and a corresponding closure. This closure
    /// is a function that will return a value for the given key from the current
    /// environment and data.
    pub fn with_arg(
        mut self,
        key: &'static str,
        f: impl Fn(&T, &Env) -> FluentValue<'static> + 'static,
    ) -> Self {
        self.args
            .get_or_insert(Vec::new())
            .push((key, ArgSource(Arc::new(f))));
        self
    }

    /// Lazily compute the localized value for this string based on the provided
    /// environment and data.
    ///
    /// Returns `true` if the current value of the string has changed.
    pub fn resolve<'a>(&'a mut self, data: &T, env: &Env) -> bool {
        //TODO: this recomputes the string if either the language has changed,
        //or *anytime* we have arguments. Ideally we would be using a lens
        //to only recompute when our actual data has changed.
        if self.args.is_some()
            || self.resolved_lang.as_ref() != Some(&env.localization_manager().current_locale)
        {
            let args: Option<FluentArgs> = self
                .args
                .as_ref()
                .map(|a| a.iter().map(|(k, v)| (*k, (v.0)(data, env))).collect());

            self.resolved_lang = Some(env.localization_manager().current_locale.clone());
            let next = env.localization_manager().localize(self.key, args.as_ref());
            let result = next != self.resolved;
            self.resolved = next;
            result
        } else {
            false
        }
    }
}

impl<T> std::fmt::Debug for ArgSource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Arg Resolver {:p}", self.0)
    }
}

/// Helper to impl display for slices of displayable things.
struct PrintLocales<'a, T>(&'a [T]);

impl<'a, T: std::fmt::Display> std::fmt::Display for PrintLocales<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[")?;
        let mut prev = false;
        for l in self.0 {
            if prev {
                write!(f, ", ")?;
            }
            prev = true;
            write!(f, "{}", l)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolve() {
        let en_us: LanguageIdentifier = "en-US".parse().unwrap();
        let en_ca: LanguageIdentifier = "en-CA".parse().unwrap();
        let en_gb: LanguageIdentifier = "en-GB".parse().unwrap();
        let fr_fr: LanguageIdentifier = "fr-FR".parse().unwrap();
        let pt_pt: LanguageIdentifier = "pt-PT".parse().unwrap();

        let resmgr = ResourceManager {
            resources: HashMap::new(),
            locales: vec![en_us.clone(), en_ca.clone(), en_gb.clone(), fr_fr.clone()],
            default_locale: en_us.clone(),
            path_scheme: String::new(),
        };

        let en_za: LanguageIdentifier = "en-GB".parse().unwrap();
        let cn_hk: LanguageIdentifier = "cn-HK".parse().unwrap();
        let fr_ca: LanguageIdentifier = "fr-CA".parse().unwrap();

        assert_eq!(
            resmgr.resolve_locales(en_ca.clone()),
            vec![en_ca.clone(), en_us.clone(), en_gb.clone()]
        );
        assert_eq!(
            resmgr.resolve_locales(en_za),
            vec![en_gb, en_us.clone(), en_ca]
        );
        assert_eq!(
            resmgr.resolve_locales(fr_ca),
            vec![fr_fr.clone(), en_us.clone()]
        );
        assert_eq!(
            resmgr.resolve_locales(fr_fr.clone()),
            vec![fr_fr, en_us.clone()]
        );
        assert_eq!(resmgr.resolve_locales(cn_hk), vec![en_us.clone()]);
        assert_eq!(resmgr.resolve_locales(pt_pt), vec![en_us]);
    }
}
