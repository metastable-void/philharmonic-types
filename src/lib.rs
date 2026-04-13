pub(crate) mod id;
pub(crate) mod jcs;
pub(crate) mod sha256;
pub(crate) mod timestamp;

use std::borrow::Cow;

pub use sha256::Sha256;

pub use id::{Id, IdKindError, InternalId, PublicId};
pub use jcs::{CanonError, CanonicalJson};
pub use timestamp::UnixMillis;
pub use uuid::Uuid;

pub trait Content: Sized {
    type Error: std::error::Error + Send + Sync + 'static;
    fn to_bytes(&'_ self) -> Cow<'_, [u8]>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;
}
