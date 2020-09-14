#[allow(clippy::module_inception)]
mod prism;
pub use prism::Variant;
#[doc(hidden)]
pub use prism::{DefaultUpgrade, PartialPrism, /*PrismRefReplacer,*/ Prism, PrismExt, PrismWrap,};
pub use prism::{Deref, Id, InArc, Index, Map, Then};
