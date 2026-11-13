use std::io;
use std::io::Read;

pub(crate) fn skip<R: Read>(r: &mut R, n: u64) -> io::Result<()> {
    let copied = io::copy(&mut r.take(n), &mut io::sink())?;
    if copied < n {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(())
}