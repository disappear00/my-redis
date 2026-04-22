//! # async fn 作为 Future (状态机)
//!
//! 本文件展示编译器如何将 `async fn main` 转换为状态机。
//! 当我们使用 `#[tokio::main]` 标注 `async fn main` 时，
//! 编译器会生成类似下面 MainFuture 的状态机代码。

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

// ==================== Delay Future (复用) ====================

struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        use std::thread;

        if Instant::now() >= self.when {
            println!("Hello world");
            Poll::Ready("done")
        } else {
            let waker = cx.waker().clone();
            let when = self.when;
            thread::spawn(move || {
                let now = Instant::now();
                if now < when {
                    thread::sleep(when - now);
                }
                waker.wake();
            });
            Poll::Pending
        }
    }
}

// ==================== 编译器生成的状态机 ====================

/// 这就是编译器将 `async fn main` 转换后的结果 —— 一个状态机！
///
/// `MainFuture` 包含了 Future 可能处于的状态:
/// - State0: 初始化状态（创建 Delay）
/// - State1: 等待 Delay 运行完成（future.await）
/// - Terminated: Future 已完成
enum MainFuture {
    /// 初始化，但永远不会被 poll
    State0,
    /// 等待 `Delay` 运行，例如 `future.await` 代码行
    State1(Delay),
    /// Future 执行完成
    Terminated,
}

impl Future for MainFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        use MainFuture::*;

        loop {
            match *self {
                State0 => {
                    // 在 State0 中创建 Delay 实例
                    let when = Instant::now() + Duration::from_millis(10);
                    let future = Delay { when };
                    // 将状态转移到 State1，携带 Delay Future
                    *self = State1(future);
                }
                State1(ref mut my_future) => {
                    // 在 State1 中 poll 内部的 Delay Future
                    match Pin::new(my_future).poll(cx) {
                        Poll::Ready(out) => {
                            // Delay 完成后验证返回值
                            assert_eq!(out, "done");
                            println!("MainFuture 状态机完成!");
                            *self = Terminated;
                            return Poll::Ready(());
                        }
                        Poll::Pending => {
                            // Delay 还没准备好，返回 Pending
                            // 执行器会在收到 wake 通知后再次 poll
                            return Poll::Pending;
                        }
                    }
                }
                Terminated => {
                    // Future 完成后不应再被 poll
                    panic!("future polled after completion")
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== async fn 状态机示例 ===\n");

    let main_future = MainFuture::State0;
    main_future.await;
}
