#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
extern crate alloc;

use alloc::vec::Vec;

pub const MAX_ZERO_RUN: usize = 128; // 0x00 runs: 1..=128
pub const MAX_FF_RUN: usize = 32;    // 0xFF runs: 1..=32

/// Canonical decoding errors (exhaustive by construction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The compressed stream ended right after a run marker (0x00),
    /// i.e. there was no CONTROL byte following it.
    RunMarkerWithoutControl,
    /// The CONTROL byte denotes an FF-run length > 32.
    /// (Decoded as len = (control & 0x7F) + 1; this error can ONLY occur for FF runs.)
    InvalidRunLength { len: usize },
}

/// Compresses `input` and XOR-negates the first 4 bytes of the *compressed* stream.
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len()); // lower bound; worst case ~2×
    let mut zero = 0usize;
    let mut ff = 0usize;

    #[inline]
    fn emit_run(out: &mut Vec<u8>, is_ff: bool, n: usize) {
        debug_assert!(n >= 1);
        debug_assert!((!is_ff && n <= MAX_ZERO_RUN) || (is_ff && n <= MAX_FF_RUN));
        let mut ctrl = ((n as u8) - 1) & 0x7f;
        if is_ff { ctrl |= 0x80; }
        out.push(0x00);
        out.push(ctrl);
    }

    #[inline]
    fn flush(out: &mut Vec<u8>, zero: &mut usize, ff: &mut usize) {
        if *ff != 0 { emit_run(out, true, *ff); *ff = 0; }
        if *zero != 0 { emit_run(out, false, *zero); *zero = 0; }
    }

    for &b in input {
        match b {
            0x00 => {
                if ff != 0 { emit_run(&mut out, true, ff); ff = 0; }
                zero += 1;
                if zero == MAX_ZERO_RUN { emit_run(&mut out, false, MAX_ZERO_RUN); zero = 0; }
            }
            0xFF => {
                if zero != 0 { emit_run(&mut out, false, zero); zero = 0; }
                ff += 1;
                if ff == MAX_FF_RUN { emit_run(&mut out, true, MAX_FF_RUN); ff = 0; }
            }
            _ => { flush(&mut out, &mut zero, &mut ff); out.push(b); }
        }
    }
    flush(&mut out, &mut zero, &mut ff);

    // Negate first 4 bytes of *compressed* stream.
    let lim = core::cmp::min(4, out.len());
    for i in 0..lim { out[i] ^= 0xFF; }
    out
}

/// Decompresses `comp` produced by `compress`.
/// Errors:
/// - RunMarkerWithoutControl  (0x00 as final byte)
/// - InvalidRunLength{len}    (FF-run with len > 32)
pub fn decompress(comp: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(comp.len()); // conservative lower bound
    let mut i = 0usize;

    #[inline]
    fn read_unneg(comp: &[u8], i: &mut usize) -> u8 {
        let mut b = comp[*i];
        if *i < 4 { b ^= 0xFF; }
        *i += 1;
        b
    }

    while i < comp.len() {
        let b = read_unneg(comp, &mut i);
        if b != 0x00 {
            out.push(b);
            continue;
        }
        if i >= comp.len() {
            return Err(Error::RunMarkerWithoutControl);
        }
        let c = read_unneg(comp, &mut i);
        let is_ff = (c & 0x80) != 0;
        let len = (c & 0x7F) as usize + 1;
        if is_ff && len > MAX_FF_RUN {
            return Err(Error::InvalidRunLength { len });
        }
        let fill = if is_ff { 0xFF } else { 0x00 };
        let base = out.len();
        out.resize(base + len, fill);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn rt(v: &[u8]) {
        let c = compress(v);
        let d = decompress(&c).unwrap();
        assert_eq!(v, d.as_slice(), "in:{:x?} cmp:{:x?} dec:{:x?}", v, c, d);
    }

    #[test] fn empty() { rt(&[]); }
    #[test] fn literals() { rt(&[1,2,3,4,5]); }
    #[test] fn zeros() {
        rt(&vec![0x00; 1]);
        rt(&vec![0x00; 127]);
        rt(&vec![0x00; 128]);
        rt(&vec![0x00; 129]); // 128 + 1
    }
    #[test] fn ffs() {
        rt(&vec![0xFF; 1]);
        rt(&vec![0xFF; 31]);
        rt(&vec![0xFF; 32]);
        rt(&vec![0xFF; 33]); // 32 + 1
    }
    #[test] fn mixed() {
        rt(&[0,0,0,0, 42, 0xFF,0xFF,0xFF, 1,2,3, 0, 0xFF, 0, 0xAA,0xBB, 0, 0xFF]);
    }
    #[test] fn err_run_marker_without_control() {
        let mut c = vec![0x00];
        for i in 0..c.len().min(4) { c[i] ^= 0xFF; }
        assert_eq!(decompress(&c), Err(Error::RunMarkerWithoutControl));
    }
    #[test] fn err_invalid_ff_run_len() {
        let mut c = vec![0x00, 0xA0]; // ff=1, len-1=32 => len=33
        for i in 0..c.len().min(4) { c[i] ^= 0xFF; }
        assert_eq!(decompress(&c), Err(Error::InvalidRunLength { len: 33 }));
    }
}
