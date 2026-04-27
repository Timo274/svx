//! File-system helpers for the CLI.
//!
//! We go out of our way to make sure identity material lands on disk
//! with permissions appropriate for secret keys (mode 0600 on Unix),
//! and that we never clobber existing files.

use std::io::Write;
use std::path::Path;

pub fn write_identity_file(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)?;
        f.flush()?;
        f.sync_all()?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let mut f = std::fs::File::create_new(path)?;
        f.write_all(bytes)?;
        f.flush()?;
        f.sync_all()?;
        Ok(())
    }
}
