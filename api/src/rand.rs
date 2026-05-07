use bytemuck::Pod;
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
use thingbuf::{Recycle, StaticThingBuf};

type Xof = raw_shake256!(128);

pub struct RecyclePod;

impl<P: Pod> Recycle<P> for RecyclePod {
    fn new_element(&self) -> P {
        bytemuck::zeroed()
    }

    fn recycle(&self, _: &mut P) {}
}

pub struct CsRand(Xof);

static EPOOL: StaticThingBuf<<Xof as RawDigest>::Block, 128, RecyclePod> =
    StaticThingBuf::with_recycle(RecyclePod);

pub fn add_enthropy(e: &[u8]) {
    let chunks = ByteArray::array_chunks(e);
    let rem = chunks.remainder();
    for chunk in chunks {
        let _ = EPOOL.push(*chunk);
    }
    let mut buf: <Xof as RawDigest>::Block = ByteArray::extend(rem);
    buf[rem.len()] = 0b111;
    *ByteArray::last_mut(&mut buf) |= 0x80;
    let _ = EPOOL.push(buf);
}

impl CsRand {
    pub fn push_enthropy(&mut self, enthropy: &[u8]) {
        let chunks = ByteArray::array_chunks(enthropy);
        let rem = chunks.remainder();
        for chunk in chunks {
            self.0.raw_update(chunk).unwrap();
        }
        self.0.raw_update_final(rem).unwrap();
    }

    pub fn push_pending_enthropy(&mut self) {
        while let Some(v) = EPOOL.pop() {
            self.0.raw_update(&v).unwrap();
        }
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
        self.push_pending_enthropy();
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
