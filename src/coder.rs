// most significant bit kept free to avoid overflows
pub const MAX_VALUE: u32 = 0x80000000u32;
pub const THREE_QUARTER_VALUE: u32 = 0x60000000u32;
pub const HALF_VALUE: u32 = 0x40000000u32;
pub const QUARTER_VALUE: u32 = 0x20000000u32;
pub const SYMBOL_MAX_FREQ: u32 = 1 << 16;
pub const EOF_SYMBOL: u32 = 256;
pub const INCREMENT_STEP: u32 = 10;

#[test]
fn test_coder_vec() {
    use encoder;
    use decoder;

    let mut buf: Vec<u8> = Vec::new();
    let mut buf2: Vec<u8> = Vec::new();
    let mut encoded: Vec<u8> = Vec::new();

    for i in 0..1024 {
        buf.push((i % 255) as u8);
    }

    encoded.resize(10000, 0);
    buf2.resize(10000, 0);

    encoder::encode(&mut buf.as_slice(), &mut encoded.as_mut_slice()).unwrap();
    decoder::decode(&mut encoded.as_slice(), &mut buf2.as_mut_slice()).unwrap();

    buf2.resize(buf.len(), 0);

    assert!(buf == buf2);
}

#[test]
fn test_coder_file() {
    use encoder;
    use decoder;

    use std::fs::File;
    use std::io::Read;
    let mut buf: Vec<u8> = Vec::new();
    let mut buf2: Vec<u8> = Vec::new();

    for i in 0..10000 {
        buf.push((i % 100) as u8);
    }

    encoder::encode(&mut buf.as_slice(),
                    &mut File::create("tmp.cargotest").expect("Unable to open file"))
        .unwrap();
    decoder::decode(&mut File::open("tmp.cargotest").expect("Unable to open file"),
                    &mut File::create("tmp.cargotest2").expect("Unable to open file"))
        .unwrap();

    buf2.clear();
    File::open("tmp.cargotest2").expect("Unable to open file").read_to_end(&mut buf2).unwrap();
    println!("size1: {}, size2: {}", buf.len(), buf2.len());
    assert!(buf == buf2);
}
