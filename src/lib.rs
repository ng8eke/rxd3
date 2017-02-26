//! # xdelta3
//!
//! This crate is a Rust binding of `xdelta3` which is written in C.
//!
//! In case you are not familar, `xdelta3` implements VCDIFF standard, which is a standard for
//! binary patches.
//! You can generate and apply VCDIFF patch for two similar (but large) binary files quickly using `xdelta3`.
//!
//! The original `xdelta3` utility is widely used for delivering software updates and ROM hacks.
//!
//! You can find out how to use this crate in this documentation and you can also consult the tests in the `test/` folder
//! to see it in action (how to generate and patch two files!)

extern crate libc;

use libc::c_uint;

#[allow(dead_code)]
mod binding {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

/// Function to generate the difference data
///
/// This function is used to generate the difference data.
/// The data in `src` will be treated as "original" data and the
/// data in `input` will be treated as "after", "patched" or "expected" data.
///
/// If you want to build an application that applies patches or differential updates,
/// this function is used to generate the patch data (or update files).
/// When generating the patch file, you might want to read your old file into a `&[u8]` and
/// pass that variable to the `src` parameter and read your new file into another `&[u8]` and
/// pass that variable to the `input` parameter. And then you could write the output of this function
/// to a file.
///
/// Here is a basic example to show how to use this function:
/// ```
/// extern crate xdelta3;
/// use xdelta3::encode;
///
/// fn main() {
///     let result = encode(&[1, 2, 3, 4, 5, 6, 7], &[1, 2, 4, 4, 7, 6, 7]);
///     assert_eq!(result.unwrap().as_slice(), &[214, 195, 196, 0, 0, 0, 13, 7, 0, 7, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
/// }
/// ```
///
/// You might notice the generated patch data is larger than both orginal data and the updated data.
/// But don't worry, if your data is large enough and kind of similar between each other (usually the case
/// for software updates or ROM patches), the patch data should be only a fraction of your updated file.
pub fn encode(input: &[u8], src: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let input_len = input.len() as c_uint;
        let src_len = src.len() as c_uint;
        let estimated_out_len = (input_len + src_len) * 2;
        let mut avail_output = 0 as c_uint;
        let mut output = Vec::with_capacity(estimated_out_len as usize);
        let result = binding::xd3_encode_memory(
            input.as_ptr(),
            input_len,
            src.as_ptr(),
            src_len,
            output.as_mut_ptr(),
            &mut avail_output,
            estimated_out_len,
            0,
        );
        if result == 0 {
            output.set_len(avail_output as usize);
            Some(output)
        } else {
            None
        }
    }
}

/// Function to decode the difference data
///
/// This function is used to decode the difference data.
/// The data in `src` will be treated as "original" data and the
/// data in `input` will be treated as "difference" or "patch" data.
/// The returned `Vec` stores the data that has been patched
///
/// As opposed to the encode function, if you are building an application that
/// applies patches or differential updates, this function should be used to
/// patch or update the old file from the patch data. It's recommeded to check
/// for the file integrity after doing the decode to prevent from creating
/// potentially corrupted files
///
/// Here is a basic example to show how to use this function:
/// ```
/// extern crate xdelta3;
/// use xdelta3::decode;
///
/// fn main() {
///     let result = decode(&[214, 195, 196, 0, 0, 0, 13, 7, 0, 7, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8], &[1, 2, 4, 4, 7, 6, 7]);
///     assert_eq!(result.unwrap().as_slice(), &[1, 2, 3, 4, 5, 6, 7]);
/// }
/// ```
pub fn decode(input: &[u8], src: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let input_len = input.len() as c_uint;
        let src_len = src.len() as c_uint;
        let estimated_out_len = (input_len + src_len) * 2;
        let mut avail_output = 0 as c_uint;
        let mut output = Vec::with_capacity(estimated_out_len as usize);
        let result = binding::xd3_decode_memory(
            input.as_ptr(),
            input_len,
            src.as_ptr(),
            src_len,
            output.as_mut_ptr(),
            &mut avail_output,
            estimated_out_len,
            0,
        );
        if result == 0 {
            output.set_len(avail_output as usize);
            Some(output)
        } else {
            None
        }
    }
}

pub fn decode2(input: &[u8], src_: &[u8]) -> Option<Vec<u8>> {
    let mut stream: binding::xd3_stream = unsafe { std::mem::zeroed() };
    let mut cfg: binding::xd3_config = unsafe { std::mem::zeroed() };

    let ret = unsafe { binding::xd3_config_stream(&mut stream, &mut cfg) };
    assert_eq!(ret, 0);

    let mut src: binding::xd3_source = unsafe { std::mem::zeroed() };
    src.max_winsize = 32768;
    src.blksize = 32768;

    let ret = unsafe { binding::xd3_set_source(&mut stream, &mut src) };
    assert_eq!(ret, 0);

    let mut out = Vec::new();
    let mut count = 0;
    let mut eof = false;
    'outer: while !eof {
        // xd3_avail_input
        stream.next_in = input.as_ptr();
        stream.avail_in = input.len() as u32;
        // xd3_set_flags
        stream.flags = binding::xd3_flags::XD3_FLUSH as i32;
        eof = true;

        'process: loop {
            let ret: binding::xd3_rvalues =
                unsafe { std::mem::transmute(binding::xd3_decode_input(&mut stream)) };

            if stream.msg != std::ptr::null() {
                println!("msg={:?}", unsafe { std::ffi::CStr::from_ptr(stream.msg) });
            }

            use binding::xd3_rvalues::*;
            match ret {
                XD3_INPUT => {
                    println!("input");
                    continue 'outer;
                    //
                }
                XD3_OUTPUT => {
                    println!("output");
                    let out_data = unsafe {
                        std::slice::from_raw_parts(stream.next_out, stream.avail_out as usize)
                    };
                    out.extend_from_slice(out_data);
                    //
                }
                XD3_GETSRCBLK => {
                    println!(
                        "getsrcblk: curblkno={}, getblkno={} len={}",
                        src.curblkno,
                        src.getblkno,
                        src_.len()
                    );

                    let blkno = src.getblkno as usize;
                    let start = src.blksize as usize * blkno;
                    let end = src.blksize as usize * (blkno + 1);
                    let end = end.min(src_.len());

                    let data = &src_[start..end];

                    src.curblkno = src.getblkno;
                    src.curblk = data.as_ptr();
                    src.onblk = data.len() as u32;

                    src.max_blkno = src.curblkno;
                    src.onlastblk = src.onblk;
                    src.eof_known = src_.is_empty() as i32;
                }
                XD3_GOTHEADER => {
                    println!("gotheader");
                    //
                }
                XD3_WINSTART => {
                    println!("winstart");
                    //
                }
                XD3_WINFINISH => {
                    println!("winfinish");
                    //
                }
                XD3_TOOFARBACK => {
                    println!("toofarback");
                    //
                }
                XD3_INTERNAL => {
                    println!("internal");
                    return None;
                    //
                }
                XD3_INVALID => {
                    println!("invalid");
                    return None;
                    //
                }
                XD3_INVALID_INPUT => {
                    println!("invalid_input");
                    return None;
                    //
                }
                XD3_NOSECOND => {
                    println!("nosecond");
                    return None;
                    //
                }
                XD3_UNIMPLEMENTED => {
                    println!("unimplemented");
                    return None;
                    //
                }
            }
        }
    }

    Some(out)
}
