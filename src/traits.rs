use std::io::{Seek, Write};
use std::path::Path;

use crate::error::Result;
use crate::helper::slot_of;
use crate::structs::{Chunk, XzPos};

pub trait RegionRead<'a> {
    fn from_bytes(bytes: &'a [u8], x: i32, z: i32) -> Result<Self>
    where
        Self: Sized;
}

/// 对区域（region）的读写访问抽象
pub trait RegionAccess
{
    /// 获取 region 的XZ坐标
    fn get_pos(&self) -> XzPos;

    /// slot即行优先的Region的槽位
    fn chunk_at_slot(&mut self, slot: u16) -> Result<&Chunk>;

    fn take_chunk_at_slot(&mut self, slot: u16) -> Result<Chunk>;

    /// 语义同 `chunk_at_slot`，返回可变引用
    fn chunk_at_slot_mut(&mut self, slot: u16) -> Result<&mut Chunk>;

    /// 通过一个Chunk在全局的坐标来获取Chunk，如果在Region内不存在这个坐标则返回None
    fn chunk_at(&mut self, x: i32, z: i32) -> Result<Option<&Chunk>>
    {
        match slot_of(self.get_pos(), x, z) {
            Some(slot) => Ok(Some(self.chunk_at_slot(slot)?)),
            None => Ok(None),
        }
    }

    /// 语义同 `chunk_at`，返回可变引用
    fn chunk_at_mut(&mut self, x: i32, z: i32) -> Result<Option<&mut Chunk>>
    {
        match slot_of(self.get_pos(), x, z) {
            Some(slot) => Ok(Some(self.chunk_at_slot_mut(slot)?)),
            None => Ok(None),
        }
    }

    fn take_chunk_at(&mut self, x: i32, z: i32) -> Result<Option<Chunk>>
    {
        match slot_of(self.get_pos(), x, z) {
            Some(slot) => Ok(Some(self.take_chunk_at_slot(slot)?)),
            None => Ok(None),
        }
    }

    /// 按照行优先顺序遍历区块
    fn chunks(&mut self) -> impl Iterator<Item = Result<&Chunk>>;

    /// 按照行优先顺序遍历区块，返回可变引用
    fn chunks_mut(&mut self) -> impl Iterator<Item = Result<&mut Chunk>>;

    /// 按照对应格式最优的方法遍历区块
    fn chunks_native(&mut self) -> impl Iterator<Item = Result<&Chunk>>
    {
        self.chunks()
    }

    /// 按照对应格式最优的方法遍历区块，返回可变引用
    fn chunks_native_mut(&mut self) -> impl Iterator<Item = Result<&mut Chunk>>
    {
        self.chunks_mut()
    }

    /// 按照行优先顺序遍历区块并move数据
    fn take_chunks(&mut self) -> impl Iterator<Item = Result<Chunk>>;

    /// 按照对应格式最优的方法遍历区块并move数据
    fn take_chunks_native(&mut self) -> impl Iterator<Item = Result<Chunk>>
    {
        self.take_chunks()
    }

    /// 判断全局坐标 `(x, y)` 处是否存在 chunk
    fn contains_chunk(&mut self, x: i32, z: i32) -> Result<bool>;

    /// 判断 slot 处是否存在 chunk
    fn contains_chunk_slot(&mut self, slot: u16) -> Result<bool>;

    /// 返回 region 中现存 chunk 的总数
    fn chunk_count(&mut self) -> Result<usize>;

    /// 当 region 中没有任何 chunk 时返回 `true`，等价于 `chunk_count() == 0`
    fn is_empty(&mut self) -> Result<bool>
    {
        Ok(self.chunk_count()? == 0)
    }
}

pub trait RegionWriter {
    fn to_writer<R: RegionAccess, W: Write + Seek>(&self, region: &mut R, writer: &mut W) -> Result<()>;
    fn to_path<R: RegionAccess, P: AsRef<Path>>(&self, region: &mut R, path: P) -> Result<()>;
    fn to_bytes<R: RegionAccess>(&self, region: &mut R) -> Result<Vec<u8>>;
}