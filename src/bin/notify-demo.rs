//! # Notify 示例
//!
//! 本文件展示使用 `tokio::sync::Notify` 来实现任务通知机制。
//! Notify 提供了基础的任务通知机制，它会自动处理 Waker 的细节，
//! 包括确保两次 Waker 的匹配问题。

use tokio::sync::Notify;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;

/// 使用 Notify 实现的延迟函数
///
/// 对比手动的 Waker 实现，Notify 更加简洁且安全：
/// - 不需要手动处理 Waker 的更新和匹配
/// - 不需要自己管理线程和唤醒逻辑
/// - 自动处理跨任务迁移时的 Waker 问题
async fn delay(dur: Duration) {
    let when = Instant::now() + dur;
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    // 在独立线程中等待指定时间，然后发出通知
    thread::spawn(move || {
        let now = Instant::now();

        if now < when {
            let sleep_duration = when - now;
            println!("后台线程开始睡眠 {:?}...", sleep_duration);
            thread::sleep(sleep_duration);
        }

        println!("后台线程发出 notify_one() 通知");
        notify2.notify_one();
    });

    println!("主任务等待通知...");
    // 阻塞直到收到通知
    notify.notified().await;
    println!("主任务收到通知，继续执行!");
}

#[tokio::main]
async fn main() {
    println!("=== Notify 示例 ===\n");

    println!("开始延迟 200ms...");
    delay(Duration::from_millis(200)).await;

    println!("\n--- 再演示一次 ---\n");
    println!("开始延迟 100ms...");
    delay(Duration::from_millis(100)).await;

    println!("\nNotify 演示完成!");
}
