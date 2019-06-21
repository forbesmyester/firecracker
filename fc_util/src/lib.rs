// Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

extern crate libc;

pub mod validators;

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

}
