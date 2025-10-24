pub use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};

#[cfg(target_feature = "sse")]
#[repr(align(16))]
pub struct InterruptContext {}

#[cfg(not(target_feature = "sse"))]
pub struct InterruptContext<'a> {
    pub frame: &'a mut InterruptStackFrame,
    pub gprs: [usize; 15],
}

use crate::hcf;

pub unsafe trait InterruptResult {
    const IS_DIVERGING: bool = false;
}

unsafe impl InterruptResult for () {}
unsafe impl InterruptResult for ! {
    const IS_DIVERGING: bool = true;
}

pub unsafe trait InterruptErrorCode: Copy {}

unsafe impl InterruptErrorCode for u64 {}
unsafe impl InterruptErrorCode for usize {}
unsafe impl InterruptErrorCode for PageFaultErrorCode {}

pub const fn exception_handler<
    F: Fn(&mut InterruptStackFrame, T) -> R,
    T: InterruptErrorCode,
    R: InterruptResult,
>(
    f: F,
) -> extern "x86-interrupt" fn(InterruptStackFrame, T) -> R {
    const {
        assert!(
            core::mem::size_of::<F>() == 0,
            "Only ZST functions (function items and non-capturing closures) are allowed"
        );
    }

    core::mem::forget(f);

    extern "C" fn call_hdl<
        F: FnOnce(&mut InterruptStackFrame, T) -> R,
        R: InterruptResult,
        T: InterruptErrorCode,
    >(
        x: &mut InterruptStackFrame,
        errc: T,
    ) -> R {
        let f = unsafe { core::mem::conjure_zst::<F>() };
        f(x, errc)
    }

    #[unsafe(naked)]

    extern "x86-interrupt" fn hdl_impl_diverging<
        F: FnOnce(&mut InterruptStackFrame, T) -> R,
        R: InterruptResult,
        T: InterruptErrorCode,
    >(
        sframe: InterruptStackFrame,
        errc: T,
    ) -> ! {
        core::arch::naked_asm! {
            "pop rsi",
            "lea rdi, [rsp]",
            "and rsp, ~15",
            "call {bounce}",
            "jmp {hcf}",
            bounce = sym call_hdl::<F, R, T>,
            hcf = sym hcf,
        }
    }

    #[cfg(target_feature = "sse")]
    #[unsafe(naked)]
    extern "x86-interrupt" fn hdl_impl<
        F: FnOnce(&mut InterruptStackFrame, T) -> R,
        R: InterruptResult,
        T: InterruptErrorCode,
    >(
        sframe: InterruptStackFrame,
        errc: T,
    ) {
        core::arch::naked_asm! {
            "xchg [rsp], rax",
            "push rbp",
            "lea rbp, [rsp]",
            "and rsp, ~15",
            "push rsi",
            "mov esi, eax",
            "push rdi",
            "push rdx",
            "push rcx",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "sub rsp, 464",
            "fxsave [rsp]",
            "fninit",
            "lea rdi, [rbp+16]",
            "call {bounce}",
            "fxrstor [rsp]",
            "add rsp, 464",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rcx",
            "pop rdx",
            "pop rdi",
            "pop rsi",
            "leave",
            "pop rax",
            "iretq",
            bounce = sym call_hdl::<F, R, T>,
        }
    }

    #[cfg(not(target_feature = "sse"))]
    #[unsafe(naked)]
    extern "x86-interrupt" fn hdl_impl<F: Fn(&mut InterruptStackFrame) -> R, R: InterruptResult>(
        sframe: InterruptStackFrame,
    ) {
        core::arch::naked_asm!(
            "push rbp",
            "mov rbp, rsp",
            "and rsp, ~15",
            "push rdi",
            "push rsi",
            "push rdx",
            "push rcx",
            "push rax",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "lea rdi, [rbp+8]",
            "call {bounce}",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rax",
            "pop rcx",
            "pop rdx",
            "pop rsi",
            "pop rdi",
            "leave",
            "iretq",
        )
    }

    if const { R::IS_DIVERGING } {
        unsafe {
            core::mem::transmute(
                hdl_impl::<F, R, T> as extern "x86-interrupt" fn(InterruptStackFrame, T),
            )
        }
    } else {
        unsafe {
            core::mem::transmute(
                hdl_impl_diverging::<F, R, T>
                    as extern "x86-interrupt" fn(InterruptStackFrame, T) -> !,
            )
        }
    }
}

pub const fn interrupt_handler<F: Fn(&mut InterruptStackFrame) -> R, R: InterruptResult>(
    f: F,
) -> extern "x86-interrupt" fn(InterruptStackFrame) -> R {
    const {
        assert!(
            core::mem::size_of::<F>() == 0,
            "Only ZST functions (function items and non-capturing closures) are allowed"
        );
    }

    core::mem::forget(f);

    extern "C" fn call_hdl<F: Fn(&mut InterruptStackFrame) -> R, R: InterruptResult>(
        x: &mut InterruptStackFrame,
    ) -> R {
        let f = unsafe { core::mem::conjure_zst::<F>() };
        f(x)
    }

    #[unsafe(naked)]
    extern "x86-interrupt" fn hdl_impl_diverging<
        F: Fn(&mut InterruptStackFrame) -> R,
        R: InterruptResult,
    >(
        sframe: InterruptStackFrame,
    ) -> ! {
        core::arch::naked_asm! {
            "lea rdi, [rsp]",
            "and rsp, ~15",
            "call {bounce}",
            "jmp {hcf}",
            bounce = sym call_hdl::<F, R>,
            hcf = sym hcf,
        }
    }

    #[unsafe(naked)]
    #[cfg(target_feature = "sse")]
    extern "x86-interrupt" fn hdl_impl<F: Fn(&mut InterruptStackFrame) -> R, R: InterruptResult>(
        sframe: InterruptStackFrame,
    ) {
        core::arch::naked_asm! {
            "push rbp",
            "mov rbp, rsp",
            "and rsp, ~15",
            "push rdi",
            "push rsi",
            "push rdx",
            "push rcx",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "sub rsp, 464",
            "fxsave [rsp]",
            "lea rdi, [rbp+8]",
            "call {bounce}",
            "fxrstor [rsp]",
            "add rsp, 464",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rcx",
            "pop rdx",
            "pop rsi",
            "pop rdi",
            "leave",
            "iretq",
            bounce = sym call_hdl::<F, R>,
        }
    }

    #[cfg(not(target_feature = "sse"))]
    #[unsafe(naked)]
    extern "x86-interrupt" fn hdl_impl<F: Fn(&mut InterruptStackFrame) -> R, R: InterruptResult>(
        sframe: InterruptStackFrame,
    ) {
        core::arch::naked_asm!(
            "push rbp",
            "mov rbp, rsp",
            "and rsp, ~15",
            "push rdi",
            "push rsi",
            "push rdx",
            "push rcx",
            "push rax",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "lea rdi, [rbp+8]",
            "call {bounce}",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rax",
            "pop rcx",
            "pop rdx",
            "pop rsi",
            "pop rdi",
            "leave",
            "iretq",
        )
    }

    if const { R::IS_DIVERGING } {
        unsafe {
            core::mem::transmute(hdl_impl::<F, R> as extern "x86-interrupt" fn(InterruptStackFrame))
        }
    } else {
        unsafe {
            core::mem::transmute(
                hdl_impl_diverging::<F, R> as extern "x86-interrupt" fn(InterruptStackFrame) -> !,
            )
        }
    }
}
