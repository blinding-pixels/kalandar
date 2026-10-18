use super::support::*;

#[test]
fn skip_random_non_date_patterns() {
    let r0 = run("casual", " 3", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "       1", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "  11 ", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", " 0.5 ", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("casual", " 35.49 ", now(), options());
    assert_eq!(r4.len(), 0);
    let r5 = run("casual", "12.53%", now(), options());
    assert_eq!(r5.len(), 0);
    let r6 = run(
        "casual",
        "6358fe2310> *5.0* / 5 Outstanding",
        now(),
        options(),
    );
    assert_eq!(r6.len(), 0);
    let r7 = run(
        "casual",
        "6358fe2310> *1.5* / 5 Outstanding",
        now(),
        options(),
    );
    assert_eq!(r7.len(), 0);
    let r8 = run(
        "casual",
        "Total: $1,194.09 [image: View Reservation",
        now(),
        options(),
    );
    assert_eq!(r8.len(), 0);
    let r9 = run("casual", "at 6.5 kilograms", now(), options());
    assert_eq!(r9.len(), 0);
    let r10 = run("casual", "ah that is unusual", now(), options().forward());
    assert_eq!(r10.len(), 0);
}

#[test]
fn urls_encoded() {
    let r0 = run("casual", "%e7%b7%8a", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "https://tenor.com/view/%e3%83%89%e3%82%ad%e3%83%89%e3%82%ad-%e7%b7%8a%e5%bc%b5-%e5%a5%bd%e3%81%8d-%e3%83%8f%e3%83%bc%e3%83%88-%e5%8f%af%e6%84%9b%e3%81%84-gif-15876325", now(), options());
    assert_eq!(r1.len(), 0);
}

#[test]
fn skip_hyphenated_numbers_pattern() {
    let r0 = run("casual", "1-2", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "1-2-3", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "4-5-6", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", "20-30-12", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("casual", "2012", now(), options());
    assert_eq!(r4.len(), 0);
    let r5 = run("casual", "2012-14", now(), options());
    assert_eq!(r5.len(), 0);
    let r6 = run("casual", "2012-1400", now(), options());
    assert_eq!(r6.len(), 0);
    let r7 = run("casual", "2200-25", now(), options());
    assert_eq!(r7.len(), 0);
}

#[test]
fn skip_impossible_dates_times() {
    let r0 = run("casual", "February 29, 2022", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "02/29/2022", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "June 31, 2022", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", "06/31/2022", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("casual", "14PM", now(), options());
    assert_eq!(r4.len(), 0);
    let r5 = run("casual", "25:12", now(), options());
    assert_eq!(r5.len(), 0);
    let r6 = run("casual", "An appointment on 13/31/2018", now(), options());
    assert_eq!(r6.len(), 0);
}

#[test]
fn skip_impossible_dates_times_ranges() {
    let r0 = run("casual", "February 20 - 29, 2022", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "June 10 - 31, 2022", now(), options());
    assert_eq!(r1.len(), 0);
}

#[test]
fn skip_version_number_pattern() {
    let r0 = run("casual", "Version: 1.1.3", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "Version: 1.1.30", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "Version: 1.10.30", now(), options());
    assert_eq!(r2.len(), 0);
}

#[test]
fn skip_incorrect_reference() {
    let r0 = run("casual", "for the year", now(), options());
    assert_eq!(r0.len(), 0);
}

#[test]
fn date_with_version_number_pattern() {
    let r0 = run("casual", "1.5.3 - 2015-09-24", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "2015-09-24");
    let r1 = run("casual", "1.5.30 - 2015-09-24", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "2015-09-24");
    let r2 = run("casual", "1.50.30 - 2015-09-24", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2015-09-24");
}
