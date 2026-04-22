use tokio::net::TcpListener;
use tokio::sync::oneshot;
use std::io;
use tokio::net::TcpStream;
#[tokio::main]
async fn main() -> io::Result<()> {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        tx.send(()).unwrap();
    });

    let mut listener = TcpListener::bind("localhost:3465").await?;

    tokio::select! {
        _ = async {
            loop {
                let (socket, _) = listener.accept().await?;
                tokio::spawn(async move { process(socket) });
            }

            // 给予 Rust 类型暗示
            Ok::<_, io::Error>(())
        } => {}
        _ = rx => {
            println!("terminating accept loop");
        }
    }

    Ok(())
}

async fn process(socket: TcpStream) {
    // TODO: 实现TCP连接处理逻辑
    println!("process");
}