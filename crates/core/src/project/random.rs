//! Injected randomness: identities are drawn from the operating system's CSPRNG in production
//! and from a fixed byte sequence in tests.

/// A source of random bytes.
pub trait RandomSource {
    fn fill(&mut self, buf: &mut [u8]);
}

/// The operating system's CSPRNG.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn fill(&mut self, buf: &mut [u8]) {
        getrandom::fill(buf).expect("the OS random number generator is unavailable");
    }
}

/// A scripted source: it returns the given bytes in order, then starts again from the first.
#[derive(Debug, Clone)]
pub struct ScriptedRandom {
    bytes: Vec<u8>,
    next: usize,
}

impl ScriptedRandom {
    /// # Panics
    /// When `bytes` is empty.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        let bytes = bytes.into();
        assert!(
            !bytes.is_empty(),
            "a scripted source needs at least one byte"
        );
        Self { bytes, next: 0 }
    }
}

impl RandomSource for ScriptedRandom {
    fn fill(&mut self, buf: &mut [u8]) {
        for slot in buf {
            *slot = self.bytes[self.next];
            self.next = (self.next + 1) % self.bytes.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_source_cycles() {
        let mut rng = ScriptedRandom::new([1, 2, 3]);
        let mut buf = [0u8; 5];
        rng.fill(&mut buf);
        assert_eq!(buf, [1, 2, 3, 1, 2]);
        rng.fill(&mut buf[..2]);
        assert_eq!(&buf[..2], &[3, 1]);
    }

    #[test]
    fn os_source_fills_the_buffer() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        OsRandom.fill(&mut a);
        OsRandom.fill(&mut b);
        assert_ne!(a, b);
    }
}
