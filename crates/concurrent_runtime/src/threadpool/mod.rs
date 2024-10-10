use std::{fmt::Debug, thread};
use crossbeam::channel::{unbounded, Sender, Receiver};
use logger::{
    info,
    error,
};
use logger_proc_macro::*;

#[derive(Debug)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Message>,
}

#[derive(Debug)]
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

pub enum Message {
    NewJob(Job),
    Terminate,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    #[log(Trace)]
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = unbounded();
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, receiver.clone()));
        }

        ThreadPool { workers, sender }
    }

    #[log(Debug)]
    pub fn execute<F>(&self, f: F)
    where
        F: Fn() + Send + 'static,
    {
        let job = Box::new(f);
        let _ = self.sender.send(Message::NewJob(job));
    }

    #[log(Trace)]
    pub fn workers_count(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for ThreadPool {
    #[log(Debug)]
    fn drop(&mut self) {
        for _ in &self.workers {
            let _ = self.sender.send(Message::Terminate);
        }

        for worker in &mut self.workers {
            info!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

impl Worker {
    fn new(id: usize, receiver: Receiver<Message>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.recv();

            let message = match message {
                Ok(message) => message,
                Err(_) => {
                    error!("Error occurred while receiving message. Worker {}. Trying again...", id);
                    continue;
                }
            };

            match message {
                Message::NewJob(job) => {
                    info!("Worker {} got a job. Executing...", id);
                    job();
                }
                Message::Terminate => {
                    info!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
