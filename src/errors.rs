use std::num::TryFromIntError;

use crate::{parse_bytes::ParseBytes, time_units};

/// An error parsing the text, with some attempt made to be specific.
///
/// Note that the `DateOutOfRange` variant only catches cases where the date used for year and month math goes out of range for the type. It does not catch other overflows.
///
/// This is marked non-exhaustive to allow narrowing down error types in the future without that breaking the API so much.
#[non_exhaustive]
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ParseError {
	#[error("Input was empty or had only spaces")]
	Empty,
	#[error("Could not parse a number where a number was expected at position {0}")]
	NoNumber(usize),
	#[error("Could not parse a unit where a unit was expected at position {0}")]
	NoUnit(usize),
	#[error("Found a unit out of sequence at position {0}. Units need to be in strictly descending order of size.")]
	UnitOutOfSequence(usize),
	#[error("Year or month supplied without a date, and without being allowed to default to now")]
	InconstantUnitWithoutDate,
	#[error("During some step in adjusting years or months, the date became out of range")]
	DateOutOfRange,
	#[error("Some operation overflowed or some number conversion failed")]
	NumberOutOfRange,
}

impl ParseError {
	pub(crate) fn diagnose_unit_error(
		bytes: &ParseBytes,
		units: &[time_units::TimeUnit],
		unit_cursor: usize,
		allow_inconstant: bool,
	) -> Self {
		let position = bytes.offset();
		match units[0..unit_cursor]
			.iter()
			.position(|unit| bytes.clone().parse_regex(&unit.regex))
		{
			Some(0..=1) if !allow_inconstant => Self::InconstantUnitWithoutDate,
			Some(_) => Self::UnitOutOfSequence(position),
			None => Self::NoUnit(position),
		}
	}
}

impl From<TryFromIntError> for ParseError {
	fn from(_value: TryFromIntError) -> Self {
		Self::NumberOutOfRange
	}
}
