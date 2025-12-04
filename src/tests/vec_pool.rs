use std::collections::HashSet;

use crate::vec_pool::VecPool;

#[test]
fn test_vec_pool_reuses_two_allocations() {
    // initial amount 2, inner capacity 16
    let mut pool: VecPool<u8> = VecPool::with_initial_amount(2, 16);

    // initially pool has two vecs
    assert_eq!(pool.pool_len(), 2);

    let mut ptrs: HashSet<usize> = HashSet::new();

    // repeatedly take and return a vec, record the pointer address
    for _ in 0..100 {
        {
            let mut guard = pool.take_vec();
            // push some data to ensure allocation
            guard.extend(std::iter::repeat_n(0u8, 8));
            let p = guard.as_ptr() as usize;
            ptrs.insert(p);
        } // guard dropped here and vector returned to pool
    }

    // The pool was seeded with 2 vecs; the implementation should only ever allocate
    // at most those two buffers and reuse them. We expect at most 2 unique pointers.
    assert!(
        ptrs.len() <= 2,
        "expected at most 2 unique buffers, got {}",
        ptrs.len()
    );
    assert!(
        pool.pool_len() == 2,
        "expected pool to have 2 vecs after returns, got {}",
        pool.pool_len()
    );
}

#[test]
fn test_take_return_changes_pool_len() {
    let mut pool: VecPool<u32> = VecPool::with_initial_amount(1, 8);
    assert_eq!(pool.pool_len(), 1);

    // take a guard, then drop it in its own scope
    {
        let _guard = pool.take_vec();
        // cannot inspect pool while guard borrowed mutably
    }

    // after guard dropped, pool should have one vec again
    assert_eq!(pool.pool_len(), 1);
}
