use chrono::{Duration, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use parse_interval::{parse_interval_with_date, parse_interval_with_now, ParseError};

fn lazy_date(interval: &str) -> Result<Duration, ParseError> {
	parse_interval_with_now(interval)
}

fn eager_date(interval: &str) -> Result<Duration, ParseError> {
	parse_interval_with_date(interval, Utc::now())
}

fn benchmark_lazy(c: &mut Criterion) {
	c.bench_function("Lazy date", |b| {
		b.iter(|| lazy_date(black_box("5 days 3 hours 10 minutes")))
	});
}

fn benchmark_eager(c: &mut Criterion) {
	c.bench_function("Eager date", |b| {
		b.iter(|| eager_date(black_box("5 days 3 hours 10 minutes")))
	});
}

fn benchmark_lazy_inconstant(c: &mut Criterion) {
	c.bench_function("Lazy date inconstant", |b| {
		b.iter(|| lazy_date(black_box("2 years 6 months 10 minutes")))
	});
}

fn benchmark_eager_inconstant(c: &mut Criterion) {
	c.bench_function("Eager date inconstant", |b| {
		b.iter(|| eager_date(black_box("2 years 6 months 10 minutes")))
	});
}

criterion_group!(
	benches,
	benchmark_lazy,
	benchmark_eager,
	benchmark_lazy_inconstant,
	benchmark_eager_inconstant
);
criterion_main!(benches);
