use std::alloc::{GlobalAlloc, Layout};

#[cfg(any(miri, target_family = "wasm"))]
#[global_allocator]
pub static ALLOCATOR: Allocator<std::alloc::System> =
    Allocator(std::alloc::System);

#[cfg(not(any(miri, target_family = "wasm")))]
#[global_allocator]
pub static ALLOCATOR: Allocator<mimalloc::MiMalloc> =
    Allocator(mimalloc::MiMalloc);

pub static mut ALLOCATED_BYTES: usize = 0;

pub struct Allocator<T>(T);

unsafe impl<T: GlobalAlloc> GlobalAlloc for Allocator<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED_BYTES += layout.size();
        self.0.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED_BYTES -= layout.size();
        self.0.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATED_BYTES += layout.size();
        self.0.alloc_zeroed(layout)
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: Layout,
        new_size: usize,
    ) -> *mut u8 {
        ALLOCATED_BYTES = (ALLOCATED_BYTES + new_size) - layout.size();
        self.0.realloc(ptr, layout, new_size)
    }
}
