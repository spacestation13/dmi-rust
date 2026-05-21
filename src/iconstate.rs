use std::{collections::HashMap, num::NonZeroU32};

use image::RgbaImage;

use crate::{
	dirs::{ALL_DIRS, CARDINAL_DIRS, Dirs},
	error::DmiError,
	icon::dir_to_dmi_index,
};

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
			Some(idx) => ((frame as usize - 1) * self.dirs as usize) + idx,
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
				self.name,
				self.images.len(),
				self.dirs,
				self.frames
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
