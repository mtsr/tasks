extern crate threadpool;
extern crate deque;

use std::sync::mpsc::{ channel };
use std::sync::mpsc::{ Receiver };
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
            // finish own queue first
            while let Some(task) = self.worker.pop() {
                println!("thread {}: {:?}", self.num, task);
            };

            // only lock long enough to receive a job from shared queue
            let msg = {
                let lock = self.task_receiver.lock().unwrap();
                lock.recv()
            };

            match msg {
                Ok(task) => {
                    println!("thread {}: {:?}", self.num, task);
                }
                Err(_) => {
                    // no more tasks in the queue (sender dropped)
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

    // create some tasks
    for id in 0..10 {
        task_sender.send(Task { id: id }).ok().expect("Sending should be OK");
    }

    // temporary list of workers
    let mut workers = Vec::new();

    // create workers/stealers all at once
    // so that each thread gets all stealers
    for _ in 0..NUM_THREADS {
        // create worker/stealer deque
        let (worker, stealer) = deque_pool.deque();
        // and store stealer
        stealers.push(stealer);

        // add worker to temporary list
        workers.push(worker);
    }

    // launch one scheduler per thread
    for num in 0..NUM_THREADS {
        // create scheduler for thread
        let mut scheduler = Scheduler {
            num: num,
            // take some worker from temporary list
            // don't care about order
            worker: workers.pop().expect("One worker per thread"),
            // clone shared central queue
            task_receiver: task_receiver.clone(),
        };

        // launch scheduler
        threadpool.execute(move|| {
            scheduler.run();
        });
    }

    // drop temporary list of workers
    drop(workers);

    // for debugging purposes drop task_sender so
    // that threads end
    drop(task_sender);

}

#[test]
fn it_works() {
}
