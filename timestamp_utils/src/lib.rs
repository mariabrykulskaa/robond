//! Граничные значения `prost_types::Timestamp` и конвертация `prost_types::Timestamp` ↔ `chrono::DateTime<Utc>`.

use prost_types::Timestamp;

use chrono::{DateTime, Utc};
use std::time::SystemTime;

pub const MIN_TIMESTAMP: Timestamp = Timestamp {
    seconds: -62135596800,
    nanos: 0,
};

pub const MAX_TIMESTAMP: Timestamp = Timestamp {
    seconds: 253402300799,
    nanos: 999_999_999,
};

pub fn timestamp_to_datetime(ts: Timestamp) -> DateTime<Utc> {
    let st: SystemTime = ts.try_into().unwrap();
    st.into()
}

pub fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp::from(SystemTime::from(dt))
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{TimeZone, Timelike};

    fn min_timestamp() -> Timestamp {
        let datetime = Utc.with_ymd_and_hms(1, 1, 1, 0, 0, 0).unwrap();
        datetime_to_timestamp(datetime)
    }

    fn max_timestamp() -> Timestamp {
        let datetime = Utc
            .with_ymd_and_hms(9999, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(999_999_999)
            .unwrap();
        datetime_to_timestamp(datetime)
    }

    #[test]
    fn test_min_timestamp() {
        assert_eq!(MIN_TIMESTAMP, min_timestamp());
    }

    #[test]
    fn test_max_timestamp() {
        assert_eq!(MAX_TIMESTAMP, max_timestamp());
    }
}
