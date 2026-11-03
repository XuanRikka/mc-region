use std::fs::File;
use std::io::stdin;
use mc_region::traits::{RegionAccess, RegionRead};
use std::path::Path;
use mc_region::formats::AnvilReader;
use anyhow::Result;
use memmap2::Mmap;

fn main()
{
    let path = "./r.0.0.mca";
    let file = File::open(path).unwrap();
    let m = unsafe { Mmap::map(&file) }.unwrap();
    stdin().read_line(&mut String::new()).unwrap();
    let mut reader = AnvilReader::from_bytes(m.as_ref(), 0, 0).unwrap();
    stdin().read_line(&mut String::new()).unwrap();
    println!("aaa");

    for i in reader.take_chunks()
    {
        println!("aaa");
        let a = i.unwrap();
        if a.is_none()
        {
            println!("跳过")
        }
        else
        {
            let a = a.unwrap();
            println!("{} {}", a.pos().x, a.pos().z)
        }
    }
    stdin().read_line(&mut String::new()).unwrap();
}