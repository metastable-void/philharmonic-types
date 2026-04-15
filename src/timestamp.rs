use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UnixMillis(pub i64);

impl UnixMillis {
    pub fn now() -> Self {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch");
        Self(d.as_millis() as i64)
    }

    pub const fn as_i64(&self) -> i64 {
        self.0
    }
}
