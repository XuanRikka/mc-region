mod structs;
mod utils;
mod reader;
mod writer;

/// 位置表长度
const LOCATION_TABLE_LENGTH: usize = 1024 * 4;
/// 时间戳表长度
const TIMESTAMP_TABLE_LENGTH: usize = 1024 * 4;
/// 头部长度
const HEADER_LENGTH: usize = LOCATION_TABLE_LENGTH + TIMESTAMP_TABLE_LENGTH;

pub use reader::AnvilReader;
pub use writer::AnvilWriter;
pub use structs::{AnvilCompressionType, AnvilConfig};
