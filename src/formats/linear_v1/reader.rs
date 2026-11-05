use std::array;
use std::io::Read;
use zerocopy::FromBytes;
use zstd::Decoder as ZstdDecoder;

use crate::{xz_of, RegionError, Result};
use crate::linear_v1::structs::{Header, SuperBlock};
use crate::structs::{Chunk, Slot, XzPos};
use crate::traits::RegionRead;
struct LinearV1Reader<'a>
{
    pos: XzPos,
    data: &'a [u8],
    compressed_data: &'a [u8],
    superblock: &'a SuperBlock,
    header: Box<Header>,
    chunks: Box<[Slot; 1024]>,
}

impl<'a> RegionRead<'a> for LinearV1Reader<'a> {
    fn from_bytes(bytes: &'a [u8], x: i32, z: i32) -> Result<Self> {
        let region_pos = XzPos::new(x, z);

        let superblock_bytes = bytes.get(..20).ok_or(RegionError::DataTooShort {
            name: "Superblock",
            need: 20,
            got: bytes.len()
        })?;
        let superblock: &SuperBlock = FromBytes::ref_from_bytes(superblock_bytes).map_err(|_| RegionError::DataTooShort {
            name: "区域文件头",
            need: 20,
            got: superblock_bytes.len(),
        })?;

        let compressed_length = (superblock.get_compressed_data_length() + 1) as usize;
        let compressed_data = bytes.get(20..compressed_length+1).ok_or(RegionError::DataTooShort {
            name: "压缩数据",
            need: 20,
            got: bytes.len()
        })?;

        let mut decoder = ZstdDecoder::new(compressed_data)?;
        let mut buf = Box::new([0u8; 8192]);
        decoder.read_exact(buf.as_mut())?;

        let header = unsafe {
            Box::from_raw(Box::into_raw(buf).cast::<Header>())
        };

        let chunks: Box<[Slot; 1024]> = Box::new(array::from_fn(|i| {
            let xz = xz_of(region_pos, i as u16);
            let info = &header.as_ref().0[i];
            if info.chunk_size == 0
            {
                Slot::Loaded(Chunk::new(xz, info.chunk_timestamp.get() as u64, Vec::new()))
            }
            else
            {
                Slot::Pending
            }
        }));

        Ok(Self {
            pos: region_pos,
            data: bytes,
            compressed_data,
            superblock,
            header,
            chunks,
        })
    }
}

