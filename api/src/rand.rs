use lc_crypto::{
    digest::{
        ContinuousOutputDigest, RawDigest,
        raw::sha3::{Keccack, RawShake128Spec},
    },
    mem::copy_from_slice_truncate,
    raw_shake256,
    traits::ByteArray,
};
use rand_core::{
    RngCore,
    impls::{next_u32_via_fill, next_u64_via_fill},
};

pub struct CsRand(raw_shake256!(128));

impl CsRand {
    pub fn push_enthropy(&mut self, enthropy: &[u8]) {
        let chunks = ByteArray::array_chunks(enthropy);
        let rem = chunks.remainder();
        for chunk in chunks {
            self.0.raw_update(chunk).unwrap();
        }
        self.0.raw_update_final(rem).unwrap();
    }
}

impl RngCore for CsRand {
    fn next_u32(&mut self) -> u32 {
        next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        let mut chunks = <[u8; 16] as ByteArray>::array_chunks_mut(dst);

        for chunk in &mut chunks {
            *chunk = self.0.next_output().unwrap();
        }

        let rem = chunks.into_remainder();

        if rem.len() != 0 {
            let last = self.0.next_output().unwrap();
            copy_from_slice_truncate(rem, &last);
        }
    }
}
