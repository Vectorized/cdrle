#![no_main]
use libfuzzer_sys::fuzz_target;
use cdrle::{compress, decompress};

fuzz_target!(|data: &[u8]| {
    // Property: decompress(compress(x)) == x
    let c = compress(data);
    match decompress(&c) {
        Ok(d) => assert_eq!(d.as_slice(), data),
        Err(e) => panic!("decompress failed on compressor output: {:?}", e),
    }
});
