extern crate alloc;

use core::{mem, ptr};
use alloc::alloc::Layout;

pub use mem::{MaybeUninit, transmute};

//Linux & win 32 bit are 8
#[cfg(not(any(target_os = "macos", all(windows, target_pointer_width = "64"))))]
const MIN_ALIGN: usize = 8;
//Mac and  win 64 bit are 16
#[cfg(any(target_os = "macos", all(windows, target_pointer_width = "64")))]
const MIN_ALIGN: usize = 16;

const LAYOUT_OFFSET: usize = mem::size_of::<usize>();

#[repr(transparent)]
///Unique ptr with allocated storage
///
///Can never be null
pub struct Unique<T>(ptr::NonNull<T>);

impl<T> Unique<T> {
    #[cold]
    #[inline(never)]
    fn unlikely_null() -> Option<Self> {
        None
    }

    #[inline(always)]
    pub fn as_mut(&mut self) -> *mut T {
        unsafe {
            self.0.as_mut()
        }
    }

    pub fn new(size: usize) -> Option<Self> {
        if let Ok(layout) = Layout::from_size_align(size + LAYOUT_OFFSET, MIN_ALIGN) {
            unsafe {
                let ptr = alloc::alloc::alloc(layout);
                if let Some(ptr) = ptr::NonNull::new(ptr) {
                    ptr::write(ptr.as_ptr() as *mut usize, size);
                    return Some(Self(ptr.add(LAYOUT_OFFSET).cast()));
                }
            }
        }

        Self::unlikely_null()
    }
}

impl<T> Drop for Unique<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let ptr = self.0.as_ptr();
            let mem = (ptr as *mut u8).offset(-(LAYOUT_OFFSET as isize));
            let size = ptr::read(ptr as *const usize);
            let layout = Layout::from_size_align_unchecked(size, MIN_ALIGN);
            alloc::alloc::dealloc(mem, layout);
        }
    }
}
