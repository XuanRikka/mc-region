use crate::RegionError;

pub fn zstd_err(code: zstd::zstd_safe::ErrorCode) -> RegionError {
    std::io::Error::other(format!("zstd error code {code}")).into()
}