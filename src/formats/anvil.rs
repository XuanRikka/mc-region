use std::io::{Cursor, Read};
use std::path::Path;
use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use crate::helper::{slot_of, xz_of};
use crate::structs::{Chunk, XzPos};
use crate::traits::{RegionAccess, RegionRead};

const LOCATION_TABLE_BYTES: usize = 1024 * 4;
const TIMESTAMP_TABLE_BYTES: usize = 1024 * 4;

pub struct AnvilReader<B: AsRef<[u8]>>
{
    x: i32,
    z: i32,
    location_table: Box<[LocationEntry; 1024]>,
    timestamp_table: Box<[u32; 1024]>,
    data: B,
    chunks: Box<[Option<Chunk>; 1024]>,
}

impl<B: AsRef<[u8]>> RegionRead<B> for AnvilReader<B> {
    fn from_bytes(bytes: B, x: i32, z: i32) -> Result<Self> {
        let location_table = read_location_table(bytes.as_ref())?;
        let timestamp_table = read_timestamp_table(bytes.as_ref())?;
        Ok(Self {
            x, z, location_table, timestamp_table,
            data: bytes,
            chunks: Box::new([const { None }; 1024]),
        })
    }
}

impl AnvilReader<Vec<u8>> {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        <Self as RegionRead<Vec<u8>>>::from_path(path)
    }
    pub fn from_reader<R: Read>(reader: &mut R, x: i32, z: i32) -> Result<Self> {
        <Self as RegionRead<Vec<u8>>>::from_reader(reader, x, z)
    }
}

impl<B: AsRef<[u8]>> RegionAccess for AnvilReader<B> {
    fn get_pos(&self) -> XzPos {
        XzPos::new(self.x, self.z)
    }

    fn chunk_at_slot(&mut self, slot: u16) -> Result<Option<&Chunk>> {
        if self.location_table[slot as usize].is_empty() {
            return Ok(None);
        }

        if self.chunks[slot as usize].is_none() {
            self.init_chunk_for_slot(slot)?;
        }

        Ok(self.chunks[slot as usize].as_ref())
    }

    fn chunk_at_slot_mut(&mut self, slot: u16) -> Result<Option<&mut Chunk>> {
        if self.location_table[slot as usize].is_empty() {
            return Ok(None);
        }

        if self.chunks[slot as usize].is_none() {
            self.init_chunk_for_slot(slot)?;
        }

        Ok(self.chunks[slot as usize].as_mut())
    }

    fn chunks(&mut self) -> impl Iterator<Item = Result<Option<&Chunk>>> {
        let Self { x, z, location_table, data, chunks, .. } = self;
        let (pos, data, location_table) = (XzPos::new(*x, *z), &*data, &*location_table);
        chunks.iter_mut().enumerate().map(move |(i, cell)| {
            load_slot(cell, data, location_table[i], pos, i as u16)?;
            Ok(cell.as_ref())
        })
    }

    fn chunks_mut(&mut self) -> impl Iterator<Item = Result<Option<&mut Chunk>>> {
        let Self { x, z, location_table, data, chunks, .. } = self;
        let (pos, data, location_table) = (XzPos::new(*x, *z), &*data, &*location_table);
        chunks.iter_mut().enumerate().map(move |(i, cell)| {
            load_slot(cell, data, location_table[i], pos, i as u16)?;
            Ok(cell.as_mut())
        })
    }

    fn take_chunks(&mut self) -> impl Iterator<Item = Result<Option<Chunk>>> {
        let Self { x, z, location_table, data, .. } = self;
        let (pos, data, location_table) = (XzPos::new(*x, *z), &*data, &*location_table);
        location_table.iter().enumerate().map(move |(i, entry)| {
            if entry.is_empty()
            {
                return Ok(None)
            }
            let chunk = decode_chunk(data, *entry, pos, i as u16)?;
            Ok(Some(chunk))
        })
    }

    fn set_chunk_slot(&mut self, chunk: Option<Chunk>, slot: u16) -> Result<()> {
        if slot > 1023 {
            return Err(anyhow!(
                "chunk slot index out of range: slot = {slot}, valid range = 0..=1023"
            ));
        }
        self.chunks[slot as usize] = chunk;
        Ok(())
    }

    fn contains_chunk(&mut self, x: i32, z: i32) -> Result<bool> {
        let pos = XzPos::new(self.x, self.z);
        let slot = slot_of(pos, x, z);
        let Some(slot) = slot else {
            return Err(anyhow!(
            "xz coordinate ({x}, {z}) is out of region bounds: region origin = ({}, {})",
            self.x, self.z
        ));
        };
        self.contains_chunk_slot(slot)
    }

    fn contains_chunk_slot(&mut self, slot: u16) -> Result<bool> {
        if slot > 1023 {
            return Err(anyhow!(
                "chunk slot index out of range: slot = {slot}, valid range = 0..=1023"
            ));
        };
        Ok(!self.location_table[slot as usize].is_empty())
    }

    fn chunk_count(&self) -> usize {
        let mut count = 0usize;
        for i in self.location_table.as_ref()
        {
            if i.sector_offset != 0 || i.sector_count != 0
            {
                count += 1
            }
        };
        count
    }
}

impl<B: AsRef<[u8]>> AnvilReader<B> {
    fn init_chunk_for_slot(&mut self, slot: u16) -> Result<()> {
        let Self { x, z, location_table,
            data, chunks, .. } = self;
        load_slot(&mut chunks[slot as usize], data, location_table[slot as usize],
                  XzPos::new(*x, *z), slot)
    }
}

#[inline]
pub fn read_u24_be(buf: &[u8; 3]) -> u32 {
    ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocationEntry {
    pub sector_offset: u32,
    pub sector_count: u8,
}

impl LocationEntry {
    #[inline]
    fn from_4bytes(b: &[u8; 4]) -> Self {
        Self {
            sector_offset: read_u24_be(&[b[0], b[1], b[2]]),
            sector_count: b[3],
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sector_offset == 0 || self.sector_count == 0
    }

    #[inline]
    pub fn offset(&self) -> usize {
        const SECTOR_SIZE: usize = 4096;
        let start = self.sector_offset as usize * SECTOR_SIZE;
        start
    }
}

fn read_location_table(buf: &[u8]) -> Result<Box<[LocationEntry; 1024]>> {
    const NEED: usize = 1024 * 4;
    if buf.len() < NEED {
        return Err(anyhow!(
            "region file too short for location table: expected at least {} bytes, got {}",
            NEED,
            buf.len()
        ));
    }

    let table: Box<[LocationEntry; 1024]> = Box::new(std::array::from_fn(|i| {
        let b: &[u8; 4] = buf[i * 4..i * 4 + 4].try_into().unwrap();
        LocationEntry::from_4bytes(b)
    }));

    Ok(table)
}

fn read_timestamp_table(buf: &[u8]) -> Result<Box<[u32; 1024]>> {
    const NEED: usize = LOCATION_TABLE_BYTES + TIMESTAMP_TABLE_BYTES;

    if buf.len() < NEED {
        return Err(anyhow!(
            "region file too short for timestamp table: expected at least {} bytes, got {}",
            NEED,
            buf.len()
        ));
    }

    let table: Box<[u32; 1024]> = Box::new(std::array::from_fn(|i| {
        let start = LOCATION_TABLE_BYTES + i * 4;
        let b: &[u8; 4] = buf[start..start + 4].try_into().unwrap();
        u32::from_be_bytes(*b)
    }));

    Ok(table)
}

fn decode_chunk<B: AsRef<[u8]>>(data: &B, location: LocationEntry, pos: XzPos, slot: u16)
    -> Result<Chunk> {
    let start = location.offset();
    let view = &data.as_ref()[start..];
    let mut cur = Cursor::new(view);
    let length = cur.read_u32::<BigEndian>()?;
    let comp_type = cur.read_u8()?;
    let mut buf = Vec::new();
    let mut decoder = ZlibDecoder::new(cur.take((length - 1) as u64));
    decoder.read_to_end(&mut buf)?;
    let chunk_pos = xz_of(pos, slot);
    Ok(Chunk::new(chunk_pos.x, chunk_pos.z, buf))
}

fn load_slot<B: AsRef<[u8]>>(cell: &mut Option<Chunk>, data: &B, location: LocationEntry,
                             pos: XzPos, slot: u16, ) -> Result<()> {
    if location.is_empty() || cell.is_some() { return Ok(()); }
    *cell = Some(decode_chunk(data, location, pos, slot)?);
    Ok(())
}
