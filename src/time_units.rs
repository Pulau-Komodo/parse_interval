use regex::bytes::Regex;

use regex::bytes::RegexBuilder;

pub(crate) struct TimeUnitRaw {
	pub(crate) seconds: i64,
	pub(crate) pattern: &'static str,
}

pub(crate) const UNITS: [TimeUnitRaw; 7] = [
	TimeUnitRaw {
		seconds: 365 * 7 * 24 * 60 * 60, // Not used
		pattern: "^y(?:ears?)?",
	},
	TimeUnitRaw {
		seconds: 30 * 7 * 24 * 60 * 60, // Not used
		pattern: "^mo(?:nths?)?",
	},
	TimeUnitRaw {
		seconds: 7 * 24 * 60 * 60,
		pattern: "^w(?:eeks?)?",
	},
	TimeUnitRaw {
		seconds: 24 * 60 * 60,
		pattern: "^d(?:ays?)?",
	},
	TimeUnitRaw {
		seconds: 60 * 60,
		pattern: "^h(?:(?:ou)?rs?)?",
	},
	TimeUnitRaw {
		seconds: 60,
		pattern: "^m(?:in(?:ute)?s?)?",
	},
	TimeUnitRaw {
		seconds: 1,
		pattern: "^s(?:ec(?:ond)?s?)?",
	},
];

impl TimeUnitRaw {
	pub(crate) fn compile(&self) -> TimeUnit {
		let regex = RegexBuilder::new(self.pattern)
			.case_insensitive(true)
			.build()
			.unwrap();
		TimeUnit {
			seconds: self.seconds,
			regex,
		}
	}
}

pub(crate) struct TimeUnit {
	pub(crate) seconds: i64,
	pub(crate) regex: Regex,
}
