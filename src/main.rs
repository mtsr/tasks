extern crate threadpool;
extern crate deque;

use std::sync::mpsc::{ channel };
use std::sync::mpsc::{ Sender, Receiver };
use std::sync::{ Arc, Mutex };

use threadpool::{ ScopedPool };
use deque::BufferPool;

const NUM_THREADS: u32 = 2;

struct Scheduler {
    num: u32,
    worker: deque::Worker<Task>,
    task_receiver: Arc<Mutex<Receiver<Task>>>,
}

impl Scheduler {
    fn run(&mut self) {

        loop {
            while let Some(task) = self.worker.pop() {
                println!("thread {}: {:?}", self.num, task);
            };

            // only lock long enough to receive a job
            let msg = {
                let lock = self.task_receiver.lock().unwrap();
                lock.recv()
            };

            match msg {
                Ok(task) => {
                    println!("thread {}: {:?}", self.num, task);
                }
                Err(err) => {
                    break;
                }
            }
        }
    }
}

#[derive(Debug)]
struct Task {
    id: u32,
}

fn main() {
    // debugger task_id
    let mut task_id: u32 = 0;

    // create pool with 1 thread per core
    let threadpool = ScopedPool::new(NUM_THREADS);

    // shared bufferpool for worker/stealer deques
    let deque_pool: deque::BufferPool<Task> = deque::BufferPool::new();

    // store stealers for cloning for each scheduler
    let mut stealers = Vec::new();

    // channel for mpmc queue
    let (task_sender, task_receiver) = channel();
    // make Send + Sync receiver
    let task_receiver = Arc::new(Mutex::new(task_receiver));

    for num in 0..NUM_THREADS {
        let (mut worker, mut stealer) = deque_pool.deque();
        stealers.push(stealer);

        task_sender.send(Task { id: task_id });
        task_id += 1;

        let mut scheduler = Scheduler {
            num: num,
            worker: worker,
            task_receiver: task_receiver.clone(),
        };

        threadpool.execute(move|| {
            scheduler.run();
        });
    }

    // for debugging purposes drop task_sender so
    // that threads end
    drop(task_sender);

}

#[test]
fn it_works() {
}
