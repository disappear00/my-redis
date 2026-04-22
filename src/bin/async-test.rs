use tokio::net::TcpStream;

async fn my_async_fn() {
    println!("hello from async");
    // 通过 .await 创建 socket 连接
    let _socket = TcpStream::connect("127.0.0.1:3000").await.unwrap();
    println!("async TCP operation complete");
    // 关闭socket
}

#[tokio::main]
async fn main() {
    let what_is_this = my_async_fn();
    // ...执行一些其他代码
    what_is_this.await;
}