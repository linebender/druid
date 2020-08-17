#[allow(clippy::module_inception)]
mod prism;
// pub use prism::{Deref, Field, Id, InArc, Index, Map, Then};
#[doc(hidden)]
pub use prism::{Prism, PrismExt, PrismWrap};
