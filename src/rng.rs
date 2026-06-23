//! A tiny PRNG.
//!
//! We spin a few reels, that's it. Pulling in `rand` for that would be silly,
//! and `rand` isn't always trivial to build static. xorshift64* is a dozen lines,
//! passes most of the usual tests and is plenty random for a slot machine.

use std::fs::File;
use std::io::Read;

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Zero is a fixed point for xorshift, so refuse it.
        let state = if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        };
        Rng { state }
    }

    /// Seed from `$SLOTS_SEED` if it is set (reproducible runs for tests),
    /// then `/dev/urandom`, then a weak time/pid fallback if even that fails.
    pub fn from_env() -> Self {
        if let Ok(s) = std::env::var("SLOTS_SEED") {
            if let Ok(n) = s.trim().parse::<u64>() {
                return Rng::new(n);
            }
        }
        Rng::new(seed_from_urandom().unwrap_or_else(weak_seed))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A value in `0..n`, uniform. Rejection sampling keeps small ranges unbiased,
    /// which matters because reel lengths are not powers of two.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "below(0) makes no sense");
        let n = n as u64;
        let limit = u64::MAX - (u64::MAX % n);
        loop {
            let v = self.next_u64();
            if v < limit {
                return (v % n) as usize;
            }
        }
    }
}

fn seed_from_urandom() -> Option<u64> {
    let mut f = File::open("/dev/urandom").ok()?;
    let mut buf = [0u8; 8];
    f.read_exact(&mut buf).ok()?;
    Some(u64::from_le_bytes(buf))
}

fn weak_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ (std::process::id() as u64).rotate_left(17)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn zero_seed_is_replaced() {
        let mut r = Rng::new(0);
        // Would be stuck at 0 forever if we hadn't guarded against it.
        assert_ne!(r.next_u64(), 0);
    }

    #[test]
    fn below_stays_in_range() {
        let mut r = Rng::new(7);
        for _ in 0..10_000 {
            let v = r.below(24);
            assert!(v < 24);
        }
    }

    #[test]
    fn below_covers_the_range() {
        let mut r = Rng::new(123);
        let mut seen = [false; 6];
        for _ in 0..1000 {
            seen[r.below(6)] = true;
        }
        assert!(seen.iter().all(|&s| s), "every bucket should show up");
    }
}
