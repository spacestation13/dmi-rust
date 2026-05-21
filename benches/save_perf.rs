use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dmi::icon::Icon;
use dmi::iconstate::{IconState, Looping};
use image::RgbaImage;
use std::io::Cursor;

/// Builds an Icon with `num_states` icon states, each having `dirs` directions
/// and `frames` frames of `size`x`size` RGBA8 pixels. Pixel values are a simple
/// gradient so the PNG compressor does not trivially collapse the data.
fn make_icon(size: u32, num_states: u32, dirs: u8, frames: u32) -> Icon {
	let mut states = Vec::with_capacity(num_states as usize);
	for s in 0..num_states {
		let mut images = Vec::with_capacity((dirs as u32 * frames) as usize);
		for i in 0..(dirs as u32 * frames) {
			let mut img = RgbaImage::new(size, size);
			for (x, y, pixel) in img.enumerate_pixels_mut() {
				let r = ((x + s * 7) % 256) as u8;
				let g = ((y + i * 13) % 256) as u8;
				let b = ((x + y + s + i) % 256) as u8;
				*pixel = image::Rgba([r, g, b, 200]);
			}
			images.push(img);
		}
		let delay = if frames > 1 {
			Some(vec![1.0f32; frames as usize])
		} else {
			None
		};
		states.push(IconState {
			name: format!("state_{s}"),
			dirs,
			frames,
			images,
			delay,
			loop_flag: Looping::Indefinitely,
			rewind: false,
			movement: false,
			hotspot: None,
			unknown_settings: None,
		});
	}
	Icon {
		version: dmi::icon::DmiVersion::default(),
		width: size,
		height: size,
		states,
	}
}

fn bench_save(c: &mut Criterion) {
	// (label, size, states, dirs, frames)
	let cases: &[(&str, u32, u32, u8, u32)] = &[
		("item_32px", 32, 10, 1, 1),     //   10 sprites
		("effect_512px", 512, 5, 1, 5),  //   25 sprites
		("mob_32px_4dir", 32, 30, 4, 4), //  480 sprites
		("mob_32px_8dir", 32, 20, 8, 8), // 1280 sprites
		("big_32px_8dir", 32, 50, 8, 8), // 3200 sprites
	];

	let mut group = c.benchmark_group("save");
	for &(label, size, num_states, dirs, frames) in cases {
		let icon = make_icon(size, num_states, dirs, frames);
		let total_sprites = num_states * dirs as u32 * frames;
		group.bench_with_input(
			BenchmarkId::new(label, format!("{total_sprites}_sprites")),
			&icon,
			|b, icon| {
				b.iter(|| {
					let mut out = Cursor::new(Vec::new());
					icon.save(&mut out).unwrap();
				});
			},
		);
	}
	group.finish();
}

criterion_group!(benches, bench_save);
criterion_main!(benches);
