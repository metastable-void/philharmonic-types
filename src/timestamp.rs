use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UnixMillis(pub i64);

impl UnixMillis {
    pub fn now() -> Self {
        // Narrow-exception panics (docs/design/13-conventions.md §Panics and
        // undefined behavior): both `.expect()` calls document unrecoverable
        // system-state failures — a clock set before 1970 or past year ~292M
        // represents a deeply broken system, not a recoverable runtime error.
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch");
        let millis = i64::try_from(d.as_millis())
            .expect("system time past year 292M — millisecond count exceeds i64");
        Self(millis)
    }

    pub const fn as_i64(&self) -> i64 {
        self.0
    }
}
