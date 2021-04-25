use super::icon;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn load_dmi() {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("tests/load_test.dmi");
	let path = Path::new(&path);
	let file = File::open(&path).unwrap_or_else(|_| panic!("No lights dmi: {:?}", path));
	let _lights_icon = icon::Icon::load(&file).expect("Unable to load lights dmi");
}
