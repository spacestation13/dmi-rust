use std::fs::File;
use std::path::PathBuf;
use dmi::icon::Icon;

#[test]
fn load_dmi() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/resources/load_test.dmi");
    let file = File::open(path.as_path()).unwrap_or_else(|_| panic!("No lights dmi: {path:?}"));
    let _lights_icon = Icon::load(&file).expect("Unable to load lights dmi");
}
