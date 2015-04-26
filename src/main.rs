extern crate threadpool;
extern crate deque;
extern crate coroutine;

use std::sync::mpsc::{ channel };
use std::sync::mpsc::{ Receiver };
use std::sync::{ Arc, Mutex };

use threadpool::{ ScopedPool };
use deque::{ BufferPool };

use coroutine::{ Coroutine, sched };
use coroutine::coroutine::State;

const NUM_THREADS: u32 = 2;

struct Scheduler {
    num: u32,
    ready_queue: deque::Worker<Task>,
    task_receiver: Arc<Mutex<Receiver<Task>>>,
}

impl Scheduler {
    fn new(num: u32, ready_queue: deque::Worker<Task>, task_receiver: Arc<Mutex<Receiver<Task>>>) -> Scheduler {
        Scheduler {
            num: num,
            ready_queue: ready_queue,
            task_receiver: task_receiver,
        }
    }

    fn run(&mut self) {

        loop {
            // finish own queue first
            while let Some(task) = self.ready_queue.pop() {
                self.spawn(task);
            };

            // only lock long enough to receive a job from shared queue
            let msg = {
                let lock = self.task_receiver.lock().unwrap();
                lock.recv()
            };

            match msg {
                Ok(task) => {
                    self.spawn(task);
                }
                Err(_) => {
                    // no more tasks in the queue (sender dropped)
                    break;
                }
            }
        }
    }

    fn spawn(&mut self, task: Task) {
        let num = self.num;
        // TODO pool/reuse Coroutines (although stacks are already pooled)
        let handle = Coroutine::spawn(move || {
            sched();
            println!("thread {}: {:?}", num, task);
        });

        // println!("{:?}", handle.state());

        // match handle.state() {
        //     State::Normal => {

        //     }
        //     State::Suspended => {

        //     }
        //     State::Running => {
                
        //     }
        //     State::Finished => {
                
        //     }
        //     State::Panicked => {
                
        //     }
        // }

        handle.join().ok().unwrap();
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
        let mut scheduler = Scheduler::new(
            num,
            // take some worker from temporary list
            // don't care about order
            workers.pop().expect("One worker per thread"),
            // clone shared central queue
            task_receiver.clone(),
        );

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
