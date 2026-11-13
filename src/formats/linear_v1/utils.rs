use std::io::{BufReader, Read};
use zstd::Decoder as ZstdDecoder;
use crate::linear_v1::structs::Header;
use crate::structs::{Chunk, Slot, XzPos};
use crate::utils::skip;
use crate::xz_of;

pub(crate) fn fill_slot<'a>(slot: u16,
                 decoder: &mut Option<ZstdDecoder<'static, BufReader<&'a [u8]>>>,
                 compressed_data: &'a [u8], cursor: &mut u16, header: &Header,
                 chunks: &mut Box<[Slot; 1024]>, pos: XzPos
) -> crate::Result<()>
{
    if decoder.is_none()
    {
        let mut cur = ZstdDecoder::new(compressed_data)?;
        skip(&mut cur, 8192)?;
        *decoder = Some(cur);
        *cursor = 0;
    }
    let mut decoder = decoder.as_mut().unwrap();

    let mut c_;
    if *cursor >= slot
    {
        c_ = ZstdDecoder::new(compressed_data)?;
        decoder = &mut c_;
    }

    for i in 0..=slot as usize
    {
        let entry = &header.0[i];

        if entry.chunk_size == 0
        {
            continue;
        }

        if chunks[slot as usize].is_loaded()
        {
            skip(decoder, entry.chunk_size.get() as u64)?;
            continue;
        }

        let mut buf = vec![0u8; entry.chunk_size.get() as usize];
        decoder.read_exact(&mut buf)?;
        chunks[i] = Slot::Loaded(Chunk::new(
            xz_of(pos, slot),
            entry.chunk_timestamp.get() as u64,
            buf
        ));
        *cursor += 1;
    }

    Ok(())
}