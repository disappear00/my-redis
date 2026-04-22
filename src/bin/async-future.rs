//! # Future 基础与 Waker 示例
//!
//! 本文件展示了:
//! 1. `async fn` 返回 Future，惰性执行直到 `.await`
//! 2. 手动实现 Future 特征
//! 3. 使用 Waker 发送 wake 通知

// ==================== async fn 基础示例 ====================

use tokio::net::TcpStream;

/// 异步函数 - 演示 async fn 返回的是 Future
async fn my_async_fn() {
    println!("hello from async");
    // 通过 .await 创建 socket 连接
    let _socket = TcpStream::connect("127.0.0.1:3000").await.unwrap();
    println!("async TCP operation complete");
}

/// 展示 Future 的惰性执行特性
/// my_async_fn() 调用本身不会产生任何效果，必须 .await 才会真正执行
#[tokio::main]
async fn main() {
    let what_is_this = my_async_fn();
    // 上面的调用不会产生任何效果

    // ... 执行一些其它代码
    println!("在 .await 之前做些其他事情...");

    // 直到 .await 后，异步函数才被真正执行
    // what_is_this.await;

    println!("\n--- 接下来演示手动实现 Future ---\n");
    run_delay_future();
}

// ==================== 手动实现 Future 示例 ====================

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use std::thread;

/// 一个简单的延迟 Future
/// 功能: 等待特定时间点到来 -> 打印文本 -> 生成字符串
struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        if Instant::now() >= self.when {
            // 时间到了，Future 可以结束
            println!("Hello world");
            // Future 执行完毕并返回 "done" 字符串
            Poll::Ready("done")
        } else {
            // 为当前任务克隆一个 waker 的句柄
            let waker = cx.waker().clone();
            let when = self.when;

            // 生成一个计时器线程来模拟阻塞资源
            // 一旦计时结束(资源准备好)，通过 waker.wake() 通知执行器
            thread::spawn(move || {
                let now = Instant::now();

                if now < when {
                    thread::sleep(when - now);
                }

                // 资源准备好后，通知执行器该任务可以继续执行
                waker.wake();
            });

            Poll::Pending
        }
    }
}

fn run_delay_future() {
    #[tokio::main]
    async fn inner() {
        let when = Instant::now() + Duration::from_millis(10);
        let future = Delay { when };

        // 运行并等待 Future 的完成
        let out = future.await;

        // 判断 Future 返回的字符串是否是 "done"
        assert_eq!(out, "done");
        println!("Delay Future 完成, 返回值: {}", out);
    }

    inner();
}
