use std::fmt::Write;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::Result;
use crate::helper::{get_xz_for_path, slot_of};
use crate::structs::{Chunk, XzPos};

pub trait RegionRead<B: AsRef<[u8]>> {
    fn from_bytes(bytes: B, x: i32, z: i32) -> Result<Self>
    where
        Self: Sized;

    fn from_reader<R: Read>(reader: &mut R, x: i32, z: i32) -> Result<Self>
    where
        Self: Sized,
        B: From<Vec<u8>>,
    {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Self::from_bytes(B::from(bytes), x, z)
    }

    fn from_path<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        Self: Sized,
        B: From<Vec<u8>>,
    {
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        let pos = get_xz_for_path(&path)?;
        Self::from_reader(&mut reader, pos.x, pos.z)
    }
}

/// 对区域（region）的读写访问抽象
pub trait RegionAccess
{
    /// 获取 region 的XZ坐标
    fn get_pos(&self) -> XzPos;

    /// 通过一个Chunk在全局的坐标来获取Chunk，如果在Region内不存在这个坐标则返回None
    fn chunk_at(&mut self, x: i32, z: i32) -> Result<Option<&Chunk>>
    {
        match slot_of(self.get_pos(), x, z) {
            Some(slot) => self.chunk_at_slot(slot),
            None => Ok(None),
        }
    }

    /// 语义同 `chunk_at`，返回可变引用
    fn chunk_at_mut(&mut self, x: i32, z: i32) -> Result<Option<&mut Chunk>>
    {
        match slot_of(self.get_pos(), x, z) {
            Some(slot) => self.chunk_at_slot_mut(slot),
            None => Ok(None),
        }
    }

    /// slot即行优先的Region的槽位
    fn chunk_at_slot(&mut self, slot: u16) -> Result<Option<&Chunk>>;

    /// 语义同 `chunk_at_slot`，返回可变引用
    fn chunk_at_slot_mut(&mut self, slot: u16) -> Result<Option<&mut Chunk>>;

    /// 按照行优先顺序遍历区块
    fn chunks(&mut self) -> impl Iterator<Item = Result<Option<&Chunk>>>;

    /// 按照行优先顺序遍历区块，返回可变引用
    fn chunks_mut(&mut self) -> impl Iterator<Item = Result<Option<&mut Chunk>>> + '_;

    /// 按照对应格式最优的方法遍历区块
    fn chunks_native(&mut self) -> impl Iterator<Item = Result<Option<&Chunk>>>
    {
        self.chunks()
    }

    /// 按照对应格式最优的方法遍历区块，返回可变引用
    fn chunks_native_mut(&mut self) -> impl Iterator<Item = Result<Option<&mut Chunk>>>
    {
        self.chunks_mut()
    }

    /// 按照行优先顺序遍历区块并move数据
    fn take_chunks(&mut self) -> impl Iterator<Item = Result<Option<Chunk>>>;

    /// 按照对应格式最优的方法遍历区块并move数据
    fn take_chunks_native(&mut self) -> impl Iterator<Item = Result<Option<Chunk>>>
    {
        self.take_chunks()
    }

    /// 按全局坐标写入 chunk。`Some` 放入或覆盖该位置，`None` 删除该位置。
    /// 越界坐标静默忽略（由 `slot_of` 返回 None 保证）。
    fn set_chunk(&mut self, chunk: Option<Chunk>, x: i32, z: i32)
    {
        if let Some(slot) = slot_of(self.get_pos(), x, z) {
            self.set_chunk_slot(chunk, slot);
        }
    }

    /// 按 slot 写入 chunk。`Some` 放入或覆盖该槽位，`None` 清空该槽位
    fn set_chunk_slot(&mut self, chunk: Option<Chunk>, slot: u16) -> Result<()>;

    /// 判断全局坐标 `(x, y)` 处是否存在 chunk
    fn contains_chunk(&mut self, x: i32, z: i32) -> Result<bool>;

    /// 判断 slot 处是否存在 chunk
    fn contains_chunk_slot(&mut self, slot: u16) -> Result<bool>;

    /// 返回 region 中现存 chunk 的总数
    fn chunk_count(&self) -> usize;

    /// 当 region 中没有任何 chunk 时返回 `true`，等价于 `chunk_count() == 0`
    fn is_empty(&self) -> bool
    {
        self.chunk_count() == 0
    }
}

pub trait RegionWrite {
    fn to_writer<R: RegionAccess, W: Write>(&self, region: &R, writer: &mut W);
    fn to_path<R: RegionAccess, P: AsRef<Path>>(&self, region: &R, path: P);
    fn to_bytes<R: RegionAccess>(&self, region: &R);
}