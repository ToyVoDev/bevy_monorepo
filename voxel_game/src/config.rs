pub const CHUNK_SIZE: usize = 32;
pub const VOXEL_SIZE: f32 = 0.1;

// LocalVoxelPos uses u8 per axis — enforce that CHUNK_SIZE fits.
const _: () = assert!(CHUNK_SIZE <= u8::MAX as usize + 1);
