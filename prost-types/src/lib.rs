#![doc(html_root_url = "https://docs.rs/prost-types/0.7.0")]

//! Protocol Buffers well-known types.
//!
//! Note that the documentation for the types defined in this crate are generated from the Protobuf
//! definitions, so code examples are not in Rust.
//!
//! See the [Protobuf reference][1] for more information about well-known types.
//!
//! [1]: https://developers.google.com/protocol-buffers/docs/reference/google.protobuf

#![cfg_attr(not(feature = "std"), no_std)]

use core::convert::TryFrom;
use core::i32;
use core::i64;
use core::time;

include!("protobuf.rs");
pub mod compiler {
    include!("compiler.rs");
}

// The Protobuf `Duration` and `Timestamp` types can't delegate to the standard library equivalents
// because the Protobuf versions are signed. To make them easier to work with, `From` conversions
// are defined in both directions.

const NANOS_PER_SECOND: i32 = 1_000_000_000;

impl Duration {
    /// Normalizes the duration to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L79-L100
    pub fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            self.seconds += (self.nanos / NANOS_PER_SECOND) as i64;
            self.nanos %= NANOS_PER_SECOND;
        }

        // nanos should have the same sign as seconds.
        if self.seconds < 0 && self.nanos > 0 {
            self.seconds += 1;
            self.nanos -= NANOS_PER_SECOND;
        } else if self.seconds > 0 && self.nanos < 0 {
            self.seconds -= 1;
            self.nanos += NANOS_PER_SECOND;
        }
        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -315_576_000_000 && self.seconds <= 315_576_000_000,
        //               "invalid duration: {:?}", self);
    }
}

/// Converts a `std::time::Duration` to a `Duration`.
impl From<time::Duration> for Duration {
    fn from(duration: time::Duration) -> Duration {
        let seconds = duration.as_secs();
        let seconds = if seconds > i64::MAX as u64 {
            i64::MAX
        } else {
            seconds as i64
        };
        let nanos = duration.subsec_nanos();
        let nanos = if nanos > i32::MAX as u32 {
            i32::MAX
        } else {
            nanos as i32
        };
        let mut duration = Duration { seconds, nanos };
        duration.normalize();
        duration
    }
}

impl TryFrom<Duration> for time::Duration {
    type Error = time::Duration;

    /// Converts a `Duration` to a result containing a positive (`Ok`) or negative (`Err`)
    /// `std::time::Duration`.
    fn try_from(mut duration: Duration) -> Result<time::Duration, time::Duration> {
        duration.normalize();
        if duration.seconds >= 0 {
            Ok(time::Duration::new(
                duration.seconds as u64,
                duration.nanos as u32,
            ))
        } else {
            Err(time::Duration::new(
                (-duration.seconds) as u64,
                (-duration.nanos) as u32,
            ))
        }
    }
}

impl Timestamp {
    /// Normalizes the timestamp to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L59-L77
    #[cfg(feature = "std")]
    pub fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            self.seconds += (self.nanos / NANOS_PER_SECOND) as i64;
            self.nanos %= NANOS_PER_SECOND;
        }

        // For Timestamp nanos should be in the range [0, 999999999].
        if self.nanos < 0 {
            self.seconds -= 1;
            self.nanos += NANOS_PER_SECOND;
        }

        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -62_135_596_800 && self.seconds <= 253_402_300_799,
        //               "invalid timestamp: {:?}", self);
    }
}

#[cfg(feature = "std")]
impl From<std::time::SystemTime> for Timestamp {
    fn from(system_time: std::time::SystemTime) -> Timestamp {
        let (seconds, nanos) = match system_time.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                (seconds, duration.subsec_nanos() as i32)
            }
            Err(error) => {
                let duration = error.duration();
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                let nanos = duration.subsec_nanos() as i32;
                if nanos == 0 {
                    (-seconds, 0)
                } else {
                    (-seconds - 1, 1_000_000_000 - nanos)
                }
            }
        };
        Timestamp { seconds, nanos }
    }
}

#[cfg(feature = "std")]
impl From<Timestamp> for std::time::SystemTime {
    fn from(mut timestamp: Timestamp) -> std::time::SystemTime {
        timestamp.normalize();
        let system_time = if timestamp.seconds >= 0 {
            std::time::UNIX_EPOCH + time::Duration::from_secs(timestamp.seconds as u64)
        } else {
            std::time::UNIX_EPOCH - time::Duration::from_secs((-timestamp.seconds) as u64)
        };

        system_time + time::Duration::from_nanos(timestamp.nanos as u64)
    }
}

/// Converts a `chrono::DateTime` to a `Timestamp`.
#[cfg(feature = "chrono-conversions")]
impl<Tz: chrono::TimeZone> From<chrono::DateTime<Tz>> for Timestamp {
    fn from(dt: chrono::DateTime<Tz>) -> Self {
        Self {
            seconds: dt.timestamp() as _,
            nanos: dt.timestamp_subsec_nanos() as _,
        }
    }
}

/// Converts a `Timestamp` to a `chrono::DateTime`.
#[cfg(feature = "chrono-conversions")]
impl Into<chrono::DateTime<chrono::Utc>> for Timestamp {
    fn into(self) -> chrono::DateTime<chrono::Utc> {
        use chrono::TimeZone;
        chrono::Utc.timestamp(self.seconds, self.nanos as _)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use proptest::prelude::*;

    use super::*;

    #[cfg(feature = "std")]
    proptest! {
        #[test]
        fn check_system_time_roundtrip(
            system_time in SystemTime::arbitrary(),
        ) {
            prop_assert_eq!(SystemTime::from(Timestamp::from(system_time)), system_time);
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn check_timestamp_negative_seconds() {
        // Representative tests for the case of timestamps before the UTC Epoch time:
        // validate the expected behaviour that "negative second values with fractions
        // must still have non-negative nanos values that count forward in time"
        // https://developers.google.com/protocol-buffers/docs/reference/google.protobuf#google.protobuf.Timestamp
        //
        // To ensure cross-platform compatibility, all nanosecond values in these
        // tests are in minimum 100 ns increments.  This does not affect the general
        // character of the behaviour being tested, but ensures that the tests are
        // valid for both POSIX (1 ns precision) and Windows (100 ns precision).
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(1_001, 0)),
            Timestamp {
                seconds: -1_001,
                nanos: 0
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(0, 999_999_900)),
            Timestamp {
                seconds: -1,
                nanos: 100
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(2_001_234, 12_300)),
            Timestamp {
                seconds: -2_001_235,
                nanos: 999_987_700
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(768, 65_432_100)),
            Timestamp {
                seconds: -769,
                nanos: 934_567_900
            }
        );
    }

    #[cfg(all(unix, feature = "std"))]
    #[test]
    fn check_timestamp_negative_seconds_1ns() {
        // UNIX-only test cases with 1 ns precision
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(0, 999_999_999)),
            Timestamp {
                seconds: -1,
                nanos: 1
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(1_234_567, 123)),
            Timestamp {
                seconds: -1_234_568,
                nanos: 999_999_877
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(890, 987_654_321)),
            Timestamp {
                seconds: -891,
                nanos: 12_345_679
            }
        );
    }

    #[cfg(feature = "chrono-conversions")]
    #[test]
    fn test_datetime_to_wkt_timestamp() {
        use chrono::{DateTime, TimeZone, Utc};

        let date_utc = Utc.ymd(2014, 7, 8).and_hms_nano(9, 10, 11, 12);
        let ts0: Timestamp = date_utc.into();
        let expected0 = Timestamp {
            seconds: 1404810611,
            nanos: 12,
        };
        assert_eq!(ts0, expected0);

        let date_6 = DateTime::parse_from_rfc2822("8 Jul 2014 09:10:11 +0600").unwrap();
        let ts6: Timestamp = date_6.into();
        let expected6 = Timestamp {
            seconds: 1404810611 - 6 * 3600,
            nanos: 0,
        };
        assert_eq!(ts6, expected6);
    }

    #[cfg(feature = "chrono-conversions")]
    #[test]
    fn test_wkt_timestamp_to_datetime() {
        use chrono::{DateTime, TimeZone, Utc};

        let ts = Timestamp {
            seconds: 1404810611,
            nanos: 12,
        };
        let expected_date = Utc.ymd(2014, 7, 8).and_hms_nano(9, 10, 11, 12);
        let date: DateTime<Utc> = ts.into();
        assert_eq!(date, expected_date);
    }
}
