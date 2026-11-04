//! Exercises std's random seeding and joined threads with the new sysroot.
use std::collections::HashMap;
mod aros_random;
fn main() {
    use rand::{SeedableRng, RngCore};
    let mut direct = [0u8; 513];
    getrandom::fill(&mut direct).expect("getrandom source read");
    let mut uninitialized = [std::mem::MaybeUninit::<u8>::uninit(); 513];
    let initialized = getrandom::fill_uninit(&mut uninitialized).expect("uninitialized output");
    assert_eq!(initialized.len(), 513);
    let mut seeded = rand::rngs::StdRng::try_from_os_rng().expect("rand source seed");
    let _ = seeded.next_u64();
    let workers: Vec<_> = (0..4).map(|id| std::thread::spawn(move || {
        let mut values = HashMap::new();
        for key in 0..128 { values.insert(key, key * 2 + id); }
        assert_eq!(values.len(), 128);
        assert_eq!(values[&42], 84 + id);
        values.len()
    })).collect();
    for worker in workers { assert_eq!(worker.join().unwrap(), 128); }
    let start = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(start.elapsed() >= std::time::Duration::from_millis(50));
    println!("STD_ENTROPY_OK: getrandom 0.3.4, rand 0.9.5, fill_uninit, four HashMap workers, Instant and sleep");
}
