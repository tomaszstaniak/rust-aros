use std::{fs, io};
fn main() -> io::Result<()> {
    let dir = tempfile::Builder::new().prefix("zed-rustix-").tempdir_in("RAM:")?;
    let path = dir.path().join("spool");
    fs::write(&path, b"temporary bytes")?;
    assert_eq!(fs::read(&path)?, b"temporary bytes");
    let root = dir.path().to_owned();
    dir.close()?;
    assert!(!root.try_exists()?);
    println!("ABIV1_TEMPDIR_OK");
    Ok(())
}
