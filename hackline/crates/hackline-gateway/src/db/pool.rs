//! r2d2 pool setup. Opens SQLite with WAL mode and the foreign-key
//! pragma. Sized conservatively against tokio's blocking-thread pool.
