use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc,Mutex};
type Db = Arc<Mutex<HashMap<String,Bytes>>>;
type ShardedDb = Arc<Vec<Mutex<HashMap<String,Vec<u8>>>>>;

fn new_sharded_db(num_shards: usize) -> ShardedDb {
    let mut db = Vec::with_capacity(num_shards);
    for _ in 0..num_shards {
        db.push(Mutex::new(HashMap::new()));
    }
    Arc::new(db)
}

#[tokio::main]
async fn main() {
    // Bind the listener to the address
    // 监听指定地址，等待TCP连接进来
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db :Db = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let db = db.clone();
        tokio::spawn(async move {
            process3(socket,db).await;
        });
    }
}

async fn process(socket: TcpStream) {
    // `Connection` 对于 redis 的读写进行了抽象封装，因此我们读到的是一个一个数据帧frame(数据帧 = redis命令 + 数据)，而不是字节流
    // `Connection` 是在 mini-redis 中定义
    let mut connection = Connection::new(socket);
    if let Some(frame) = connection.read_frame().await.unwrap() {
        print!("GOT: {:?}", frame);
        // 回复一个错误
        let response = Frame::Error("unimplement".to_string());
        connection.write_frame(&response).await.unwrap();
    }
}

async fn process2(socket: TcpStream) {
    use mini_redis::Command::{self,Get,Set};
    use std::collections::HashMap;
// 使用Hashmap来存储redis的数据
    let mut db = HashMap::new();
    let mut connection = Connection::new(socket);
    // 使用read_frame方法从链接获取一个数据帧： 一个redis命令+相应的数据
    while let Some(frame) = 
    connection.read_frame().await.unwrap(){
        let response = match Command::from_frame(frame).unwrap(){
            Set(cmd) => {
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()){
                    // `Frame::Bulk` 期待数据的类型是 `Bytes`， 该类型会在后面章节讲解，
                    // 此时，你只要知道 `&Vec<u8>` 可以使用 `into()` 方法转换成 `Bytes` 类型                    
                    Frame::Bulk(value.clone().into())
                }else{
                    Frame::Null
                }
            }
            cmd => {
                panic!("unimplemented {:?}",cmd)
            }
        };
        connection.write_frame(&response).await.unwrap();
    }
}

async fn process3(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};

    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        connection.write_frame(&response).await.unwrap();
    }
}