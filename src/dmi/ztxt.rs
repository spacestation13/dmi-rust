use super::chunk;
use super::crc;
use super::error;
use deflate;
use inflate;
use std::convert::TryFrom;
use std::fmt;
use std::io::prelude::*;

pub const ZTXT_TYPE: [u8; 4] = [b'z', b'T', b'X', b't'];

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RawZtxtChunk {
	pub data_length: [u8; 4],
	pub chunk_type: [u8; 4],
	pub data: RawZtxtData,
	pub crc: [u8; 4],
}

pub fn create_ztxt_chunk(dmi_signature: &[u8]) -> Result<RawZtxtChunk, error::DmiError> {
	let compressed_text = encode(dmi_signature);
	let data = RawZtxtData {
		compressed_text,
		..Default::default()
	};
	let mut data_bytes = vec![];
	data.save(&mut data_bytes)?;
	let data_length = (data_bytes.len() as u32).to_be_bytes();
	let chunk_type = ZTXT_TYPE;
	let crc = crc::calculate_crc(chunk_type.iter().chain(data_bytes.iter())).to_be_bytes();
	Ok(RawZtxtChunk {
		data_length,
		chunk_type,
		data,
		crc,
	})
}

impl RawZtxtChunk {
	pub fn load<R: Read>(reader: &mut R) -> Result<RawZtxtChunk, error::DmiError> {
		let mut raw_chunk_bytes = Vec::new();
		reader.read_to_end(&mut raw_chunk_bytes)?;
		let total_bytes_length = raw_chunk_bytes.len();
		if total_bytes_length < 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to load RawZtxtChunk from reader. Size: {}. Minimum necessary is 12.",
				raw_chunk_bytes.len()
			)));
		}
		let data_length = [
			raw_chunk_bytes[0],
			raw_chunk_bytes[1],
			raw_chunk_bytes[2],
			raw_chunk_bytes[3],
		];
		if u32::from_be_bytes(data_length) != total_bytes_length as u32 - 12 {
			return Err(error::DmiError::Generic(format!("Failed to load RawZtxtChunk from reader. Lengh field value ({}) does not match the actual data field size ({}).", u32::from_be_bytes(data_length), total_bytes_length -12)));
		}
		let chunk_type = [
			raw_chunk_bytes[4],
			raw_chunk_bytes[5],
			raw_chunk_bytes[6],
			raw_chunk_bytes[7],
		];
		if chunk_type != ZTXT_TYPE {
			return Err(error::DmiError::Generic(format!(
				"Failed to load RawZtxtChunk from reader. Chunk type is not zTXt: {:#?}. Should be {:#?}.",
				chunk_type, ZTXT_TYPE
			)));
		}
		let data_bytes = &raw_chunk_bytes[8..(total_bytes_length - 4)].to_vec();
		let data = RawZtxtData::load(&mut &**data_bytes)?;
		let crc = [
			raw_chunk_bytes[total_bytes_length - 4],
			raw_chunk_bytes[total_bytes_length - 3],
			raw_chunk_bytes[total_bytes_length - 2],
			raw_chunk_bytes[total_bytes_length - 1],
		];
		let calculated_crc = crc::calculate_crc(chunk_type.iter().chain(data_bytes.iter()));
		if u32::from_be_bytes(crc) != calculated_crc {
			return Err(error::DmiError::Generic(format!("Failed to load RawZtxtChunk from reader. Given CRC ({}) does not match the calculated one ({}).", u32::from_be_bytes(crc), calculated_crc)));
		}
		Ok(RawZtxtChunk {
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
				"Failed to save zTXt chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.chunk_type)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.chunk_type.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save zTXt chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = self.data.save(&mut *writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.data_length) as usize {
			return Err(error::DmiError::Generic(format!(
				"Failed to save zTXt chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.crc)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.crc.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save zTXt chunk. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		Ok(total_bytes_written)
	}

	pub fn set_data(&self, data: RawZtxtData) -> Result<RawZtxtChunk, error::DmiError> {
		let mut data_bytes = vec![];
		data.save(&mut data_bytes)?;
		let data_length = (data_bytes.len() as u32).to_be_bytes();
		let chunk_type = ZTXT_TYPE;
		let crc = crc::calculate_crc(chunk_type.iter().chain(data_bytes.iter())).to_be_bytes();
		Ok(RawZtxtChunk {
			data_length,
			chunk_type,
			data,
			crc,
		})
	}
}

impl Default for RawZtxtChunk {
	fn default() -> Self {
		let data: RawZtxtData = Default::default();
		let data_length = (data.length() as u32).to_be_bytes();
		let chunk_type = ZTXT_TYPE;
		let crc = data.crc().to_be_bytes();
		RawZtxtChunk {
			data_length,
			chunk_type,
			data,
			crc,
		}
	}
}

impl TryFrom<chunk::RawGenericChunk> for RawZtxtChunk {
	type Error = error::DmiError;
	fn try_from(raw_generic_chunk: chunk::RawGenericChunk) -> Result<Self, Self::Error> {
		let data_length = raw_generic_chunk.data_length;
		let chunk_type = raw_generic_chunk.chunk_type;
		if chunk_type != ZTXT_TYPE {
			return Err(error::DmiError::Generic(format!(
				"Failed to convert RawGenericChunk into RawZtxtChunk. Wrong type: {:#?}. Expected: {:#?}.",
				chunk_type, ZTXT_TYPE
			)));
		};
		let chunk_data = &raw_generic_chunk.data;
		let data = RawZtxtData::load(&mut &**chunk_data)?;
		let crc = raw_generic_chunk.crc;
		Ok(RawZtxtChunk {
			data_length,
			chunk_type,
			data,
			crc,
		})
	}
}

/*
impl TryFrom<Vec<u8>> for RawZtxtChunk {
	type Error = anyhow::Error;
	fn try_from(raw_chunk_bytes: Vec<u8>) -> Result<Self, Self::Error> {
		let total_bytes_length = raw_chunk_bytes.len();
		if total_bytes_length < 12 {
			bail!("Failed to convert Vec<u8> into RawZtxtChunk. Size: {}. Minimum necessary is 12.", raw_chunk_bytes.len())
		}
		let length = [raw_chunk_bytes[0], raw_chunk_bytes[1], raw_chunk_bytes[2], raw_chunk_bytes[3]];
		if u32::from_be_bytes(length) != total_bytes_length as u32 - 12 {
			bail!("Failed to convert Vec<u8> into RawZtxtChunk. Lengh field value ({}) does not match the actual data field size ({}).", u32::from_be_bytes(length), total_bytes_length -12)
		}
		let chunk_type = [raw_chunk_bytes[4], raw_chunk_bytes[5], raw_chunk_bytes[6], raw_chunk_bytes[7]];
		if chunk_type != ZTXT_TYPE {
			bail!("Failed to convert Vec<u8> into RawZtxtChunk. Chunk type is not zTXt: {:#?}. Should be {:#?}.", chunk_type, ZTXT_TYPE)
		}
		let data_bytes = &raw_chunk_bytes[8..(total_bytes_length - 4)];
		let data = RawZtxtData::load(data_bytes)?;
		let crc = [raw_chunk_bytes[total_bytes_length - 4], raw_chunk_bytes[total_bytes_length - 3], raw_chunk_bytes[total_bytes_length - 2], raw_chunk_bytes[total_bytes_length - 1]];
		let calculated_crc = crc::calculate_crc(chunk_type.iter().chain(data_bytes.iter()));
		if u32::from_be_bytes(crc) != calculated_crc {
			bail!("Failed to convert Vec<u8> into RawZtxtChunk. Given CRC ({}) does not match the calculated one ({}).", u32::from_be_bytes(crc), calculated_crc)
		}
		Ok(RawZtxtChunk {
			length,
			chunk_type,
			data,
			crc,
		})
	}
}
*/

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RawZtxtData {
	pub keyword: Vec<u8>,
	pub null_separator: u8,
	pub compression_method: u8,
	pub compressed_text: Vec<u8>,
}

impl RawZtxtData {
	pub fn load<R: Read>(reader: &mut R) -> Result<RawZtxtData, error::DmiError> {
		let mut data_bytes = Vec::new();
		reader.read_to_end(&mut data_bytes)?;
		let mut data_bytes_iter = data_bytes.iter().cloned();
		let keyword = data_bytes_iter.by_ref().take_while(|x| *x != 0).collect();
		let null_separator = 0;
		let compression_method = data_bytes_iter.next().ok_or_else(|| {
			error::DmiError::Generic(format!(
				"Failed to load RawZtxtData from reader, during compression method reading.\nVector: {:#?}",
				data_bytes
			))
		})?;
		//let compressed_text = RawCompressedText::try_from(back_to_vector)?;
		let compressed_text = data_bytes_iter.collect();
		//let compressed_text = RawCompressedText::load(&back_to_vector[..])?;

		Ok(RawZtxtData {
			keyword,
			null_separator,
			compression_method,
			compressed_text,
		})
	}

	pub fn save<W: Write>(&self, writter: &mut W) -> Result<usize, error::DmiError> {
		let bytes_written = writter.write(&self.keyword)?;
		let mut total_bytes_written = bytes_written;
		if bytes_written < self.keyword.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save zTXt data. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&[self.null_separator])?;
		total_bytes_written += bytes_written;
		if bytes_written < 1 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save zTXt data. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&[self.compression_method])?;
		total_bytes_written += bytes_written;
		if bytes_written < 1 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save zTXt data. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = writter.write(&self.compressed_text)?;
		total_bytes_written += bytes_written;
		if bytes_written < self.compressed_text.len() {
			return Err(error::DmiError::Generic(format!(
				"Failed to save zTXt data. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		Ok(total_bytes_written)
	}

	pub fn decode(&self) -> Result<Vec<u8>, error::DmiError> {
		match inflate::inflate_bytes_zlib(&self.compressed_text) {
			Ok(decompressed_text) => Ok(decompressed_text),
			Err(text) => {
				return Err(error::DmiError::Generic(format!(
					"Failed to read compressed text. Error: {}",
					text
				)))
			}
		}
	}

	fn length(&self) -> usize {
		self.keyword.len() + 2 + self.compressed_text.len()
	}

	fn crc(&self) -> u32 {
		crc::calculate_crc(
			ZTXT_TYPE
				.iter()
				.chain(self.keyword.iter())
				.chain([self.null_separator, self.compression_method].iter())
				.chain(self.compressed_text.iter()),
		)
	}
}

pub fn encode(text_to_compress: &[u8]) -> Vec<u8> {
	deflate::deflate_bytes_zlib(text_to_compress)
}

impl Default for RawZtxtData {
	fn default() -> Self {
		RawZtxtData {
			keyword: "Description".as_bytes().to_vec(),
			null_separator: 0,
			compression_method: 0,
			compressed_text: vec![],
		}
	}
}

impl fmt::Display for RawZtxtData {
	fn fmt(&self, feedback: &mut fmt::Formatter) -> fmt::Result {
		write!(feedback, "RawZtxtData chunk error.\nkeyword: {:#?}\nnull_separator: {:#?}\ncompression_method: {:#?}\ncompressed_text: {:#?}", self.keyword, self.null_separator, self.compression_method, self.compressed_text)
	}
}
