// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

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
