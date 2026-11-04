use std::path::PathBuf;


#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RegionError {
    /// 路径为空，或指向的文件不存在。
    #[error("路径为空或不存在: {path}")]
    PathMissing { path: PathBuf },

    /// 路径存在但不是普通文件。
    #[error("路径不是文件: {path}")]
    NotAFile { path: PathBuf },

    /// 无法从文件名解析出 region 坐标。
    #[error("无法从文件名解析 region 坐标: {path}")]
    InvalidFileName { path: PathBuf },

    /// 文件名中缺少某个坐标分量。
    #[error("{axis} 坐标缺失: {name}")]
    MissingCoordinate {
        axis: &'static str,
        name: String,
    },

    /// 坐标分量不是合法整数。
    #[error("{axis} 坐标不是合法整数: {value}")]
    InvalidCoordinate {
        axis: &'static str,
        value: String,
    },

    /// 文件长度不足以容纳位置表 / 时间戳表。
    #[error("{table} 数据不足: 需要至少 {need} 字节, 实际 {got} 字节")]
    TableTooShort {
        table: &'static str,
        need: usize,
        got: usize,
    },

    /// 区块数据的扇区偏移落在文件之外，或声明的扇区区间超出文件末尾。
    #[error("chunk 扇区越界: offset = {offset}, capacity = {capacity}, 文件长度 = {len}")]
    SectorOutOfBounds { offset: usize, capacity: usize, len: usize },

    /// 区块的扇区偏移指进了 8 KiB 文件头（位置表 / 时间戳表）。
    #[error("chunk 扇区偏移 {offset} 落在 {header} 字节的文件头内")]
    SectorInHeader { offset: usize, header: usize },

    /// 区块的 length 字段非法：为 0（缺少 comp_type 字节），或 payload 装不进本扇区。
    #[error("chunk slot {slot} 的 length 字段非法: length = {length}, 本扇区可用 = {capacity} 字节")]
    ChunkLength { slot: u16, length: u32, capacity: usize },

    /// slot 超出 `0..=1023`。
    #[error("chunk slot 越界: slot = {slot}, 有效范围 = 0..=1023")]
    SlotOutOfRange { slot: u16 },

    /// 全局坐标不属于该 region。
    #[error("坐标 ({x}, {z}) 越出 region 边界: region 原点 = ({origin_x}, {origin_z})")]
    OutOfRegion {
        x: i32,
        z: i32,
        origin_x: i32,
        origin_z: i32,
    },

    /// 底层 I/O 错误（读取文件、读取字段等）。
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("区块 ({x}, {z}) 过大，无mcc文件的mca文件单区块最大大小为 1044480，实为 {size}")]
    ChunkTooLarge {
        x: i32,
        z: i32,
        size: usize,
    }
}

pub type Result<T, E = RegionError> = std::result::Result<T, E>;

