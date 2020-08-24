#[allow(clippy::module_inception)]
pub mod lens;
#[doc(hidden)]
pub use lens::{Lens, LensExt, LensWrap};

#[allow(clippy::module_inception)]
pub mod prism;
#[doc(hidden)]
pub use prism::{Prism, PrismExt, PrismRefReplacer, PrismReplacer, PrismWrap};

#[allow(clippy::module_inception)]
pub mod traversal;
