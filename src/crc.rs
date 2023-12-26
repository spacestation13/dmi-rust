pub(crate) fn calculate_crc<'a, I: IntoIterator<Item = &'a u8>>(buffer: I) -> u32 {
	const CRC_POLYNOMIAL: u32 = 0xedb8_8320;

	fn update_crc(crc: u32, message: u8) -> u32 {
		let message: u32 = u32::from(message);
		let mut crc = crc ^ message;
		for _ in 0..8 {
			crc = (if crc & 1 != 0 { CRC_POLYNOMIAL } else { 0 }) ^ (crc >> 1);
		}
		crc
	}

	buffer
		.into_iter()
		.fold(u32::MAX, |crc, message| update_crc(crc, *message))
		^ u32::MAX
}
