use std::{ops::Index, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use futures::{
    future::BoxFuture,
    task::{Context, Poll},
    Future
};
use crossbeam::{epoch::{pin, Atomic}, queue::SegQueue};
mod threadpool;

use logger::info;
use logger_proc_macro::*;

type Task = BoxFuture<'static, ()>;
type GlobalTaskQueue = SegQueue<Task>;

#[derive(Debug)]
pub struct Executor {
    global_queue: Arc<GlobalTaskQueue>,
    termination_flag: Arc<AtomicBool>,
}

impl Executor {
    #[log(Trace)]
    fn new(global_queue: Arc<GlobalTaskQueue>) -> Self {
        Executor {
            global_queue,
            termination_flag: Arc::new(AtomicBool::new(false)),
        }
    }
    
    #[log(Trace)]
    fn run(&mut self) {
        loop {
            if !self.termination_flag.load(Ordering::Relaxed) {
                if let Some(mut task) = self.global_queue.pop() {
                    let waker = futures::task::noop_waker_ref();
                    let mut context = Context::from_waker(waker);

                    match task.as_mut().poll(&mut context) {
                        Poll::Ready(_) => info!("Async coroutine finished"),
                        Poll::Pending => self.global_queue.push(task),
                    }
                }
            }
        }
    }

    #[log(Trace)]
    fn stop(&mut self) {
        self.termination_flag.store(true, Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct ExecutorManager {
    executors: Vec<Arc<Atomic<Executor>>>,
    global_async_queue: Arc<GlobalTaskQueue>,
}

impl ExecutorManager {
    #[log(Trace)]
    fn new() -> Self {
        ExecutorManager {
            executors: Vec::new(),
            global_async_queue: Arc::new(SegQueue::new()),
        }
    }

    #[log(Trace)]
    fn create_executor(&mut self) -> Arc<Atomic<Executor>> {
        let executor = Arc::new(Atomic::new(Executor::new(
            self.global_async_queue.clone()
        )));

        self.executors.push(executor.clone());
        executor
    }

    #[log(Debug)]
    fn create_async_task(&self, task: Task) {
        self.global_async_queue.push(task);
    }
    
    #[log(Trace)]
    fn stop(&mut self) {
        for executor in self.executors.clone() {
            let guard = pin();
            let mut executor = executor.load(Ordering::Relaxed, &guard);
            unsafe { executor.deref_mut().stop() };
        }
    }
}

impl Index<usize> for ExecutorManager {
    type Output = Arc<Atomic<Executor>>;
    
    #[log(Trace)]
    fn index(&self, index: usize) -> &Arc<Atomic<Executor>> {
        self.executors
            .get(index)
            .expect("Index out of bounds")
    }
}

#[derive(Debug)]
pub struct ConcurrentRuntime {
    executors_manager: ExecutorManager,
    threadpool: threadpool::ThreadPool,
}

impl ConcurrentRuntime {
    #[log(Trace)]
    pub fn new(num_threads: usize) -> Self {
        let executors_manager = ExecutorManager::new();
        let threadpool = threadpool::ThreadPool::new(num_threads);

        Self {
            executors_manager,
            threadpool,
        }
    }

    #[log(Trace)]
    pub fn start(&mut self) {
        for _ in 0..self.threadpool.workers_count() {
            let executor = self.executors_manager.create_executor();
            let executor_clone = executor.clone();
            
            self.threadpool.execute(move || {
                let guard = pin();
                unsafe {
                    executor_clone.load(Ordering::Relaxed, &guard).deref_mut().run()
                }
            });
        }
    }

    #[log(Debug)]
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        let task: Task = Box::pin(future);
        self.executors_manager.create_async_task(task);
    }

    #[log(Trace)]
    pub fn stop(&mut self) {
        self.executors_manager.stop();
    }
}
