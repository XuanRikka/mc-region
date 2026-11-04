pub struct Chunk
{
    pos: XzPos,
    timestamp: u64,
    raw_nbt: Vec<u8>
}

impl Chunk {
    pub fn new(pos: XzPos, timestamp: u64, raw_nbt: Vec<u8>) -> Self
    {
        Chunk { pos, timestamp, raw_nbt }
    }

    pub fn pos(&self) -> XzPos
    {
        self.pos
    }

    pub fn timestamp(&self) -> u64
    {
        self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: u64)
    {
        self.timestamp = timestamp;
    }
    
    pub fn raw_nbt(&self) -> &[u8]
    {
        &self.raw_nbt
    }

    pub fn set_raw_nbt(&mut self, raw_nbt: Vec<u8>)
    {
        self.raw_nbt = raw_nbt;
    }
    
    pub fn is_empty(&self) -> bool
    {
        self.raw_nbt.is_empty()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct XzPos
{
    pub x: i32,
    pub z: i32
}

impl XzPos {
    pub fn new(x: i32, z: i32) -> Self
    {
        XzPos {
            x,z
        }
    }
}

pub(crate) enum Slot {
    /// 还没解码
    Pending,
    /// 已解码
    Loaded(Chunk),
}

impl Slot {
    pub fn is_pending(&self) -> bool {
        matches!(self, Slot::Pending)
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, Slot::Loaded(_))
    }

    pub fn get_chunk(&self) -> &Chunk {
        match self {
            Slot::Loaded(chunk) => chunk,
            _ => unreachable!("get_chunk called on non-Loaded slot"),
        }
    }

    pub fn get_chunk_mut(&mut self) -> &mut Chunk
    {
        match self {
            Slot::Loaded(chunk) => chunk,
            _ => unreachable!("get_chunk_mut called on non-Loaded slot"),
        }
    }
    
    pub fn take_chunk(self) -> Chunk
    {
        match self {
            Slot::Loaded(chunk) => chunk,
            _ => unreachable!("take_chunk called on non-Loaded slot"),
        }
    }
}
