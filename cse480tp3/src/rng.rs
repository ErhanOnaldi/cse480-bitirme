use std::num::NonZeroU64;

#[derive(Clone)]
pub struct XorShift64 {
    state: NonZeroU64,
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        let seed = NonZeroU64::new(seed).unwrap_or(NonZeroU64::new(0x9E37_79B9_7F4A_7C15).unwrap());
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = NonZeroU64::new(x).unwrap_or(NonZeroU64::new(0xD1B5_4A32_D192_ED03).unwrap());
        self.state.get()
    }

    pub fn gen_f64(&mut self) -> f64 {
        // 53-bit precision in [0,1)
        let v = self.next_u64() >> 11;
        (v as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    pub fn gen_range_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 {
            return 0;
        }
        (self.next_u64() as usize) % upper_exclusive
    }

    pub fn gen_range_u32(&mut self, low: u32, high_inclusive: u32) -> u32 {
        debug_assert!(low <= high_inclusive);
        let span = (high_inclusive - low) as u64 + 1;
        low + ((self.next_u64() % span) as u32)
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.gen_range_usize(i + 1);
            slice.swap(i, j);
        }
    }
}

