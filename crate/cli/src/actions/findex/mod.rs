pub mod insert_or_delete;
pub mod parameters;
pub mod search;
/// Maximum number of concurrent network calls allowed per CLI findex-command invocation.
///
/// Each network call opens a socket which consumes a file descriptor. While the system's file descriptor
/// limit can be configured (via `ulimit -n` or `/etc/security/limits.conf`), we enforce this fixed limit
/// to avoid OS-level file descriptor exhaustion.
pub(crate) const MAX_PERMITS: usize = 256;
