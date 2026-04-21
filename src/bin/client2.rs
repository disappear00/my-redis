use tokio::sync::{mpsc,oneshot};
use bytes::Bytes;
use mini_redis::client;

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[derive(Debug)]
enum Command{
    Get{
        key: String,
        resp:Responder<Option<Bytes>>,
    },
    Set{
        key: String,
        val: Bytes,
        resp:Responder<()>
    },
}

#[tokio::main]
async fn main(){
    // 创建一个新通道，缓冲队列长度为32
    let (tx,mut rx) = mpsc::channel(32);
    //将消息通道接收者rx的所有权交给manage来管理
    let manager = tokio::spawn(async move{
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // 开始接受信息
        while let Some(cmd) = rx.recv().await{
            match cmd {
                Command::Get{key,resp} => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                }
                Command::Set{key,val,resp} => {
                    let res = client.set(&key, val).await;
                    let _ = resp.send(res);
                }
            }
        }
    });

    let tx2 = tx.clone();

    // 任务1
    let t1 = tokio::spawn(async move{
        let (resp_tx,resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "hello".to_string(),
            resp: resp_tx,
        };
        tx.send(cmd).await.unwrap();
        let res = resp_rx.await;
        print!("GOT={:?}",res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx,resp_rx) = oneshot::channel();
        let cmd = Command::Set { key: "foo".to_string(), 
        val: "bar".into(), 
        resp: resp_tx 
        };
        tx2.send(cmd).await.unwrap();
        // 等待回复
        let res = resp_rx.await;
        println!("GOT = {:?}",res);
    });

    // 等待所有任务完成
    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();

}