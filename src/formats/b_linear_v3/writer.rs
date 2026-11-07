use std::fs::File;
use std::io::{BufWriter, Cursor, Read, SeekFrom};
use std::io::{Seek, Write};
use std::path::Path;
use byteorder::{BigEndian, WriteBytesExt};
use xxhash_rust::xxh32::xxh32;
use zerocopy::{IntoBytes};
use zstd::stream::write::Encoder;

use crate::b_linear_v3::structs::{BufferedLinearV3Config, EmptyBucketPolicy, Header};
use crate::{RegionAccess, RegionWriter, Result};

pub struct BufferedLinearV3Writer
{
    config: BufferedLinearV3Config
}

impl RegionWriter for BufferedLinearV3Writer
{
    fn to_writer<R: RegionAccess, W: Write + Seek>(&self, region: &mut R, writer: &mut W) -> Result<()> {
        let header = Header::new(self.config.compression_level, self.config.xxhash32_seed);
        writer.write_all(header.as_bytes())?;

        let position_table_start = writer.stream_position()?;
        let mut src = std::io::repeat(0).take(128);
        std::io::copy(&mut src, writer)?;

        let position_table = match self.config.empty_bucket_policy
        {
            EmptyBucketPolicy::Write => {
                self.write_chunk1(region, writer)
            }
            EmptyBucketPolicy::Skip => {
                self.write_chunk2(region, writer)
            }
        }?;

        let end_pos = writer.stream_position()?;
        writer.seek(SeekFrom::Start(position_table_start))?;
        for i in position_table
        {
            writer.write_u64::<BigEndian>(i)?;
        }
        writer.seek(SeekFrom::Start(end_pos))?;

        Ok(())
    }

    fn to_path<R: RegionAccess, P: AsRef<Path>>(&self, region: &mut R, path: P) -> Result<()>
    {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);

        self.to_writer(region, &mut writer)?;

        writer.flush()?;

        Ok(())
    }

    fn to_bytes<R: RegionAccess>(&self, region: &mut R) -> Result<Vec<u8>>
    {
        let mut cursor = Cursor::new(Vec::new());

        self.to_writer(region, &mut cursor)?;

        Ok(cursor.into_inner())
    }
}

impl BufferedLinearV3Writer
{
    pub fn new(config: BufferedLinearV3Config) -> Self
    {
        BufferedLinearV3Writer {
            config
        }
    }

    // write empty bucket
    fn write_chunk1<R: RegionAccess, W: Write + Seek>(&self, region: &mut R, writer: &mut W)
        -> Result<[u64; 16]>
    {
        let mut position_table = [0u64; 16];

        for bucket_index in 0..16u16
        {
            let mut raw_len = 4 * 64;

            let bucket_pos = writer.stream_position()?;
            writer.write_u64::<BigEndian>(0)?; // 占位 raw_len + com_len
            position_table[bucket_index as usize] = bucket_pos;

            let mut encoder =
                Encoder::new(&mut *writer, self.config.compression_level as i32)?;
            for chunk_index in 0..64u16
            {
                let global_index = bucket_index * 64 + chunk_index;
                let chunk = region.take_chunk_at_slot(global_index)?;

                if chunk.is_empty()
                {
                    encoder.write_u32::<BigEndian>(0)?;
                    continue
                }
                let entry_size = (chunk.raw_nbt().len() + 4 + 8 + 4) as u32;
                encoder.write_u32::<BigEndian>(entry_size)?;
                raw_len += entry_size;

                let checksum = xxh32(chunk.raw_nbt(), self.config.xxhash32_seed);

                encoder.write_u32::<BigEndian>(chunk.raw_nbt().len() as u32)?;
                encoder.write_u64::<BigEndian>(chunk.timestamp())?;
                encoder.write_u32::<BigEndian>(checksum)?;
                encoder.write_all(chunk.raw_nbt())?;
            }
            encoder.finish()?;

            let end_pos = writer.stream_position()?;

            let cmp_size = (end_pos - bucket_pos - 8) as u32;
            writer.seek(SeekFrom::Start(bucket_pos))?;
            writer.write_u32::<BigEndian>(raw_len)?;
            writer.write_u32::<BigEndian>(cmp_size)?;

            writer.seek(SeekFrom::Start(end_pos))?;
        }

        Ok(position_table)
    }

    // skip empty bucket
    fn write_chunk2<R: RegionAccess, W: Write + Seek>(&self, region: &mut R, writer: &mut W)
                                                      -> Result<[u64; 16]>
    {
        let mut position_table = [0u64; 16];

        for bucket_index in 0..16u16
        {
            let mut chunks = Vec::with_capacity(64);
            for chunk_index in 0..64u16
            {
                let global_index = bucket_index * 64 + chunk_index;
                chunks.push(region.take_chunk_at_slot(global_index)?);
            }

            if chunks.iter().all(|x| x.is_empty())
            {
                continue
            }

            let bucket_pos = writer.stream_position()?;
            writer.write_u64::<BigEndian>(0)?; // 占位 raw_len + com_len
            position_table[bucket_index as usize] = bucket_pos;

            let mut raw_len = 4 * 64;
            {
                let mut encoder =
                    Encoder::new(&mut *writer, self.config.compression_level as i32)?;

                for chunk in &chunks
                {
                    if chunk.is_empty()
                    {
                        encoder.write_u32::<BigEndian>(0)?;
                        continue
                    }
                    let entry_size = (chunk.raw_nbt().len() + 4 + 8 + 4) as u32;
                    encoder.write_u32::<BigEndian>(entry_size)?;
                    raw_len += entry_size;

                    let checksum = xxh32(chunk.raw_nbt(), self.config.xxhash32_seed);

                    encoder.write_u32::<BigEndian>(chunk.raw_nbt().len() as u32)?;
                    encoder.write_u64::<BigEndian>(chunk.timestamp())?;
                    encoder.write_u32::<BigEndian>(checksum)?;
                    encoder.write_all(chunk.raw_nbt())?;
                }

                encoder.finish()?;
            }

            let end_pos = writer.stream_position()?;
            let cmp_size = (end_pos - bucket_pos - 8) as u32;

            writer.seek(SeekFrom::Start(bucket_pos))?;
            writer.write_u32::<BigEndian>(raw_len)?;
            writer.write_u32::<BigEndian>(cmp_size)?;


            writer.seek(SeekFrom::Start(end_pos))?;
        }

        Ok(position_table)
    }
}
