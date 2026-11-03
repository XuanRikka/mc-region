use std::fs::File;
use std::hint::black_box;
use std::io::stdin;
use std::time::Instant;

use memmap2::Mmap;
use mc_region::formats::anvil::LocationEntry;
use mc_region::formats::AnvilReader;
use mc_region::structs::Chunk;
use mc_region::traits::RegionRead;

fn wait() {
    let mut s = String::new();
    stdin().read_line(&mut s).unwrap();
}

fn main() {
    let path = "./r.0.0.mca";
    let file = File::open(path).unwrap();
    let m = unsafe { Mmap::map(&file) }.unwrap();
    println!("mmap len = {} bytes", m.len());

    // 每个 reader 的堆占用估算（'static 只是随便给个具体生命周期，size 与生命周期无关）
    println!(
        "per reader ≈ struct {} B + location_table {} B + timestamp_table {} B + chunks {} B",
        std::mem::size_of::<AnvilReader<'static>>(),
        std::mem::size_of::<LocationEntry>() * 1024,
        std::mem::size_of::<u32>() * 1024,
        std::mem::size_of::<Option<Chunk>>() * 1024,
    );

    wait(); // ← 挂起点 1：记录「创建前」RSS

    const N: usize = 1000;
    let t0 = Instant::now();
    let readers: Vec<AnvilReader<'_>> = (0..N)
        .map(|i| {
            // 坐标只在结构体里存着，不影响解析；用不同值避免看起来全一样
            let x = (i % 32) as i32;
            let z = ((i / 32) % 32) as i32;
            AnvilReader::from_bytes(m.as_ref(), x, z)
                .unwrap_or_else(|e| panic!("reader {i} at ({x},{z}) failed: {e:?}"))
        })
        .collect();
    println!("created {} readers in {:?}", readers.len(), t0.elapsed());

    wait(); // ← 挂起点 2：记录「100 个 reader 同时存活」的 RSS

    // 让编译器认为 readers 被用到了，避免整段被优化掉
    black_box(&readers);

    wait(); // ← 挂起点 3：观察 drop 后 RSS 回落
}