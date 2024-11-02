pub mod chunk;
pub(crate) mod crc;
pub mod dirs;
pub mod error;
pub mod icon;
pub mod iend;
pub mod ztxt;

use std::io::{Read, Write};

/// The PNG magic header
pub const PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

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
				"PNG header mismatch (expected {:#?}, found {:#?})",
				PNG_HEADER, header
			)));
		};
		let header = PNG_HEADER;
		let mut chunk_ihdr = None;
		let mut chunk_ztxt = None;
		let mut chunk_plte = None;
		let mut chunks_idat = vec![];
		let chunk_iend;
		let mut other_chunks = vec![];

		// Index starts after the PNG header.
		let mut index = 8;

		loop {
			if index + 12 > dmi_bytes.len() {
				return Err(error::DmiError::Generic(
					"Failed to load DMI. Buffer end reached without finding an IEND chunk.".to_string(),
				));
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
			return Err(error::DmiError::Generic(
				"Failed to load DMI. Buffer end reached without finding an IHDR chunk.".to_string(),
			));
		};
		if chunks_idat.is_empty() {
			return Err(error::DmiError::Generic(
				"Failed to load DMI. Buffer end reached without finding an IDAT chunk.".to_string(),
			));
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

	pub fn save<W: Write>(&self, mut writter: &mut W) -> Result<usize, error::DmiError> {
		let bytes_written = writter.write(&self.header)?;
		let mut total_bytes_written = bytes_written;
		if bytes_written < 8 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = self.chunk_ihdr.save(&mut writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.chunk_ihdr.data_length) as usize + 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		if let Some(chunk_ztxt) = &self.chunk_ztxt {
  				let bytes_written = chunk_ztxt.save(&mut writter)?;
  				total_bytes_written += bytes_written;
  				if bytes_written < u32::from_be_bytes(chunk_ztxt.data_length) as usize + 12 {
  					return Err(error::DmiError::Generic(format!(
  						"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
  						total_bytes_written
  					)));
  				};
  			};

		if let Some(chunk_plte) = &self.chunk_plte {
  				let bytes_written = chunk_plte.save(&mut writter)?;
  				total_bytes_written += bytes_written;
  				if bytes_written < u32::from_be_bytes(chunk_plte.data_length) as usize + 12 {
  					return Err(error::DmiError::Generic(format!(
  						"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
  						total_bytes_written
  					)));
  				};
  			};

		if let Some(other_chunks) = &self.other_chunks {
  				for chunk in other_chunks {
  					let bytes_written = chunk.save(&mut writter)?;
  					total_bytes_written += bytes_written;
  					if bytes_written < u32::from_be_bytes(chunk.data_length) as usize + 12 {
  						return Err(error::DmiError::Generic(format!(
  							"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
  							total_bytes_written
  						)));
  					};
  				}
  			}

		for chunk in &self.chunks_idat {
			let bytes_written = chunk.save(&mut writter)?;
			total_bytes_written += bytes_written;
			if bytes_written < u32::from_be_bytes(chunk.data_length) as usize + 12 {
				return Err(error::DmiError::Generic(format!(
					"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
					total_bytes_written
				)));
			};
		}

		let bytes_written = self.chunk_iend.save(&mut writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.chunk_iend.data_length) as usize + 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		Ok(total_bytes_written)
	}
}
