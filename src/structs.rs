pub struct Chunk
{
    x: i32,
    z: i32,
    raw_nbt: Vec<u8>
}

impl Chunk {
    pub fn new(x: i32, z: i32, raw_nbt: Vec<u8>) -> Self
    {
        Chunk { x, z, raw_nbt }
    }
    
    pub fn pos(&self) -> XzPos
    {
        XzPos::new(self.x, self.z)
    }
}

#[derive(Copy, Clone)]
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