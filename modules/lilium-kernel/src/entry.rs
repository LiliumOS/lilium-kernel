use core::{
    cell::SyncUnsafeCell,
    ffi::{CStr, c_char, c_void},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

use bytemuck::Zeroable;
use los_api::{
    auxv::{AT_IGNORE, AT_LILIUM_MAX_KERNEL, AuxvEnt},
    hcf,
};

#[cfg(target_arch = "x86_64")]
mod x86;

#[repr(transparent)]
pub struct CStrRef<'a>(NonNull<c_char>, PhantomData<&'a ()>);

unsafe impl<'a> Send for CStrRef<'a> {}
unsafe impl<'a> Sync for CStrRef<'a> {}

impl<'a> core::fmt::Debug for CStrRef<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<'a> core::fmt::Display for CStrRef<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<'a> CStrRef<'a> {
    /// # Safety
    ///
    /// The behaviour is undefined unless:
    /// * `x` must be a non-null pointer that points to a live allocation,
    /// * There exists a value `p` of type [`usize`], such that `x.add(p)` is well-defined and valid for reading, and `x.add(p).read() == 0` is true,
    /// * The range `[x, x.add(p)]` must be valid for reading for the duration of 'a,
    /// * The range `[x, x.add(p)]` must not be modified for the duration of 'a,
    /// * The range `[x, x.add(p))` must consist of valid UTF-8.
    pub const unsafe fn from_raw(x: *const c_char) -> Self {
        Self(unsafe { NonNull::new_unchecked(x.cast_mut()) }, PhantomData)
    }

    pub const fn as_cstr(&self) -> &'a CStr {
        unsafe { CStr::from_ptr(self.0.as_ptr()) }
    }

    pub const fn as_str(&self) -> &'a str {
        unsafe { core::str::from_utf8_unchecked(self.as_cstr().to_bytes()) }
    }
}

impl<'a> Deref for CStrRef<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<'a> AsRef<str> for CStrRef<'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> AsRef<[u8]> for CStrRef<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_cstr().to_bytes()
    }
}

impl<'a> AsRef<CStr> for CStrRef<'a> {
    fn as_ref(&self) -> &CStr {
        self.as_cstr()
    }
}

unsafe extern "system" fn begin_kernel(
    argc: isize,
    argv: *const CStrRef<'static>,
    envp: *const CStrRef<'static>,
    auxv: *const AuxvEnt,
) -> ! {
    let args = unsafe { core::slice::from_raw_parts(argv, argc as usize) };

    let mut end = envp;

    while unsafe { end.cast::<Option<CStrRef>>().read().is_some() } {
        end = unsafe { end.add(1) };
    }

    let env = unsafe { core::slice::from_ptr_range(envp..end) };

    let mut end = auxv;

    while unsafe { end.cast::<usize>().read() != 0 } {
        end = unsafe { end.add(1) };
    }

    let aux = unsafe { core::slice::from_ptr_range(auxv..end) };

    let mut block = InitBlock {
        args,
        env,
        aux: bytemuck::zeroed(),
    };

    for at in aux {
        if at.a_type == AT_IGNORE {
            continue;
        }

        block.aux[(at.a_type - 2) as usize] = AuxVal(at.a_value);
    }

    unsafe {
        INIT.set(Some(block));
    }
    hcf()
}

static INIT: InitCell<Option<InitBlock>> = InitCell::new(None);

struct InitCell<T>(SyncUnsafeCell<T>);

impl<T> InitCell<T> {
    pub const fn new(val: T) -> Self {
        Self(SyncUnsafeCell::new(val))
    }

    pub unsafe fn set(&self, val: T)
    where
        T: Copy,
    {
        unsafe {
            self.0.get().write(val);
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }
}

#[derive(Copy, Clone, Debug, Zeroable)]
pub struct AuxVal(*mut c_void);

unsafe impl Send for AuxVal {}
unsafe impl Sync for AuxVal {}

impl AuxVal {
    pub fn as_ptr(self) -> *mut c_void {
        self.0
    }

    pub fn as_int(self) -> usize {
        self.0.addr()
    }

    pub fn as_fnptr(self) -> Option<unsafe extern "C" fn()> {
        unsafe { core::mem::transmute(self.0) }
    }
}

#[derive(Copy, Clone, Debug)]
struct InitBlock<'a> {
    args: &'a [CStrRef<'a>],
    env: &'a [CStrRef<'a>],
    aux: [AuxVal; AT_LILIUM_MAX_KERNEL - 2],
}

pub fn args() -> impl Iterator<Item = &'static str> {
    INIT.get().iter().flat_map(|v| v.args).map(|v| v.as_str())
}

pub fn env() -> impl Iterator<Item = (&'static str, &'static str)> {
    INIT.get()
        .iter()
        .flat_map(|v| v.args)
        .map(|v| v.as_str())
        .flat_map(|v| v.split_once('='))
}

pub fn getauxval(aux: usize) -> AuxVal {
    if aux < 2 || aux > AT_LILIUM_MAX_KERNEL {
        return AuxVal(core::ptr::null_mut());
    }

    INIT.get()
        .as_ref()
        .map(|v| v.aux[(aux - 2) as usize])
        .unwrap_or_else(bytemuck::zeroed)
}
