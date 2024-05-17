mod counter;

use std::{
    alloc::{GlobalAlloc, Layout},
    marker::PhantomData,
};

use self::counter::{add, flush, get, remove};

#[derive(Default, Clone, Debug)]
pub struct AllocationInfo {
    pub allocations: usize,
    pub deallocations: usize,
    pub allocation_count: usize,
    pub deallocation_count: usize,
}

impl AllocationInfo {
    pub fn is_empty(&self) -> bool {
        self.allocations == 0
            && self.deallocations == 0
            && self.allocation_count == 0
            && self.deallocation_count == 0
    }
}

#[derive(Default, Clone, Debug)]
pub struct AllocationCounters {
    pub allocations: usize,
    pub deallocations: usize,
    pub allocation_count: usize,
    pub deallocation_count: usize,
    _not_send: PhantomData<*mut ()>,
}

impl AllocationCounters {
    pub fn until_now(&self) -> AllocationInfo {
        let new = TurboMalloc::allocation_counters();
        AllocationInfo {
            allocations: new.allocations - self.allocations,
            deallocations: new.deallocations - self.deallocations,
            allocation_count: new.allocation_count - self.allocation_count,
            deallocation_count: new.deallocation_count - self.deallocation_count,
        }
    }
}

/// Turbo's preferred global allocator. This is a new type instead of a type
/// alias because you can't use type aliases to instantiate unit types (E0423).
pub struct TurboMalloc;

impl TurboMalloc {
    pub fn memory_usage() -> usize {
        get()
    }

    pub fn thread_stop() {
        flush();
    }

    pub fn allocation_counters() -> AllocationCounters {
        self::counter::allocation_counters()
    }
}

#[cfg(all(
    feature = "custom_allocator",
    not(all(target_os = "linux", target_arch = "aarch64"))
))]
unsafe impl GlobalAlloc for TurboMalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = mimalloc::MiMalloc.alloc(layout);
        if !ret.is_null() {
            add(layout.size());
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        mimalloc::MiMalloc.dealloc(ptr, layout);
        remove(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ret = mimalloc::MiMalloc.alloc_zeroed(layout);
        if !ret.is_null() {
            add(layout.size());
        }
        ret
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let ret = mimalloc::MiMalloc.realloc(ptr, layout, new_size);
        if !ret.is_null() {
            let old_size = layout.size();
            if old_size < new_size {
                add(new_size - old_size);
            } else {
                remove(old_size - new_size);
            }
        }
        ret
    }
}

#[cfg(any(
    not(feature = "custom_allocator"),
    all(target_os = "linux", target_arch = "aarch64")
))]
unsafe impl GlobalAlloc for TurboMalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = std::alloc::System.alloc(layout);
        if !ret.is_null() {
            add(layout.size());
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        std::alloc::System.dealloc(ptr, layout);
        remove(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ret = std::alloc::System.alloc_zeroed(layout);
        if !ret.is_null() {
            add(layout.size());
        }
        ret
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let ret = std::alloc::System.realloc(ptr, layout, new_size);
        if !ret.is_null() {
            let old_size = layout.size();
            if old_size < new_size {
                add(new_size - old_size);
            } else {
                remove(old_size - new_size);
            }
        }
        ret
    }
}