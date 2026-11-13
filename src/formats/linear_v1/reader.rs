use std::{array, mem};
use std::io::{BufReader, Read};
use zerocopy::FromBytes;
use zstd::Decoder as ZstdDecoder;

use crate::{xz_of, RegionError, Result, RegionAccess};
use crate::linear_v1::structs::{Header, SuperBlock};
use crate::linear_v1::utils::fill_slot;
use crate::structs::{Chunk, Slot, XzPos};
use crate::traits::RegionRead;
use crate::utils::skip;

struct LinearV1Reader<'a>
{
    pos: XzPos,
    data: &'a [u8],
    compressed_data: &'a [u8],
    superblock: &'a SuperBlock,
    header: Box<Header>,
    chunks: Box<[Slot; 1024]>,

    decoder: Option<ZstdDecoder<'static, BufReader<&'a [u8]>>>,
    cursor: u16
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
            decoder: Some(decoder),
            cursor: 0
        })
    }
}

impl RegionAccess for LinearV1Reader<'_>
{
    fn get_pos(&self) -> XzPos {
        self.pos
    }

    fn chunk_at_slot(&mut self, slot: u16) -> Result<&Chunk> {
        if self.chunks[slot as usize].is_loaded()
        {
            return Ok(self.chunks[slot as usize].get_chunk())
        }

        self.init_chunk_for_slot(slot)?;

        Ok(self.chunks[slot as usize].get_chunk())
    }

    fn take_chunk_at_slot(&mut self, slot: u16) -> Result<Chunk> {
        if self.chunks[slot as usize].is_pending()
        {
            self.init_chunk_for_slot(slot)?;
        }

        let chunk = mem::replace(&mut self.chunks[slot as usize], Slot::Pending).take_chunk();

        Ok(chunk)
    }

    fn chunk_at_slot_mut(&mut self, slot: u16) -> Result<&mut Chunk> {
        if self.chunks[slot as usize].is_loaded()
        {
            return Ok(self.chunks[slot as usize].get_chunk_mut())
        }

        self.init_chunk_for_slot(slot)?;

        Ok(self.chunks[slot as usize].get_chunk_mut())
    }

    fn chunks(&mut self) -> impl Iterator<Item=Result<&Chunk>> {
        let Self {decoder,
            compressed_data, cursor, header,
            chunks, ..} = self;
        (0..1024).map(move |i| {
            fill_slot(i as u16, decoder, compressed_data, cursor, header, chunks, self.pos)?;
            Ok(chunks[i].get_chunk())
        })
    }

    fn chunks_mut(&mut self) -> impl Iterator<Item=Result<&mut Chunk>> {
        todo!()
    }

    fn take_chunks(&mut self) -> impl Iterator<Item=Result<Chunk>> {
        todo!()
    }

    fn contains_chunk(&mut self, x: i32, z: i32) -> Result<bool> {
        todo!()
    }

    fn contains_chunk_slot(&mut self, slot: u16) -> Result<bool> {
        todo!()
    }

    fn chunk_count(&mut self) -> Result<usize> {
        todo!()
    }
}

impl LinearV1Reader<'_> {
    fn init_chunk_for_slot(&mut self, slot: u16) -> Result<()>
    {
        let Self {decoder,
            compressed_data, cursor, header,
            chunks, ..} = self;
        fill_slot(slot, decoder, compressed_data, cursor, header, chunks, self.pos)
    }
}
