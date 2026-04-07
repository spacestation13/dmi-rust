use dmi::dirs::Dirs;
use dmi::icon::{DmiVersion, Icon, IconState, Looping};
use image::{ImageReader, RgbaImage};
use std::fs;
use std::path::{Path, PathBuf};

fn test_path(relative: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests/resources")
		.join(relative)
}

fn load_icon_from_res(file_name: &str, label: &str) -> Icon {
	let path = test_path(file_name);
	let file = fs::File::open(&path).unwrap_or_else(|_| panic!("No {label} dmi at {path:?}"));

	Icon::load(&file).unwrap_or_else(|e| panic!("Unable to load {label} dmi: {e}"))
}

#[test]
fn load_and_save_dmi() {
	// test some atypical icon loading
	load_icon_from_res("empty.dmi", "empty");
	load_icon_from_res("greyscale_alpha.dmi", "greyscale_alpha");

	let lights_icon = load_icon_from_res("load_test.dmi", "lights");

	assert_eq!(lights_icon.version, DmiVersion::default());
	assert_eq!(lights_icon.width, 160);
	assert_eq!(lights_icon.height, 160);
	assert_eq!(lights_icon.states.len(), 4);

	assert_default_state(&lights_icon.states[0], "0_1");
	assert_default_state(&lights_icon.states[1], "1_1");
	assert_default_state(&lights_icon.states[2], "");
	assert_default_state(&lights_icon.states[3], r#"\\ \    \"\t\st\\\T+e=5235=!""#);

	// test state name re-encoding
	let write_path = test_path("tmp/save_test.dmi");
	fs::create_dir_all(write_path.parent().unwrap()).expect("Failed to create tmp dir");

	let mut write_file = fs::File::create(&write_path).expect("Failed to create dmi file");
	lights_icon
		.save(&mut write_file)
		.expect("Failed to save lights dmi");

	let reload_file = fs::File::open(&write_path).expect("Failed to open saved dmi");
	let reloaded = Icon::load_meta(&reload_file).expect("Unable to load metadata");

	assert_eq!(reloaded.states.len(), 4);
	assert_default_state(&reloaded.states[3], r#"\\ \    \"\t\st\\\T+e=5235=!""#);
}

fn assert_default_state(state: &IconState, name: &str) {
	assert_eq!(state.name, name);
	assert_eq!(state.dirs, 1);
	assert_eq!(state.frames, 1);
	assert_eq!(state.delay, None);
	assert_eq!(state.loop_flag, Looping::Indefinitely);
	assert!(!state.rewind);
	assert!(!state.movement);
	assert_eq!(state.hotspot, None);
	assert_eq!(state.unknown_settings, None);
}

#[test]
fn frames_iter_test() {
	let load_path = test_path("dirs_frames.dmi");
	let file = fs::File::open(&load_path).expect("Missing test dmi");
	let icon = Icon::load(&file).expect("Failed to load dmi");
	let state = icon.states.first().expect("DMI has no states");

	let frames_dir = test_path("frames");

	let mut frames_extracted = Vec::with_capacity(state.frames as usize);
	let mut frames_saved = Vec::with_capacity(state.frames as usize);

	for i in 1..=state.frames {
		let frame_name = format!("frame_{i}.png");

		let south_img = state
			.get_image(&Dirs::SOUTH, i)
			.expect("Failed to get image");
		frames_extracted.push(south_img.clone());

		let saved_image = ImageReader::open(frames_dir.join(&frame_name))
			.expect("Missing reference frame")
			.decode()
			.expect("Failed to decode reference frame")
			.to_rgba8();

		frames_saved.push(saved_image);
	}

	if images_differ(&frames_extracted, &frames_saved) {
		panic!("Frames extracted from DMI differed from saved frame_x.png!")
	}
}

#[test]
fn dirs_iter_test() {
	let load_path = test_path("dirs_frames.dmi");
	let file = fs::File::open(&load_path).expect("Missing test dmi");
	let icon = Icon::load(&file).expect("Failed to load dmi");
	let state = icon.states.first().expect("DMI has no states");

	let dirs_ref_dir = test_path("dirs");

	let target_frame = 2;
	let mut dirs_extracted = Vec::with_capacity(dmi::dirs::CARDINAL_DIRS.len());
	let mut dirs_saved = Vec::with_capacity(dmi::dirs::CARDINAL_DIRS.len());

	for (i, dir) in dmi::dirs::CARDINAL_DIRS.iter().enumerate() {
		let img = state
			.get_image(dir, target_frame)
			.unwrap_or_else(|_| panic!("Failed to get image for dir {dir:?}"));
		dirs_extracted.push(img.clone());

		let ref_path = dirs_ref_dir.join(format!("dir_{i}.png"));

		let saved_image = ImageReader::open(&ref_path)
			.unwrap_or_else(|_| panic!("Missing reference dir image: {ref_path:?}"))
			.decode()
			.expect("Failed to decode reference dir image")
			.to_rgba8();

		dirs_saved.push(saved_image);
	}

	if images_differ(&dirs_extracted, &dirs_saved) {
		panic!(
			"Images extracted via direction iteration differed from saved dir_x.png reference files!"
		);
	}
}

fn images_differ(image_1s: &[RgbaImage], image_2s: &[RgbaImage]) -> bool {
	image_1s.iter().zip(image_2s).any(|(img1, img2)| {
		if img1.dimensions() != img2.dimensions() {
			return true;
		}

		img1.pixels().zip(img2.pixels()).any(|(p1, p2)| {
			// If both are fully transparent, they are equal regardless of RGB
			if p1[3] == 0 && p2[3] == 0 {
				return false;
			}

			// Check if any channel differs by more than 2
			for i in 0..4 {
				if (p1[i] as i32 - p2[i] as i32).abs() > 2 {
					return true;
				}
			}
			false
		})
	})
}
