fn main() -> std::io::Result<()> {
    let path = std::path::Path::new("RAM:zed-filetime-probe");
    let file = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    drop(file);
    let result = (|| -> std::io::Result<()> {
        let name = c"RAM:zed-filetime-probe";
        assert_eq!(unsafe { libc::access(name.as_ptr(), libc::F_OK | libc::R_OK) }, 0);
        let missing = c"RAM:zed-filetime-missing";
        assert_ne!(unsafe { libc::access(missing.as_ptr(), libc::F_OK) }, 0);
        let time = filetime::FileTime::from_unix_time(1_700_000_004, 0);
        filetime::set_file_mtime(path, time)?;
        assert_eq!(filetime::FileTime::from_last_modification_time(&path.metadata()?), time);
        assert_eq!(filetime::set_symlink_file_times(path, time, time).unwrap_err().kind(), std::io::ErrorKind::Unsupported);
        Ok(())
    })();
    std::fs::remove_file(path)?;
    result?;
    println!("FILETIME_ACCESS_AROS_OK");
    Ok(())
}
