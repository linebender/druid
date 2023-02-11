// Copyright 2022 The Druid Authors.
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

pub fn locale() -> String {
    fn locale_env_var(var: &str) -> Option<String> {
        match std::env::var(var) {
            Ok(s) if s.is_empty() => {
                tracing::debug!("locale: ignoring empty env var {}", var);
                None
            }
            Ok(s) => {
                tracing::debug!("locale: env var {} found: {:?}", var, &s);
                Some(s)
            }
            Err(std::env::VarError::NotPresent) => {
                tracing::debug!("locale: env var {} not found", var);
                None
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                tracing::debug!("locale: ignoring invalid unicode env var {}", var);
                None
            }
        }
    }

    // from gettext manual
    // https://www.gnu.org/software/gettext/manual/html_node/Locale-Environment-Variables.html#Locale-Environment-Variables
    let mut locale = locale_env_var("LANGUAGE")
        // the LANGUAGE value is priority list separated by :
        // See: https://www.gnu.org/software/gettext/manual/html_node/The-LANGUAGE-variable.html#The-LANGUAGE-variable
        .and_then(|locale| locale.split(':').next().map(String::from))
        .or_else(|| locale_env_var("LC_ALL"))
        .or_else(|| locale_env_var("LC_MESSAGES"))
        .or_else(|| locale_env_var("LANG"))
        .unwrap_or_else(|| "en-US".to_string());

    // This is done because the locale parsing library we use expects an unicode locale, but these vars have an ISO locale
    if let Some(idx) = locale.chars().position(|c| c == '.' || c == '@') {
        locale.truncate(idx);
    }
    locale
}
