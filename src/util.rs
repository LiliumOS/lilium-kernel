use core::ops::{Deref, DerefMut};

pub struct UnsafeSync<T>(T);

impl<T> UnsafeSync<T> {
    pub const unsafe fn new(val: T) -> Self {
        Self(val)
    }
}

unsafe impl<T> Sync for UnsafeSync<T> {}

impl<T> Deref for UnsafeSync<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for UnsafeSync<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
