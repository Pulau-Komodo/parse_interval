use std::sync::OnceLock;

use chrono::{DateTime, Duration, Months, Utc};

pub use errors::ParseError;
use parse_bytes::ParseBytes;

mod errors;
mod parse_bytes;
mod time_units;

/// Parse an interval like "4 weeks 12 hours". It can include weeks, days, hours, minutes and seconds. It can not include years or months.
pub fn parse_interval_simple(interval: &str) -> Result<Duration, ParseError> {
	parse_interval(interval, None, false)
}

/// Parse an interval like "1 year 6 months".
///
/// It can include years, months, weeks, days, hours, minutes and seconds.
///
/// Years and months will be evaluated as offset from the present (current system time).
pub fn parse_interval_now(interval: &str) -> Result<Duration, ParseError> {
	parse_interval(interval, None, true)
}

/// Parse an interval like "1 year 6 months".
///
/// It can include years, months, weeks, days, hours, minutes and seconds.
///
/// Years and months will be evaluated as offset from the specified date.
pub fn parse_interval_date(interval: &str, date: DateTime<Utc>) -> Result<Duration, ParseError> {
	parse_interval(interval, Some(date), false)
}

/// Parse an interval like "1 year 5 days".
///
/// If a date is provided, or `default_to_now` is true, it can include years and months. Either way it can include weeks, days, hours, minutes and seconds.
///
/// The years and months are evaluated as offset from the specified date, or from the present (current system time) if defaulting to that.
pub fn parse_interval(
	interval: &str,
	mut date: Option<DateTime<Utc>>,
	default_to_now: bool,
) -> Result<Duration, ParseError> {
	static PATTERNS: OnceLock<[time_units::TimeUnit; 7]> = OnceLock::new();
	let units = PATTERNS.get_or_init(|| time_units::UNITS.map(|unit| unit.compile()));

	let allow_inconstant = date.is_some() || default_to_now;

	let mut bytes = ParseBytes::from_str(interval);
	let mut duration = Duration::seconds(0);
	let mut offset_date = None;
	let mut is_subtracting = false;
	let mut unit_cursor = if allow_inconstant {
		0
	} else {
		2 // Skip years and months
	};
	bytes.skip_spaces();
	if bytes.is_empty() {
		return Err(ParseError::Empty);
	}
	'outer: while !bytes.is_empty() {
		if bytes.parse_minus() {
			is_subtracting = !is_subtracting;
			bytes.skip_spaces();
		}
		let mut number = bytes.parse_number()?;
		bytes.skip_spaces();
		if is_subtracting {
			number *= -1;
		}
		for (unit_index, unit) in units.iter().enumerate().skip(unit_cursor) {
			unit_cursor += 1;
			if bytes.parse_regex(&unit.regex) {
				match unit_index {
					// Years
					0 => {
						let date = date.get_or_insert_with(Utc::now);
						let offset_date = offset_date.get_or_insert(*date);
						let number = number.unsigned_abs();
						let number: u32 = number.try_into().map_err(|_| ParseError::Overflow)?;
						let number = number.checked_mul(12).ok_or(ParseError::Overflow)?;
						*offset_date = if number > 0 {
							offset_date
								.checked_add_months(Months::new(number))
								.ok_or(ParseError::DateOutOfRange)?
						} else {
							offset_date
								.checked_sub_months(Months::new(number))
								.ok_or(ParseError::DateOutOfRange)?
						};
					}
					// Months
					1 => {
						let date = date.get_or_insert_with(Utc::now);
						let offset_date = offset_date.get_or_insert(*date);
						*offset_date = if number > 0 {
							offset_date
								.checked_add_months(Months::new(number as u32))
								.ok_or(ParseError::DateOutOfRange)?
						} else {
							offset_date
								.checked_sub_months(Months::new((-number) as u32))
								.ok_or(ParseError::DateOutOfRange)?
						};
					}
					// Other
					_ => {
						duration += Duration::seconds(number * unit.seconds);
					}
				}
				bytes.skip_spaces();
				continue 'outer;
			}
		}
		return Err(ParseError::diagnose_unit_error(
			&bytes,
			units,
			unit_cursor,
			allow_inconstant,
		));
	}

	if let (Some(date), Some(offset_date)) = (date, offset_date) {
		duration = duration.checked_add(&(offset_date - date)).ok_or(ParseError::Overflow)?;
	}
	Ok(duration)
}

const _PATTERN: &str = r"^(?:(?:(-) ?)?(\d+) ?y(?:ears?)?\s?)?(?:(?:(-) ?)?(\d+) ?mo(?:nths?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?w(?:eeks?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?d(?:ays?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?h(?:(?:ou)?rs?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?m(?:in(?:ute)?s?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?s(?:ec(?:ond)?s?)?\s?)?$/i";

#[cfg(test)]
mod tests {
	use chrono::Datelike;

	use super::*;

	/// Date subtractions never overflow.
	#[test]
	fn overflow_date() {
		let _ = DateTime::<Utc>::MIN_UTC - DateTime::<Utc>::MAX_UTC;
	}
	#[test]
	fn simple() {
		assert_eq!(
			parse_interval_simple("5 weeks 3 days"),
			Ok(Duration::seconds(3283200))
		);
	}
	#[test]
	fn short() {
		assert_eq!(
			parse_interval_simple("5w3d1h30m30s"),
			Ok(Duration::seconds(3288630))
		);
	}
	#[test]
	fn subtraction() {
		assert_eq!(
			parse_interval_simple("5 weeks -3 days"),
			Ok(Duration::seconds(2764800))
		);
	}
	#[test]
	fn negative_duration() {
		assert_eq!(
			parse_interval_simple("-5 weeks 3 days"),
			Ok(Duration::seconds(-3283200))
		);
	}
	#[test]
	fn double_subtraction() {
		assert_eq!(
			parse_interval_simple("-5 weeks -3 days"),
			Ok(Duration::seconds(-2764800))
		);
	}
	#[test]
	fn space_mess() {
		assert_eq!(
			parse_interval_simple("  -  5   weeks    -   3   days  "),
			Ok(Duration::seconds(-2764800))
		);
	}
	#[test]
	fn ignore_case() {
		assert_eq!(
			parse_interval_simple("5 WEEKS 3 days"),
			Ok(Duration::seconds(3283200))
		);
	}
	#[test]
	fn empty_input() {
		assert_eq!(parse_interval_simple(""), Err(ParseError::Empty));
	}
	#[test]
	fn spaces_input() {
		assert_eq!(parse_interval_simple("  "), Err(ParseError::Empty));
	}
	#[test]
	fn duplicate_units() {
		assert_eq!(
			parse_interval_simple("5 days 3 days"),
			Err(ParseError::UnitOutOfSequence(9))
		);
	}
	#[test]
	fn out_of_order_units() {
		assert_eq!(
			parse_interval_simple("5 days 3 weeks"),
			Err(ParseError::UnitOutOfSequence(9))
		);
	}
	#[test]
	fn non_units() {
		assert_eq!(
			parse_interval_simple("5 days 3 apples"),
			Err(ParseError::NoUnit(9))
		);
	}
	#[test]
	fn missing_number() {
		assert_eq!(
			parse_interval_simple("5 days weeks"),
			Err(ParseError::NoNumber(7))
		);
	}
	#[test]
	fn years_without_date() {
		assert_eq!(
			parse_interval_simple("5 years 3 days"),
			Err(ParseError::InconstantUnitWithoutDate)
		);
	}
	#[test]
	fn out_of_range() {
		assert_eq!(
			parse_interval_date("-1 year - 12 months", DateTime::<Utc>::MIN_UTC),
			Err(ParseError::DateOutOfRange)
		);
	}
	fn date_year_month_day(year: i32, month: u32, day: u32) -> DateTime<Utc> {
		DateTime::<Utc>::default()
			.with_year(year)
			.unwrap()
			.with_month(month)
			.unwrap()
			.with_day(day)
			.unwrap()
	}
	#[test]
	fn leap_year_forward() {
		assert_eq!(
			parse_interval_date("1 month", date_year_month_day(2000, 2, 1)),
			Ok(Duration::days(29))
		);
	}
	#[test]
	fn leap_year_backward() {
		assert_eq!(
			parse_interval_date("-1 month", date_year_month_day(2000, 2, 1)),
			Ok(Duration::days(-31))
		);
	}
	#[test]
	fn year_equals_twelve_months_forwards() {
		assert_eq!(
			parse_interval_date("1 year -12 months", date_year_month_day(2000, 2, 1)),
			Ok(Duration::default())
		);
	}
	#[test]
	fn year_equals_twelve_months_backwards() {
		assert_eq!(
			parse_interval_date("-1 year -12 months", date_year_month_day(2000, 2, 1)),
			Ok(Duration::default())
		);
	}
}
