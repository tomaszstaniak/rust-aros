use std::{fs::File, io::IsTerminal};
use terminal_size::{Height, Width, terminal_size, terminal_size_of};

fn main() {
    let input = std::io::stdin();
    assert!(input.is_terminal(), "run this probe from an interactive Shell");
    assert_eq!(terminal_size_of(&File::open("NIL:").unwrap()), None);
    let size = terminal_size_of(&input);
    if let Some((Width(columns), Height(rows))) = size {
        assert!(columns > 0 && rows > 0);
    }
    println!("stdin={size:?}, detected={:?}; NIL=None", terminal_size());
    println!("ABIV1_TERMINAL_SIZE_PROBE_OK");
}
