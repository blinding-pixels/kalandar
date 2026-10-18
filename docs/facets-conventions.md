# Facets resolution conventions

These rules say what a date or time phrase in a Facets capture should resolve to. The `facets` module implements them, and the evaluation gold in `facets-eval/` was written from them by hand. Where a rule doesn't cover a phrase, the expected answer is what a reasonable person typing the line would mean.

## Context used for the evaluation

- Now: 2026-10-18 10:00 local time, America/Los_Angeles (PDT, UTC-07:00). That instant is 2026-10-18T17:00:00Z. 2026-10-18 is a **Sunday**.
- Settings are the defaults below.

## Settings and their defaults

| Setting | Default |
|---|---|
| Clock | 12-hour |
| Week start | Monday |
| Morning | 09:00 |
| Afternoon | 14:00 |
| Evening | 19:00 |
| Tonight / night | 20:00 |
| After work | 18:00 |
| End of day | 17:00 |
| Breakfast | 08:00 |
| Brunch | 11:00 |
| Lunch | 12:00 (after lunch 13:00, before lunch 12:00) |
| Afternoon tea | 15:00 |
| Dinner, supper | 18:00 |
| Drinks | 21:00 |
| Default notification time (`default_notification_time`): when the app notifies a date-only due or all-day reminder. It is passed through as `notify_at` and never becomes part of the due. | 09:00 |
| Weekend days | Saturday, Sunday |
| Day starts at | 00:00 |
| Default event length | 60 minutes |

Fixed words: noon and midday are 12:00. Midnight is 00:00 of the following day. "First thing" is the morning time.

Activity times (breakfast, brunch, lunch, afternoon tea, dinner, supper, drinks) apply when the line names the activity and gives no explicit time: "dinner Friday" is Friday 18:00. An explicit hour always wins: "lunch at 1" is 13:00, "dinner at 7" is 19:00.

"Day starts at" moves the boundary of "today": before that hour, today, tonight and tomorrow are counted from the previous calendar day. With the default 00:00 nothing moves.

## Output values

- A moment is a local date and time, written `YYYY-MM-DDTHH:MM`.
- An all-day value is a date, written `YYYY-MM-DD`. Only calendar events are all-day.
- A range has a start and an end.

## Dates

- Weekday name alone ("Friday", "on Friday", "this Friday", "this coming Friday"): the next such day whose resolved moment is not in the past. Today counts if the moment is still ahead.
- "next <weekday>": the weekday in the following week, where weeks start on the week-start setting. Today is Sunday, the last day of its Monday-start week, so "next Wednesday" is Wednesday 2026-10-21.
- "next week": the first day of the next week (Monday 2026-10-19). "this weekend": the coming Saturday. "end of week" / "by end of week": Friday of the current week, or of next week if the current week's Friday has passed.
- "end of the month", "end of month", "the last day of the month": the last day of the current month.
- "early next month": the 1st of next month. "sometime in March", "in June": the 1st of that month, flagged as a guess.
- Bare ordinal day ("on the 14th", "the 22nd", "by the 12th", "before the 28th"): the next date with that day number that is today or later. On 2026-10-18, the 14th is 2026-11-14 and the 22nd is 2026-10-22.
- Ordinal range without a month ("from the 8th to the 10th"): both days in the first month where the start day is today or later.
- Month and day with no year ("June 21st", "March 2"): the next such date, today or later.
- "in N minutes/hours/days/weeks": now plus that amount. A day-or-longer amount keeps the date only, with the role's default time.
- "by", "before" and "until" before a date or time give that date or time as the deadline.
- "in a fortnight" is 14 days. "a" and "an" mean one: "in an hour", "in a week".
- "this weekend", "next weekend": the coming weekend's first day (from the weekend-days setting). "weekend" and "weekday" also work as units: "every weekend", "in 2 weekdays" (counting only weekdays).
- Seasons are date ranges, northern hemisphere, by month: spring Mar 1 to May 31, summer Jun 1 to Aug 31, autumn or fall Sep 1 to Nov 30, winter Dec 1 to the end of February. "this summer" is the next or current one.

## Holidays

Holidays resolve to the next date (today or later) unless a year is given. Common spelling, spacing and apostrophe variants are accepted ("xmas", "New Years Eve", "St. Patrick's", "Mothers Day").

| Holiday | Date |
|---|---|
| New Year's Day / New Year's Eve | Jan 1 / Dec 31 |
| Valentine's Day | Feb 14 |
| St Patrick's Day | Mar 17 |
| Halloween | Oct 31 |
| Christmas Eve / Christmas | Dec 24 / Dec 25 |
| Boxing Day | Dec 26 |
| Canada Day | Jul 1 |
| Presidents' Day (US) | third Monday of February |
| Memorial Day (US) | last Monday of May |
| Mother's Day (US) | second Sunday of May |
| Father's Day (US) | third Sunday of June |
| Labor Day (US) | first Monday of September |
| Thanksgiving (US) | fourth Thursday of November |
| Black Friday | the day after Thanksgiving |
| Easter Sunday | computed (Gregorian computus) |
| Good Friday | two days before Easter |
| Mardi Gras | 47 days before Easter |

Lunar and Hebrew calendar holidays (Lunar New Year, Diwali, Hanukkah, Eid, Passover) are not resolved.

## Times

- An explicit am/pm or a 24-hour time (13:00, 09:30, 22:00) is taken as written.
- **Bare hour** (1 to 11, no am/pm, not written as 24-hour with a leading zero):
  1. Words in the line decide when present. Morning, breakfast or "first thing" mean am. Afternoon, evening, tonight, night, dinner, supper or drinks mean pm.
  2. Lunch or brunch: 11 is am, 12 is noon, 1 to 4 are pm.
  3. Otherwise 1 to 6 is pm and 7 to 11 is am, flagged "ambiguous am/pm".
  4. With the 24-hour clock setting, rule 3 does not apply and the hour is taken as written. Rules 1 and 2 still apply.
- "quarter past 3", "half past 3", "quarter to 4": 3:15, 3:30, 3:45, then the bare-hour rule. "3 o'clock" in any apostrophe form (o'clock, o’clock, oclock, o clock) is a bare 3.
- "half an hour", "a half hour" and "half hour" are 30 minutes.
- "around 3", "about 3", "3ish", "approximately 3": the time is used but flagged `approximate_time`.
- Dayparts: "tomorrow morning" is 09:00, "Friday afternoon" 14:00, "this evening" 19:00, "tonight" and "Sunday night" 20:00. "tonight at 6" is 18:00.
- A clock time with a date part uses that date. A time with no date is today, or tomorrow if that time has already passed today.
- Ranges: "14:00 to 15:30", "9-12", "4-6pm", "10 am to 11 am" give a start and an end. An end earlier than the start is on the next day. In "4-6pm" the pm applies to both ends. "9-12" is 09:00 to 12:00.
- Durations: "for 30 minutes" or "for an hour" set end = start + duration.
- An end is resolved against its start: "Friday 9pm to 1am" ends Saturday 01:00.

## Role defaults

- **Calendar event start**: a date with no time is an all-day event. A time with no date follows the time rule above.
- **Calendar event end**: an end time without a date is on the start's date, or the next day if earlier than the start. A timed event with no end or duration ends after the default event length (60 minutes), flagged `end_assumed`. All-day events get no end unless one is stated.
- **Task due** (changed by the owner's date-only decision; see the last section): a date with no time is a date-only due, a plain local date with no time and no `time_assumed`. A time with no date is today, or tomorrow if passed.
- **Reminder trigger** (changed by the date-only decision): a date with no time is an all-day reminder with `no_time`, notified at the default notification time, unless a daypart word gives the time. A time that has already passed today rolls to tomorrow. A reminder whose date is today and whose default 09:00 has passed moves to the next occurrence: for a weekday name, next week; for "today", it stays today and is flagged.
- **Lead time** ("30 minutes before", "the day before"): stored as minutes before the event it belongs to. The alert moment is the event start minus that offset. Without an event, only the offset is known.

## Alerts

Alerts are a separate output from the event time.

- "remind me 10 minutes before", "alert me an hour before", "notify me the day before": a relative alert, minutes before the start.
- "alert at 9am", "remind me at 8 that morning": an absolute alert at that moment.
- "no alert", "no reminder", "alerts off", "without an alert": alerts are turned off.

## Task signals

A line that starts with "todo", "to-do", "to do", "task", "remember to", "remind me to", "✓", "✔", "[ ]" or "[x]" carries a task signal. It is reported as a signal only and doesn't change routing.

## Recurrence

A recurrence has: frequency (`hourly`, `daily`, `weekly`, `monthly`, `yearly`), interval, weekdays (with an optional position such as first or last), month days (negative counts from the end, -1 is the last day), months, an optional count, start date, end date, and exceptions. Exceptions keep their raw text, an optional condition, and an optional action.

| Phrase | Rule |
|---|---|
| every day, daily, each day | daily, interval 1 |
| every other day | daily, interval 2 |
| every N days / weeks / months | interval N |
| every week, weekly | weekly, on the start's weekday |
| every other week, biweekly, fortnightly | weekly, interval 2 |
| every Monday, Mondays, on Mondays, each Monday | weekly, MO |
| every other Tuesday | weekly, interval 2, TU |
| every Monday and Wednesday, Mon Wed Fri | weekly, MO,WE / MO,WE,FR |
| every weekday, every workday, business days | weekly, MO,TU,WE,TH,FR |
| every weekend | weekly, SA,SU |
| every first Monday (of the month) | monthly, BYDAY=1MO |
| every last Friday | monthly, BYDAY=-1FR |
| every 15th, the 15th of every month, monthly on the 15th | monthly, month day 15 |
| last day of every month | monthly, month day -1 |
| every month, monthly | monthly, on the start's day |
| every quarter, quarterly | monthly, interval 3 |
| every year, yearly, annually | yearly |
| every December 20 | yearly, month 12, month day 20 |
| every hour, hourly | hourly |
| every morning / evening / night | daily, with that daypart's time |

- A time in the phrase ("every Monday at 9") is the occurrence time. The bare-hour rule applies.
- No time given: the repeat is all-day for every kind (date-only occurrences, no time in the RRULE). Reminders keep `no_time`; events keep `all_day_assumed` and `no_time`.
- Start ("starting next week", "from Monday", "beginning March 1"): the date the rule starts.
- End ("until October", "through December", "until the 30th"): "until <month>" ends on the last day of the month before that month, flagged as a guess. "until <date>" includes that date.
- "for 3 months", "for 6 weeks": end = start + that span, counted from the first occurrence. "10 times": count 10.
- The start must agree with the rule. If "every Monday starting October 21" starts on a Wednesday, the first occurrence is the first Monday on or after the start (October 26), flagged `start_adjusted`.
- Exceptions ("except holidays", "except on Fridays", "but skip the first one"): kept as raw text. Weekday exceptions also get condition `weekday:friday`.
- **First occurrence**: the earliest occurrence at or after now, and at or after the start date if one is given.

## Warnings

Every guess is reported: `time_assumed` (a default time was used), `date_assumed`, `day_assumed` (month only), `ambiguous_meridiem`, `rolled_to_tomorrow`, `moved_to_next_week`, `all_day_assumed`, `end_assumed`, `dst_gap_shifted`, `dst_overlap_earlier`, `approximate_time`, `start_adjusted`, `unparsed`.

# Phase 3 conventions

These rules are added on top of the ones above. Where they disagree, these win.

## Recurrence output

A recurrence now also has `hours` (for BYHOUR), a time window (`window_start`, `window_end`) for minutely and hourly rules, and `exdates`.

- **Count.** A repeat limited by a length or a number of times gets a COUNT, not an UNTIL. That covers "10 times", "for 4 sessions", "for 6 weeks", "for the next 3 months" and "for 3 days". The count is the exact number of occurrences inside [first occurrence, first occurrence + length).
  - Every Monday for 6 weeks: COUNT=6.
  - Every Monday and Wednesday for 2 weeks: COUNT=4.
  - Daily for 3 weeks: COUNT=21.
  - Monthly for the next 3 months: COUNT=3.
- **Until.** UNTIL is used only for "until/through/till <date>". The end date is still reported for both. For a length it is the first occurrence plus the length (the exclusive bound).
- **Minutely and hourly.** "every 15 minutes": minutely, interval 15. "every 2 hours": hourly, interval 2. "hourly between 9 and 5" (or "from 9 to 5", "9am-5pm") sets a window of 09:00 to 17:00.
  - Hourly rules get BYHOUR for every hour from the window start to the window end, both included (9 to 17).
  - Minutely rules get BYHOUR from the start hour up to the hour before the end hour (9 to 16).
  - Without a window, the first occurrence is now plus the interval, to the minute. With a window, it is the first slot inside the window at or after now plus the interval. Slots are counted from the window start.
- **Month days.** "on the 15th of every month": month day 15. "every 1st and 15th": month days 1 and 15. "last Friday of the month": monthly, -1FR.
- **Weekly lists.** "every mon/wed/fri", "mon, wed and fri", "weekdays", "mon thru fri", "monday to friday", "every other week on Tuesday" (weekly, interval 2, TU).
- **Repeat starts.** "starting next Monday", "from Nov 1", "beginning in January", "starting the 1st": the start date; "beginning in January" is January 1. The first occurrence is the first match on or after the start.
- **Exceptions become exdates.** Each exception is resolved to concrete dates that are occurrences of the rule. They are searched from the first occurrence to the end: the last counted occurrence, the UNTIL date, or one year after the first occurrence when the repeat is open-ended. Each exdate is a local date and time in the occurrence's time, or a date when the occurrence is all-day.
  - "except Dec 25": every Dec 25 in that span that is an occurrence.
  - "not on the 24th": every 24th of a month that is an occurrence.
  - "skip the week of the 10th": every occurrence in the week (by the week-start setting) that holds the next 10th, today or later.
  - "except holidays": every date in the holiday set (the holidays table above) that is an occurrence.
  - "except Fridays" on a weekly rule that lists Friday removes Friday from the weekdays instead of adding exdates.
  - An exception that matches no occurrence produces no exdates and keeps its raw text.
  - An excluded occurrence is not counted toward COUNT. The count stays the number the phrase asked for, and the first occurrence skips an excluded date.
- **Holiday repeats.** "every Thanksgiving": yearly, month 11, 4TH. "every Christmas": yearly, month 12, month day 25. "every Mother's Day": yearly, month 5, 2SU. A holiday with no fixed rule ("every Easter") is yearly with the first occurrence on the next Easter and a `holiday_dates` warning.
- **biweekly.** "biweekly" and "bi-weekly" are read as every two weeks, with an `ambiguous_biweekly` warning that names twice a week as the alternative.

## Weekdays

Weeks start on the week-start setting, Monday by default.

- **Bare weekday, "on Friday", "this Friday", "this coming Friday":** the next such day, today or later. Today counts if the resolved time is still ahead; otherwise it is next week, with `moved_to_next_week`.
- **"next Friday":** the Friday of the next week.
  - If that Friday is further away than the very next Friday, because Friday is still ahead in the current week, the phrase is ambiguous. The answer is still the next-week Friday, with an `ambiguous_next_weekday` warning, and the alternative is the nearer Friday.
  - Said on a Wednesday (Mon-start weeks), "next Friday" is 9 days later, with the alternative 2 days later.
  - Said on a Sunday, "next Friday" is 5 days later. That is also the very next Friday, so there is no warning.
- **"last Friday":** the most recent Friday before today.
- **"next Friday" said on a Friday** is the Friday of the next week, with no warning.
- **"this weekend" / "next weekend":** "this weekend" is the coming first weekend day. On a weekend day it is the coming Saturday, as before. "next weekend" is the first weekend day of the next week.
- **Past midnight.** A time from 00:00 to 04:59 attached to "tonight" or "<day> night" falls on the following calendar day ("tonight at 1am", "Friday night at 2" is Saturday 02:00). A time-only phrase that has already passed today rolls to tomorrow, as before.

## Explicit time zones

"3pm EST", "9am London time", "noon PT", "17:00 UTC", "10am Pacific", "2pm Eastern time" and "8pm CET" are read in the named zone and converted into the user's zone. The result's local time is in the user's zone, with a `converted_timezone` warning that keeps the original wording and the zone.

- Fixed abbreviations (EST, PST, EDT, PDT, BST, UTC, GMT and the rest of chrono-node's table) use their fixed offset, as in chrono-node.
- CET, ET, CT, MT and PT follow daylight saving time, as in chrono-node: "8pm CET" on a date in summer time is 20:00 CEST (UTC+2). This was changed after the first round of phase 3 rows.
- Region names (Pacific, Eastern, Central, Mountain) and city names ("London time", "New York time", "Paris time", "Berlin time", "Tokyo time", "Sydney time", "Chicago time", "LA time", "Denver time", "India time", "Singapore time", "Dubai time") use that place's real zone on that date.

## 24-hour times

"17:30", "0930", "1700h", "1700 hrs", "21h", "21h30", "at 0800" and ranges like "0900-1700" or "13:00 to 14:30" are taken as written.

- A 4-digit time counts only when it is a valid time (00 to 23, then 00 to 59) and either follows at/by/from/until/before/after, carries an "h"/"hrs" suffix, or is part of a range.
- A 4-digit number that is a year ("in 2027") is not a time.

## Reminder lead times

"30 min before", "an hour beforehand", "the day before", "2 days ahead", "a week prior", "15 minutes early" set the lead, and the trigger is the reference moment minus the lead.

- **The reference** is the event in the line (its date and time) when there is one, or else the stated time.
- **A stated trigger isn't shifted.** When the time comes straight after "remind me" ("remind me next Friday to renew the passport 1 week before", "remind me at 5pm … 1 day before"), that time is the trigger. The lead is kept in `lead_minutes` but not subtracted.
- **A day-or-longer lead with a date-only reference** gives an all-day reminder on that date minus the lead ("remind me the day before Mom's birthday on Friday" is all-day Thursday).
- **A lead with no reference** keeps only the lead minutes and no trigger.

## New warnings

- `no_time`: a reminder or calendar event whose phrase has a date but no time. The reminder is all-day, and so is the event.
- `ambiguous_date_format`: a numeric date like 03/04 where both numbers could be the month. It resolves with the `day_first` setting (default false: month first, 03/04 is March 4) and names the other reading.
- `ambiguous_biweekly`: see above.
- `ambiguous_next_weekday`: see above.
- `converted_timezone`: see above.

## Slang and typos

tmrw, tmr, tomoro, tomorow, tommorow, 2moro, 2morrow (tomorrow); nxt (next); wk (week); wkend, wknd (weekend); thru (through); tonite, 2nite (tonight); eod, cob (end of day); eow (end of week); mon thru fri, mon-fri (Monday to Friday).

## Holidays in tasks

Holidays work anywhere a date works. "pay the bills before Christmas" is due Dec 25 (date only). "every Thanksgiving" is a repeat. "the day after Thanksgiving" is Nov 27.

# Phase 4 conventions

These rules are added on top of the ones above. Where they disagree, these win.

## Repeats

- **Windowed repeats can be limited to days.** "hourly from 10am to 4pm on weekdays" is hourly with a 10:00 to 16:00 window, BYHOUR 10..16, and BYDAY MO..FR. Occurrences fall only on those days. "every 30 min between 9 and 5" is minutely, interval 30, window 09:00 to 17:00. The first occurrence follows the phase 3 slot rule, on an allowed day.
- **More interval forms.**
  - "every half hour": minutely 30.
  - "every hour and a half", "every 90 minutes": minutely 90.
  - "every couple of hours": hourly 2.
  - "every few hours": hourly 3, with `underspecified_recurrence`.
  - "every other hour": hourly 2.
- **Weekday ranges.**
  - With a repeat word (every, each, weekly, or plural days) or a reminder time, "Mon to Wed" or "mon-wed" is weekly MO,TU,WE: "remind me mon to wed at 8am", "gym every week mon to wed".
  - In a calendar line with no repeat word, it is one multi-day all-day event from the coming Monday to the following Wednesday.
- **Date ranges.**
  - "the 3rd through the 5th" in a calendar line is a multi-day all-day event, as in phase 2.
  - In a reminder line, it is a daily repeat from the start date with UNTIL the end date.
  - In a task line, the task is due on the end date.
- **An ordinal weekday is monthly.** "2nd Tuesday", "the 2nd Tuesday", "on the second Tuesday", "the last Thursday of every month", "1st and 3rd Monday", "monthly on the first Friday". These are all monthly with the position (2TU, -1TH, 1MO,3MO, 1FR) on calendar and reminder lines.
  - Exception: when a month is named ("the second Tuesday in November", "the first Monday of December"), it is a one-off date in that month.
  - Phase 3's "every second Tuesday" (alternate weeks) is unchanged.
- **More start phrases.** "starting Monday", "from next week", "beginning the 1st", "effective Nov 1", "as of next month", "commencing", "kicking off", "begins". "starting next week" is the first day of next week. "as of next month" is the 1st of next month.

## Exdates

- **"except in August", "not in December", "no class in July":** every occurrence in that month, within the exdate span.
- **"skip Thanksgiving week", "except Christmas week", "not the week of Easter":** every occurrence in the week (by the week-start setting) that holds the holiday.
- **"except in summer" (a season):** every occurrence in the season's months.
- **An exception the layer can't turn into dates** ("except school holidays", "unless it rains", "when I'm travelling") keeps its raw text, produces no exdates, and raises `unresolved_exception`.

## Words that name a date but aren't one

When a word is ambiguous, don't extract a date. That is better than inventing one.

- **(Replaced in phase 4b by the fixed holiday list: a holiday is always a date.)** The original phase 4 rule was that a holiday is a date only when one of these holds:
  - A date word comes right before it: on, by, before, after, for, until, till, at, over, during, this, next, every, each, except, skip, from, since, or "the day before/after".
  - It is followed in the line by a clock time, a year or a daypart: "Christmas Eve dinner at 6", "Christmas 2027", "Thanksgiving morning".
  - It is the whole phrase, optionally with these event words: party, dinner, lunch, brunch, breakfast, drinks, celebration, service, mass, parade, picnic, bbq, barbecue, eve, day, weekend, break, holiday.
  - So "buy Christmas lights", "Halloween costume ideas", "Easter egg hunt planning" and "christmas shopping list" give no date. "Halloween party" and "Mother's Day brunch" do.
- **"sat" and "sun"** (any case) are days only when one of these holds:
  - Preceded by on, this, next, last, every, each or by.
  - Followed by a digit, "at", a daypart, a date, or a comma before a time.
  - They are the only word in the phrase.
  - They end the line or are followed by punctuation ("return library books Sat", "farmers market sat."). This case was added after a phase 2 row failed.
  - So "sat with Ana", "sun cream", "buy sun cream" and "sat exam" give no date. "dinner sat at 7" and "on sat" do. ("SAT" in capitals is a test, as before.)
- **A month name is a person's name** when it comes after with, call, tell, ask, text, email, message, meet, see, visit, ping, invite, thank, remind, or "for" plus a person-like verb, or when it is followed by "'s". So "lunch with June", "call April", "tell May", "April's birthday" give no date from the name. It is still a date when a day number follows ("call April 3" is a date), or when "in", "on", "by", "until" or "next" comes before it ("meeting with June in June" takes the second June).

## End of month and month-relative phrases

- **"eom", "end of the month", "by end of month":** the last day of the current month, as before.
- **"end of next month", "by the end of next month", "late next month":** the last day of next month.
- **"mid-month", "mid month", "middle of the month":** the 15th of the current month, or of next month if the 15th has passed. "mid next month" is the 15th of next month.
- **"early next month", "beginning of next month":** the 1st of next month.
- **Ordinals.**
  - A bare ordinal ("the 1st", "on the 30th", "the twenty-third", "the fifteenth") is the next date with that day number, today or later. A month that lacks that day is skipped, so on Feb 10 "on the 30th" is March 30.
  - "the 3rd of next month" is the 3rd of next month.
  - "the 2nd of November" is Nov 2.

## Abbreviations and slang

thurs, thur, weds, tues, tue (weekdays); sept, sep, nov., dec. (months); 2day (today); 2nite, tonite (tonight); rn, right now (now); this arvo, arvo (afternoon, 14:00); fortnight (14 days; "every fortnight" is weekly, interval 2).

"asap", "ASAP" and "as soon as possible" give no date and raise `vague_time`.

## Events

- **Durations set the end.** "for 90 min", "for 2h", "2h" or "2 hrs" right after a time, "(2h)", "for half a day" (4 hours), "for a full day" (all-day). "all day" makes the event all-day.
- **A meal word with a date and no time** starts at that meal's time, as in phase 2: "lunch with Sam friday" is Friday 12:00.
- **Multi-day events:** "Fri to Sun", "Oct 3-5", "from the 3rd to the 5th", "conference Oct 3 to Oct 5". These are all-day from the first day to the last day, both included, with the end reported as the last day's date.

## Reminders

- **Dayparts are the trigger time:** tomorrow morning 09:00, this evening 19:00, tonight 20:00, this afternoon 14:00.
- **A holiday works as the trigger date** under the holiday rule above, all-day or with a daypart's time: "remind me on Christmas Eve to call grandma" is all-day Dec 24.

## Warnings added in phase 4

| Code | When |
|---|---|
| `multiple_items` | The line holds two items, each with its own date or time, joined by "and", "then", ";" or ",". "call mom at 5 and pay rent on the 1st". The first item is resolved. |
| `unresolved_reference` | The timing depends on something without a date: "after the meeting", "when Sam's back", "once the build passes", "before the trip", "before guests arrive". No date is invented. |
| `unresolved_exception` | See Exdates above. |
| `underspecified_recurrence` | The line asks for a repeat without saying how often: "every so often", "regularly", "from time to time", "periodically", "now and then", "every few days/weeks/hours". "every few" still gets the phase 2 number (3) as a guess. |
| `missing_trigger` | A reminder with no time, date or place. |
| `missing_date` | A calendar event with no date or time at all. Tasks may have no due date, so they don't get this warning. |
| `vague_time` | "sometime next week", "later", "soon", "at some point", "in a bit", "asap", "this week sometime", "one of these days". "sometime next week" still resolves to the first day of next week as a guess. "later", "soon", "asap" and "in a bit" give no date. |

# Phase 4b conventions

These rules replace parts of the phase 4 rules. Where they disagree, these win.

## Holidays: public holiday data (phase 5)

This replaces the hand-rolled list from phase 4b. The phase 4b rule still holds: a matched holiday is always a date, with no guessing from neighbouring words.

- **Data.** Holidays come from date-holidays, pinned in `tools/holidays/date-holidays.pin` and embedded as `src/facets/holidays.bin`. It covers every country, state and region the package has, for 1970 to 2100.
  - Each holiday has its dates (substitute days flagged), its type (public, bank, school, optional or observance), a stable id and its names in every language the package provides.
  - India also gets the computed Hindu festivals and four Islamic holidays (see `docs/indian-festivals.md`).
  - Nothing is fetched at runtime.
- **The `holidays` setting:**
  - **`auto` (default)** works out the region in this order:
    1. The locale's region subtag, if `locale` is set ("en-GB" gives GB).
    2. Otherwise the time zone: the first country whose date-holidays zone list includes the zone. The order is US, GB, CA, AU, IN, IE, NZ and ZA, then alphabetical. If exactly one state of that country lists the zone, and the country lists more than one zone, that state is used. America/Los_Angeles gives US, America/Toronto gives CA, Asia/Kolkata gives IN and Europe/London gives GB.
    3. If neither gives a region, there is no home region, and every holiday uses the best-known region with `holiday_other_region`.
  - **An explicit country and optional region** ("US" plus "CA" means California) uses that region.
  - **`off`:** holiday names are plain words. They give no date and no warning, and stay in the title.
- **Name matching:**
  - The package's names in English and in the home region's languages.
  - The common variants this crate already handled: xmas, nye, valentines, st paddy's, mum's day, dad's day, pancake day, black friday, and so on.
  - Indian variants: Deepavali, Vijayadashami, Bakrid, Eid ul-Fitr, Rakshabandhan, Thiruvonam, and others.
  - Apostrophes, dots and case don't matter.
- **The date is the home region's.** Thanksgiving is the 4th Thursday of November in the US and the 2nd Monday of October in Canada. Mother's Day in the UK is Mothering Sunday.
- **A holiday the home region doesn't have** uses the best-known region that has it, in the same country order, and adds `holiday_other_region`. Boxing Day in the US uses the UK date.
- **Warnings:**
  - `holiday_computed`: a festival whose date was computed (the Indian lunisolar festivals).
  - `holiday_moon_sighting`: an Islamic holiday in India, whose observed date can move by a day.
- **Repeats.** "every <holiday>" is yearly. A holiday on the same month and day every year gives BYMONTH and BYMONTHDAY. One on the same nth (or last) weekday of a month gives BYDAY. Any other repeats by its listed dates, with `holiday_dates`.
- **"except holidays"** removes the home region's public and bank holidays, including substitute days. With holidays `off`, or no home region, it can't be resolved and gives `unresolved_exception`.
- **Years.** A date outside 1970–2100 gets no holiday date.
- **Data version.** `holiday_data_info()` (wasm `holidayData()`) returns the source, version, integrity hash and generation date.

## Warning precision

- **`missing_trigger`** doesn't fire when the reminder names a place. A place is a trigger: "when I get home", "when I leave work", "at the office", "at the store", "when I'm at the gym", "next time I'm at the pharmacy", "when I arrive", "when I land".
- **`missing_date`** fires only for a calendar event that has no date, no time, no daypart, no relative phrase ("in 2 hours") and no recognised holiday, which means nothing time-like at all. An event with only a time resolves to today or tomorrow and gets no warning.
- **`unresolved_reference`** fires only for an anchor that is a real event or state the resolver can't place: after, before, once, when, until, till or "as soon as", followed by a determiner, possessive, name or pronoun phrase ("after the meeting", "when Sam's back", "once the build is green", "before the trip"). It doesn't fire:
  - for a resolved relative phrase ("3 days later", "after lunch", "after work", "before 5pm", "before the 28th", "until October");
  - for a lead time with a stated time;
  - for a place trigger ("when I get home").
- **`vague_time`** fires only for genuinely vague timing: sometime, some time, later (alone, not "3 days later" and not "later today" or "later this week"), soon, soonish, at some point, one of these days, eventually, in a bit, in a while, whenever, asap. A daypart that maps to a default time is not vague.
- **`multiple_items`** fires when the line splits on "and", "then", ";", "also" or "plus" into two parts that each have their own verb or noun phrase and their own time or date. "call mom at 5 and pay rent on the 1st", "gym at 6 then dinner at 8", "dentist Monday; haircut Tuesday", "email Jo at 9 plus call Sam at 4". It doesn't fire on lists of times or days with no second verb ("Monday and Wednesday at 3", "at 7am and 7pm").
- **`underspecified_recurrence`** fires on "every so often", "regularly", "from time to time", "now and then", "now and again", "every now and then", "every once in a while", "occasionally", "periodically", "sometimes", "often", "frequently", "routinely" and "every few …". It is reported in the line's warnings, and in the recurrence's warnings when there is a recurrence.

## Date-only dues (owner decision, phase 5)

This section replaces every earlier statement about a 17:00 task default or a 09:00 reminder default.

- **A date with no time is date-only, for every kind.** `start` is all-day: `all_day` is true, and `time`, `local` and `instant` are empty.
  - A task gets a date-only due, with no `time_assumed`.
  - A reminder is all-day with `no_time`.
  - An event is all-day with `all_day_assumed` and `no_time`, as before.
- **`notify_at`** on the line result is the due date at `default_notification_time` (default 09:00), for date-only tasks and reminders. It is a display and notification input only and never changes the due.
- **An explicit time always gives a timed due:** "at 3pm", "eod" (17:00), a daypart ("tomorrow morning" 09:00), or a meal or activity word.
- **Repeats with no time** are all-day for every kind.
- **A day-or-longer lead from a date-only reference** is all-day on the earlier date.
- **A bare weekday that names today, with no time, is today.** The date hasn't passed. It used to move to next week only because the assumed 09:00 had passed. "before <today's weekday>" means next week's, with `moved_to_next_week`.
- **Unchanged:** a time with no date (today, or tomorrow if passed), and a line with neither a date nor a time.
