async fn action(input: Option<i32>) 
-> Option<String> {
    // 若input(输入)是None,则返回None
    let i = match input {
        Some(input) => input,
        None => return None
    };
    
    // 处理输入值并返回结果
    Some(i.to_string())
}
#[tokio::main]
async fn main(){
    let (mut tx,mut rx) = tokio::sync::mpsc::channel(128);
    let mut done = false;
    let operation = action(None);
    tokio::pin!(operation);
    tokio::spawn(async move {
        let _ = tx.send(1).await;
        let _ = tx.send(3).await;
        let _ = tx.send(2).await;
    });

    loop {
        tokio::select! {
            res = &mut operation ,if !done => {
                done = true;
                if let Some(v) = res {
                    println!("Got result: {}", v);
                    return ;
                }
            }
            Some(v) = rx.recv() => {
                if v % 2 == 0{
                    // .set为pin上定义的方法
                    operation.set(action(Some(v)));
                    done = false;
                }
            }
        }
    }
}
