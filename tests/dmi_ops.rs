use dmi::icon::Icon;
use std::fs::File;
use std::path::PathBuf;

#[test]
fn load_and_save_dmi() {
	let mut load_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	load_path.push("tests/resources/load_test.dmi");
	let load_file = File::open(load_path.as_path()).unwrap_or_else(|_| panic!("No lights dmi: {load_path:?}"));
	let lights_icon = Icon::load(&load_file).expect("Unable to load lights dmi");
	let mut write_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	write_path.push("tests/resources/save_test.dmi");
	let mut write_file = File::create(write_path.as_path()).expect("Failed to create dmi file");
	let _written_dmi = lights_icon.save(&mut write_file).expect("Failed to save lights dmi");
}
