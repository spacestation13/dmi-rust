pub(crate) fn calculate_chunk_data_crc(chunk_type: [u8; 4], data: &[u8]) -> u32 {
	let mut hasher = crc32fast::Hasher::new();
	hasher.update(&chunk_type);
	hasher.update(data);
	hasher.finalize()
}
