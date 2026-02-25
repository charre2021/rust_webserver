use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(job) => {
                        println!("Worker {id} got a job; executing.");

                        job();
                    }
                    Err(_) => {
                        println!("Worker {id} disconnected; shutting down.");
                        break;
                    }
                }
            }
        });

        Worker { id, thread }
    }
}

#[derive(Debug)]
pub enum PoolCreationErrorType {
    OverSubscribed(String),
    UnderSubscribed(String),
}

#[derive(Debug)]
pub struct PoolCreationError {
    pub error_type: PoolCreationErrorType,
}

impl PoolCreationError {
    fn new(error_type: PoolCreationErrorType) -> Self {
        PoolCreationError { error_type }
    }
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn build(size: i32) -> Result<ThreadPool, PoolCreationError> {
        if size < 0 {
            Err(PoolCreationError::new(
                PoolCreationErrorType::UnderSubscribed(String::from(
                    "Too few threads in the pool.",
                )),
            ))
        } else if size > 4 {
            Err(PoolCreationError::new(
                PoolCreationErrorType::OverSubscribed(String::from(
                    "Too many threads in the pool.",
                )),
            ))
        } else {
            let unsigned_size = size as usize;
            let (sender, receiver) = mpsc::channel();
            let receiver = Arc::new(Mutex::new(receiver));
            let mut workers = Vec::with_capacity(unsigned_size);
            for id in 0..unsigned_size {
                workers.push(Worker::new(id, Arc::clone(&receiver)));
            }

            Ok(ThreadPool {
                workers,
                sender: Some(sender),
            })
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in self.workers.drain(..) {
            println!("Shutting down worker {}", worker.id);

            worker.thread.join().unwrap();
        }
    }
}
