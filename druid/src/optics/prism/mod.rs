#[allow(clippy::module_inception)]
mod prism;
// pub use prism::{Deref, Field, Id, InArc, Index, Map, Then};
pub use prism::Variant;
#[doc(hidden)]
pub use prism::{DefaultUpgrade, Prism, PrismExt, PrismWrap, /*PrismRefReplacer,*/ Replace};
