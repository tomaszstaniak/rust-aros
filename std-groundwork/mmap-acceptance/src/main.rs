use memmap2::{Mmap, MmapOptions};
use read_fonts::{FontRef, TableProvider};
use std::{
    error::Error,
    fs::File,
    io::{Seek, SeekFrom},
};
fn gate() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or("font path required")?;
    println!(
        "START mmap Fontique-path memmap2=0.9.11 read-fonts=0.41.0 run={}",
        env!("AROS_MMAP_RUN_ID")
    );
    let bytes = std::fs::read(&path)?;
    let from_read = FontRef::from_index(&bytes, 0)?;
    let glyphs = from_read.maxp()?.num_glyphs();
    assert!(glyphs > 0);
    assert!(from_read.name()?.name_record().len() > 0);
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    println!(
        "OBSERVE sysconf selector={} page={} font_bytes={} glyphs={}",
        libc::_SC_PAGESIZE,
        page,
        bytes.len(),
        glyphs
    );
    let mut file = File::open(&path)?;
    file.seek(SeekFrom::Start(17))?;
    for index in 0..128 {
        let map = unsafe { Mmap::map(&file) }
            .map_err(|e| format!("mapping iteration {index} failed: {e}"))?;
        assert_eq!(&map[..], &bytes);
        let parsed = FontRef::from_index(&map, 0)?;
        assert_eq!(parsed.maxp()?.num_glyphs(), glyphs);
        assert!(parsed.name()?.name_record().len() > 0);
        drop(map);
    }
    assert_eq!(file.stream_position()?, 17);
    println!("PASS 128 repeated file-map-font-parse-drop cycles; bytes match fs::read");
    assert_eq!(
        page, 4096,
        "mapping adapter granule must be reported to memmap2"
    );
    let map = unsafe { MmapOptions::new().offset(3).len(31).map(&file) }?;
    assert_eq!(&map[..], &bytes[3..34]);
    drop(map);
    println!("PASS unaligned view offset and base reconstruction on drop");
    let mut live = Vec::new();
    for _ in 0..64 {
        live.push(unsafe { Mmap::map(&file) }?);
    }
    let error = unsafe { Mmap::map(&file) }.unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::ENOMEM));
    drop(live);
    let map = unsafe { Mmap::map(&file) }?;
    drop(file);
    assert_eq!(&map[..], &bytes);
    drop(map);
    println!("PASS 64 live maps; bounded capacity error; release/reuse and closed file lifetime");
    println!("PASS mmap font acceptance run={}", env!("AROS_MMAP_RUN_ID"));
    Ok(())
}
fn main() {
    if let Err(error) = gate() {
        println!("FAIL mmap font acceptance: {error}");
        std::process::exit(1);
    }
}
