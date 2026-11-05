use std::array;

use zerocopy::{FromBytes};

use crate::formats::anvil::HEADER_LENGTH;
use crate::formats::anvil::structs::{LocationEntry, AnvilHeader, Timestamp};
use crate::formats::anvil::utils::{decode_chunk, load_slot};
use crate::helper::{slot_of, xz_of};
use crate::{RegionError, Result};
use crate::structs::{Chunk, Slot, XzPos};
use crate::traits::{RegionAccess, RegionRead};

pub struct AnvilReader<'a>
{
    pos: XzPos,
    location_table: &'a [LocationEntry; 1024],
    timestamp_table: &'a [Timestamp; 1024],
    data: &'a [u8],
    chunks: Box<[Slot; 1024]>,
}

impl<'a> RegionRead<'a> for AnvilReader<'a> {
    fn from_bytes(bytes: &'a [u8], x: i32, z: i32) -> Result<Self> {
        let header_bytes = bytes.get(..HEADER_LENGTH).ok_or(RegionError::DataTooShort {
            name: "区域文件头",
            need: HEADER_LENGTH,
            got: bytes.len(),
        })?;
        let header: &AnvilHeader =
            FromBytes::ref_from_bytes(header_bytes).map_err(|_| RegionError::DataTooShort {
                name: "区域文件头",
                need: HEADER_LENGTH,
                got: header_bytes.len(),
            })?;

        let region_pos = XzPos::new(x, z);

        let chunks: Box<[Slot; 1024]> = Box::new(array::from_fn(|i| {
            let entry = &header.locations[i];
            let xz = xz_of(region_pos, i as u16);
            let timestamp = header.timestamps[i].0.get();
            if entry.is_empty() {
                Slot::Loaded(Chunk::new(xz, timestamp as u64, Vec::new()))
            } else {
                Slot::Pending
            }
        }));

        Ok(Self {
            pos: region_pos,
            location_table: &header.locations,
            timestamp_table: &header.timestamps,
            data: bytes,
            chunks,
        })
    }
}

impl RegionAccess for AnvilReader<'_> {
    fn get_pos(&self) -> XzPos {
        self.pos
    }

    fn chunk_at_slot(&mut self, slot: u16) -> Result<&Chunk> {
        if slot > 1023 {
            return Err(RegionError::SlotOutOfRange { slot });
        }

        if self.chunks[slot as usize].is_pending() {
            self.init_chunk_for_slot(slot)?;
        }

        Ok(self.chunks[slot as usize].get_chunk())
    }

    fn take_chunk_at_slot(&mut self, slot: u16) -> Result<Chunk> {
        if slot > 1023 
        {
            return Err(RegionError::SlotOutOfRange { slot });
        }
        let i = slot as usize;
        
        if self.chunks[i].is_loaded() 
        {
            let chunk = std::mem::replace(&mut self.chunks[i], Slot::Pending);
            return Ok(chunk.take_chunk())
        }
        
        let Self { pos, location_table, timestamp_table, data, .. } = self;
        let (pos, data, location_table, timestamp_table) =
            (*pos, &*data, &*location_table, &*timestamp_table);
        
        let entry = &location_table[i];
        if entry.is_empty() {
            return Ok(Chunk::new(xz_of(pos, slot), 0, Vec::new()));
        }
        decode_chunk(data, entry, &timestamp_table[i], pos, slot)
    }

    fn chunk_at_slot_mut(&mut self, slot: u16) -> Result<&mut Chunk> {
        if slot > 1023 {
            return Err(RegionError::SlotOutOfRange { slot });
        }

        if self.chunks[slot as usize].is_pending() {
            self.init_chunk_for_slot(slot)?;
        }

        Ok(self.chunks[slot as usize].get_chunk_mut())
    }

    fn chunks(&mut self) -> impl Iterator<Item = Result<&Chunk>> {
        let Self { pos, location_table, timestamp_table, data, chunks, .. } = self;
        let (pos, data, location_table, timestamp_table) =
            (*pos, &*data, &*location_table, &*timestamp_table);
        chunks.iter_mut().enumerate().map(move |(i, cell)| {
            load_slot(cell, data, &location_table[i], &timestamp_table[i], pos, i as u16)?;
            Ok(cell.get_chunk())
        })
    }

    fn chunks_mut(&mut self) -> impl Iterator<Item = Result<&mut Chunk>> {
        let Self { pos, location_table, timestamp_table, data, chunks, .. } = self;
        let (pos, data, location_table, timestamp_table) =
            (*pos, &*data, &*location_table, &*timestamp_table);
        chunks.iter_mut().enumerate().map(move |(i, cell)| {
            load_slot(cell, data, &location_table[i], &timestamp_table[i], pos, i as u16)?;
            Ok(cell.get_chunk_mut())
        })
    }

    fn take_chunks(&mut self) -> impl Iterator<Item = Result<Chunk>> {
        let Self { pos, location_table, .. } = self;
        let (pos, location_table) =
            (*pos, &*location_table);
        location_table.iter().enumerate().map(move |(i, entry)| {
            if entry.is_empty()
            {
                return Ok(Chunk::new(xz_of(pos, i as u16), 0, Vec::new()));
            }
            let chunk = self.take_chunk_at_slot(i as u16)?;
            Ok(chunk)
        })
    }

    fn contains_chunk(&mut self, x: i32, z: i32) -> Result<bool> {
        let pos = self.pos;
        let slot = slot_of(pos, x, z);
        let Some(slot) = slot else {
            return Err(RegionError::OutOfRegion {
                x,
                z,
                origin_x: self.pos.x,
                origin_z: self.pos.z,
            });
        };
        self.contains_chunk_slot(slot)
    }

    fn contains_chunk_slot(&mut self, slot: u16) -> Result<bool> {
        if slot > 1023 {
            return Err(RegionError::SlotOutOfRange { slot });
        };
        Ok(!self.location_table[slot as usize].is_empty())
    }

    fn chunk_count(&mut self) -> Result<usize> {
        let mut count = 0;
        for c in self.chunks() {
            if !c?.is_empty() {
                count += 1;
            }
        }
        Ok(count)
    }
}

impl AnvilReader<'_> {
    fn init_chunk_for_slot(&mut self, slot: u16) -> Result<()> {
        let Self { pos, location_table, timestamp_table,
            data, chunks, .. } = self;
        load_slot(&mut chunks[slot as usize], data, &location_table[slot as usize],
                  &timestamp_table[slot as usize], *pos, slot)
    }
}
