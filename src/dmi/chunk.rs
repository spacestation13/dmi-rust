use super::crc;
use super::error;
use std::io::prelude::*;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RawGenericChunk {
	pub data_length: [u8; 4],
	pub chunk_type: [u8; 4],
	pub data: Vec<u8>,
	pub crc: [u8; 4],
}

impl RawGenericChunk {
	pub fn load<R: Read>(reader: &mut R) -> Result<RawGenericChunk, error::DmiError> {
		let mut chunk_bytes = Vec::new();
		reader.read_to_end(&mut chunk_bytes)?;

		// 4 bytes for the length.
		// 4 bytes for the type.
		// Data can be 0 bytes.
		// 4 bytes for the CRC.

		// Total minimum size for an undetermined PNG chunk: 12 bytes.
		let chunk_length = chunk_bytes.len();

		if chunk_length < 12 {
			return Err(error::DmiError::Generic(format!("Failed to load Chunk. Supplied reader contained size of {} bytes, lower than the required 12.", chunk_length)));
		};

		let data_length = [
			chunk_bytes[0],
			chunk_bytes[1],
			chunk_bytes[2],
			chunk_bytes[3],
		];

		let chunk_type = [
			chunk_bytes[4],
			chunk_bytes[5],
			chunk_bytes[6],
			chunk_bytes[7],
		];

		// The chunk type is made of four ascii characters. The valid ranges are A-Z and a-z.
		if !chunk_type
			.iter()
			.all(|c| (b'A' <= *c && *c <= b'Z') || (b'a' <= *c && *c <= b'z'))
		{
			return Err(error::DmiError::Generic(format!(
				"Failed to load Chunk. Type contained unlawful characters: {:#?}",
				chunk_type
			)));
		};

		let data: Vec<u8> = chunk_bytes[8..(chunk_length - 4)].iter().cloned().collect();

		let crc = [
			chunk_bytes[chunk_length - 4],
			chunk_bytes[chunk_length - 3],
			chunk_bytes[chunk_length - 2],
			chunk_bytes[chunk_length - 1],
		];

		let recalculated_crc = crc::calculate_crc(chunk_type.iter().chain(data.iter()));
		if u32::from_be_bytes(crc) != recalculated_crc {
			let chunk_name = String::from_utf8(chunk_type.to_vec())?;
			return Err(error::DmiError::Generic(format!("Failed to load Chunk of type {}. Supplied CRC invalid: {:#?}. Its value ({}) does not match the recalculated one ({}).", chunk_name, crc, u32::from_be_bytes(crc), recalculated_crc)));
		}

		Ok(RawGenericChunk {
			data_length,
			chunk_type,
			data,
			crc,
		})
	}

	pub fn save<W: Write>(&self, writter: &mut W) -> Result<usize, error::DmiError> {
		let bytes_written = writter.write(&self.data_length)?;
		let mut total_bytes_written = bytes_written;
		if bytes_written < self.data_length.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save Chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.chunk_type)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.chunk_type.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save Chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.data)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.data.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save Chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.crc)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.crc.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save Chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		Ok(total_bytes_written)
	}
}
