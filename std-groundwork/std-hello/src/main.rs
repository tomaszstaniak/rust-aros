//! The test from the forum discussion: cargo add serde anyhow, build, run.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Machine { name: String, cpu: String, ram_mb: u32, tags: Vec<String> }

fn main() -> Result<()> {
    let m = Machine { name: "AROS One".into(), cpu: "x86_64".into(), ram_mb: 2048, tags: vec!["qemu".into(), "rust".into()] };
    let json = serde_json::to_string_pretty(&m).context("serialising")?;
    println!("{json}");
    std::fs::write("RAM:machine.json", &json).context("writing RAM:machine.json")?;
    let back: Machine = serde_json::from_str(&std::fs::read_to_string("RAM:machine.json")?).context("parsing it back")?;
    println!("round trip {}", if back == m { "ok" } else { "MISMATCH" });
    std::fs::remove_file("RAM:machine.json")?;
    let missing: Result<String> = std::fs::read_to_string("RAM:nope.json").context("reading a file that does not exist");
    if let Err(e) = missing { println!("anyhow error chain: {e:#}"); }
    Ok(())
}
