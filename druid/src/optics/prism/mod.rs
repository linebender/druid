#[allow(clippy::module_inception)]
mod prism;
pub use prism::Variant;
#[doc(hidden)]
pub use prism::{DefaultUpgrade, Prism, PrismExt, PrismWrap, /*PrismRefReplacer,*/ Replace};
pub use prism::{Deref, Id, InArc, Index, Map, Then};
