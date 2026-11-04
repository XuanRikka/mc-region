use std::array;
use std::fs::File;
use std::io::{BufWriter, Cursor, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{BigEndian, WriteBytesExt};
use zerocopy::big_endian::U32;
use zerocopy::IntoBytes;

use crate::error::Result;
use crate::formats::anvil::structs::{AnvilConfig, LocationEntry, Timestamp};
use crate::formats::anvil::utils::encode_data;
use crate::helper::slot_of;
use crate::RegionError;
use crate::traits::{RegionAccess, RegionWriter};

pub struct AnvilWriter
{
    config: AnvilConfig
}

impl AnvilWriter {
    pub fn new(config: AnvilConfig) -> AnvilWriter {
        AnvilWriter { config }
    }
}

impl RegionWriter for AnvilWriter {
    fn to_writer<R: RegionAccess, W: Write + Seek>(&self, region: &mut R, writer: &mut W) -> Result<()> {
        let start_pos = writer.stream_position()?;
        let region_pos = region.get_pos();

        let mut location_table: Box<[LocationEntry; 1024]> =
            Box::new(array::from_fn(|_| LocationEntry::new_empty()));

        let mut timestamps = Box::new([Timestamp(U32::new(0)); 1024]);

        let mut chunks = region.chunks();
        for slot in timestamps.iter_mut() {
            let chunk = chunks.next();
            if let Some(chunk) = chunk {
                *slot = Timestamp(U32::new(chunk?.timestamp().min(u32::MAX as u64) as u32));
            }
        }
        drop(chunks);

        for location in location_table.iter() {
            writer.write_all(location.as_bytes())?;
        }
        for timestamp in timestamps.iter()
        {
            writer.write_all(timestamp.0.as_bytes())?;
        }

        let comp_type = self.config.compression_type;
        let comp_level = self.config.compression_level;

        for chunk in region.take_chunks()
        {
            let chunk = chunk?;

            let chunk_start_count = writer.stream_position()?;

            writer.write_u32::<BigEndian>(0)?;
            writer.write_u8(comp_type)?;
            let length = encode_data(chunk.raw_nbt(), writer, comp_type, comp_level)?;

            let pos = writer.stream_position()?;
            let target_pos = pos.div_ceil(4096) * 4096;
            if target_pos > pos {
                writer.seek(SeekFrom::Start(target_pos - 1))?;
                writer.write_all(&[0u8])?;
            }

            let sector_count = (target_pos - chunk_start_count) / 4096;
            if sector_count >= 256 {
                return Err(RegionError::ChunkTooLarge {
                    x: chunk.pos().x,
                    z: chunk.pos().z,
                    size: length as usize
                });   // 需要 .mcc
            }

            writer.seek(SeekFrom::Start(chunk_start_count))?;
            writer.write_u32::<BigEndian>((length + 1) as u32)?;
            writer.seek(SeekFrom::Start(target_pos))?;

            let chunk_pos = chunk.pos();
            let index = slot_of(region_pos, chunk_pos.x, chunk_pos.z).unwrap(); // 这里我是想破脑袋也想不出什么情况下会出错
            location_table[index as usize] = LocationEntry::new((chunk_start_count / 4096) as u32, sector_count as u8);
        }

        writer.seek(SeekFrom::Start(start_pos))?;
        writer.write_all(location_table.as_bytes())?;

        Ok(())
    }

    fn to_path<R: RegionAccess, P: AsRef<Path>>(&self, region: &mut R, path: P) -> Result<()> {
        let mut file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(&mut file);
        self.to_writer(region, &mut writer)?;
        Ok(())
    }

    fn to_bytes<R: RegionAccess>(&self, region: &mut R) -> Result<Vec<u8>> {
        let mut buf = Cursor::new(Vec::new());

        self.to_writer(region, &mut buf)?;

        Ok(buf.into_inner())
    }
}
