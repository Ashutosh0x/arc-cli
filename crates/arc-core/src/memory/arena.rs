//! Memory Arena — contiguous allocation for high-performance string processing.
//! Used during context window compression and observation generation to minimize
//! allocator overhead when processing thousands of tokens of chat history.

use bumpalo::Bump;
use std::cell::RefCell;

thread_local! {
    /// Thread-local memory arena for temporary string allocations during compression.
    /// Reused across turns to avoid repeated system allocator calls.
    static COMPRESSION_ARENA: RefCell<Bump> = RefCell::new(Bump::with_capacity(64 * 1024)); // 64KB initial
}

/// Executes a closure with a temporary zero-cost allocation arena.
/// The arena is automatically cleared when the closure returns.
pub fn with_compression_arena<F, R>(f: F) -> R
where
    F: FnOnce(&Bump) -> R,
{
    COMPRESSION_ARENA.with(|arena_cell| {
        let result = {
            let arena = arena_cell.borrow();
            f(&arena)
        };
        // Clear all allocations made during this closure while keeping capacity
        arena_cell.borrow_mut().reset();
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_reset() {
        let bytes_after_alloc = with_compression_arena(|bump| {
            let _s = bump.alloc_str("temporary string");
            let used = bump.allocated_bytes();
            assert!(used > 0, "should have allocated bytes");
            used
        });

        with_compression_arena(|bump| {
            // After reset, allocated_bytes should be less than what we saw during use,
            // or the arena should be reusable. Bumpalo may retain internal bookkeeping
            // so we just verify it's functional and smaller than the previous allocation.
            assert!(
                bump.allocated_bytes() < bytes_after_alloc,
                "arena should have been reset: current={}, previous={}",
                bump.allocated_bytes(),
                bytes_after_alloc
            );
        });
    }
}
