use std::fs;
use std::fs::File;
use std::io::BufWriter;
use mc_region::anvil::AnvilReader;
use mc_region::b_linear_v3::{BufferedLinearV3Config, BufferedLinearV3Writer};
use mc_region::{RegionRead, RegionWriter};

fn main()
{
    let mca_data = fs::read("r.0.0.mca").unwrap();
    let mut mca = AnvilReader::from_bytes(&mca_data, 0, 0).unwrap();

    let config = BufferedLinearV3Config::default()
        .compression_level(22)
        .skip_empty_buckets()
        .auto_set_zstd_params(true);
    let mut writer = BufWriter::new(File::create("r.0.0.0b_linear").unwrap());
    let bl_writer = BufferedLinearV3Writer::new(config);
    bl_writer.to_writer(&mut mca, &mut writer).unwrap();
}