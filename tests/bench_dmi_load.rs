/*
use dmi::icon::Icon;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

const ICONS_FOLDER: &'static str = "UPDATEME";

#[test]
fn bench_dmi_load() {
	let icons_folder_path = Path::new(ICONS_FOLDER);

	println!("Icon::load_meta bench\n---");

	let mut num_calls = 0;
	let mut microsec_calls = 0;
	for _ in 0..25 {
		recurse_process(
			icons_folder_path,
			&mut num_calls,
			&mut microsec_calls,
			false,
		);
	}
	println!("Num calls: {num_calls}");
	println!("Total Call Duration (μs): {microsec_calls}");
	let mtpc = microsec_calls / num_calls as u128;
	println!("MTPC (μs): {mtpc}");

	println!("Icon::load bench\n---");

	num_calls = 0;
	microsec_calls = 0;
	// this is seriously slow. 2 iterations max or you'll be waiting all day
	for _ in 0..1 {
		recurse_process(
			icons_folder_path,
			&mut num_calls,
			&mut microsec_calls,
			false,
		);
	}
	println!("Num calls: {num_calls}");
	println!("Total Call Duration (μs): {microsec_calls}");
	let mtpc = microsec_calls / num_calls as u128;
	println!("MTPC (μs): {mtpc}");
}

fn recurse_process(path: &Path, num_calls: &mut u32, microsec_calls: &mut u128, load_images: bool) {
	if path.is_dir() {
		if let Ok(entries) = std::fs::read_dir(path) {
			for entry in entries.flatten() {
				let path = entry.path();
				recurse_process(&path, num_calls, microsec_calls, load_images);
			}
		}
	} else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
		if ext != "dmi" {
			return;
		}
		let load_file = File::open(path).unwrap_or_else(|_| panic!("No dmi: {path:?}"));
		let start = Instant::now();
		if load_images {
			let _ = Icon::load(&load_file).expect("Unable to dmi metadata");
		} else {
			let _ = Icon::load_meta(&load_file).expect("Unable to dmi metadata");
		}
		*microsec_calls += start.elapsed().as_micros();
		*num_calls += 1;
	}
}
*/
