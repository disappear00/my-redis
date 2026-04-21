use tokio::sync::{mpsc, oneshot};
use bytes::Bytes;
use mini_redis::client;

/// 管理任务可以使用该发送端将命令执行的结果传回给发出命令的任务
type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

#[tokio::main]
async fn main() {
    // 创建一个新通道，缓冲队列长度是 32
    let (tx, mut rx) = mpsc::channel(32);

    // 将消息通道接收者 rx 的所有权转移到管理任务中
    let manager = tokio::spawn(async move {
        // 建立到 redis 服务器的连接
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // 开始接收消息
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get { key, resp } => {
                    let res = client.get(&key).await;
                    // 忽略错误
                    let _ = resp.send(res);
                }
                Command::Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    // 忽略错误
                    let _ = resp.send(res);
                }
            }
        }
    });

    // 由于有两个任务，因此我们需要两个发送者
    let tx2 = tx.clone();

    // 生成两个任务，一个用于获取 key，一个用于设置 key
    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "hello".to_string(),
            resp: resp_tx,
        };

        // 发送 GET 请求
        tx.send(cmd).await.unwrap();

        // 等待回复
        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            resp: resp_tx,
        };

        // 发送 SET 请求
        tx2.send(cmd).await.unwrap();

        // 等待回复
        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    // 等待所有任务完成
    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
