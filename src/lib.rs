use std::{thread, sync::{mpsc::{self, Receiver}, Mutex, Arc}};

enum Message {
   NewJob(Job),
   Terminate
}

struct Worker {
   id: usize,
   thread: Option<thread::JoinHandle<()>>
}

impl Worker {
   fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Worker {
      let thread = thread::spawn(move || loop {
         let message = receiver
            .lock()
            .unwrap()
            .recv()
            .unwrap();
         match message {
            Message::NewJob(job) => {
               // println!("Worker {id} got a job and is executing");
               job();
               // println!("Worker {id} finished job execution");
            }
            Message::Terminate => { break; }
         }

      });
      
      return Worker {id, thread: Some(thread)};
   }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
   workers: Vec<Worker>,
   sender: mpsc::Sender<Message>,
}

impl ThreadPool {
   pub fn new(size: usize) -> Self {
      assert!(size > 0);
      
      let (sender, receiver) = 
         mpsc::channel();
      let receiver = Arc::new(Mutex::new(receiver));
      let mut threads: Vec<Worker> = Vec::with_capacity(size);
      
      for i in 0..size {
         // receiver should be shared by all threads and be mutable
         // can get this behavior through thread-safe (mutex) smart pointers
         let worker = Worker::new(i, Arc::clone(&receiver));
         threads.push(worker);
      }
      
      return ThreadPool{workers: threads, sender: sender};
   }
   
   pub fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static {
      // f is a closure (anon function)
      // can use a channel to send the closure to one of the workers
      let job = Box::new(f);
      self.sender.send(Message::NewJob(job)).unwrap();

   }
}

impl Drop for ThreadPool {
   fn drop(&mut self) {
      for _ in &self.workers {
         self.sender.send(Message::Terminate).unwrap();
      }

      for worker in &mut self.workers {
         // println!("Shutting down worker {}", worker.id);
         if let Some(thread) = worker.thread.take() {
            thread.join().unwrap();
         }
      }
   }
}