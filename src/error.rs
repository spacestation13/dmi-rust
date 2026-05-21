use png::DecodingError;
use png::EncodingError;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DmiError {
	#[error("IO error: {0}")]
	Io(#[from] io::Error),
	#[error("PNG decoding error: {0}")]
	PngDecoding(#[from] DecodingError),
	#[error("PNG encoding error: {0}")]
	PngEncoding(#[from] EncodingError),
	#[error("ParseInt error: {0}")]
	ParseInt(#[from] std::num::ParseIntError),
	#[error("ParseFloat error: {0}")]
	ParseFloat(#[from] std::num::ParseFloatError),
	#[error("Dmi error: {0}")]
	Generic(String),
	#[error("Dmi block entry error: {0}")]
	BlockEntry(String),
	#[error("Dmi IconState error: {0}")]
	IconState(String),
}
