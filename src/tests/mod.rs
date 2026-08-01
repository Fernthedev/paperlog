// Tests that only exercise `paper2`'s public API live in `tests/` as
// integration tests instead. These two stay here because they reach into
// crate-private items (`SemaphoreLite` isn't public, `VecPool::pool_len` is
// `pub(crate)`) that an integration test can't see.
mod semaphore_lite;
mod vec_pool;
