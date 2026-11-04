use std::io::{Cursor, Read, Write};

use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder};
use lz4_java_wrc::{Lz4BlockInput, Lz4BlockOutput};

use crate::formats::anvil::HEADER_LENGTH;
use crate::formats::anvil::structs::{LocationEntry, Timestamp};
use crate::helper::xz_of;
use crate::{RegionError, Result};
use crate::structs::{Chunk, Slot, XzPos};

/// 一个扇区的大小
const SECTOR_BYTES: usize = 4096;

#[inline]
pub(super) fn read_u24_be(buf: &[u8; 3]) -> u32 {
    ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32)
}

#[inline]
pub(super) fn write_u24_be(value: u32) -> [u8; 3] {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8]
}

pub(super) fn load_slot(cell: &mut Slot, data: &[u8], location: &LocationEntry,
                        timestamp: &Timestamp, pos: XzPos, slot: u16, ) -> Result<()> {
    if location.is_empty() || cell.is_loaded() { return Ok(()); }
    *cell = Slot::Loaded(decode_chunk(data, location, timestamp, pos, slot)?);
    Ok(())
}

pub(super) fn decode_chunk(data: &[u8], location: &LocationEntry, timestamp: &Timestamp,
                           pos: XzPos, slot: u16) -> Result<Chunk> {
    let start = location.offset();
    let capacity = location.sector_count() as usize * SECTOR_BYTES;

    if start < HEADER_LENGTH {
        return Err(RegionError::SectorInHeader { offset: start, header: HEADER_LENGTH });
    }

    // 检查数据范围是否在文件内
    let end = start + capacity;
    let raw = data
        .get(start..end)
        .ok_or(RegionError::SectorOutOfBounds { offset: start, capacity, len: data.len() })?;

    let len_bytes: &[u8; 4] = raw[0..4].try_into().unwrap();
    let length = u32::from_be_bytes(*len_bytes);
    let comp_type = raw[4];

    let payload_len = (length as usize)
        .checked_sub(1) // 因为要装comp_type，所以至少得是1
        .filter(|n| *n <= raw.len() - 5)
        .ok_or(RegionError::ChunkLength { slot, length, capacity: raw.len() })?;

    let buf = decode_data(&raw[5..5 + payload_len], comp_type)?;

    let chunk_pos = xz_of(pos, slot);
    Ok(Chunk::new(chunk_pos, timestamp.seconds() as u64, buf))
}

pub(super) fn decode_data(payload: &[u8], comp_type: u8) -> Result<Vec<u8>>
{
    let mut buf = Vec::new();
    match comp_type
    {
        1 => {
            let mut decoder = GzDecoder::new(payload);
            decoder.read_to_end(&mut buf)?;
        }
        2 => {
            let mut decoder = ZlibDecoder::new(payload);
            decoder.read_to_end(&mut buf)?;
        }
        3 => {
            buf = payload.to_vec()
        }
        4 => {
            let mut decoder = Lz4BlockInput::new(payload);
            decoder.read_to_end(&mut buf)?;
        }
        _ => {
            todo!()
        }
    }
    Ok(buf)
}

pub(super) fn encode_data<W: Write>(payload: &[u8], buf: &mut W, comp_type: u8,
                                    compression_level: u32) -> Result<u64>
{
    Ok(match comp_type
    {
        1 => {
            let mut encoder = GzEncoder::new(payload,
                                             Compression::new(compression_level));
            std::io::copy(&mut encoder, buf)?
        }
        2 => {
            let mut encoder = ZlibEncoder::new(payload,
                                               Compression::new(compression_level));
            std::io::copy(&mut encoder, buf)?
        }
        3 => {
            std::io::copy(&mut Cursor::new(payload), buf)?
        }
        4 => {
            let mut out = Vec::new();
            {
                let mut encoder = Lz4BlockOutput::new(&mut out);
                encoder.write_all(payload)?;
                encoder.flush()?;
            }
            buf.write_all(&out)?;
            out.len() as u64
        }
        _ => {
            todo!()
        }
    })
}

