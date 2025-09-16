#![no_main]
use libfuzzer_sys::fuzz_target;
use cdrle::decompress;

fuzz_target!(|comp: &[u8]| {
    // Property: decompressor must never panic on arbitrary input.
    // It may return Ok(_) or a defined Error, but must not crash or loop.
    let _ = decompress(comp);
});
