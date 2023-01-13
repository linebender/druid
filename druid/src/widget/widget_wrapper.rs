// Copyright 2021 The Druid Authors.
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

/// A trait for widgets that wrap a single child to expose that child for access and mutation
pub trait WidgetWrapper {
    /// The type of the wrapped widget.
    /// Maybe we would like to constrain this to `Widget<impl Data>` (if existential bounds were supported).
    /// Any other scheme leads to `T` being unconstrained in unification at some point
    type Wrapped;
    /// Get immutable access to the wrapped child
    fn wrapped(&self) -> &Self::Wrapped;
    /// Get mutable access to the wrapped child
    fn wrapped_mut(&mut self) -> &mut Self::Wrapped;
}

/// A macro to help implementation of WidgetWrapper for a direct wrapper.
/// Use it in the body of the impl.
///
#[macro_export]
macro_rules! widget_wrapper_body {
    ($wrapped:ty, $field:ident) => {
        type Wrapped = $wrapped;

        fn wrapped(&self) -> &Self::Wrapped {
            &self.$field
        }

        fn wrapped_mut(&mut self) -> &mut Self::Wrapped {
            &mut self.$field
        }
    };
}

/// A macro to help implementation of WidgetWrapper for a wrapper of a typed pod.
/// Use it in the body of the impl.
///
#[macro_export]
macro_rules! widget_wrapper_pod_body {
    ($wrapped:ty, $field:ident) => {
        type Wrapped = $wrapped;

        fn wrapped(&self) -> &Self::Wrapped {
            self.$field.widget()
        }

        fn wrapped_mut(&mut self) -> &mut Self::Wrapped {
            self.$field.widget_mut()
        }
    };
}
