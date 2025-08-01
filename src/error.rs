use png::DecodingError;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DmiError {
	#[error("IO error: {0}")]
	Io(#[from] io::Error),
	#[error("PNG decoding error: {0}")]
	PngDecoding(#[from] DecodingError),
	#[error("Image-processing error: {0}")]
	Image(#[from] image::error::ImageError),
	#[error("FromUtf8 error: {0}")]
	FromUtf8(#[from] std::string::FromUtf8Error),
	#[error("ParseInt error: {0}")]
	ParseInt(#[from] std::num::ParseIntError),
	#[error("ParseFloat error: {0}")]
	ParseFloat(#[from] std::num::ParseFloatError),
	#[error("Invalid chunk type (byte outside the range `A-Za-z`): {chunk_type:?}")]
	InvalidChunkType { chunk_type: [u8; 4] },
	#[error("CRC mismatch (stated {stated:?}, calculated {calculated:?})")]
	CrcMismatch { stated: u32, calculated: u32 },
	#[error("Dmi error: {0}")]
	Generic(String),
	#[error("Dmi block entry error: {0}")]
	BlockEntry(String),
	#[error("Dmi IconState error: {0}")]
	IconState(String),
	#[error("Encoding error: {0}")]
	Encoding(String),
	#[error("Conversion error: {0}")]
	Conversion(String),
}
