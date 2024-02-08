Parses strings like "5y6mo" and "3.5 minutes" into `chrono::Duration`s. It is intended to be a quick and simple solution for text-based user input, like for scheduling reminders or other events.

It is faithful with regards to the variable durations of years and months. For example, starting from 2000-02-01 (February of a leap year), 1 month means 29 days, and -1 month means -31 days.

## Usage

```rs
let duration = parse_interval_with_now("2 days 15 hours 15 mins");
assert_eq!(duration, Ok(chrono::Duration::seconds(227700)));
```
```rs
let duration = parse_interval_with_lazy_date("1 month", || {
	NaiveDate::from_ymd_opt(2000, 2, 1)
		.unwrap()
		.and_time(NaiveTime::default())
		.and_utc()
});
assert_eq!(duration, Ok(chrono::Duration::days(29)));
```

The input format is designed to be somewhat flexible, but it has a particular, opinionated set of rules.

The format is as follows:

`<number> years <number> months <number> weeks <number> days <number> hours <number> minutes <number> seconds`

Each unit is optional, but all present units need to be in order. All units are case insensitive. All spaces are optional and excess spaces are allowed. Numbers for years and months can't have decimals, but those for the other units can. The combination of units does not need to make sense, e.g. "1 week 20 days" will simply be 27 days, and ".5d12h" will simply be 1 day. It also does not validate grammar, so it would accept "1 weeks 20 day".

* `years` can also be written as `year` or `y`
* `months` can also be written as `month` or `mo`
* `weeks` can also be written as `week` or `w`
* `days` can also be written as `day` or `d`
* `hours` can also be written as `hour`, `hrs`, `hr` or `h`
* `minutes` can also be written as `minute`, `mins`, `min` or `m`
* `seconds` can also be written as `second`, `secs`, `sec` or `s`

A `-` can be inserted before any number to subtract all the units that follow it. Another `-` will make the following units additive again (as if subtracting from the previous subtraction). For example, "1d - 10m 30s" describes an interval 10.5 minutes short of a day. "1d - 10m - 30s" describes an interval 9.5 minutes short of a day. Intervals can be negative as a whole, resulting in a negative `Duration`.

Because years and months vary in their actual duration, to process them, some date needs to be chosen as a starting point. A `DateTime<Utc>` can be supplied for this purpose, or the library can use the current system time. It is also an option to just not handle years and months at all.