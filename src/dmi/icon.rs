use super::error;
use super::ztxt;
use super::RawDmi;

use image::imageops;
use image::GenericImageView;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::Cursor;

#[derive(Clone, Default)]
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
			let mut loop_flag = None;
			let mut rewind = None;
			let mut movement = None;
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
					"\tloop" => loop_flag = Some(split_version[1].parse::<u32>()?),
					"\trewind" => rewind = Some(split_version[1].parse::<u32>()?),
					"\tmovement" => movement = Some(split_version[1].parse::<u32>()?),
					"\thotspot" => {
						let text_coordinates: Vec<&str> = split_version[1].split_terminator(',').collect();
						if text_coordinates.len() != 3 {
							return Err(error::DmiError::Generic(format!(
								"Error loading icon: improper hotspot found: {:#?}",
								split_version
							)));
						};
						hotspot = Some([
							text_coordinates[0].parse::<u32>()?,
							text_coordinates[1].parse::<u32>()?,
							text_coordinates[2].parse::<u32>()?,
						]);
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

			if dirs == None || frames == None {
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
				if let Some(flag) = icon_state.loop_flag {
					signature.push_str(&format!("\tloop = {}\n", flag))
				}
				if let Some(flag) = icon_state.rewind {
					signature.push_str(&format!("\trewind = {}\n", flag))
				}
				if let Some(flag) = icon_state.movement {
					signature.push_str(&format!("\tmovement = {}\n", flag))
				}
			};

			if let Some(array) = icon_state.hotspot {
				signature.push_str(&format!(
					"\tarray = {},{},{}\n",
					array[0], array[1], array[2]
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
		new_png.write_to(&mut dmi_data, image::ImageOutputFormat::Png)?;
		let mut new_dmi = RawDmi::load(&dmi_data.into_inner()[..])?;

		let new_ztxt = ztxt::create_ztxt_chunk(signature.as_bytes())?;

		new_dmi.chunk_ztxt = Some(new_ztxt);

		new_dmi.save(&mut writter)
	}
}

#[derive(Clone)]
pub struct IconState {
	pub name: String,
	pub dirs: u8,
	pub frames: u32,
	pub images: Vec<image::DynamicImage>,
	pub delay: Option<Vec<f32>>,
	pub loop_flag: Option<u32>,
	pub rewind: Option<u32>,
	pub movement: Option<u32>,
	pub hotspot: Option<[u32; 3]>,
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
			loop_flag: None,
			rewind: None,
			movement: None,
			hotspot: None,
			unknown_settings: None,
		}
	}
}

#[derive(Clone)]
pub struct DmiVersion(String);

impl Default for DmiVersion {
	fn default() -> Self {
		DmiVersion("4.0".to_string())
	}
}
