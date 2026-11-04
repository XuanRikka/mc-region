use std::path::Path;

use crate::error::{RegionError, Result};
use crate::structs::XzPos;

pub fn get_xz_for_path<P: AsRef<Path>>(path: P) -> Result<XzPos>
{
    let path = path.as_ref();
    if path.is_empty()
    {
        return Err(RegionError::PathMissing { path: path.to_path_buf() });
    }
    if !path.is_file()
    {
        return Err(RegionError::NotAFile { path: path.to_path_buf() });
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| RegionError::InvalidFileName { path: path.to_path_buf() })?;

    let mut it = name.rsplit('.');
    let z = it
        .next()
        .ok_or_else(|| RegionError::MissingCoordinate { axis: "z", name: name.to_owned() })?;
    let x = it
        .next()
        .ok_or_else(|| RegionError::MissingCoordinate { axis: "x", name: name.to_owned() })?;

    let z: i32 = z
        .parse()
        .map_err(|_| RegionError::InvalidCoordinate { axis: "z", value: z.to_owned() })?;
    let x: i32 = x
        .parse()
        .map_err(|_| RegionError::InvalidCoordinate { axis: "x", value: x.to_owned() })?;

    Ok(XzPos::new(x, z))
}

/// 把全局 chunk 坐标换算成 region 内 slot
pub fn slot_of(pos: XzPos, x: i32, z: i32) -> Option<u16>
{
    let dx = x - pos.x * 32;
    let dz = z - pos.z * 32;
    if (0..32).contains(&dx) && (0..32).contains(&dz) {
        Some((dx + dz * 32) as u16)
    } else {
        None
    }
}

/// 把 region 内 slot 换算成全局 chunk 坐标
pub fn xz_of(pos: XzPos, slot: u16) -> XzPos
{
    let dx = (slot % 32) as i32;
    let dz = (slot / 32) as i32;
    XzPos::new(pos.x * 32 + dx, pos.z * 32 + dz)
}
