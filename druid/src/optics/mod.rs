#![allow(missing_docs)]

#[allow(clippy::module_inception)]
#[macro_use]
pub mod lens;
#[doc(hidden)]
pub use lens::{Lens, LensExt};

#[allow(clippy::module_inception)]
#[macro_use]
pub mod prism;
#[doc(hidden)]
pub use prism::{DefaultUpgrade, PartialPrism, Prism, PrismExt};

#[allow(clippy::module_inception)]
pub mod affine_traversal;
