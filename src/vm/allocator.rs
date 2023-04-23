use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(any(miri, target_family = "wasm"))]
#[global_allocator]
pub static GLOBAL: Allocator<std::alloc::System> = Allocator::new(std::alloc::System);

#[cfg(not(any(miri, target_family = "wasm")))]
#[global_allocator]
pub static GLOBAL: Allocator<mimalloc::MiMalloc> = Allocator::new(mimalloc::MiMalloc);

#[derive(Debug)]
pub struct Allocator<T> {
    inner: T,
    allocated_bytes: AtomicUsize,
}

impl<T> Allocator<T> {
    const fn new(inner: T) -> Self {
        Self { inner, allocated_bytes: AtomicUsize::new(0) }
    }

    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes.load(Ordering::Relaxed)
    }
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for Allocator<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocated_bytes.fetch_add(layout.size(), Ordering::Relaxed);
        self.inner.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.allocated_bytes.fetch_sub(layout.size(), Ordering::Relaxed);
        self.inner.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.allocated_bytes.fetch_add(layout.size(), Ordering::Relaxed);
        self.inner.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.allocated_bytes.fetch_add(new_size.wrapping_sub(layout.size()), Ordering::Relaxed);
        self.inner.realloc(ptr, layout, new_size)
    }
}
