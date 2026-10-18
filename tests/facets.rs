#![cfg(feature = "facets")]

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_english_rs::facets::{
    resolve_capture, resolve_line, resolve_phrase, resolve_unlabelled, task_signal, Alert,
    CaptureKind, Clock, Context, LabelledSpan, LineResolution, Moment, Role, Settings, WarningCode,
};

use CaptureKind::{CalendarEvent as Cal, Reminder as Rem, Task};

fn now() -> DateTime<Utc> {
    "2026-10-18T17:00:00Z".parse().expect("now")
}

fn ctx() -> Context {
    Context::new(now(), chrono_tz::America::Los_Angeles)
}

fn ctx_with(settings: Settings) -> Context {
    ctx().with_settings(settings)
}

fn hm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).expect("time")
}

fn show(m: &Option<Moment>) -> String {
    match m {
        Some(m) if m.all_day => m.date.format("%Y-%m-%d").to_string(),
        Some(m) => m
            .local
            .map(|l| l.format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_default(),
        None => "-".to_string(),
    }
}

fn start(kind: CaptureKind, line: &str) -> String {
    show(&resolve_unlabelled(line, kind, &ctx()).start)
}

fn start_with(kind: CaptureKind, line: &str, c: &Context) -> String {
    show(&resolve_unlabelled(line, kind, c).start)
}

fn warnings(r: &LineResolution) -> Vec<WarningCode> {
    let mut out: Vec<WarningCode> = r.warnings.iter().map(|w| w.code).collect();
    for m in [&r.start, &r.end].into_iter().flatten() {
        out.extend(m.warnings.iter().map(|w| w.code));
    }
    if let Some(rec) = &r.recurrence {
        out.extend(rec.warnings.iter().map(|w| w.code));
    }
    out
}

fn has_warning(kind: CaptureKind, line: &str, code: WarningCode) -> bool {
    warnings(&resolve_unlabelled(line, kind, &ctx())).contains(&code)
}

fn rrule(kind: CaptureKind, line: &str) -> String {
    resolve_unlabelled(line, kind, &ctx())
        .recurrence
        .map(|r| r.rrule)
        .unwrap_or_default()
}

#[test]
fn role_defaults() {
    let rows = [
        (Cal, "lunch at 1", "2026-10-18T13:00"),
        (Cal, "lunch tomorrow at 1", "2026-10-19T13:00"),
        (Cal, "dinner at 7", "2026-10-18T19:00"),
        (Cal, "dentist Friday", "2026-10-23"),
        (Cal, "meeting at 3", "2026-10-18T15:00"),
        (Task, "file taxes Friday", "2026-10-23"),
        (Task, "send report by 5pm", "2026-10-18T17:00"),
        (Rem, "remind me to call Dad Friday", "2026-10-23"),
        (Rem, "remind me at 9 to stretch", "2026-10-19T09:00"),
        (Rem, "remind me to call Dad Sunday", "2026-10-18"),
        (
            Rem,
            "remind me to move the laundry in 40 minutes",
            "2026-10-18T10:40",
        ),
        (
            Rem,
            "remind me to water the plants tomorrow morning",
            "2026-10-19T09:00",
        ),
        (Task, "pay rent on the 1st", "2026-11-01"),
        (Task, "update resume by end of month", "2026-10-31"),
        (Task, "pick up dry cleaning after work", "2026-10-18T18:00"),
        (Task, "send weekly update end of day", "2026-10-18T17:00"),
        (
            Rem,
            "remind me to defrost the chicken this afternoon",
            "2026-10-18T14:00",
        ),
        (Rem, "remind me to lock up tonight", "2026-10-18T20:00"),
        (Cal, "Anna's wedding June 21st", "2027-06-21"),
        (Cal, "team offsite next Wednesday", "2026-10-21"),
    ];
    for (kind, line, expected) in rows {
        assert_eq!(start(kind, line), expected, "{line}");
    }
}

#[test]
fn guesses_are_reported() {
    assert!(has_warning(
        Rem,
        "remind me at 9 to stretch",
        WarningCode::RolledToTomorrow
    ));
    assert!(has_warning(
        Rem,
        "remind me at 9 to stretch",
        WarningCode::AmbiguousMeridiem
    ));
    assert!(!has_warning(
        Task,
        "file taxes Friday",
        WarningCode::TimeAssumed
    ));
    assert!(has_warning(
        Rem,
        "remind me to call Dad Friday",
        WarningCode::NoTime
    ));
    assert!(has_warning(
        Rem,
        "remind me to buy stamps before Sunday",
        WarningCode::MovedToNextWeek
    ));
    assert!(has_warning(
        Cal,
        "dentist Friday",
        WarningCode::AllDayAssumed
    ));
    assert!(has_warning(
        Cal,
        "meeting at 3",
        WarningCode::AmbiguousMeridiem
    ));
    assert!(!has_warning(
        Cal,
        "meeting at 3pm",
        WarningCode::AmbiguousMeridiem
    ));
    let r = resolve_unlabelled("dinner Friday", Cal, &ctx());
    let start = r.start.expect("start");
    assert!(start.implied.contains(&"time"));
    assert!(!start.implied.contains(&"date"));
}

#[test]
fn activity_default_times() {
    let rows = [
        ("breakfast with Jo tomorrow", "2026-10-19T08:00"),
        ("brunch Saturday", "2026-10-24T11:00"),
        ("lunch with the team Friday", "2026-10-23T12:00"),
        ("afternoon tea Thursday", "2026-10-22T15:00"),
        ("dinner Friday", "2026-10-23T18:00"),
        ("supper on Monday", "2026-10-19T18:00"),
        ("drinks Friday", "2026-10-23T21:00"),
        ("lunch Friday at 1", "2026-10-23T13:00"),
    ];
    for (line, expected) in rows {
        assert_eq!(start(Cal, line), expected, "{line}");
    }
    let settings = Settings {
        dinner: hm(19, 30),
        ..Settings::default()
    };
    assert_eq!(
        start_with(Cal, "dinner Friday", &ctx_with(settings)),
        "2026-10-23T19:30"
    );
}

#[test]
fn holidays() {
    let rows = [
        ("Christmas", "2026-12-25"),
        ("xmas party", "2026-12-25"),
        ("Christmas Eve dinner at 6", "2026-12-24T18:00"),
        ("New Year's Eve", "2026-12-31"),
        ("new years day", "2027-01-01"),
        ("Thanksgiving", "2026-11-26"),
        ("Black Friday", "2026-11-27"),
        ("Halloween party", "2026-10-31"),
        ("Hallowe'en", "2026-10-31"),
        ("Easter", "2027-03-28"),
        ("Good Friday", "2027-03-26"),
        ("Mardi Gras", "2027-02-09"),
        ("Mother's Day brunch", "2027-05-09T11:00"),
        ("Mothers Day", "2027-05-09"),
        ("Father’s Day", "2027-06-20"),
        ("Valentines day", "2027-02-14"),
        ("St. Patrick's Day", "2027-03-17"),
        ("Labor Day", "2027-09-06"),
        ("Labour Day", "2027-09-06"),
        ("Memorial Day", "2027-05-31"),
        ("Presidents' Day", "2027-02-15"),
        ("Presidents Day", "2027-02-15"),
        ("Boxing Day", "2026-12-26"),
        ("Canada Day", "2027-07-01"),
        ("Christmas 2027", "2027-12-25"),
    ];
    for (line, expected) in rows {
        assert_eq!(start(Cal, line), expected, "{line}");
    }
}

#[test]
fn time_phrases() {
    let rows = [
        (Rem, "remind me at quarter past 3", "2026-10-18T15:15"),
        (Rem, "remind me at half past 4", "2026-10-18T16:30"),
        (Rem, "remind me at quarter to 5", "2026-10-18T16:45"),
        (Rem, "remind me at 3 o'clock", "2026-10-18T15:00"),
        (Rem, "remind me at 3 o’clock", "2026-10-18T15:00"),
        (Rem, "remind me at 3 oclock", "2026-10-18T15:00"),
        (Rem, "remind me in half an hour", "2026-10-18T10:30"),
        (Rem, "remind me in a half hour", "2026-10-18T10:30"),
        (Rem, "remind me in an hour", "2026-10-18T11:00"),
        (Rem, "remind me in an hour and a half", "2026-10-18T11:30"),
        (Task, "renew passport in a fortnight", "2026-11-01"),
        (Task, "reply in 2 weekdays", "2026-10-20"),
        (Task, "reply in 3 business days", "2026-10-21"),
        (Rem, "remind me at five", "2026-10-18T17:00"),
    ];
    for (kind, line, expected) in rows {
        assert_eq!(start(kind, line), expected, "{line}");
    }
    let summer = resolve_unlabelled("road trip this summer", Cal, &ctx());
    assert_eq!(show(&summer.start), "2027-06-01");
    assert_eq!(show(&summer.end), "2027-08-31");
    let winter = resolve_unlabelled("ski trip in winter", Cal, &ctx());
    assert_eq!(show(&winter.start), "2026-12-01");
    assert_eq!(show(&winter.end), "2027-02-28");
}

#[test]
fn approximate_times() {
    assert_eq!(start(Cal, "call Sam around 3"), "2026-10-18T15:00");
    assert!(has_warning(
        Cal,
        "call Sam around 3",
        WarningCode::ApproximateTime
    ));
    assert_eq!(start(Cal, "coffee 3ish"), "2026-10-18T15:00");
    assert!(has_warning(
        Cal,
        "coffee 3ish",
        WarningCode::ApproximateTime
    ));
    assert!(!has_warning(
        Cal,
        "coffee at 3pm",
        WarningCode::ApproximateTime
    ));
}

#[test]
fn ranges_and_ends() {
    let r = resolve_unlabelled("party Friday 9pm to 1am", Cal, &ctx());
    assert_eq!(show(&r.start), "2026-10-23T21:00");
    assert_eq!(show(&r.end), "2026-10-24T01:00");
    let r = resolve_unlabelled("demo day next Friday 9-12", Cal, &ctx());
    assert_eq!(show(&r.start), "2026-10-23T09:00");
    assert_eq!(show(&r.end), "2026-10-23T12:00");
    let r = resolve_unlabelled("wine tasting Saturday 4-6pm", Cal, &ctx());
    assert_eq!(show(&r.start), "2026-10-24T16:00");
    assert_eq!(show(&r.end), "2026-10-24T18:00");
    let r = resolve_unlabelled("1:1 Tuesday at 10am for 30 minutes", Cal, &ctx());
    assert_eq!(show(&r.end), "2026-10-20T10:30");
    let r = resolve_unlabelled("conference from the 8th to the 10th", Cal, &ctx());
    assert_eq!(show(&r.start), "2026-11-08");
    assert_eq!(show(&r.end), "2026-11-10");
    let r = resolve_unlabelled("call at 3pm", Cal, &ctx());
    assert_eq!(show(&r.end), "2026-10-18T16:00");
    assert!(warnings(&r).contains(&WarningCode::EndAssumed));
}

#[test]
fn alerts() {
    let r = resolve_unlabelled(
        "dentist Friday at 3pm, remind me 30 minutes before",
        Cal,
        &ctx(),
    );
    match r.alert {
        Some(Alert::Relative { minutes_before, at }) => {
            assert_eq!(minutes_before, 30);
            assert_eq!(show(&at), "2026-10-23T14:30");
        }
        other => panic!("expected relative alert, got {other:?}"),
    }
    let r = resolve_unlabelled("standup tomorrow at 9am, alert at 8:45am", Cal, &ctx());
    assert_eq!(show(&r.start), "2026-10-19T09:00");
    match r.alert {
        Some(Alert::Absolute { at }) => assert_eq!(show(&Some(at)), "2026-10-19T08:45"),
        other => panic!("expected absolute alert, got {other:?}"),
    }
    for line in [
        "team sync Monday at 10, no alert",
        "team sync Monday at 10 alerts off",
    ] {
        assert_eq!(
            resolve_unlabelled(line, Cal, &ctx()).alert,
            Some(Alert::Off),
            "{line}"
        );
    }
    let r = resolve_unlabelled(
        "remind me to leave for the dentist 30 minutes before",
        Rem,
        &ctx(),
    );
    assert_eq!(r.lead_minutes, Some(30));
}

#[test]
fn task_signals() {
    let rows = [
        ("todo buy milk", Some("todo")),
        ("TODO: buy milk", Some("todo")),
        ("to-do call Sam", Some("to-do")),
        ("task: file the report", Some("task")),
        ("remember to call Sam", Some("remember to")),
        ("remind me to call Sam", Some("remind me to")),
        ("✓ buy milk", Some("✓")),
        ("[ ] buy milk", Some("[ ]")),
        ("buy milk", None),
        ("call the taskforce", None),
    ];
    for (line, expected) in rows {
        assert_eq!(task_signal(line).as_deref(), expected, "{line}");
    }
}

#[test]
fn user_settings() {
    let h24 = ctx_with(Settings {
        clock: Clock::TwentyFourHour,
        ..Settings::default()
    });
    assert_eq!(start_with(Cal, "meeting at 3", &ctx()), "2026-10-18T15:00");
    assert_eq!(start_with(Cal, "meeting at 3", &h24), "2026-10-19T03:00");
    assert_eq!(start_with(Cal, "lunch at 1", &h24), "2026-10-18T13:00");

    let sunday_weeks = ctx_with(Settings {
        week_start: Weekday::Sun,
        ..Settings::default()
    });
    assert_eq!(
        start_with(Cal, "offsite next Wednesday", &ctx()),
        "2026-10-21"
    );
    assert_eq!(
        start_with(Cal, "offsite next Wednesday", &sunday_weeks),
        "2026-10-28"
    );
    assert_eq!(
        start_with(Task, "send report next week", &ctx()),
        "2026-10-19"
    );
    assert_eq!(
        start_with(Task, "send report next week", &sunday_weeks),
        "2026-10-25"
    );

    let dayparts = ctx_with(Settings {
        morning: hm(7, 30),
        afternoon: hm(13, 0),
        evening: hm(18, 0),
        tonight: hm(21, 0),
        after_work: hm(17, 30),
        ..Settings::default()
    });
    assert_eq!(
        start_with(Rem, "remind me tomorrow morning", &dayparts),
        "2026-10-19T07:30"
    );
    assert_eq!(
        start_with(Rem, "remind me this afternoon", &dayparts),
        "2026-10-18T13:00"
    );
    assert_eq!(
        start_with(Rem, "remind me this evening", &dayparts),
        "2026-10-18T18:00"
    );
    assert_eq!(
        start_with(Rem, "remind me tonight", &dayparts),
        "2026-10-18T21:00"
    );
    assert_eq!(
        start_with(Task, "pick up parcel after work", &dayparts),
        "2026-10-18T17:30"
    );

    let gulf_weekend = ctx_with(Settings {
        weekend: vec![Weekday::Fri, Weekday::Sat],
        ..Settings::default()
    });
    assert_eq!(start_with(Cal, "picnic this weekend", &ctx()), "2026-10-24");
    assert_eq!(
        start_with(Cal, "picnic this weekend", &gulf_weekend),
        "2026-10-23"
    );
    assert_eq!(
        rrule_with(Task, "clean bathroom every weekend", &gulf_weekend),
        "FREQ=WEEKLY;INTERVAL=1;BYDAY=FR,SA"
    );

    let short_events = ctx_with(Settings {
        default_event_minutes: 30,
        ..Settings::default()
    });
    assert_eq!(
        show(&resolve_unlabelled("call at 3pm", Cal, &short_events).end),
        "2026-10-18T15:30"
    );

    let late_night: DateTime<Utc> = "2026-10-18T08:00:00Z".parse().expect("now");
    let night_owl =
        Context::new(late_night, chrono_tz::America::Los_Angeles).with_settings(Settings {
            day_start: hm(4, 0),
            ..Settings::default()
        });
    assert_eq!(
        start_with(Cal, "dentist tomorrow", &night_owl),
        "2026-10-18"
    );
    let midnight_boundary = Context::new(late_night, chrono_tz::America::Los_Angeles);
    assert_eq!(
        start_with(Cal, "dentist tomorrow", &midnight_boundary),
        "2026-10-19"
    );
}

fn rrule_with(kind: CaptureKind, line: &str, c: &Context) -> String {
    resolve_unlabelled(line, kind, c)
        .recurrence
        .map(|r| r.rrule)
        .unwrap_or_default()
}

#[test]
fn recurrence_rules() {
    let rows = [
        (
            "remind me every Monday",
            "FREQ=WEEKLY;INTERVAL=1;BYDAY=MO",
            "2026-10-19",
        ),
        (
            "remind me every other Tuesday",
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU",
            "2026-10-20",
        ),
        (
            "remind me every first Monday",
            "FREQ=MONTHLY;INTERVAL=1;BYDAY=1MO",
            "2026-11-02",
        ),
        (
            "remind me every last Friday of the month",
            "FREQ=MONTHLY;INTERVAL=1;BYDAY=-1FR",
            "2026-10-30",
        ),
        (
            "remind me every Monday and Wednesday",
            "FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE",
            "2026-10-19",
        ),
        (
            "remind me Mon Wed Fri at 7am",
            "FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR",
            "2026-10-19T07:00",
        ),
        (
            "remind me every 15th",
            "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=15",
            "2026-11-15",
        ),
        (
            "remind me on the last day of every month",
            "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=-1",
            "2026-10-31",
        ),
        (
            "remind me every workday at 8:30am",
            "FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR",
            "2026-10-19T08:30",
        ),
        (
            "remind me every December 20",
            "FREQ=YEARLY;INTERVAL=1;BYMONTHDAY=20;BYMONTH=12",
            "2026-12-20",
        ),
        (
            "remind me every day at 8pm",
            "FREQ=DAILY;INTERVAL=1",
            "2026-10-18T20:00",
        ),
        (
            "remind me every 3 months",
            "FREQ=MONTHLY;INTERVAL=3",
            "2027-01-18",
        ),
        (
            "remind me every year",
            "FREQ=YEARLY;INTERVAL=1",
            "2027-10-18",
        ),
    ];
    for (line, rule, first) in rows {
        let r = resolve_unlabelled(line, Rem, &ctx());
        let rec = r
            .recurrence
            .unwrap_or_else(|| panic!("no recurrence for {line}"));
        assert_eq!(rec.rrule, rule, "{line}");
        assert_eq!(show(&rec.first), first, "{line}");
    }
    let facets = resolve_unlabelled("remind me every Monday and Wednesday", Rem, &ctx())
        .recurrence
        .and_then(|r| r.facets)
        .expect("facets shape");
    assert_eq!(facets.frequency, "weekly");
    assert_eq!(facets.interval, 1);
    assert_eq!(facets.days, Some(vec!["monday", "wednesday"]));
    assert!(
        resolve_unlabelled("remind me every first Monday", Rem, &ctx())
            .recurrence
            .and_then(|r| r.facets)
            .is_none()
    );
}

#[test]
fn recurrence_bounds() {
    let r = resolve_unlabelled("remind me every Monday for 6 weeks", Rem, &ctx())
        .recurrence
        .expect("rule");
    assert_eq!(show(&r.first), "2026-10-19");
    assert_eq!(
        r.end_date.map(|d| d.to_string()),
        Some("2026-11-30".to_string())
    );
    assert_eq!(r.rrule, "FREQ=WEEKLY;INTERVAL=1;BYDAY=MO;COUNT=6");

    let r = resolve_unlabelled("remind me every day 10 times", Rem, &ctx())
        .recurrence
        .expect("rule");
    assert_eq!(r.rule.count, Some(10));

    let r = resolve_unlabelled("yoga every Monday at 6pm starting October 21", Cal, &ctx());
    let rec = r.recurrence.expect("rule");
    assert_eq!(
        rec.start_date.map(|d| d.to_string()),
        Some("2026-10-21".to_string())
    );
    assert_eq!(show(&rec.first), "2026-10-26T18:00");
    assert!(rec
        .warnings
        .iter()
        .any(|w| w.code == WarningCode::StartAdjusted));

    let r = resolve_unlabelled("mow the lawn every Saturday until October", Task, &ctx())
        .recurrence
        .expect("rule");
    assert_eq!(
        r.end_date.map(|d| d.to_string()),
        Some("2027-09-30".to_string())
    );
    assert!(r.warnings.iter().any(|w| w.code == WarningCode::EndAssumed));

    let r = resolve_unlabelled(
        "spin class every Tuesday at 6am except holidays",
        Cal,
        &ctx(),
    )
    .recurrence
    .expect("rule");
    assert_eq!(r.exceptions.len(), 1);
    assert_eq!(r.exceptions[0].raw, "except holidays");
    assert_eq!(r.exceptions[0].condition.as_deref(), Some("holidays"));

    let r = resolve_unlabelled(
        "1:1 prep every Thursday but skip the first one",
        Task,
        &ctx(),
    );
    assert_eq!(show(&r.start), "2026-10-29");
}

#[test]
fn dst_gaps_and_overlaps_keep_the_date() {
    let r = resolve_unlabelled("flight March 14 2027 at 2:30am", Cal, &ctx());
    let start = r.start.expect("start");
    assert_eq!(
        start.instant.map(|i| i.to_rfc3339()),
        Some("2027-03-14T03:30:00-07:00".to_string())
    );
    assert!(start
        .warnings
        .iter()
        .any(|w| w.code == WarningCode::DstGapShifted));

    let r = resolve_unlabelled("flight November 7 2027 at 1:30am", Cal, &ctx());
    let start = r.start.expect("start");
    assert_eq!(
        start.instant.map(|i| i.to_rfc3339()),
        Some("2027-11-07T01:30:00-07:00".to_string())
    );
    assert!(start
        .warnings
        .iter()
        .any(|w| w.code == WarningCode::DstOverlapEarlier));

    let london = Context::new(now(), chrono_tz::Europe::London);
    let r = resolve_unlabelled("call March 28 2027 at 1:30am", Cal, &london);
    assert_eq!(
        r.start.and_then(|s| s.instant).map(|i| i.to_rfc3339()),
        Some("2027-03-28T02:30:00+01:00".to_string())
    );
}

#[test]
fn spans_from_the_model() {
    let line = "lunch with Sam tomorrow at 1";
    let spans = [
        LabelledSpan::new(Role::EventStart, 15, 23),
        LabelledSpan::new(Role::EventStart, 24, 28),
    ];
    let r = resolve_line(line, Cal, &spans, &ctx());
    assert_eq!(show(&r.start), "2026-10-19T13:00");
    assert_eq!(show(&r.end), "2026-10-19T14:00");

    let only_title = [LabelledSpan::new(Role::EventStart, 0, 5)];
    assert_eq!(
        show(&resolve_line(line, Cal, &only_title, &ctx()).start),
        "-"
    );

    let fragments = [
        LabelledSpan::new(Role::Trigger, 44, 48),
        LabelledSpan::new(Role::Recurrence, 49, 59),
        LabelledSpan::new(Role::Recurrence, 60, 70),
    ];
    let line = "remind me to pay the credit card on the last day of every month";
    let r = resolve_capture(line, Rem, &fragments, &ctx());
    assert_eq!(
        r.recurrence.map(|r| r.rrule).as_deref(),
        Some("FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=-1")
    );

    let r = resolve_phrase("next Friday at 3", Role::EventStart, Cal, &ctx());
    assert_eq!(show(&r.start), "2026-10-23T15:00");
}

#[test]
fn bare_ordinal_ranges_and_days() {
    assert_eq!(
        start(Cal, "parent-teacher conference on the 14th at 4:15"),
        "2026-11-14T16:15"
    );
    assert_eq!(start(Cal, "housewarming the 22nd at 5"), "2026-10-22T17:00");
    let r = resolve_unlabelled("off from the 20th to the 3rd", Cal, &ctx());
    assert_eq!(show(&r.start), "2026-10-20");
    assert_eq!(show(&r.end), "2026-11-03");
}

#[test]
fn typos_and_shorthand() {
    let rows = [
        (Task, "pay water bill wednsday", "2026-10-21"),
        (Task, "buy milk 2moro", "2026-10-19"),
        (Cal, "gym thurs 7a", "2026-10-22T07:00"),
        (Cal, "sync @ 4", "2026-10-18T16:00"),
        (Rem, "remind me to feed the fish 2nite", "2026-10-18T20:00"),
    ];
    for (kind, line, expected) in rows {
        assert_eq!(start(kind, line), expected, "{line}");
    }
}

#[test]
fn rrule_shortcut() {
    assert_eq!(
        rrule(Rem, "take meds at 8 every day"),
        "FREQ=DAILY;INTERVAL=1"
    );
    assert_eq!(rrule(Rem, "take meds"), "");
}

#[test]
fn holiday_forms_resolve_to_their_dates() {
    use chrono_english_rs::facets::{find_holidays, HolidayScope};
    let us = HolidayScope::from_settings(&Settings::default(), Some("America/Los_Angeles"));
    let cases = [
        ("nye", "2026-12-31", false),
        ("new years eve", "2026-12-31", false),
        ("New Year's Day", "2027-01-01", false),
        ("valentines", "2027-02-14", false),
        ("st. patrick's day", "2027-03-17", false),
        ("hallowe'en", "2026-10-31", false),
        ("xmas eve", "2026-12-24", false),
        ("x-mas", "2026-12-25", false),
        ("christmas", "2026-12-25", false),
        ("boxing day", "2026-12-26", true),
        ("canada day", "2027-07-01", true),
        ("presidents' day", "2027-02-15", false),
        ("memorial day", "2027-05-31", false),
        ("mother's day", "2027-05-09", false),
        ("father's day", "2027-06-20", false),
        ("labor day", "2027-09-06", false),
        ("thanksgiving", "2026-11-26", false),
        ("easter sunday", "2027-03-28", false),
        ("good friday", "2027-03-26", true),
        ("diwali", "2026-11-08", true),
    ];
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("today");
    for (text, date, foreign) in cases {
        let found = find_holidays(text, us);
        assert_eq!(found.len(), 1, "{text}");
        assert_eq!((found[0].start, found[0].end), (0, text.len()), "{text}");
        assert_eq!(found[0].foreign, foreign, "{text}");
        assert_eq!(
            found[0]
                .holiday
                .next_on_or_after(today)
                .map(|d| d.to_string()),
            Some(date.to_string()),
            "{text}"
        );
    }
    assert!(find_holidays("christmas", HolidayScope::Off).is_empty());
}

#[test]
fn holidays_are_always_dates() {
    for (text, kind, want) in [
        ("buy Christmas lights", Task, "2026-12-25"),
        ("Halloween costume ideas", Task, "2026-10-31"),
        ("plan Easter egg hunt", Task, "2027-03-28"),
        ("remind me to wrap Christmas presents", Rem, "2026-12-25"),
        ("Christmas card list by dec 20", Task, "2026-12-20"),
    ] {
        let r = resolve_unlabelled(text, kind, &ctx());
        assert_eq!(show(&r.start), want, "{text}");
    }
}

#[test]
fn phase4b_warning_precision() {
    let codes = |text: &str, kind: CaptureKind| -> Vec<WarningCode> {
        resolve_unlabelled(text, kind, &ctx())
            .warnings
            .iter()
            .map(|w| w.code)
            .collect()
    };
    assert!(
        !codes("remind me to buy stamps when I get to the post office", Rem)
            .contains(&WarningCode::MissingTrigger)
    );
    assert!(codes("remind me to buy stamps", Rem).contains(&WarningCode::MissingTrigger));
    assert!(!codes("standup at 9:30", Cal).contains(&WarningCode::MissingDate));
    assert!(codes("team offsite", Cal).contains(&WarningCode::MissingDate));
    assert!(
        codes("send invoice after the meeting", Task).contains(&WarningCode::UnresolvedReference)
    );
    assert!(!codes("send invoice before 5pm", Task).contains(&WarningCode::UnresolvedReference));
    assert!(!codes("go for a run after work", Task).contains(&WarningCode::UnresolvedReference));
    assert!(codes("tidy the shed at some point", Task).contains(&WarningCode::VagueTime));
    assert!(!codes("call the bank later today", Task).contains(&WarningCode::VagueTime));
    assert!(!codes("yoga tomorrow evening", Cal).contains(&WarningCode::VagueTime));
    assert!(codes("gym at 6 then dinner at 8", Cal).contains(&WarningCode::MultipleItems));
    assert!(!codes("swim Monday and Wednesday at 7am", Cal).contains(&WarningCode::MultipleItems));
    assert!(codes("water the ferns every so often", Task)
        .contains(&WarningCode::UnderspecifiedRecurrence));
    assert!(codes("check the smoke alarm from time to time", Task)
        .contains(&WarningCode::UnderspecifiedRecurrence));
    assert!(!codes("water the ferns every 3 days", Task)
        .contains(&WarningCode::UnderspecifiedRecurrence));
}

#[test]
fn date_only_dues() {
    let r = resolve_unlabelled("file taxes Friday", Task, &ctx());
    let due = r.start.as_ref().expect("due");
    assert!(due.all_day);
    assert_eq!(due.local, None);
    assert_eq!(due.date.to_string(), "2026-10-23");
    assert!(warnings(&r).is_empty(), "{:?}", warnings(&r));
    assert_eq!(
        r.notify_at.map(|t| t.to_string()),
        Some("2026-10-23 09:00:00".to_string())
    );

    let late = Settings {
        default_notification_time: hm(7, 30),
        ..Settings::default()
    };
    let r = resolve_unlabelled("file taxes Friday", Task, &ctx_with(late));
    assert_eq!(show(&r.start), "2026-10-23");
    assert_eq!(
        r.notify_at.map(|t| t.to_string()),
        Some("2026-10-23 07:30:00".to_string())
    );

    let r = resolve_unlabelled("remind me to call Dad Friday", Rem, &ctx());
    assert_eq!(show(&r.start), "2026-10-23");
    assert!(r.start.as_ref().is_some_and(|s| s.all_day));
    let codes = warnings(&r);
    assert!(codes.contains(&WarningCode::NoTime));
    assert!(!codes.contains(&WarningCode::TimeAssumed));

    let r = resolve_unlabelled("offsite Friday", Cal, &ctx());
    assert!(r.start.as_ref().is_some_and(|s| s.all_day));
    assert_eq!(r.notify_at, None);

    assert_eq!(start(Task, "file taxes Friday at 3pm"), "2026-10-23T15:00");
    assert_eq!(start(Task, "file taxes Friday eod"), "2026-10-23T17:00");
    assert_eq!(
        start(Rem, "remind me to call Dad Friday at 6pm"),
        "2026-10-23T18:00"
    );
    assert_eq!(
        start(Task, "pay rent on the 1st of every month"),
        "2026-11-01"
    );
}
