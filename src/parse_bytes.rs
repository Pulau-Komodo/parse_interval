use regex::bytes::Regex;

use crate::errors::ParseError;

#[derive(Clone)]
pub(crate) struct ParseBytes<'l> {
	original: &'l str,
	shrinking: &'l [u8],
}

impl<'l> ParseBytes<'l> {
	pub(crate) fn from_str(str: &'l str) -> Self {
		Self {
			original: str,
			shrinking: str.as_bytes(),
		}
	}
	pub(crate) fn parse_regex(&mut self, regex: &Regex) -> bool {
		if let Some(found) = regex.find(self.shrinking) {
			self.shrinking = &self.shrinking[found.end()..];
			true
		} else {
			false
		}
	}
	pub(crate) fn parse_minus(&mut self) -> bool {
		if self.shrinking.first() == Some(&b'-') {
			self.shrinking = &self.shrinking[1..];
			true
		} else {
			false
		}
	}
	/// Parse digits into a number until a non-digit is encountered.
	///
	/// Will return an error on empty input or on overflow.
	pub(crate) fn parse_number(&mut self) -> Result<(i64, f32), ParseError> {
		let mut number: i64 = 0;
		let mut fraction: f32 = 0.0;
		let mut fractional_position = 0;
		let mut highest_index = 0;
		#[allow(clippy::manual_is_ascii_check)]
		for byte in self
			.shrinking
			.iter()
			.take_while(|byte| (b'0'..=b'9').contains(byte) || **byte == b'.')
		{
			if byte == &b'.' {
				if fractional_position > 0 {
					break;
				}
				fractional_position = 1;
			} else if fractional_position == 0 {
				number = number
					.checked_mul(10)
					.and_then(|n| n.checked_add((byte - b'0') as i64))
					.ok_or(ParseError::NumberOutOfRange)?;
			} else {
				fraction += (byte - b'0') as f32 / 10.0f32.powi(fractional_position);
				fractional_position += 1;
			}
			highest_index += 1;
		}
		if highest_index > 0 && (highest_index > 1 || fractional_position == 0) {
			self.shrinking = &self.shrinking[highest_index..];
			Ok((number, fraction))
		} else {
			Err(ParseError::NoNumber(self.offset()))
		}
	}
	pub(crate) fn skip_spaces(&mut self) {
		if let Some(index) = self.shrinking.iter().position(|&byte| byte != b' ') {
			self.shrinking = &self.shrinking[index..];
		} else {
			self.shrinking = &[];
		}
	}
	pub(crate) fn is_empty(&self) -> bool {
		self.shrinking.is_empty()
	}
	pub(crate) fn offset(&self) -> usize {
		self.shrinking.as_ptr() as usize - self.original.as_bytes().as_ptr() as usize
	}
}
