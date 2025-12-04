use std::{collections::VecDeque, ops::{Deref, DerefMut}};

/// A pool of reusable Vec<T> instances to minimize allocations.
/// When a Vec<T> is taken from the pool, it is wrapped in a VecPoolGuard,
/// which returns the Vec<T> to the pool when dropped.
#[derive(Default)]
pub struct VecPool<T> {
    pool: VecDeque<Vec<T>>,
    inner_capacity: usize,
}

/// A guard that returns the Vec<T> to the pool when dropped.
/// The Vec<T> can be accessed via Deref and DerefMut.
pub struct VecPoolGuard<'a, T> {
    vec: Vec<T>,
    pool: &'a mut VecPool<T>,
}

impl<T> VecPool<T> {
    /// Creates a new VecPool with the specified inner vector capacity.
    pub fn with_initial_amount(amount: usize, inner_capacity: usize) -> Self {
        let mut pool = VecDeque::with_capacity(amount);
        for _ in 0..amount {
            pool.push_back(Vec::with_capacity(inner_capacity));
        }
        VecPool {
            pool,
            inner_capacity,
        }
    }

    pub fn take_vec(&'_ mut self) -> VecPoolGuard<'_, T> {
        let vec: Vec<T> = self
            .pool
            .pop_front()
            .unwrap_or_else(|| Vec::with_capacity(self.inner_capacity));
        VecPoolGuard { vec, pool: self }
    }

    pub fn resize_all(&mut self, new_capacity: usize) {
        self.inner_capacity = new_capacity;
        for vec in &mut self.pool {
            vec.reserve(new_capacity.saturating_sub(vec.capacity()));
        }
    }

    pub fn return_vec(&mut self, mut vec: Vec<T>) {
        vec.clear();
        self.pool.push_back(vec);
    }
}

impl<T> Drop for VecPoolGuard<'_, T> {
    fn drop(&mut self) {
        self.pool.return_vec(std::mem::take(&mut self.vec));
    }
}

impl<T> Deref for VecPoolGuard<'_, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl<T> DerefMut for VecPoolGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}