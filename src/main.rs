extern crate threadpool;
extern crate deque;
extern crate coroutine;

use std::sync::mpsc::{ Receiver, channel };
use std::sync::{ Arc, Mutex };

use threadpool::{ ScopedPool };

use coroutine::{ Coroutine, sched };
use coroutine::coroutine::State;

const NUM_THREADS: u32 = 2;

struct Scheduler {
    num: u32,
    task_receiver: Arc<Mutex<Receiver<Task>>>,
}

impl Scheduler {
    fn new(num: u32, task_receiver: Arc<Mutex<Receiver<Task>>>) -> Scheduler {
        Scheduler {
            num: num,
            task_receiver: task_receiver,
        }
    }

    fn run(&mut self) {

        loop {
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

    // channel for mpmc queue
    let (task_sender, task_receiver) = channel();
    // make Send + Sync receiver
    let task_receiver = Arc::new(Mutex::new(task_receiver));

    // create some tasks
    for id in 0..10 {
        task_sender.send(Task { id: id }).ok().expect("Sending should be OK");
    }

    // launch one scheduler per thread
    for num in 0..NUM_THREADS {
        // create scheduler for thread
        let mut scheduler = Scheduler::new(
            num,
            // clone shared central queue
            task_receiver.clone(),
        );

        // launch scheduler
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
