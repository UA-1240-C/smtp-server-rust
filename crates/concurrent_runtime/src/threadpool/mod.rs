use std::thread;
use crossbeam::channel::{unbounded, Sender, Receiver};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Message>,
}

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
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = unbounded();
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, receiver.clone()));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: Fn() + Send + 'static
    {
        let job = Box::new(f);
        let _ = self.sender.send(Message::NewJob(job));
    }

    pub fn workers_count(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            let _ = self.sender.send(Message::Terminate);
        }

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

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
                    println!("
                        Error occurred while receiving message. \
                        Worker {}. \
                        Trying again...", id);
                    continue;
                }
            };

            match message {
                Message::NewJob(job) => {
                    println!("Worker {} got a job. Executing...", id);
                    job();
                }
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);
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
