// =============================================================================
// Florynx Kernel — CMOS Real-Time Clock Driver
// =============================================================================
// Reads wall-clock time from the PC CMOS RTC via I/O ports 0x70/0x71.
// Provides boot-time epoch calibration for the system clock.
// =============================================================================

use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicU64, Ordering};

const CMOS_ADDR: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x02;
const REG_HOURS:   u8 = 0x04;
const REG_DAY:     u8 = 0x07;
const REG_MONTH:   u8 = 0x08;
const REG_YEAR:    u8 = 0x09;
const REG_STATUS_A: u8 = 0x0A;
const REG_STATUS_B: u8 = 0x0B;

/// Wall-clock seconds at kernel boot (Unix epoch, set by init_rtc).
static BOOT_EPOCH: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct RtcTime {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

/// Read a single CMOS register.
fn cmos_read(reg: u8) -> u8 {
    let mut addr_port: Port<u8> = Port::new(CMOS_ADDR);
    let mut data_port: Port<u8> = Port::new(CMOS_DATA);
    unsafe {
        addr_port.write(reg & 0x7F); // Bit 7 = NMI disable; keep it clear
        data_port.read()
    }
}

/// Wait until the RTC update-in-progress flag clears (max ~2ms).
fn wait_not_updating() {
    for _ in 0..1_000_000u32 {
        if cmos_read(REG_STATUS_A) & 0x80 == 0 {
            return;
        }
    }
}

/// Convert BCD byte to binary.
#[inline]
fn bcd_to_bin(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd >> 4) * 10)
}

/// Read the current RTC time (retries until two consecutive reads agree).
pub fn read_rtc() -> RtcTime {
    let status_b = cmos_read(REG_STATUS_B);
    let is_binary = status_b & 0x04 != 0;
    let is_24h    = status_b & 0x02 != 0;

    let mut last = read_raw(is_binary, is_24h);
    loop {
        wait_not_updating();
        let current = read_raw(is_binary, is_24h);
        if current.seconds == last.seconds
            && current.minutes == last.minutes
            && current.hours == last.hours
        {
            return current;
        }
        last = current;
    }
}

fn read_raw(is_binary: bool, is_24h: bool) -> RtcTime {
    wait_not_updating();
    let mut secs  = cmos_read(REG_SECONDS);
    let mut mins  = cmos_read(REG_MINUTES);
    let mut hours = cmos_read(REG_HOURS);
    let day       = cmos_read(REG_DAY);
    let month     = cmos_read(REG_MONTH);
    let year_raw  = cmos_read(REG_YEAR);

    if !is_binary {
        secs  = bcd_to_bin(secs);
        mins  = bcd_to_bin(mins);
        let pm = hours & 0x80 != 0;
        hours = bcd_to_bin(hours & 0x7F);
        if !is_24h && pm && hours != 12 { hours += 12; }
        if !is_24h && !pm && hours == 12 { hours = 0; }
    }

    // Assume 21st century for years 00-99.
    let year = 2000u16 + year_raw as u16;

    RtcTime { seconds: secs, minutes: mins, hours, day, month, year }
}

/// Convert an RtcTime to a Unix timestamp (seconds since 1970-01-01 00:00:00 UTC).
/// Uses a simple Gregorian approximation — accurate to within a day.
pub fn rtc_to_unix(t: &RtcTime) -> u64 {
    // Days from 1970 to the start of the year
    let y = t.year as u64;
    let m = t.month as u64;
    let d = t.day as u64;

    // Cumulative days per month (non-leap)
    const DAYS_PER_MONTH: [u64; 12] = [0,31,59,90,120,151,181,212,243,273,304,334];
    let leap = if y % 400 == 0 { 1u64 }
               else if y % 100 == 0 { 0 }
               else if y % 4 == 0 { 1 }
               else { 0 };

    // Days from epoch to Jan 1 of `y`
    let years_from_epoch = y.saturating_sub(1970);
    let leap_years = (years_from_epoch + 1) / 4
                   - (years_from_epoch + 69) / 100
                   + (years_from_epoch + 369) / 400;
    let days_from_epoch = years_from_epoch * 365 + leap_years;

    // Days within the year
    let month_days = if m > 0 { DAYS_PER_MONTH[(m - 1) as usize] } else { 0 };
    let leap_day = if m > 2 { leap } else { 0 };
    let day_of_year = month_days + leap_day + d.saturating_sub(1);

    let total_days = days_from_epoch + day_of_year;
    total_days * 86400
        + t.hours as u64 * 3600
        + t.minutes as u64 * 60
        + t.seconds as u64
}

/// Initialize the RTC subsystem: read current time and store boot epoch.
pub fn init() {
    let t = read_rtc();
    let epoch = rtc_to_unix(&t);
    BOOT_EPOCH.store(epoch, Ordering::Relaxed);
    crate::serial_println!(
        "[rtc] {:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC  (epoch={})",
        t.year, t.month, t.day,
        t.hours, t.minutes, t.seconds,
        epoch
    );
}

/// Return the Unix timestamp at boot.
pub fn boot_epoch() -> u64 {
    BOOT_EPOCH.load(Ordering::Relaxed)
}

/// Return the current best-guess Unix timestamp
/// (boot RTC + PIT uptime seconds).
pub fn now_unix() -> u64 {
    boot_epoch() + crate::drivers::timer::pit::uptime_seconds()
}

/// Return current wall-clock time by advancing from boot RTC.
pub fn now_rtc() -> RtcTime {
    let unix = now_unix();
    unix_to_rtc(unix)
}

/// Minimal Unix → calendar conversion (UTC, no DST).
pub fn unix_to_rtc(unix: u64) -> RtcTime {
    let total_secs = unix;
    let secs   = (total_secs % 60) as u8;
    let mins   = ((total_secs / 60) % 60) as u8;
    let hours  = ((total_secs / 3600) % 24) as u8;

    let total_days = total_secs / 86400;

    // Gregorian calendar: find year/month/day
    let mut y = 1970u16;
    let mut days_left = total_days;
    loop {
        let year_days = if (y % 400 == 0) || (y % 4 == 0 && y % 100 != 0) { 366u64 } else { 365 };
        if days_left < year_days { break; }
        days_left -= year_days;
        y += 1;
    }

    let leap = (y % 400 == 0) || (y % 4 == 0 && y % 100 != 0);
    const MONTH_DAYS: [u64; 12] = [31,28,31,30,31,30,31,31,30,31,30,31];
    let mut m = 0u8;
    for i in 0..12 {
        let md = MONTH_DAYS[i] + if i == 1 && leap { 1 } else { 0 };
        if days_left < md { m = i as u8 + 1; break; }
        days_left -= md;
    }
    let day = days_left as u8 + 1;

    RtcTime { seconds: secs, minutes: mins, hours, day, month: m, year: y }
}
