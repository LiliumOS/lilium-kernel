use core::ffi::c_void;
use core::marker::PhantomData;

use alloc::{alloc::Allocator, vec::Vec};

pub struct ContextAllocator<C>(PhantomData<C>);

unsafe impl<C: Context> Allocator for ContextAllocator<C> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        todo!()
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        todo!()
    }
}

#[repr(C)]
pub struct KernelContext {
    pub ksp: *mut c_void,
}

#[repr(C)]
pub struct ProcessContext {
    all_threads: Vec<*mut ThreadContext, ContextAllocator<ProcessContext>>,
}

unsafe impl Context for ProcessContext {
    fn lookup() -> *mut Self {
        let x = ThreadContext::lookup();

        unsafe { (*x).process }
    }
}

pub struct ThreadContext {
    pub process: *mut ProcessContext,
}

unsafe impl Context for ThreadContext {
    fn lookup() -> *mut Self {
        let x = CoreContext::lookup();

        unsafe { (*x).ucontext }
    }
}

pub unsafe trait Context {
    fn lookup() -> *mut Self;
}

unsafe impl Context for KernelContext {
    fn lookup() -> *mut Self {
        let x = CoreContext::lookup();

        unsafe { (*x).kcontext }
    }
}

#[repr(C, align(4096))]
pub struct CoreContext {
    pub core_ctx_addr: *mut CoreContext,
    reserved: [*mut c_void; 3],
    pub kcontext: *mut KernelContext,
    pub ucontext: *mut ThreadContext,
}

#[cfg(target_arch = "x86_64")]
unsafe impl Context for CoreContext {
    fn lookup() -> *mut Self {
        let x: *mut Self;
        unsafe {
            core::arch::asm!("mov {}, gs:[0]", out(reg) x);
        }
        x
    }
}
