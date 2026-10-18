use kalandar_dates::capture::{find_holidays, HolidayScope, Settings};

fn main() {
    let scope = HolidayScope::from_settings(&Settings::default(), Some("America/Los_Angeles"));
    for text in std::env::args().skip(1) {
        for m in find_holidays(&text, scope) {
            println!(
                "{text:?}: {} {}..{} foreign={} region={} next={:?}",
                m.holiday.name(),
                m.start,
                m.end,
                m.foreign,
                m.holiday.region_code(),
                m.holiday.next_on_or_after(
                    chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap_or_default()
                )
            );
        }
    }
}
