use super::chunk;
use super::error;
use std::convert::TryFrom;
use std::io::prelude::*;

pub const IEND_TYPE: [u8; 4] = [b'I', b'E', b'N', b'D'];

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RawIendChunk {
	pub data_length: [u8; 4],
	pub chunk_type: [u8; 4],
	pub crc: [u8; 4],
}

impl RawIendChunk {
	pub fn new() -> RawIendChunk {
		RawIendChunk {
			..Default::default()
		}
	}

	pub fn length(&self) -> usize {
		self.data_length.len() + self.chunk_type.len() + self.crc.len()
	}

	pub fn load<R: Read>(reader: &mut R) -> Result<RawIendChunk, error::DmiError> {
		let default_iend_chunk = RawIendChunk::new();

		let mut raw_chunk_bytes = Vec::new();
		reader.read_to_end(&mut raw_chunk_bytes)?;

		let total_bytes_length = raw_chunk_bytes.len();
		if total_bytes_length != default_iend_chunk.length() {
			return Err(error::DmiError::Generic(format!(
				"Failed to load RawIendChunk from reader. Size: {}. Expected: {}.",
				raw_chunk_bytes.len(), default_iend_chunk.length()
			)));
		}

		let data_length = [
			raw_chunk_bytes[0],
			raw_chunk_bytes[1],
			raw_chunk_bytes[2],
			raw_chunk_bytes[3],
		];
		if data_length != default_iend_chunk.data_length {
			return Err(error::DmiError::Generic(format!("Failed to load RawIendChunk from reader. Lengh field value: {:#?}. Expected: {:#?}.", data_length, default_iend_chunk.data_length)));
		}

		let chunk_type = [
			raw_chunk_bytes[4],
			raw_chunk_bytes[5],
			raw_chunk_bytes[6],
			raw_chunk_bytes[7],
		];
		if chunk_type != default_iend_chunk.chunk_type {
			return Err(error::DmiError::Generic(format!("Failed to load RawIendChunk from reader. Chunk type: {:#?}. Expected {:#?}.", chunk_type, default_iend_chunk.chunk_type)));
		}

		let crc = [
			raw_chunk_bytes[total_bytes_length - 4],
			raw_chunk_bytes[total_bytes_length - 3],
			raw_chunk_bytes[total_bytes_length - 2],
			raw_chunk_bytes[total_bytes_length - 1],
		];
		if crc != default_iend_chunk.crc {
			return Err(error::DmiError::Generic(format!("Failed to load RawIendChunk from reader. CRC: {:#?}. Expected {:#?}.", crc, default_iend_chunk.crc)));
		}

		Ok(default_iend_chunk)
	}

	pub fn save<W: Write>(&self, writter: &mut W) -> Result<usize, error::DmiError> {
		let bytes_written = writter.write(&self.data_length)?;
		let mut total_bytes_written = bytes_written;
		if bytes_written < self.data_length.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save IEND chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.chunk_type)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.chunk_type.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save IEND chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.crc)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.crc.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save IEND chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		Ok(total_bytes_written)
	}
}

impl Default for RawIendChunk {
	fn default() -> Self {
		let data_length = [0, 0, 0, 0];
		let chunk_type = IEND_TYPE;
		let crc = [174, 66, 96, 130];
		RawIendChunk {
			data_length,
			chunk_type,
			crc,
		}
	}
}

impl TryFrom<chunk::RawGenericChunk> for RawIendChunk {
	type Error = error::DmiError;
	fn try_from(raw_generic_chunk: chunk::RawGenericChunk) -> Result<Self, Self::Error> {
		if raw_generic_chunk.data.len() > 0 {
			return Err(error::DmiError::Generic(format!("Failed to convert RawGenericChunk into RawIendChunk. Non-empty data field. Chunk: {:#?}.", raw_generic_chunk)));
		};

		let default_iend_chunk = RawIendChunk::new();

		if raw_generic_chunk.chunk_type != default_iend_chunk.chunk_type {
			return Err(error::DmiError::Generic(format!("Failed to convert RawGenericChunk into RawIendChunk. Wrong type: {:#?}. Expected: {:#?}.", raw_generic_chunk.chunk_type, default_iend_chunk.chunk_type)));
		};
		if raw_generic_chunk.crc != default_iend_chunk.crc {
			return Err(error::DmiError::Generic(format!("Failed to convert RawGenericChunk into RawIendChunk. Mismatching CRC: {:#?}. Expected: {:#?}.", raw_generic_chunk.crc, default_iend_chunk.crc)));
		}

		Ok(default_iend_chunk)
	}
}
