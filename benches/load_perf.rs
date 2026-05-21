use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dmi::icon::Icon;
use std::io::Cursor;
use std::path::Path;

fn load_dmi_bytes(name: &str) -> Vec<u8> {
	let path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests/resources")
		.join(name);
	std::fs::read(path).unwrap_or_else(|_| panic!("missing {name}"))
}

fn bench_load(c: &mut Criterion) {
	let files = &["load_test.dmi", "dirs_frames.dmi"];

	let mut group = c.benchmark_group("load");
	for &name in files {
		let bytes = load_dmi_bytes(name);

		group.bench_with_input(BenchmarkId::new("load_full", name), &bytes, |b, bytes| {
			b.iter(|| {
				let cursor = Cursor::new(bytes.as_slice());
				Icon::load(cursor).unwrap();
			});
		});

		group.bench_with_input(BenchmarkId::new("load_meta", name), &bytes, |b, bytes| {
			b.iter(|| {
				let cursor = Cursor::new(bytes.as_slice());
				Icon::load_meta(cursor).unwrap();
			});
		});
	}
	group.finish();
}

criterion_group!(benches, bench_load);
criterion_main!(benches);
