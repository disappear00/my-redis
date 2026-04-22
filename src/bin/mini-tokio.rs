//! # MiniTokio - 手写迷你 Tokio 执行器
//!
//! 本文件完整实现了一个简易版的 Tokio 执行器，包含:
//! 1. 任务队列管理 (消息通道)
//! 2. Waker 支持 (通过 futures::task::ArcWake)
//! 3. 基于通知的任务调度机制

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

use futures::task::{self, ArcWake};

// ==================== Delay Future ====================

struct Delay {
    when: Instant,
    /// 用于记录是否已经生成线程，以及存储当前的 waker
    waker: Option<Arc<Mutex<Waker>>>,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        // 若这是 Future 第一次被调用，需要先生成一个计时器线程
        // 若不是第一次调用（线程已在运行），要确保已存储的 Waker 跟当前任务的 Waker 匹配
        if let Some(waker) = &self.waker {
            let mut waker = waker.lock().unwrap();

            // 检查之前存储的 waker 是否跟当前任务的 waker 相匹配
            // Future 可能会在两次 poll 之间被转移到另一个任务中，
            // 因此 Context 包含的 waker 可能不同于之前存储的 waker
            if !waker.will_wake(cx.waker()) {
                *waker = cx.waker().clone();
            }
        } else {
            let when = self.when;
            let waker = Arc::new(Mutex::new(cx.waker().clone()));
            self.waker = Some(waker.clone());

            // 第一次调用 poll，生成计时器线程
            thread::spawn(move || {
                let now = Instant::now();

                if now < when {
                    thread::sleep(when - now);
                }

                // 计时结束，通过调用 waker 来通知执行器
                let waker = waker.lock().unwrap();
                waker.wake_by_ref();
            });
        }

        // 检查 delay 是否已经完成
        if Instant::now() >= self.when {
            println!("Hello world from MiniTokio!");
            Poll::Ready("done")
        } else {
            // 计时尚未结束，返回 Pending
            // 必须确保当时间到时 waker 会被调用，否则任务将永远挂起
            Poll::Pending
        }
    }
}

// ==================== MiniTokio 执行器 ====================

struct MiniTokio {
    scheduled: mpsc::Receiver<Arc<Task>>,
    sender: mpsc::Sender<Arc<Task>>,
}

struct Task {
    /// Mutex 是为了让 Task 实现 Sync 特征
    /// 保证同一时间只有一个线程可以访问 Future
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    executor: mpsc::Sender<Arc<Task>>,
}

impl Task {
    fn schedule(self: &Arc<Self>) {
        let _ = self.executor.send(self.clone());
    }

    fn poll(self: Arc<Self>) {
        // 基于 Task 实例创建一个 waker, 它使用了 ArcWake 特征
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        // 没有其他线程竞争锁时，获取目标 future
        let mut future = self.future.try_lock().unwrap();

        // 对 future 进行 poll
        let _ = future.as_mut().poll(&mut cx);
    }

    /// 使用给定的 future 来生成新任务
    fn spawn<F>(future: F, sender: &mpsc::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
    }
}

/// 为 Task 实现 ArcWake 特征
/// 这样就可以将我们的 Task 转变成一个 Waker
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.schedule();
    }
}

impl MiniTokio {
    fn new() -> MiniTokio {
        let (sender, scheduled) = mpsc::channel();
        MiniTokio { scheduled, sender }
    }

    /// 生成一个 Future 并放入 mini-tokio 的任务队列中
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender);
    }

    /// 从消息通道中接收任务，然后通过 poll 来执行
    fn run(&self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }
}

// ==================== Main ====================

fn main() {
    println!("=== MiniTokio 执行器示例 ===\n");

    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async {
        println!("任务开始执行...");
        let when = Instant::now() + Duration::from_millis(100);
        let future = Delay {
            when,
            waker: None,
        };

        let out = future.await;
        assert_eq!(out, "done");
        println!("任务完成! 返回值: {}", out);
    });

    println!("启动 MiniTokio 运行循环...");
    mini_tokio.run();
}
