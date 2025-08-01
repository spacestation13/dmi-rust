use dmi::icon::{DmiVersion, Icon, IconState, Looping};
use std::fs::File;
use std::path::PathBuf;

#[test]
fn load_and_save_dmi() {

	let mut load_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	load_path.push("tests/resources/empty.dmi");
	let load_file =
		File::open(load_path.as_path()).unwrap_or_else(|_| panic!("No empty dmi: {load_path:?}"));
	let _ = Icon::load(&load_file).expect("Unable to load empty dmi");

	let mut load_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	load_path.push("tests/resources/greyscale_alpha.dmi");
	let load_file =
		File::open(load_path.as_path()).unwrap_or_else(|_| panic!("No greyscale_alpha dmi: {load_path:?}"));
	let _ = Icon::load(&load_file).expect("Unable to greyscale_alpha dmi");

	let mut load_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	load_path.push("tests/resources/load_test.dmi");
	let load_file =
		File::open(load_path.as_path()).unwrap_or_else(|_| panic!("No lights dmi: {load_path:?}"));
	let lights_icon = Icon::load(&load_file).expect("Unable to load lights dmi");

	assert_eq!(lights_icon.version, DmiVersion::default());
	assert_eq!(lights_icon.width, 160);
	assert_eq!(lights_icon.height, 160);
	assert_eq!(lights_icon.states.len(), 4);

	assert_default_state(&lights_icon.states[0], "0_1");
	assert_default_state(&lights_icon.states[1], "1_1");
	assert_default_state(&lights_icon.states[2], "");
	assert_default_state(
		&lights_icon.states[3],
		"\\\\ \\    \\\"\\t\\st\\\\\\T+e=5235=!\"",
	);

	let mut write_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	write_path.push("tests/resources/save_test.dmi");
	let mut write_file = File::create(write_path.as_path()).expect("Failed to create dmi file");
	let _written_dmi = lights_icon
		.save(&mut write_file)
		.expect("Failed to save lights dmi");

	let load_write_file =
		File::open(write_path.as_path()).unwrap_or_else(|_| panic!("No lights dmi: {load_path:?}"));
	let reloaded_lights_icon = Icon::load_meta(&load_write_file).expect("Unable to load lights dmi");

	assert_eq!(reloaded_lights_icon.version, DmiVersion::default());
	assert_eq!(reloaded_lights_icon.width, 160);
	assert_eq!(reloaded_lights_icon.height, 160);
	assert_eq!(reloaded_lights_icon.states.len(), 4);

	assert_default_state(&reloaded_lights_icon.states[0], "0_1");
	assert_default_state(&reloaded_lights_icon.states[1], "1_1");
	assert_default_state(&reloaded_lights_icon.states[2], "");
	assert_default_state(
		&reloaded_lights_icon.states[3],
		"\\\\ \\    \\\"\\t\\st\\\\\\T+e=5235=!\"",
	);
}

fn assert_default_state(state: &IconState, name: &'static str) {
	assert_eq!(state.name, name);
	assert_eq!(state.dirs, 1);
	assert_eq!(state.frames, 1);
	assert_eq!(state.delay, None);
	assert_eq!(state.loop_flag, Looping::Indefinitely);
	assert_eq!(state.rewind, false);
	assert_eq!(state.movement, false);
	assert_eq!(state.hotspot, None);
	assert_eq!(state.unknown_settings, None);
}
