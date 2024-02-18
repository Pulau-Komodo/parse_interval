//! parse_interval parses text into a `chrono::Duration`, if that text matches the specified format. It is intended to be a quick and simple solution for text-based user input, like for scheduling reminders or specifying durations.
//!
//! The format is as follows:
//!
//! `<number> years <number> months <number> weeks <number> days <number> hours <number> minutes <number> seconds`
//!
//! Each unit is optional, but all present units need to be in order. To process years and months, it needs to use some date.

use std::sync::OnceLock;

use chrono::{DateTime, Duration, Months, Utc};

pub use errors::ParseError;
use parse_bytes::ParseBytes;

mod errors;
mod parse_bytes;
mod time_units;

/// Parse an interval like "15 days 12 hours". It can include weeks, days, hours, minutes and seconds. It can not include years or months.
pub fn simple(interval: &str) -> Result<Duration, ParseError> {
	parse_interval(interval, None)
}

/// Parse an interval like "1 year 15 days". Years and months will be evaluated as offset from the specified date.
///
/// It can include years, months, weeks, days, hours, minutes and seconds.
///
/// If you don't already have a date, it may be more efficient to use [`parse_interval_with_lazy_date`], since it avoids constructing it if it doesn't end up needing it (because there were no years or months).
pub fn with_date(interval: &str, date: DateTime<Utc>) -> Result<Duration, ParseError> {
	parse_interval(interval, Some(Box::new(move || date)))
}

/// Parse an interval like "1 year 15 days". Years and months will be evaluated as offset from the date generated by the passed function.
///
/// It can include years, months, weeks, days, hours, minutes and seconds.
///
/// This avoids constructing the date if it doesn't end up needing it (because there were no years or months).
pub fn with_lazy_date<D>(interval: &str, get_date: D) -> Result<Duration, ParseError>
where
	D: FnOnce() -> DateTime<Utc> + 'static,
{
	parse_interval(interval, Some(Box::new(get_date)))
}

/// Parse an interval like "1 year 15 days". Years and months will be evaluated as offset from the present (current system time).
///
/// It can include years, months, weeks, days, hours, minutes and seconds.
pub fn with_now(interval: &str) -> Result<Duration, ParseError> {
	with_lazy_date(interval, Utc::now)
}

/// Parse an interval like "1 year 15 days". The years and months are evaluated as offset from the generated date.
///
/// If a date constructor is provided, it can include years and months. Either way it can include weeks, days, hours, minutes and seconds.
fn parse_interval(
	interval: &str,
	mut get_date: Option<Box<dyn FnOnce() -> DateTime<Utc>>>,
) -> Result<Duration, ParseError> {
	static PATTERNS: OnceLock<[time_units::TimeUnit; 7]> = OnceLock::new();
	let units = PATTERNS.get_or_init(|| time_units::UNITS.map(|unit| unit.compile()));

	let allow_inconstant = get_date.is_some();

	let mut date = None;
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
		let (number, fraction) = bytes.parse_number()?;
		bytes.skip_spaces();
		for (unit_index, unit) in units.iter().enumerate().skip(unit_cursor) {
			unit_cursor += 1;
			if bytes.parse_regex(&unit.regex) {
				match unit_index {
					// Years
					0 => {
						if fraction > 0.0 {
							return Err(ParseError::InconstantUnitWithFraction);
						}
						let date =
							date.get_or_insert_with(|| get_date.take().map(|f| f()).unwrap());
						let offset_date = offset_date.get_or_insert(*date);
						let months = Months::new(
							number
								.checked_mul(12)
								.ok_or(ParseError::NumberOutOfRange)?
								.try_into()?,
						);
						*offset_date = if is_subtracting {
							offset_date.checked_sub_months(months)
						} else {
							offset_date.checked_add_months(months)
						}
						.ok_or(ParseError::DateOutOfRange)?;
					}
					// Months
					1 => {
						if fraction > 0.0 {
							return Err(ParseError::InconstantUnitWithFraction);
						}
						let date =
							date.get_or_insert_with(|| get_date.take().map(|f| f()).unwrap());
						let offset_date = offset_date.get_or_insert(*date);
						let months = Months::new(number.try_into()?);
						*offset_date = if is_subtracting {
							offset_date.checked_sub_months(months)
						} else {
							offset_date.checked_add_months(months)
						}
						.ok_or(ParseError::DateOutOfRange)?;
					}
					// Other
					_ => {
						let fraction_part =
							Duration::seconds((fraction * unit.seconds as f32) as i64);
						duration = number
							.checked_mul(unit.seconds)
							.map(Duration::seconds)
							.and_then(|d| {
								if is_subtracting {
									duration
										.checked_sub(&d)
										.and_then(|d| d.checked_sub(&fraction_part))
								} else {
									duration
										.checked_add(&d)
										.and_then(|d| d.checked_add(&fraction_part))
								}
							})
							.ok_or(ParseError::NumberOutOfRange)?;
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
		duration = duration
			.checked_add(&(offset_date - date))
			.ok_or(ParseError::NumberOutOfRange)?;
	}
	Ok(duration)
}

const _PATTERN: &str = r"^(?:(?:(-) ?)?(\d+) ?y(?:ears?)?\s?)?(?:(?:(-) ?)?(\d+) ?mo(?:nths?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?w(?:eeks?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?d(?:ays?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?h(?:(?:ou)?rs?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?m(?:in(?:ute)?s?)?\s?)?(?:(?:(-) ?)?(\d+(?:\.\d+)?|\.\d+) ?s(?:ec(?:ond)?s?)?\s?)?$/i";

#[cfg(test)]
mod tests {
	use chrono::{NaiveDate, NaiveTime};

	use super::*;

	/// Date subtractions never overflow.
	#[test]
	fn overflow_date() {
		let _ = DateTime::<Utc>::MIN_UTC - DateTime::<Utc>::MAX_UTC;
	}
	#[test]
	fn simple_case() {
		assert_eq!(simple("5 weeks 3 days"), Ok(Duration::seconds(3283200)));
	}
	#[test]
	fn short() {
		assert_eq!(simple("5w3d1h30m30s"), Ok(Duration::seconds(3288630)));
	}
	#[test]
	fn subtraction() {
		assert_eq!(simple("5 weeks -3 days"), Ok(Duration::seconds(2764800)));
	}
	#[test]
	fn negative_duration() {
		assert_eq!(simple("-5 weeks 3 days"), Ok(Duration::seconds(-3283200)));
	}
	#[test]
	fn double_subtraction() {
		assert_eq!(simple("-5 weeks -3 days"), Ok(Duration::seconds(-2764800)));
	}
	#[test]
	fn space_mess() {
		assert_eq!(
			simple("  -  5   weeks    -   3   days  "),
			Ok(Duration::seconds(-2764800))
		);
	}
	#[test]
	fn ignore_case() {
		assert_eq!(simple("5 WEEKS 3 days"), Ok(Duration::seconds(3283200)));
	}
	#[test]
	fn fractions() {
		assert_eq!(
			simple("0.5 week 2.5 days 3.55 hours .5 minutes 1 second"),
			Ok(Duration::seconds(531211))
		);
	}
	/// I don't have any particular rounding behaviour in mind, but if it changes, I'd like to know.
	#[test]
	fn fraction_rounding() {
		assert_eq!(simple("0.1s"), Ok(Duration::seconds(0)));
		assert_eq!(simple("0.017m"), Ok(Duration::seconds(1)));
	}
	#[test]
	fn invalid_fraction() {
		assert_eq!(simple("0.5.0d"), Err(ParseError::NoUnit(3)));
	}
	#[test]
	fn lone_period() {
		assert_eq!(simple(".d"), Err(ParseError::NoNumber(0)));
	}
	#[test]
	fn inconstant_fraction() {
		assert_eq!(
			with_date("0.5y", date_year_month_day(2020, 6, 20)),
			Err(ParseError::InconstantUnitWithFraction)
		);
	}
	#[test]
	fn empty_input() {
		assert_eq!(simple(""), Err(ParseError::Empty));
	}
	#[test]
	fn spaces_input() {
		assert_eq!(simple("  "), Err(ParseError::Empty));
	}
	#[test]
	fn duplicate_units() {
		assert_eq!(
			simple("5 days 3 days"),
			Err(ParseError::UnitOutOfSequence(9))
		);
	}
	#[test]
	fn out_of_order_units() {
		assert_eq!(
			simple("5 days 3 weeks"),
			Err(ParseError::UnitOutOfSequence(9))
		);
	}
	#[test]
	fn non_units() {
		assert_eq!(simple("5 days 3 apples"), Err(ParseError::NoUnit(9)));
	}
	#[test]
	fn missing_number() {
		assert_eq!(simple("5 days weeks"), Err(ParseError::NoNumber(7)));
	}
	#[test]
	fn years_without_date() {
		assert_eq!(
			simple("5 years 3 days"),
			Err(ParseError::InconstantUnitWithoutDate)
		);
	}
	#[test]
	fn out_of_range() {
		assert_eq!(
			with_date("-1 year - 12 months", DateTime::<Utc>::MIN_UTC),
			Err(ParseError::DateOutOfRange)
		);
	}
	fn date_year_month_day(year: i32, month: u32, day: u32) -> DateTime<Utc> {
		NaiveDate::from_ymd_opt(year, month, day)
			.unwrap()
			.and_time(NaiveTime::default())
			.and_utc()
	}
	#[test]
	fn leap_year_forward() {
		assert_eq!(
			with_date("1 month", date_year_month_day(2000, 2, 1)),
			Ok(Duration::days(29))
		);
	}
	#[test]
	fn leap_year_backward() {
		assert_eq!(
			with_date("-1 month", date_year_month_day(2000, 2, 1)),
			Ok(Duration::days(-31))
		);
	}
	#[test]
	fn year_equals_twelve_months_forwards() {
		assert_eq!(
			with_date("1 year -12 months", date_year_month_day(2000, 2, 1)),
			Ok(Duration::default())
		);
	}
	#[test]
	fn year_equals_twelve_months_backwards() {
		assert_eq!(
			with_date("-1 year -12 months", date_year_month_day(2000, 2, 1)),
			Ok(Duration::default())
		);
	}
	#[test]
	fn lazy_eager_same_outcome() {
		let date = date_year_month_day(2000, 2, 1);
		let interval = "1 year 3 months 15 minutes";
		assert_eq!(
			with_date(interval, date),
			with_lazy_date(interval, move || date)
		);
	}
	#[test]
	fn doc_examples() {
		let duration = self::with_now("2 days 15 hours 15 mins");
		assert_eq!(duration, Ok(chrono::Duration::seconds(227700)));

		let duration = self::with_lazy_date("1 month", || {
			NaiveDate::from_ymd_opt(2000, 2, 1)
				.unwrap()
				.and_time(NaiveTime::default())
				.and_utc()
		});
		assert_eq!(duration, Ok(chrono::Duration::days(29)));
	}
}
