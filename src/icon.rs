use crate::{error, ztxt, RawDmi};
use image::codecs::png;
use image::imageops;
use image::GenericImageView;
use image::ImageEncoder;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::Cursor;
use std::num::NonZeroU32;

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Icon {
	pub version: DmiVersion,
	pub width: u32,
	pub height: u32,
	pub states: Vec<IconState>,
}

impl Icon {
	pub fn load<R: Read>(reader: R) -> Result<Icon, error::DmiError> {
		let raw_dmi = RawDmi::load(reader)?;
		let chunk_ztxt = match &raw_dmi.chunk_ztxt {
			Some(chunk) => chunk.clone(),
			None => {
				return Err(error::DmiError::Generic(
					"Error loading icon: no zTXt chunk found.".to_string(),
				))
			}
		};
		let decompressed_text = chunk_ztxt.data.decode()?;
		let decompressed_text = String::from_utf8(decompressed_text)?;
		let mut decompressed_text = decompressed_text.lines();

		let current_line = decompressed_text.next();
		if current_line != Some("# BEGIN DMI") {
			return Err(error::DmiError::Generic(format!(
				"Error loading icon: no DMI header found. Beginning: {:#?}",
				current_line
			)));
		};

		let current_line = match decompressed_text.next() {
			Some(thing) => thing,
			None => {
				return Err(error::DmiError::Generic(
					"Error loading icon: no version header found.".to_string(),
				))
			}
		};
		let split_version: Vec<&str> = current_line.split_terminator(" = ").collect();
		if split_version.len() != 2 || split_version[0] != "version" {
			return Err(error::DmiError::Generic(format!(
				"Error loading icon: improper version header found: {:#?}",
				split_version
			)));
		};
		let version = split_version[1].to_string();

		let current_line = match decompressed_text.next() {
			Some(thing) => thing,
			None => {
				return Err(error::DmiError::Generic(
					"Error loading icon: no width found.".to_string(),
				))
			}
		};
		let split_version: Vec<&str> = current_line.split_terminator(" = ").collect();
		if split_version.len() != 2 || split_version[0] != "\twidth" {
			return Err(error::DmiError::Generic(format!(
				"Error loading icon: improper width found: {:#?}",
				split_version
			)));
		};
		let width = split_version[1].parse::<u32>()?;

		let current_line = match decompressed_text.next() {
			Some(thing) => thing,
			None => {
				return Err(error::DmiError::Generic(
					"Error loading icon: no height found.".to_string(),
				))
			}
		};
		let split_version: Vec<&str> = current_line.split_terminator(" = ").collect();
		if split_version.len() != 2 || split_version[0] != "\theight" {
			return Err(error::DmiError::Generic(format!(
				"Error loading icon: improper height found: {:#?}",
				split_version
			)));
		};
		let height = split_version[1].parse::<u32>()?;

		if width == 0 || height == 0 {
			return Err(error::DmiError::Generic(format!(
				"Error loading icon: invalid width ({}) / height ({}) values.",
				width, height
			)));
		};

		// Image time.
		let mut reader = vec![];
		raw_dmi.save(&mut reader)?;
		let base_image = image::load_from_memory_with_format(&reader, image::ImageFormat::Png)?;

		let dimensions = base_image.dimensions();
		let img_width = dimensions.0;
		let img_height = dimensions.1;

		if img_width == 0 || img_height == 0 || img_width % width != 0 || img_height % height != 0 {
			return Err(error::DmiError::Generic(format!("Error loading icon: invalid image width ({}) / height ({}) values. Missmatch with metadata width ({}) / height ({}).", img_width, img_height, width, height)));
		};

		let width_in_states = img_width / width;
		let height_in_states = img_height / height;
		let max_possible_states = width_in_states * height_in_states;

		let mut index = 0;

		let mut current_line = match decompressed_text.next() {
			Some(thing) => thing,
			None => {
				return Err(error::DmiError::Generic(
					"Error loading icon: no DMI trailer nor states found.".to_string(),
				))
			}
		};

		let mut states = vec![];

		loop {
			if current_line.contains("# END DMI") {
				break;
			};

			let split_version: Vec<&str> = current_line.split_terminator(" = ").collect();
			if split_version.len() != 2 || split_version[0] != "state" {
				return Err(error::DmiError::Generic(format!(
					"Error loading icon: improper state found: {:#?}",
					split_version
				)));
			};

			let name = split_version[1].as_bytes();
			if !name.starts_with(&[b'\"']) || !name.ends_with(&[b'\"']) {
				return Err(error::DmiError::Generic(format!("Error loading icon: invalid name icon_state found in metadata, should be preceded and succeeded by double-quotes (\"): {:#?}", name)));
			};
			let name = match name.len() {
				0 | 1 => {
					return Err(error::DmiError::Generic(format!(
						"Error loading icon: invalid name icon_state found in metadata, improper size: {:#?}",
						name
					)))
				}
				2 => String::new(), //Only the quotes, empty name otherwise.
				length => String::from_utf8(name[1..(length - 1)].to_vec())?, //Hacky way to trim. Blame the cool methods being nightly experimental.
			};

			let mut dirs = None;
			let mut frames = None;
			let mut delay = None;
			let mut loop_flag = Looping::Indefinitely;
			let mut rewind = false;
			let mut movement = false;
			let mut hotspot = None;
			let mut unknown_settings = None;

			loop {
				current_line = match decompressed_text.next() {
					Some(thing) => thing,
					None => {
						return Err(error::DmiError::Generic(
							"Error loading icon: no DMI trailer found.".to_string(),
						))
					}
				};

				if current_line.contains("# END DMI") || current_line.contains("state = \"") {
					break;
				};
				let split_version: Vec<&str> = current_line.split_terminator(" = ").collect();
				if split_version.len() != 2 {
					return Err(error::DmiError::Generic(format!(
						"Error loading icon: improper state found: {:#?}",
						split_version
					)));
				};

				match split_version[0] {
					"\tdirs" => dirs = Some(split_version[1].parse::<u8>()?),
					"\tframes" => frames = Some(split_version[1].parse::<u32>()?),
					"\tdelay" => {
						let mut delay_vector = vec![];
						let text_delays = split_version[1].split_terminator(',');
						for text_entry in text_delays {
							delay_vector.push(text_entry.parse::<f32>()?);
						}
						delay = Some(delay_vector);
					}
					"\tloop" => loop_flag = Looping::new(split_version[1].parse::<u32>()?),
					"\trewind" => rewind = split_version[1].parse::<u8>()? != 0,
					"\tmovement" => movement = split_version[1].parse::<u8>()? != 0,
					"\thotspot" => {
						let text_coordinates: Vec<&str> = split_version[1].split_terminator(',').collect();
						// Hotspot includes a mysterious 3rd parameter that always seems to be 1.
						if text_coordinates.len() != 3 {
							return Err(error::DmiError::Generic(format!(
								"Error loading icon: improper hotspot found: {:#?}",
								split_version
							)));
						};
						hotspot = Some(Hotspot {
							x: text_coordinates[0].parse::<u32>()?,
							y: text_coordinates[1].parse::<u32>()?,
						});
					}
					_ => {
						unknown_settings = match unknown_settings {
							None => {
								let mut new_map = HashMap::new();
								new_map.insert(split_version[0].to_string(), split_version[1].to_string());
								Some(new_map)
							}
							Some(mut thing) => {
								thing.insert(split_version[0].to_string(), split_version[1].to_string());
								Some(thing)
							}
						};
					}
				};
			}

			if dirs.is_none() || frames.is_none() {
				return Err(error::DmiError::Generic(format!(
					"Error loading icon: state lacks essential settings. dirs: {:#?}. frames: {:#?}.",
					dirs, frames
				)));
			};
			let dirs = dirs.unwrap();
			let frames = frames.unwrap();

			if index + (dirs as u32 * frames) > max_possible_states {
				return Err(error::DmiError::Generic(format!("Error loading icon: metadata settings exceeded the maximum number of states possible ({}).", max_possible_states)));
			};

			let mut images = vec![];

			for _frame in 0..frames {
				for _dir in 0..dirs {
					let x = (index % width_in_states) * width;
					//This operation rounds towards zero, truncating any fractional part of the exact result, essentially a floor() function.
					let y = (index / width_in_states) * height;
					images.push(base_image.crop_imm(x, y, width, height));
					index += 1;
				}
			}

			states.push(IconState {
				name,
				dirs,
				frames,
				images,
				delay,
				loop_flag,
				rewind,
				movement,
				hotspot,
				unknown_settings,
			});
		}

		Ok(Icon {
			version: DmiVersion(version),
			width,
			height,
			states,
		})
	}

	pub fn save<W: Write>(&self, mut writter: &mut W) -> Result<usize, error::DmiError> {
		let mut sprites = vec![];
		let mut signature = format!(
			"# BEGIN DMI\nversion = {}\n\twidth = {}\n\theight = {}\n",
			self.version.0, self.width, self.height
		);

		for icon_state in &self.states {
			if icon_state.images.len() as u32 != icon_state.dirs as u32 * icon_state.frames {
				return Err(error::DmiError::Generic(format!("Error saving Icon: number of images ({}) differs from the stated metadata. Dirs: {}. Frames: {}. Name: \"{}\".", icon_state.images.len(), icon_state.dirs, icon_state.frames, icon_state.name)));
			};

			signature.push_str(&format!(
				"state = \"{}\"\n\tdirs = {}\n\tframes = {}\n",
				icon_state.name, icon_state.dirs, icon_state.frames
			));

			if icon_state.frames > 1 {
				match &icon_state.delay {
					Some(delay) => {
						if delay.len() as u32 != icon_state.frames {
							return Err(error::DmiError::Generic(format!("Error saving Icon: number of frames ({}) differs from the delay entry ({:3?}). Name: \"{}\".", icon_state.frames, delay, icon_state.name)))
						};
						let delay: Vec<String>= delay.iter().map(|&c| c.to_string()).collect();
						signature.push_str(&format!("\tdelay = {}\n", delay.join(",")));
					},
					None => return Err(error::DmiError::Generic(format!("Error saving Icon: number of frames ({}) larger than one without a delay entry in icon state of name \"{}\".", icon_state.frames, icon_state.name)))
				};
				if let Looping::NTimes(flag) = icon_state.loop_flag {
					signature.push_str(&format!("\tloop = {}\n", flag))
				}
				if icon_state.rewind {
					signature.push_str("\trewind = 1\n");
				}
				if icon_state.movement {
					signature.push_str("\tmovement = 1\n");
				}
			};

			if let Some(Hotspot { x, y }) = icon_state.hotspot {
				signature.push_str(&format!(
					// Mysterious third parameter here doesn't seem to do anything. Unable to find
					// any example of it not being 1.
					"\thotspot = {x},{y},1\n"
				))
			};

			match &icon_state.unknown_settings {
				Some(hashmap) => {
					for (setting, value) in hashmap.iter() {
						signature.push_str(&format!("\t{} = {}\n", setting, value));
					}
				}
				None => (),
			};

			sprites.extend(icon_state.images.iter());
		}

		signature.push_str("# END DMI\n");

		let max_index = (sprites.len() as f64).sqrt().ceil() as u32;
		let mut new_png =
			image::DynamicImage::new_rgba8(max_index * self.width, max_index * self.height);

		for image in sprites.iter().enumerate() {
			let index = image.0 as u32;
			let image = image.1;
			imageops::replace(
				&mut new_png,
				*image,
				(self.width * (index % max_index)).into(),
				(self.height * (index / max_index)).into(),
			);
		}

		let mut dmi_data = Cursor::new(vec![]);
		// We're futzing around with pngs directly here so we can use the best possible compression
		let bytes = new_png.as_bytes();
		let (width, height) = new_png.dimensions();
		let color = new_png.color();
		let encoder = png::PngEncoder::new_with_quality(&mut dmi_data, png::CompressionType::Best, png::FilterType::Adaptive);
		encoder.write_image(bytes, width, height, color)?;
		let mut new_dmi = RawDmi::load(&dmi_data.into_inner()[..])?;

		let new_ztxt = ztxt::create_ztxt_chunk(signature.as_bytes())?;

		new_dmi.chunk_ztxt = Some(new_ztxt);

		new_dmi.save(&mut writter)
	}
}

/// Represents the Looping flag in an [IconState], which is used to determine how to loop an
/// animated [IconState]
///
/// - `Indefinitely`: Loop repeatedly as long as the [IconState] is displayed
/// - `NTimes(NonZeroU32)`: Loop N times before freezing on the final frame. Stored as a `NonZeroU32`
/// for memory efficiency reasons, looping 0 times is an invalid state.
///
/// This type is effectively a newtype of `Option<NonZeroU32>`. As such, `From<Looping>` is
/// implemented for `Option<NonZeroU32>` as well as `Option<u32>`. If the more advanced combinators
/// or `?` operator of the native `Option` type are desired, this type can be `into` either
/// previously mentioned types.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Looping {
	#[default]
	Indefinitely,
	NTimes(NonZeroU32),
}

impl Looping {
	/// Creates a new `NTimes` variant with `x` number of times to loop
	pub fn new(x: u32) -> Self {
		Self::NTimes(NonZeroU32::new(x).unwrap())
	}

	/// Unwraps the Looping yielding the `u32` if the `Looping` is a `Looping::NTimes`
	/// # Panics
	/// Panics if `self` is `Looping::Indefinitely`
	pub fn unwrap(self) -> u32 {
		match self {
			Self::NTimes(times) => times.get(),
			_ => panic!("Attempted to unwrap a looping that was indefinite"),
		}
	}

	/// Unwraps the Looping yielding the `u32` if the `Looping` is an `NTimes`
	/// If the `Looping` is an `Indefinitely`, yields `u32::default()` which is 0
	pub fn unwrap_or_default(self) -> u32 {
		match self {
			Self::NTimes(times) => times.get(),
			_ => u32::default(), // 0
		}
	}

	/// Unwraps the Looping yielding the `u32` if the `Looping` is an `NTimes`
	/// If the `Looping` is an `Indefinitely`, yields the value provided as `default`
	pub fn unwrap_or(self, default: u32) -> u32 {
		match self {
			Self::NTimes(times) => times.get(),
			_ => default,
		}
	}
}

impl From<Looping> for Option<u32> {
	fn from(value: Looping) -> Self {
		match value {
			Looping::Indefinitely => None,
			Looping::NTimes(backing) => Some(backing.get()),
		}
	}
}

impl From<Looping> for Option<NonZeroU32> {
	fn from(value: Looping) -> Self {
		match value {
			Looping::Indefinitely => None,
			Looping::NTimes(backing) => Some(backing),
		}
	}
}

/// Represents a "Hotspot" as used by an [IconState]. A "Hotspot" is a marked pixel on an [IconState]
/// which is used as the click location when the [IconState] is used as a cursor. The default cursor
/// places it at the tip, but a crosshair may want to have it centered.
///
/// Note that "y" is inverted from standard image axes, bottom left of the sprite is used as 0 and
/// y increases as you move upwards.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct Hotspot {
	pub x: u32,
	pub y: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct IconState {
	pub name: String,
	pub dirs: u8,
	pub frames: u32,
	pub images: Vec<image::DynamicImage>,
	pub delay: Option<Vec<f32>>,
	pub loop_flag: Looping,
	pub rewind: bool,
	pub movement: bool,
	pub hotspot: Option<Hotspot>,
	pub unknown_settings: Option<HashMap<String, String>>,
}

impl Default for IconState {
	fn default() -> Self {
		IconState {
			name: String::new(),
			dirs: 1,
			frames: 1,
			images: vec![],
			delay: None,
			loop_flag: Looping::Indefinitely,
			rewind: false,
			movement: false,
			hotspot: None,
			unknown_settings: None,
		}
	}
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct DmiVersion(String);

impl Default for DmiVersion {
	fn default() -> Self {
		DmiVersion("4.0".to_string())
	}
}
