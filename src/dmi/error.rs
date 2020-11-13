use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DmiError {
	#[error("IO error")]
	Io(#[from] io::Error),
	#[error("Invalid chunk type (byte outside the range `A-Za-z`): {chunk_type:?}")]
	InvalidChunkType { chunk_type: [u8; 4] },
	#[error("CRC mismatch (stated {stated:?}, calculated {calculated:?})")]
	CrcMismatch { stated: u32, calculated: u32 },
	#[error("Image-processing error")]
	Image(#[from] image::error::ImageError),
	#[error("Dmi error")]
	Generic(String),
	#[error("Encoding error")]
	Encoding(String),
	#[error("Conversion error")]
	Conversion(String),
	#[error("FromUtf8 error")]
	FromUtf8(#[from] std::string::FromUtf8Error),
	#[error("ParseInt error")]
	ParseInt(#[from] std::num::ParseIntError),
	#[error("ParseFloat error")]
	ParseFloat(#[from] std::num::ParseFloatError),
}
