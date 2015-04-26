extern crate threadpool;
extern crate deque;
extern crate coroutine;

use std::sync::mpsc::{ Sender, Receiver, channel };
use std::sync::{ Arc, Mutex };

use threadpool::{ ScopedPool };

#[allow(unused_imports)]
use coroutine::{ Coroutine, sched };
use coroutine::coroutine::{ Handle, State };

const NUM_THREADS: u32 = 2;

struct Scheduler<F> {
    id: u32,
    sender: Sender<Task<F>>,
    receiver: Arc<Mutex<Receiver<Task<F>>>>
}

impl<F> Scheduler<F> where F: FnOnce() -> () + Send + 'static {
    fn new(id: u32, sender: Sender<Task<F>>, receiver: Arc<Mutex<Receiver<Task<F>>>>) -> Scheduler<F> {
        Scheduler {
            id: id,
            sender: sender,
            receiver: receiver,
        }
    }

    fn run(mut self) {
        println!("Scheduler {} run()", self.id);
        // until all senders hang up
        while let Ok(task) = {
            let lock = self.receiver.lock().unwrap();
            lock.recv()
        } {
            task.run(&self);
        }
    }
}

#[derive(Debug)]
struct Task<F> {
    id: u32,
    coroutine: Option<Handle>,
    work: Option<F>,
}

impl<F> Task<F> where F: FnOnce() -> () + Send + 'static {
    fn new(id: u32, work: F) -> Task<F> {
        Task {
            id: id,
            coroutine: None,
            work: Some(work),
        }
    }

    fn run(mut self, scheduler: &Scheduler<F>) {
        match self.coroutine {
            Some(coroutine) => {
                println!("Task {} on Scheduler {}: resuming", self.id, scheduler.id);
                coroutine.resume().ok().unwrap();
            }
            None => {
                println!("Task {} on Scheduler {}: starting", self.id, scheduler.id);
                let work = self.work.take();

                // TODO pool/reuse Coroutines (although stacks are already pooled)
                let coroutine = Coroutine::spawn(move|| (work.unwrap())());

                // actually start processing on coroutine
                coroutine.resume().ok().unwrap();

                match coroutine.state() {
                    State::Normal => {
                        println!("Normal");
                        unimplemented!();
                    }
                    State::Suspended => {
                        // send this task to the back of the queue
                        // Note that the back of the queue means might not be very efficient,
                        // since it could mean accumulating lots of coroutines
                        self.coroutine = Some(coroutine);
                        scheduler.sender.send(self).ok().unwrap();
                    }
                    State::Running => {
                        println!("Running");
                        unimplemented!();
                    }
                    State::Finished => {
                        // No need to do anything here
                    }
                    State::Panicked => {
                        println!("Panicked");
                        unimplemented!();
                    }
                }
            }
        }
    }
}

fn main() {
    // create pool with 1 thread per core
    let threadpool = ScopedPool::new(NUM_THREADS);

    // TODO priorities
    // channel for mpmc queue
    let (sender, receiver) = channel();
    let receiver = Arc::new(Mutex::new(receiver));

    // create some tasks
    for id in 0..10 {
        sender.send(Task::new(id, move|| {
            sched();
            println!("Task {}: done", id);
        })).ok().expect("Sending should be OK");
    }

    // launch one scheduler per thread
    for num in 0..NUM_THREADS {
        // create scheduler for thread
        let scheduler = Scheduler::new(
            num,
            // clone shared central queue
            sender.clone(),
            receiver.clone(),
        );

        // launch scheduler
        threadpool.execute(move|| {
            scheduler.run();
        });
    }
}

#[test]
fn it_works() {
}
