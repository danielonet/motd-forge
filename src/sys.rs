//! Thin libc bindings. std already links libc, so this adds no dependency.
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long};

#[repr(C)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
#[repr(C)]
struct StatVfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
    spare: [c_int; 16], // glibc/musl use 6; extra room is harmless
}

extern "C" {
    fn geteuid() -> u32;
    fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    fn statvfs(path: *const c_char, buf: *mut StatVfs) -> c_int;
}

pub fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions.
    unsafe { geteuid() == 0 }
}

/// Used percentage of the filesystem holding `path` (same formula as `df`'s used/total).
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub fn disk_used_percent(path: &str) -> Option<f64> {
    let c = CString::new(path).ok()?;
    let mut s = std::mem::MaybeUninit::<StatVfs>::zeroed();
    // SAFETY: c is a valid NUL-terminated string and s points to a writable StatVfs.
    let rc = unsafe { statvfs(c.as_ptr(), s.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: statvfs succeeded and fully initialised the struct (it was zeroed anyway).
    let s = unsafe { s.assume_init() };
    let total = s.f_blocks as f64 * s.f_frsize as f64;
    let used = s.f_blocks.saturating_sub(s.f_bfree) as f64 * s.f_frsize as f64;
    (total > 0.0).then(|| 100.0 * used / total)
}

#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
pub fn disk_used_percent(_path: &str) -> Option<f64> {
    let _ = CString::new("");
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Date {
        Date { year, month, day }
    }

    /// "MM-DD", for comparing against seasonal ranges.
    pub fn md(&self) -> String {
        format!("{:02}-{:02}", self.month, self.day)
    }

    /// Proleptic Gregorian ordinal (0001-01-01 = 1), the same numbering as Python's date.toordinal().
    pub fn ordinal(&self) -> i64 {
        let (y, m, d) = (self.year as i64, self.month as i64, self.day as i64);
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468 + 719163
    }
}

/// Seconds since the Unix epoch.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Today's local date.
pub fn today() -> Date {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut tm = std::mem::MaybeUninit::<Tm>::zeroed();
    // SAFETY: both pointers are valid; localtime_r fills tm on success.
    let ok = unsafe { !localtime_r(&secs, tm.as_mut_ptr()).is_null() };
    if ok {
        // SAFETY: localtime_r succeeded.
        let tm = unsafe { tm.assume_init() };
        Date::new(tm.tm_year + 1900, (tm.tm_mon + 1) as u32, tm.tm_mday as u32)
    } else {
        Date::new(1970, 1, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_matches_python() {
        assert_eq!(Date::new(1, 1, 1).ordinal(), 1);
        assert_eq!(Date::new(1970, 1, 1).ordinal(), 719163);
        assert_eq!(Date::new(2026, 11, 20).ordinal(), 739940);
    }
}
