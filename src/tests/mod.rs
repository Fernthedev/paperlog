// Tests that only exercise `paper2`'s public API live in `tests/` as
// integration tests instead. This one stays here because it reaches into
// crate-private items (`SemaphoreLite` isn't public) that an integration
// test can't see.
mod semaphore_lite;
