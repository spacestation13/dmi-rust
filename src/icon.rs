use crate::dirs::{Dirs, ALL_DIRS, CARDINAL_DIRS};
use crate::{error::DmiError, ztxt, RawDmi, RawDmiMetadata};
use ::png::{ColorType, Decoder, Transformations};
use image::codecs::png;
use image::{imageops, RgbaImage};
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::Cursor;
use std::num::NonZeroU32;

#[derive(Clone, Default, PartialEq, Debug)]
/// A DMI Icon, which is a collection of [IconState]s.
pub struct Icon {
	pub version: DmiVersion,
	pub width: u32,
	pub height: u32,
	pub states: Vec<IconState>,
}

/// The ordering of directions within a DMI file.
pub const DIR_ORDERING: [Dirs; 8] = [
	Dirs::SOUTH,
	Dirs::NORTH,
	Dirs::EAST,
	Dirs::WEST,
	Dirs::SOUTHEAST,
	Dirs::SOUTHWEST,
	Dirs::NORTHEAST,
	Dirs::NORTHWEST,
];

/// Given a Dir, gives its order within a DMI file (equivalent: DIR_ORDERING.iter().position(|d| d == dir))
pub fn dir_to_dmi_index(dir: &Dirs) -> Option<usize> {
	match *dir {
		Dirs::SOUTH => Some(0),
		Dirs::NORTH => Some(1),
		Dirs::EAST => Some(2),
		Dirs::WEST => Some(3),
		Dirs::SOUTHEAST => Some(4),
		Dirs::SOUTHWEST => Some(5),
		Dirs::NORTHEAST => Some(6),
		Dirs::NORTHWEST => Some(7),
		_ => None,
	}
}

struct DmiHeaders {
	version: String,
	width: Option<u32>,
	height: Option<u32>,
}

/// Splits the line of a DMI entry into a key/value pair on the equals sign.
/// The second string cannot be empty (a value must exist), or a DmiError is returned.
/// Only one set of quotes is allowed if allow_quotes is true, and it must wrap the entire value.
/// If require_quotes is set, will error if there are not quotes around the value.
///
/// Other details about this function:
///
/// Keys have very little validation and are meant to be checked against a known value in most cases.
/// Spaces are only allowed in the value if they are inside quotes or directly after the equals sign (where they are removed).
/// Tabs and equals signs are only allowed in the value if they are not inside quotes.
/// Removes quotes around values and removes backslashes for quotes inside the quotes.
/// Removes backslashes used to escape other backslashes.
fn parse_dmi_line(
	line: &str,
	allow_quotes: bool,
	require_quotes: bool,
) -> Result<(&str, String), DmiError> {
	let line_split = line.split_once(" = ");
	if line_split.is_none() {
		return Err(DmiError::BlockEntry(format!(
			"No value was found for line: '{line}' (must contain ' = ')!"
		)));
	}
	let line_split = line_split.unwrap();
	// Now we need to parse after the equals
	let num_chars = line_split.1.len();
	let mut post_equals = String::with_capacity(num_chars - if require_quotes { 2 } else { 0 });

	// Flags
	let mut quoted = false;
	let mut escaped = false;
	let mut used_quotes = false;

	let value_bytes = line_split.1.as_bytes();
	for (char_idx, char) in value_bytes.iter().enumerate() {
		let char = *char as char;
		let escape_this_char = escaped;
		escaped = false;
		match char {
			'\\' => {
				if !quoted {
					return Err(DmiError::Generic(format!(
						"Backslash found in line with value '{line}' after first equals without quotes."
					)));
				}
				if !escape_this_char {
					escaped = true;
					continue;
				}
			}
			'"' => {
				if !allow_quotes {
					return Err(DmiError::Generic(format!("Quote found in line with value '{line}' after first equals where they are not allowed.")));
				}
				if !escape_this_char {
					if quoted && char_idx + 1 != num_chars {
						return Err(DmiError::BlockEntry(format!("Line with value '{line}' ends quotes prior to the last character on the line. This is not allowed.")));
					} else if !quoted && !post_equals.is_empty() {
						return Err(DmiError::BlockEntry(format!("Line with value '{line}' starts quotes after the first character in its value. This is not allowed.")));
					}
					quoted = !quoted;
					used_quotes = true;
					continue;
				}
			}
			'\t' | '=' | ' ' => {
				if !quoted {
					return Err(DmiError::BlockEntry(format!("Invalid character {char} found in line with value '{line}' after first equals without quotes.")));
				}
			}
			_ => {}
		}
		post_equals.push(char);
	}
	if allow_quotes && require_quotes && !used_quotes {
		return Err(DmiError::Generic(format!("Line with value '{line}' is required to have quotes after the equals sign, but does not wrap its contents in quotes!")));
	}
	Ok((line_split.0, post_equals))
}

fn read_dmi_headers(
	decompressed_text: &mut std::iter::Peekable<std::str::Lines<'_>>,
) -> Result<DmiHeaders, DmiError> {
	let current_line = decompressed_text.next();
	if current_line != Some("# BEGIN DMI") {
		return Err(DmiError::Generic(format!(
			"Error loading icon: no DMI header found. Beginning: {current_line:#?}"
		)));
	};

	let current_line = match decompressed_text.next() {
		Some(thing) => thing,
		None => {
			return Err(DmiError::Generic(String::from(
				"Error loading icon: no version header found.",
			)))
		}
	};
	let (key, value) = parse_dmi_line(current_line, false, false)?;
	if key != "version" {
		return Err(DmiError::Generic(format!(
			"Error loading icon: improper version header found: {key} = {value} ('{current_line}')"
		)));
	};
	let version = value;

	let mut width = None;
	let mut height = None;
	for _ in 0..2 {
		let current_line = match decompressed_text.peek() {
			Some(thing) => *thing,
			None => {
				return Err(DmiError::Generic(String::from(
					"Error loading icon: DMI definition abruptly ends.",
				)))
			}
		};
		let (key, value) = parse_dmi_line(current_line, false, false)?;
		match key {
			"\twidth" => {
				width = Some(value.parse::<u32>()?);
				decompressed_text.next(); // consume the peeked value
			}
			"\theight" => {
				height = Some(value.parse::<u32>()?);
				decompressed_text.next(); // consume the peeked value
			}
			_ => {
				break;
			}
		}
	}

	if width == Some(0) || height == Some(0) {
		return Err(DmiError::Generic(format!(
			"Error loading icon: invalid width ({width:#?}) / height ({height:#?}) values."
		)));
	};

	Ok(DmiHeaders {
		version,
		width,
		height,
	})
}

impl Icon {
	pub fn load<R: Read + Seek>(reader: R) -> Result<Icon, DmiError> {
		Self::load_internal(reader, true)
	}

	/// Returns an Icon {} without any images inside of the IconStates and with less error validation.
	/// This is suitable for reading DMI metadata without caring about the actual images within.
	/// Can load a full DMI about 10x faster than Icon::load.
	pub fn load_meta<R: Read + Seek>(reader: R) -> Result<Icon, DmiError> {
		Self::load_internal(reader, false)
	}

	fn load_internal<R: Read + Seek>(reader: R, load_images: bool) -> Result<Icon, DmiError> {
		let (dmi_meta, rgba_bytes) = if load_images {
			let raw_dmi = RawDmi::load(reader)?;

			// Reconstruct the full PNG from memory. Preallocating the size saves a lot of compute here.
			let mut png_data = Vec::with_capacity(raw_dmi.output_buffer_size(false));
			raw_dmi.save(&mut png_data, false)?;

			let mut png_decoder = Decoder::new(std::io::Cursor::new(png_data));
			// this will convert RGB->RGBA and increase bit depth to 8, interpret tRNS chunks, interpret PLTE chunks
			// notably does not convert greyscale color types to RGB.
			png_decoder.set_transformations(Transformations::EXPAND | Transformations::ALPHA);
			let mut png_reader = png_decoder.read_info()?;
			let mut rgba_buf = vec![0u8; png_reader.output_buffer_size()];
			let info = png_reader.next_frame(&mut rgba_buf)?;

			// EXPAND and ALPHA do not expand grayscale images into RGBA. We can just do this manually.
			match info.color_type {
				ColorType::GrayscaleAlpha => {
					if rgba_buf.len() as u32 != info.width * info.height * 2 {
						return Err(DmiError::Generic(String::from(
							"GrayscaleAlpha buffer length mismatch",
						)));
					}
					let mut new_buf = Vec::with_capacity((info.width * info.height * 4) as usize);
					for chunk in rgba_buf.chunks(2) {
						let gray = chunk[0];
						let alpha = chunk[1];
						new_buf.push(gray);
						new_buf.push(gray);
						new_buf.push(gray);
						new_buf.push(alpha);
					}
					rgba_buf = new_buf;
				}
				ColorType::Grayscale => {
					if rgba_buf.len() as u32 != info.width * info.height {
						return Err(DmiError::Generic(String::from(
							"Grayscale buffer length mismatch",
						)));
					}
					let mut new_buf = Vec::with_capacity((info.width * info.height * 4) as usize);
					for gray in rgba_buf {
						new_buf.push(gray);
						new_buf.push(gray);
						new_buf.push(gray);
						new_buf.push(255);
					}
					rgba_buf = new_buf;
				}
				ColorType::Rgba => {}
				_ => {
					return Err(DmiError::Generic(format!(
						"Unsupported ColorType (must be RGBA or convertible to RGBA): {:#?}",
						info.color_type
					)));
				}
			}

			let dmi_meta = RawDmiMetadata {
				chunk_ihdr: raw_dmi.chunk_ihdr,
				chunk_ztxt: raw_dmi.chunk_ztxt.ok_or_else(|| {
					DmiError::Generic(String::from("Error loading icon: no zTXt chunk found."))
				})?,
			};

			(dmi_meta, Some(rgba_buf))
		} else {
			(RawDmi::load_meta(reader)?, None)
		};

		let chunk_ztxt = &dmi_meta.chunk_ztxt;
		let decompressed_text = chunk_ztxt.data.decode()?;
		let decompressed_text = String::from_utf8(decompressed_text)?;
		let mut decompressed_text = decompressed_text.lines().peekable();

		let dmi_headers = read_dmi_headers(&mut decompressed_text)?;
		let version = dmi_headers.version;

		// yes you can make a DMI without a width or height. it defaults to 32x32
		let width = dmi_headers.width.unwrap_or(32);
		let height = dmi_headers.height.unwrap_or(32);

		let ihdr_data = dmi_meta.chunk_ihdr.data;

		let img_width: u32 =
			u32::from_be_bytes([ihdr_data[0], ihdr_data[1], ihdr_data[2], ihdr_data[3]]);
		let img_height = u32::from_be_bytes([ihdr_data[4], ihdr_data[5], ihdr_data[6], ihdr_data[7]]);

		if img_width == 0 || img_height == 0 || img_width % width != 0 || img_height % height != 0 {
			return Err(DmiError::Generic(format!("Error loading icon: invalid image width ({img_width}) / height ({img_height}) values. Mismatch with metadata width ({width}) / height ({height}).")));
		};

		let width_in_states = img_width / width;
		let height_in_states = img_height / height;
		let max_possible_states = width_in_states * height_in_states;

		let mut index = 0;

		let mut current_line = match decompressed_text.next() {
			Some(thing) => thing,
			None => {
				return Err(DmiError::Generic(
					"Error loading icon: no DMI trailer nor states found.".to_string(),
				))
			}
		};

		let mut states = vec![];

		loop {
			if current_line == "# END DMI" {
				break;
			};

			let (key, value) = parse_dmi_line(current_line, true, true)?;
			if key != "state" {
				return Err(DmiError::Generic(format!(
					"Error loading icon: Was expecting the next line's entry to have a key of 'state', but encountered '{key}'! The full line contents are as follows: '{current_line}'"
				)));
			};

			let name: String = value;

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
						return Err(DmiError::Generic(
							"Error loading icon: no DMI trailer found.".to_string(),
						))
					}
				};

				if current_line == "# END DMI" || !current_line.starts_with('\t') {
					break;
				};
				let (key, value) = parse_dmi_line(current_line, false, false)?;

				match key {
					"\tdirs" => dirs = Some(value.parse::<u8>()?),
					"\tframes" => frames = Some(value.parse::<u32>()?),
					"\tdelay" => {
						let mut delay_vector = Vec::with_capacity(frames.unwrap_or(0) as usize);
						let text_delays = value.split_terminator(',');
						for text_entry in text_delays {
							delay_vector.push(text_entry.parse::<f32>()?);
						}
						delay = Some(delay_vector);
					}
					"\tloop" => loop_flag = Looping::new(value.parse::<u32>()?),
					"\trewind" => rewind = value.parse::<u8>()? != 0,
					"\tmovement" => movement = value.parse::<u8>()? != 0,
					"\thotspot" => {
						let text_coordinates: Vec<&str> = value.split_terminator(',').collect();
						// Hotspot includes a mysterious 3rd parameter that always seems to be 1.
						if text_coordinates.len() != 3 {
							return Err(DmiError::Generic(format!(
								"Error loading icon: improper hotspot found: {current_line:#?}"
							)));
						};
						hotspot = Some(Hotspot {
							x: text_coordinates[0].parse::<u32>()?,
							y: text_coordinates[1].parse::<u32>()?,
						});
					}
					_ => {
						if let Some((key, value)) = current_line.split_once(" = ") {
							unknown_settings = match unknown_settings {
								None => {
									let mut new_map = HashMap::new();
									new_map.insert(key.to_string(), value.to_string());
									Some(new_map)
								}
								Some(mut thing) => {
									thing.insert(key.to_string(), value.to_string());
									Some(thing)
								}
							};
						}
					}
				};
			}

			if dirs.is_none() || frames.is_none() {
				return Err(DmiError::Generic(format!(
					"Error loading icon: state lacks essential settings. dirs: {dirs:#?}. frames: {frames:#?}."
				)));
			};
			let dirs = dirs.unwrap();
			let frames = frames.unwrap();

			let next_index = index + (dirs as u32 * frames);
			if next_index > max_possible_states {
				return Err(DmiError::Generic(format!("Error loading icon: metadata settings exceeded the maximum number of states possible ({max_possible_states}).")));
			};

			let mut images = Vec::with_capacity((frames * dirs as u32) as usize);

			if let Some(rgba_bytes) = &rgba_bytes {
				const RGBA_PIXEL_STRIDE: usize = 4;
				let row_stride = img_width as usize * RGBA_PIXEL_STRIDE;
				let expected_buffer_len = row_stride * (img_height as usize);
				if rgba_bytes.len() != expected_buffer_len {
					panic!("{} != {}", rgba_bytes.len(), expected_buffer_len);
				}

				for image_idx in index..next_index {
					let x = (image_idx % width_in_states) * width;
					let y = (image_idx / width_in_states) * height;

					let mut cropped =
						Vec::with_capacity((width * height * RGBA_PIXEL_STRIDE as u32) as usize);
					for row in y..(y + height) {
						let start = (row as usize * row_stride) + (x as usize * RGBA_PIXEL_STRIDE);
						let end = start + (width as usize * RGBA_PIXEL_STRIDE);
						cropped.extend_from_slice(&rgba_bytes[start..end]);
					}

					let tile = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, cropped)
						.ok_or_else(|| DmiError::Generic("Failed to create image tile".to_string()))?;

					images.push(tile);
				}
			}

			index = next_index;

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

	pub fn save<W: Write>(&self, mut writer: &mut W) -> Result<usize, DmiError> {
		let mut sprites = vec![];
		let mut signature = format!(
			"# BEGIN DMI\nversion = {}\n\twidth = {}\n\theight = {}\n",
			self.version.0, self.width, self.height
		);

		for icon_state in &self.states {
			if icon_state.images.len() as u32 != icon_state.dirs as u32 * icon_state.frames {
				return Err(DmiError::Generic(format!("Error saving Icon: number of images ({}) differs from the stated metadata. Dirs: {}. Frames: {}. Name: \"{}\".", icon_state.images.len(), icon_state.dirs, icon_state.frames, icon_state.name)));
			};

			signature.push_str(&format!(
				"state = \"{}\"\n\tdirs = {}\n\tframes = {}\n",
				icon_state.name.replace("\\", "\\\\").replace("\"", "\\\""),
				icon_state.dirs,
				icon_state.frames
			));

			if icon_state.frames > 1 {
				match &icon_state.delay {
					Some(delay) => {
						if delay.len() as u32 != icon_state.frames {
							return Err(DmiError::Generic(format!("Error saving Icon: number of frames ({}) differs from the delay entry ({delay:3?}). Name: \"{}\".", icon_state.frames, icon_state.name)))
						};
						let delay: Vec<String>= delay.iter().map(|&c| c.to_string()).collect();
						signature.push_str(&format!("\tdelay = {}\n", delay.join(",")));
					},
					None => return Err(DmiError::Generic(format!("Error saving Icon: number of frames ({}) larger than one without a delay entry in icon state of name \"{}\".", icon_state.frames, icon_state.name)))
				};
				if let Looping::NTimes(flag) = icon_state.loop_flag {
					signature.push_str(&format!("\tloop = {flag}\n"))
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

			if let Some(hashmap) = &icon_state.unknown_settings {
				for (setting, value) in hashmap.iter() {
					signature.push_str(&format!("\t{setting} = {value}\n"));
				}
			};

			sprites.extend(icon_state.images.iter());
		}

		signature.push_str("# END DMI\n");

		// We try to make a square png as output
		let states_rooted = (sprites.len() as f64).sqrt().ceil();
		// Then if it turns out we would have empty rows, we remove them
		let cell_width = states_rooted as u32;
		let cell_height = ((sprites.len() as f64) / states_rooted).ceil() as u32;
		let mut new_png =
			image::DynamicImage::new_rgba8(cell_width * self.width, cell_height * self.height);

		for image in sprites.iter().enumerate() {
			let index = image.0 as u32;
			let image = image.1;
			imageops::replace(
				&mut new_png,
				*image,
				(self.width * (index % cell_width)).into(),
				(self.height * (index / cell_width)).into(),
			);
		}

		let mut dmi_data = Cursor::new(vec![]);
		// Use the 'Default' compression - the actual default for the library is 'Fast'
		let encoder = png::PngEncoder::new_with_quality(
			&mut dmi_data,
			png::CompressionType::Default,
			png::FilterType::Adaptive,
		);
		new_png.write_with_encoder(encoder)?;
		let mut new_dmi = RawDmi::load(&dmi_data.into_inner()[..])?;

		let new_ztxt = ztxt::create_ztxt_chunk(signature.as_bytes())?;

		new_dmi.chunk_ztxt = Some(new_ztxt);

		new_dmi.save(&mut writer, true)
	}
}

/// Represents the Looping flag in an [IconState], which is used to determine how to loop an
/// animated [IconState]
///
/// - `Indefinitely`: Loop repeatedly as long as the [IconState] is displayed
/// - `NTimes(NonZeroU32)`: Loop N times before freezing on the final frame. Stored as a `NonZeroU32`
///
/// For memory efficiency reasons, looping 0 times is an invalid state.
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
		match x {
			0 => Self::default(),
			_ => Self::NTimes(NonZeroU32::new(x).unwrap()),
		}
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
	pub images: Vec<image::RgbaImage>,
	pub delay: Option<Vec<f32>>,
	pub loop_flag: Looping,
	pub rewind: bool,
	pub movement: bool,
	pub hotspot: Option<Hotspot>,
	pub unknown_settings: Option<HashMap<String, String>>,
}

impl IconState {
	/// Gets a specific DynamicImage from `images`, given a dir and frame.
	/// If the dir or frame is invalid, returns a DmiError.
	pub fn get_image(&self, dir: &Dirs, frame: u32) -> Result<&RgbaImage, DmiError> {
		if self.frames < frame {
			return Err(DmiError::IconState(format!(
				"Specified frame \"{frame}\" is larger than the number of frames ({}) for icon_state \"{}\"",
				self.frames, self.name
			)));
		}

		if (self.dirs == 1 && *dir != Dirs::SOUTH)
			|| (self.dirs == 4 && !CARDINAL_DIRS.contains(dir))
			|| (self.dirs == 8 && !ALL_DIRS.contains(dir))
		{
			return Err(DmiError::IconState(format!(
				"Dir specified {dir} is not in the set of valid dirs ({} dirs) for icon_state \"{}\"",
				self.dirs, self.name
			)));
		}

		let image_idx = match dir_to_dmi_index(dir) {
			Some(idx) => (idx + 1) * frame as usize - 1,
			None => {
				return Err(DmiError::IconState(format!(
					"Dir specified {dir} is not a valid dir within DMI ordering! (icon_state: {})",
					self.name
				)));
			}
		};

		match self.images.get(image_idx) {
			Some(image) => Ok(image),
			None => Err(DmiError::IconState(format!(
				"Out of bounds index {image_idx} in icon_state \"{}\" (images len: {} dirs: {}, frames: {} - dir: {dir}, frame: {frame})",
				self.name, self.images.len(), self.dirs, self.frames
			))),
		}
	}
}

impl Default for IconState {
	fn default() -> Self {
		Self {
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
