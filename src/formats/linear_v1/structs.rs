use zerocopy::{BigEndian, FromBytes, Immutable, IntoBytes, KnownLayout, I16, U64, U32};

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct SuperBlock
{
    signature: [u8; 4],
    version: u8,
    newest_timestamp: U64<BigEndian>,
    compression_level: i8,
    chunk_count: I16<BigEndian>,
    compressed_data_length: U32<BigEndian>,
    reserved: U64<BigEndian>,
}

impl SuperBlock
{
    pub fn get_compressed_data_length(&self) -> u32
    {
        self.compressed_data_length.get()
    }
}

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct Header(pub [HeaderEntry; 1024]);

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct HeaderEntry
{
    pub chunk_size: U32<BigEndian>,
    pub chunk_timestamp: U32<BigEndian>,
}
