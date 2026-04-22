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

#[cfg(test)]
mod tests {
    use super::*;

    // `SystemTime::now()` is blocked by miri's default isolation (it
    // forwards to `clock_gettime(REALTIME)`). Gate the tests that call
    // `UnixMillis::now()` with `#[cfg(not(miri))]` so miri still runs the
    // other in-memory timestamp tests; contributors can opt back in with
    // `MIRIFLAGS=-Zmiri-disable-isolation ./scripts/miri-test.sh
    // philharmonic-types` if they want full coverage.
    #[test]
    #[cfg(not(miri))]
    fn now_returns_positive_value() {
        assert!(UnixMillis::now().as_i64() > 0);
    }

    #[test]
    #[cfg(not(miri))]
    fn now_is_monotonic_under_sleep() {
        let a = UnixMillis::now();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = UnixMillis::now();
        assert!(
            b.as_i64() >= a.as_i64(),
            "expected b ({}) >= a ({})",
            b.as_i64(),
            a.as_i64(),
        );
    }

    #[test]
    fn as_i64_returns_wrapped_value() {
        assert_eq!(UnixMillis(42).as_i64(), 42);
    }

    #[test]
    fn serde_is_transparent_over_i64() {
        let t = UnixMillis(1_700_000_000_000);
        let json = serde_json::to_string(&t).unwrap();
        // `#[serde(transparent)]` — the wrapping struct disappears.
        assert_eq!(json, "1700000000000");
        let back: UnixMillis = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn ord_impl_matches_i64_order() {
        assert!(UnixMillis(100) < UnixMillis(200));
        assert!(UnixMillis(-1) < UnixMillis(0));
        assert_eq!(UnixMillis(5).cmp(&UnixMillis(5)), std::cmp::Ordering::Equal,);
    }

    #[test]
    fn derived_traits_cover_expected_shape() {
        let t = UnixMillis(12345);
        let copied = t;
        let cloned = Clone::clone(&t);
        assert_eq!(t, copied);
        assert_eq!(t, cloned);
        // Debug should render the i64 somewhere in its output.
        assert!(format!("{t:?}").contains("12345"));
    }
}
