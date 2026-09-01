#[tokio::main]
async fn main() {
    println!("tokio on AROS");
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    tokio::spawn(async move { for i in 0..3 { tx.send(i).await.unwrap(); } });
    let mut got = Vec::new();
    while let Some(v) = rx.recv().await { got.push(v); }
    println!("  channel: {got:?}");
    let t0 = std::time::Instant::now();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!("  timer slept {} ms", t0.elapsed().as_millis());
    println!("done");
}
