use tokio::io::{self, AsyncWriteExt};
use tokio::fs::File;

#[tokio::main]
async fn main() -> io::Result<()>{
    let mut file = File::create("foo2.txt").await?;
    let n = file.write(b"some bytes").await?;
    println!("Wrote the first {} bytes of 'some bytes'.", n);
    Ok(())
}
