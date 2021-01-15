pub fn unix_timestamp() -> i64 {
    crate::arch::dev::rtc::get_unix_ts()
}
