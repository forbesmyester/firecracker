// Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

extern crate libc;
use std::ffi::OsString;
use std::str;

pub mod validators;
pub mod fs;

pub fn timestamp_cycles() -> u64 {
    #[cfg(target_arch = "x86_64")]
    // Safe because there's nothing that can go wrong with this call.
    unsafe {
        std::arch::x86_64::_rdtsc() as u64
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        unsafe {
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
        }
        (ts.tv_sec as u64) * 1000000000 + (ts.tv_nsec as u64)
    }
}

fn timespec_to_us(time_struct: &libc::timespec) -> u64 {
    (time_struct.tv_sec as u64) * 1_000_000 + (time_struct.tv_nsec as u64) / 1000
}

pub fn now_cputime_us() -> u64 {
    let mut time_struct = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Safe because the parameters are valid.
    unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut time_struct) };
    timespec_to_us(&time_struct)
}

// This generates pseudo random u32 numbers based on the current timestamp. Only works for x86_64,
// but can find something else if we ever need to support different architectures.
pub fn xor_rng_u32() -> u32 {
    let mut t: u32 = timestamp_cycles() as u32;
    // Taken from https://en.wikipedia.org/wiki/Xorshift
    t ^= t << 13;
    t ^= t >> 17;
    t ^ (t << 5)
}


fn xor_rng_u8_alphanumerics(rand_fn: &Fn() -> u32) -> Vec<u8> {
    let mut r = vec!();
    for n in &rand_fn().to_ne_bytes() {
        if (48..58).contains(n) || (65..91).contains(n) || (97..123).contains(n) {
            r.push(*n);
        }
    }
    r
}


fn rand_alphanumerics_impl(rand_fn: &Fn() -> u32, len: usize) -> OsString {

    let mut buf = OsString::new();
    let mut done = 0;
    loop {
        for n in xor_rng_u8_alphanumerics(rand_fn) {
            done = done + 1;
            buf.push(str::from_utf8(&[n]).unwrap_or("_"));
            // unsafe { // Safe because the bounds of n are defined as [a-zA-Z0-9]
            //     buf.push(str::from_utf8_unchecked(&[n]));
            // }
            if done >= len {
                return buf;
            }
        }
    }
}


pub fn rand_alphanumerics(len: usize) -> OsString {
    return rand_alphanumerics_impl(&xor_rng_u32, len);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_cycles() {
        for _ in 0..1000 {
            assert!(timestamp_cycles() < timestamp_cycles());
        }
    }

    #[test]
    fn test_now_cputime_us() {
        for _ in 0..1000 {
            assert!(now_cputime_us() <= now_cputime_us());
        }
    }

    #[test]
    fn test_xor_rng_u32() {
        for _ in 0..1000 {
            assert_ne!(xor_rng_u32(), xor_rng_u32());
        }
    }

    #[test]
    fn test_xor_rng_u8_alphas() {
        let s = xor_rng_u8_alphanumerics(&|| { 14134 });
        assert_eq!(vec![54, 55], s);
    }

    #[test]
    fn test_rand_alphanumerics_impl() {
        let s = rand_alphanumerics_impl(&|| { 14134 }, 5);
        println!("{:?}", s);
        assert_eq!("67676", s);
    }

    #[test]
    fn test_rand_alphanumerics() {
        let s = rand_alphanumerics(5);
        assert_eq!(5, s.len());
    }

}
