use bitflags::bitflags;

bitflags! {
	/// The possible values for a direction in DM.
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct Dirs: u8 {
		const NORTH =	1 << 0;
		const SOUTH =	1 << 1;
		const EAST =	1 << 2;
		const WEST =	1 << 3;
		const SOUTHEAST = Self::SOUTH.bits() | Self::EAST.bits();
		const SOUTHWEST = Self::SOUTH.bits() | Self::WEST.bits();
		const NORTHEAST = Self::NORTH.bits() | Self::EAST.bits();
		const NORTHWEST = Self::NORTH.bits() | Self::WEST.bits();
	}
}

impl std::fmt::Display for Dirs {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{} ({:?})", self.bits(), self)
	}
}

/// A list of every cardinal direction.
pub const CARDINAL_DIRS: [Dirs; 4] = [Dirs::NORTH, Dirs::SOUTH, Dirs::EAST, Dirs::WEST];

/// A list of every ordinal direction.
pub const ORDINAL_DIRS: [Dirs; 4] = [
	Dirs::NORTHEAST,
	Dirs::NORTHWEST,
	Dirs::SOUTHEAST,
	Dirs::SOUTHWEST,
];

/// A list of every direction, cardinals and ordinals.
pub const ALL_DIRS: [Dirs; 8] = [
	Dirs::NORTH,
	Dirs::SOUTH,
	Dirs::EAST,
	Dirs::WEST,
	Dirs::NORTHEAST,
	Dirs::NORTHWEST,
	Dirs::SOUTHEAST,
	Dirs::SOUTHWEST,
];
