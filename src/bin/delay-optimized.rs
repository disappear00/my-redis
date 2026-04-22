//! # Delay 优化版 - 处理跨任务迁移场景
//!
//! 本文件展示了 Future 在跨任务迁移时的 Waker 更新问题。
//! Rust 的异步模型允许一个 Future 在执行过程中可以跨任务迁移。
//! 关键点：每次 poll 调用时都必须检查并更新 Waker。

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};
use futures::future::poll_fn;

/// 优化版 Delay Future
///
/// 改进点:
/// 1. 记录是否已生成线程，避免每次 poll 都创建新线程
/// 2. 正确处理 Waker 更新，支持跨任务迁移
struct Delay {
    when: Instant,
    /// 用于说明是否已经生成一个线程
    /// Some 代表已经生成，None 代表还没有
    waker: Option<Arc<Mutex<Waker>>>,
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // 若这是 Future 第一次被调用，那么需要先生成一个计时器线程。
        // 若不是第一次调用（该线程已在运行），那要确保已存储的 `Waker` 跟当前任务的 `waker` 匹配
        if let Some(waker) = &self.waker {
            let mut waker = waker.lock().unwrap();

            // 检查之前存储的 `waker` 是否跟当前任务的 `waker` 相匹配.
            // 这是必要的，原因是 `Delay Future` 的实例可能会在两次 `poll` 之间被转移到另一个任务中，
            // 存储的 waker 被该任务进行了更新。
            // 这种情况一旦发生，`Context` 包含的 `waker` 将不同于存储的 `waker`。
            if !waker.will_wake(cx.waker()) {
                *waker = cx.waker().clone();
            }
        } else {
            let when = self.when;
            let waker = Arc::new(Mutex::new(cx.waker().clone()));
            self.waker = Some(waker.clone());

            // 第一次调用 `poll`，生成计时器线程
            thread::spawn(move || {
                let now = Instant::now();

                if now < when {
                    thread::sleep(when - now);
                }

                // 计时结束，通过调用 `waker` 来通知执行器
                let waker = waker.lock().unwrap();
                waker.wake_by_ref();
            });
        }

        // 一旦 waker 被存储且计时器线程已经开始，检查 delay 是否已经完成
        if Instant::now() >= self.when {
            println!("Delay 完成! (优化版)");
            Poll::Ready(())
        } else {
            // 计时尚未结束，Future 还未完成
            //
            // `Future` 特征要求当 `Pending` 被返回时，必须确保当资源准备好时
            // 会调用 `waker` 以通知执行器。在我们的例子中，会通过生成的计时线程来保证
            //
            // 如果忘记调用 waker，该任务将被永远挂起，无法再执行
            Poll::Pending
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Delay 优化版 (跨任务迁移) ===\n");

    let when = Instant::now() + Duration::from_millis(100);
    let mut delay = Some(Delay { when, waker: None });

    // poll_fn 使用闭包创建一个 Future
    // 这里模拟 Future 跨任务迁移的场景：
    // 1. 先在当前任务中 poll 一次 Delay
    // 2. 然后将 Delay 移动到一个新的 tokio 任务中继续 await
    poll_fn(move |cx| {
        let mut delay = delay.take().unwrap();
        let res = Pin::new(&mut delay).poll(cx);
        assert!(res.is_pending(), "第一次 poll 应该返回 Pending");

        println!("第一次 poll 完成，将 Delay 转移到新任务...");

        // 将 Delay 实例发送到一个新的任务中
        // 此时 Delay 会被不同的 Waker 进行 poll！
        tokio::spawn(async move {
            println!("新任务开始 await Delay...");
            delay.await;
            println!("新任务中的 Delay await 完成!");
        });

        Poll::Ready(())
    }).await;

    // 等待新任务完成
    tokio::time::sleep(Duration::from_millis(200)).await;

    println!("\n演示完成！");
}
