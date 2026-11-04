use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};
use zerocopy::big_endian::U32;

use crate::formats::anvil::HEADER_LENGTH;
use crate::formats::anvil::utils::{read_u24_be, write_u24_be};

#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct LocationEntry {
    offset: [u8; 3],
    count: u8,
}

impl LocationEntry {
    pub fn new_empty() -> LocationEntry
    {
        LocationEntry { offset: [0; 3], count: 0 }
    }

    pub fn new(offset: u32, count: u8) -> LocationEntry
    {
        LocationEntry
        {
            offset: write_u24_be(offset),
            count
        }
    }

    #[inline]
    pub(super) fn sector_offset(&self) -> u32
    {
        read_u24_be(&self.offset)
    }

    #[inline]
    pub(super) fn sector_count(&self) -> u8
    {
        self.count
    }

    #[inline]
    pub(super) fn is_empty(&self) -> bool
    {
        self.sector_offset() == 0 || self.count == 0
    }

    #[inline]
    pub(super) fn offset(&self) -> usize
    {
        self.sector_offset() as usize * 4096
    }
}

#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct Timestamp(pub U32);

impl Timestamp {
    #[inline]
    pub fn seconds(&self) -> u32 { self.0.get() }
}

#[derive(FromBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct AnvilHeader {
    pub(crate) locations: [LocationEntry; 1024],
    pub(crate) timestamps: [Timestamp; 1024],
}

const _: () = assert!(size_of::<AnvilHeader>() == HEADER_LENGTH);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnvilCompressionType {
    Gzip = 1,
    Zlib = 2,
    Uncompressed = 3,
    Lz4 = 4
}

impl AsRef<u8> for AnvilCompressionType {
    #[inline]
    fn as_ref(&self) -> &u8 {
        match self {
            AnvilCompressionType::Gzip => &1u8,
            AnvilCompressionType::Zlib => &2u8,
            AnvilCompressionType::Uncompressed => &3u8,
            AnvilCompressionType::Lz4 => &4u8
        }
    }
}

pub struct AnvilConfig
{
    pub compression_level: u32,
    pub compression_type: u8
}

impl AnvilConfig {
    pub fn new<T: AsRef<u8>>(compression_level: u32, compression_type: T) -> AnvilConfig
    {
        AnvilConfig
        {
            compression_level,
            compression_type: *compression_type.as_ref()
        }
    }
}
