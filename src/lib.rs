pub mod chunk;
pub(crate) mod crc;
pub mod dirs;
pub mod error;
pub mod icon;
pub mod iend;
pub mod ztxt;

use std::io::{Cursor, Read, Seek, Write};

/// The PNG magic header
pub const PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
pub const IHDR_HEADER: [u8; 8] = [0, 0, 0, 13, 73, 72, 68, 82];
const ASSUMED_ZTXT_MAX: usize = 500;

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct RawDmi {
	pub header: [u8; 8],
	pub chunk_ihdr: chunk::RawGenericChunk,
	pub chunk_ztxt: Option<ztxt::RawZtxtChunk>,
	pub chunk_plte: Option<chunk::RawGenericChunk>,
	pub other_chunks: Option<Vec<chunk::RawGenericChunk>>,
	pub chunks_idat: Vec<chunk::RawGenericChunk>,
	pub chunk_iend: iend::RawIendChunk,
}

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct RawDmiMetadata {
	pub chunk_ihdr: chunk::RawGenericChunk,
	pub chunk_ztxt: ztxt::RawZtxtChunk,
}

impl RawDmi {
	pub fn new() -> RawDmi {
		RawDmi {
			..Default::default()
		}
	}

	pub fn load<R: Read>(mut reader: R) -> Result<RawDmi, error::DmiError> {
		let mut dmi_bytes = Vec::new();
		reader.read_to_end(&mut dmi_bytes)?;
		// 8 bytes for the PNG file signature.
		// 12 + 13 bytes for the IHDR chunk.
		// 12 for the IDAT chunk.
		// 12 + 3 for the zTXt chunk.
		// 12 for the IEND chunk.

		// Total minimum size for a DMI file: 72 bytes.

		if dmi_bytes.len() < 72 {
			return Err(error::DmiError::Generic(format!("Failed to load DMI. Supplied reader contained size of {} bytes, lower than the required 72.", dmi_bytes.len())));
		};

		let header = &dmi_bytes[0..8];
		if dmi_bytes[0..8] != PNG_HEADER {
			return Err(error::DmiError::Generic(format!(
				"PNG header mismatch (expected {PNG_HEADER:#?}, found {header:#?})"
			)));
		};
		let header = PNG_HEADER;
		let mut chunk_ihdr = None;
		let mut chunk_ztxt = None;
		let mut chunk_plte = None;
		let mut chunks_idat: Vec<chunk::RawGenericChunk> = vec![];
		let chunk_iend;
		let mut other_chunks = vec![];

		// Index starts after the PNG header.
		let mut index = 8;

		loop {
			if index + 12 > dmi_bytes.len() {
				return Err(error::DmiError::Generic(String::from(
					"Failed to load DMI. Buffer end reached without finding an IEND chunk.",
				)));
			}

			let chunk_data_length = u32::from_be_bytes([
				dmi_bytes[index],
				dmi_bytes[index + 1],
				dmi_bytes[index + 2],
				dmi_bytes[index + 3],
			]) as usize;

			// 12 minimum necessary bytes from the chunk plus the data length.
			let chunk_bytes = dmi_bytes[index..(index + 12 + chunk_data_length)].to_vec();
			let raw_chunk = chunk::RawGenericChunk::load(&mut &*chunk_bytes)?;
			index += 12 + chunk_data_length;

			match &raw_chunk.chunk_type {
				b"IHDR" => chunk_ihdr = Some(raw_chunk),
				b"zTXt" => chunk_ztxt = Some(ztxt::RawZtxtChunk::try_from(raw_chunk)?),
				b"PLTE" => chunk_plte = Some(raw_chunk),
				b"IDAT" => chunks_idat.push(raw_chunk),
				b"IEND" => {
					chunk_iend = Some(iend::RawIendChunk::try_from(raw_chunk)?);
					break;
				}
				_ => other_chunks.push(raw_chunk),
			}
		}
		if chunk_ihdr.is_none() {
			return Err(error::DmiError::Generic(String::from(
				"Failed to load DMI. Buffer end reached without finding an IHDR chunk.",
			)));
		};
		if chunks_idat.is_empty() {
			return Err(error::DmiError::Generic(String::from(
				"Failed to load DMI. Buffer end reached without finding an IDAT chunk.",
			)));
		}
		let other_chunks = match other_chunks.len() {
			0 => None,
			_ => Some(other_chunks),
		};
		let chunk_ihdr = chunk_ihdr.unwrap();
		let chunk_iend = chunk_iend.unwrap();

		Ok(RawDmi {
			header,
			chunk_ihdr,
			chunk_ztxt,
			chunk_plte,
			other_chunks,
			chunks_idat,
			chunk_iend,
		})
	}

	/// Equivalent of load, but only parses IHDR and zTXt. May not catch an improperly formatted PNG file, because it only reads those headers.
	pub fn load_meta<R: Read + Seek>(mut reader: R) -> Result<RawDmiMetadata, error::DmiError> {
		let mut dmi_bytes = vec![0u8; ASSUMED_ZTXT_MAX];

		// Since we only want the zTXt it's unlikely to be any longer than ASSUMED_ZTXT_MAX bytes when combined with headers until we encounter it
		// If the zTxt is especially long and its length exceeds our index we can read extra bytes later.
		let mut dmi_bytes_read = reader.read(&mut dmi_bytes)?;

		if dmi_bytes_read < 72 {
			return Err(error::DmiError::Generic(format!("Failed to load DMI. Supplied reader contained size of {} bytes, lower than the required 72.", dmi_bytes.len())));
		};

		let mut buffered_dmi_bytes = Cursor::new(dmi_bytes);

		// 8 bytes for the PNG file signature.
		let mut png_header = [0u8; 8];
		buffered_dmi_bytes.read_exact(&mut png_header)?;
		if png_header != PNG_HEADER {
			return Err(error::DmiError::Generic(format!(
				"PNG header mismatch (expected {PNG_HEADER:#?}, found {png_header:#?})"
			)));
		};
		// 4 (size) + 4 (type) + 13 (data) + 4 (crc) for the IHDR chunk.
		let mut ihdr = [0u8; 25];
		buffered_dmi_bytes.read_exact(&mut ihdr)?;
		if ihdr[0..8] != IHDR_HEADER {
			return Err(error::DmiError::Generic(
				String::from("Failed to load DMI. IHDR chunk is not in the correct location (1st chunk), has an invalid size, or an invalid identifier."),
			));
		}
		let chunk_ihdr = chunk::RawGenericChunk::load(&mut &ihdr[0..25])?;

		let mut chunk_ztxt = None;

		loop {
			// Read len
			let mut chunk_len_be: [u8; 4] = [0u8; 4];
			buffered_dmi_bytes.read_exact(&mut chunk_len_be)?;
			let chunk_len = u32::from_be_bytes(chunk_len_be) as usize;

			// Create vec for full chunk data
			let mut chunk_full: Vec<u8> = Vec::with_capacity(chunk_len + 12);
			chunk_full.extend_from_slice(&chunk_len_be);

			// Read header into full chunk data
			let mut chunk_header = [0u8; 4];
			buffered_dmi_bytes.read_exact(&mut chunk_header)?;
			chunk_full.extend_from_slice(&chunk_header);

			// If we encounter IDAT or IEND we can just break because the zTXt header aint happening
			if &chunk_header == b"IDAT" || &chunk_header == b"IEND" {
				break;
			}

			// We will overread the file's buffer.
			let original_position = buffered_dmi_bytes.position();
			if original_position + chunk_len as u64 + 12 > dmi_bytes_read as u64 {
				// Read the remainder of the chunk + 4 bytes for CRC + 8 bytes for the next header.
				// There will always be a next header because IEND headers break before this check.
				let mut new_dmi_bytes = vec![0u8; chunk_len + 12];
				reader.read_exact(&mut new_dmi_bytes)?;
				// Append all the new bytes to our cursor and go back to our old spot
				buffered_dmi_bytes.seek_relative(dmi_bytes_read as i64 - original_position as i64)?;
				buffered_dmi_bytes.write_all(&new_dmi_bytes)?;
				dmi_bytes_read += new_dmi_bytes.len();
				buffered_dmi_bytes.seek_relative(original_position as i64 - dmi_bytes_read as i64)?;
			}

			// Skip non-zTXt chunks
			if &chunk_header != b"zTXt" {
				buffered_dmi_bytes.seek_relative((chunk_len + 4) as i64)?;
				continue;
			}

			// Read actual chunk data and append
			let mut chunk_data = vec![0; chunk_len];
			buffered_dmi_bytes.read_exact(&mut chunk_data)?;
			chunk_full.extend_from_slice(&chunk_data);

			// Read CRC into full chunk data
			let mut chunk_crc = [0u8; 4];
			buffered_dmi_bytes.read_exact(&mut chunk_crc)?;
			chunk_full.extend_from_slice(&chunk_crc);

			let raw_chunk = chunk::RawGenericChunk::load(&mut &*chunk_full)?;

			chunk_ztxt = Some(ztxt::RawZtxtChunk::try_from(raw_chunk)?);
		}

		if chunk_ztxt.is_none() {
			return Err(error::DmiError::Generic(String::from(
				"Failed to load DMI. zTXt chunk was not found or is after the first IDAT chunk.",
			)));
		}
		let chunk_ztxt = chunk_ztxt.unwrap();

		Ok(RawDmiMetadata {
			chunk_ihdr,
			chunk_ztxt,
		})
	}

	pub fn save<W: Write>(&self, mut writter: &mut W) -> Result<usize, error::DmiError> {
		let bytes_written = writter.write(&self.header)?;
		let mut total_bytes_written = bytes_written;
		if bytes_written < 8 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {total_bytes_written} bytes written."
			)));
		};

		let bytes_written = self.chunk_ihdr.save(&mut writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.chunk_ihdr.data_length) as usize + 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {total_bytes_written} bytes written."
			)));
		};

		if let Some(chunk_ztxt) = &self.chunk_ztxt {
			let bytes_written = chunk_ztxt.save(&mut writter)?;
			total_bytes_written += bytes_written;
			if bytes_written < u32::from_be_bytes(chunk_ztxt.data_length) as usize + 12 {
				return Err(error::DmiError::Generic(format!(
					"Failed to save DMI. Buffer unable to hold the data, only {total_bytes_written} bytes written."
				)));
			};
		};

		if let Some(chunk_plte) = &self.chunk_plte {
			let bytes_written = chunk_plte.save(&mut writter)?;
			total_bytes_written += bytes_written;
			if bytes_written < u32::from_be_bytes(chunk_plte.data_length) as usize + 12 {
				return Err(error::DmiError::Generic(format!(
					"Failed to save DMI. Buffer unable to hold the data, only {total_bytes_written} bytes written."
				)));
			};
		};

		if let Some(other_chunks) = &self.other_chunks {
			for chunk in other_chunks {
				let bytes_written = chunk.save(&mut writter)?;
				total_bytes_written += bytes_written;
				if bytes_written < u32::from_be_bytes(chunk.data_length) as usize + 12 {
					return Err(error::DmiError::Generic(format!(
						"Failed to save DMI. Buffer unable to hold the data, only {total_bytes_written} bytes written."
					)));
				};
			}
		}

		for chunk in &self.chunks_idat {
			let bytes_written = chunk.save(&mut writter)?;
			total_bytes_written += bytes_written;
			if bytes_written < u32::from_be_bytes(chunk.data_length) as usize + 12 {
				return Err(error::DmiError::Generic(format!(
					"Failed to save DMI. Buffer unable to hold the data, only {total_bytes_written} bytes written."
				)));
			};
		}

		let bytes_written = self.chunk_iend.save(&mut writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.chunk_iend.data_length) as usize + 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {total_bytes_written} bytes written."
			)));
		};

		Ok(total_bytes_written)
	}
}
