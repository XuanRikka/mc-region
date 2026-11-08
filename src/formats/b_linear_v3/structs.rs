use zerocopy::{BigEndian, FromBytes, Immutable, IntoBytes, KnownLayout, U64, U32};

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(crate) struct Header
{
    magic: U64<BigEndian>,
    version: u8,
    compression_level: i8,
    xxhash32_seed: U32<BigEndian>
}

impl Header
{
    pub(crate) fn new(compression_level: i8, xxhash32_seed: u32) -> Self
    {
        Header {
            magic: U64::from_bytes([0xFF, 0xFF, 0xDF, 0xF7, 0xED, 0xDA, 0xFD, 0x97]),
            version: 3,
            compression_level,
            xxhash32_seed: U32::from(xxhash32_seed)
        }
    }
}

#[derive(FromBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(crate) struct PositionTable(pub [U64<BigEndian>; 16]);

/// 空 bucket 的写入策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyBucketPolicy {
    /// 完全还原官方行为
    /// 但是代价是写一个 bucket 时要初始化这个 bucket 的全部 chunk
    /// 对于一个非常大的 chunk，这可能会导致潜在的大内存占用，但是可以绝对保证兼容
    /// 可以通过 auto_set_zstd_params 选项来根据源大小自动设置 Zstd 各项参数
    /// 在大压缩等级下往往这才是真正的内存杀手
    Skip,

    /// 参考官方实现细节，我们完全可以写一个全是空区块的bucket
    /// 这样可以减少内存占用，因为不再需要提前初始化需要的 chunk
    /// 自然也不需要一次性解压 64 个 chunk 带来的内存占用
    /// 按理说直接使用了 luminol 的服务端和对齐了官方行为的解码器都可以正常解码
    /// 但是天知道会不会有不兼容的情况呢？比如某个服务端/转换器自己实现了解码器
    /// 如果你需要尽可能少的内存占用并且不怕兼容性出问题就使用这个吧！
    /// 不过当你的 Zstd 压缩等级很高的时候这个路径将无法自动根据大小设置 Zstd 窗口等参数的大小
    /// 所以会出现内存占用反而更大的情况，这种情况下请自己更改 Zstd 参数
    Write,
}

pub struct BufferedLinearV3Config {
    pub(crate) compression_level: i8,
    pub(crate) xxhash32_seed: u32,
    pub(crate) empty_bucket_policy: EmptyBucketPolicy,
    pub(crate) zstd_window_log: Option<u32>,
    pub(crate) zstd_hash_log: Option<u32>,
    pub(crate) zstd_chain_log: Option<u32>,
    pub(crate) auto_set_zstd_params: bool
}

impl Default for BufferedLinearV3Config {
    fn default() -> Self {
        Self {
            compression_level: 6,
            xxhash32_seed: 0x0721,
            empty_bucket_policy: EmptyBucketPolicy::Skip,
            zstd_hash_log: None,
            zstd_window_log: None,
            zstd_chain_log: None,
            auto_set_zstd_params: false
        }
    }
}

impl BufferedLinearV3Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置压缩等级，不调用则用默认值 6
    pub fn compression_level(mut self, level: i8) -> Self {
        self.compression_level = level;
        self
    }

    /// 设置 xxhash32 seed，不调用则用默认值 0x0721
    pub fn xxhash32_seed(mut self, seed: u32) -> Self {
        self.xxhash32_seed = seed;
        self
    }

    pub fn empty_bucket_policy(mut self, policy: EmptyBucketPolicy) -> Self {
        self.empty_bucket_policy = policy;
        self
    }

    /// 完全还原官方行为
    /// 但是代价是写一个 bucket 时要初始化这个 bucket 的全部 chunk
    /// 对于一个非常大的 chunk，这可能会导致潜在的大内存占用，但是可以绝对保证兼容
    /// 可以通过 auto_set_zstd_params 选项来根据源大小自动设置 Zstd 各项参数
    /// 在大压缩等级下往往这才是真正的内存杀手
    pub fn skip_empty_buckets(self) -> Self {
        self.empty_bucket_policy(EmptyBucketPolicy::Skip)
    }

    /// 参考官方实现细节，我们完全可以写一个全是空区块的bucket
    /// 这样可以减少内存占用，因为不再需要提前初始化需要的 chunk
    /// 自然也不需要一次性解压 64 个 chunk 带来的内存占用
    /// 按理说直接使用了 luminol 的服务端和对齐了官方行为的解码器都可以正常解码
    /// 但是天知道会不会有不兼容的情况呢？比如某个服务端/转换器自己实现了解码器
    /// 如果你需要尽可能少的内存占用并且不怕兼容性出问题就使用这个吧！
    /// 不过当你的 Zstd 压缩等级很高的时候这个路径将无法自动根据大小设置 Zstd 窗口等参数的大小
    /// 所以会出现内存占用反而更大的情况，这种情况下请自己更改 Zstd 参数
    pub fn write_empty_buckets(self) -> Self {
        self.empty_bucket_policy(EmptyBucketPolicy::Write)
    }

    pub fn zstd_window_log(mut self, log: u32) -> Self {
        self.zstd_window_log = Some(log);
        self
    }

    /// 设置 Zstd hash log，默认值 0
    pub fn zstd_hash_log(mut self, log: u32) -> Self {
        self.zstd_hash_log = Some(log);
        self
    }

    /// 设置 Zstd chain log，默认值 0
    pub fn zstd_chain_log(mut self, log: u32) -> Self {
        self.zstd_chain_log = Some(log);
        self
    }

    /// 是否根据源大小自动设置 Zstd 参数，默认值 false
    /// 仅在 EmptyBucketPolicy 为 Skip 时有效
    pub fn auto_set_zstd_params(mut self, enabled: bool) -> Self {
        self.auto_set_zstd_params = enabled;
        self
    }
}
