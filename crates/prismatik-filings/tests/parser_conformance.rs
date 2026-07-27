use prismatik_filings::{holdings_observable_as_of, parse_13f, parse_form4, FilingParseError};
use time::{Date, Month};

#[test]
fn parses_13f_xml_and_enforces_filing_date_observability() {
    let rows = parse_13f(include_str!("fixtures/13f.xml")).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].period_end, date(2024, Month::June, 30));
    assert_eq!(rows[0].filing_date, date(2024, Month::August, 14));
    assert_eq!(rows[0].observable_at, rows[0].filing_date);
    assert!(holdings_observable_as_of(&rows, date(2024, Month::August, 13)).is_empty());
    assert_eq!(
        holdings_observable_as_of(&rows, date(2024, Month::August, 14)).len(),
        2
    );
}

#[test]
fn parses_13f_json_fixture() {
    let rows = parse_13f(include_str!("fixtures/13f.json")).unwrap();
    assert_eq!(rows[0].cusip, "037833100");
    assert_eq!(rows[0].shares, "120000");
}

#[test]
fn parses_form4_insider_transaction() -> Result<(), FilingParseError> {
    let rows = parse_form4(include_str!("fixtures/form4.xml"))?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].transaction_code, "S");
    assert_eq!(rows[0].shares, "5000");
    assert_eq!(rows[0].filing_date, date(2024, Month::October, 17));
    Ok(())
}

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}
