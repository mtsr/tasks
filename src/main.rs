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

enum SchedulerMessage<F> {
    Task(Task<F>),
    Done,
}

struct Scheduler<F> {
    id: u32,
    sender: Sender<SchedulerMessage<F>>,
    receiver: Arc<Mutex<Receiver<SchedulerMessage<F>>>>
}

impl<F> Scheduler<F> where F: FnOnce() -> () + Send + 'static {
    fn new(id: u32, sender: Sender<SchedulerMessage<F>>, receiver: Arc<Mutex<Receiver<SchedulerMessage<F>>>>) -> Scheduler<F> {
        Scheduler {
            id: id,
            sender: sender,
            receiver: receiver,
        }
    }

    fn run(mut self) {
        println!("Scheduler {} run()", self.id);
        // until all senders hang up
        while let Ok(msg) = {
            let lock = self.receiver.lock().unwrap();
            lock.recv()
        } {
            match msg {
                SchedulerMessage::Task(task) => {
                    task.run(&self);
                }
                // or we get the message Done
                SchedulerMessage::Done => {
                }
            }
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
        println!("Task {} run()", self.id);

        match self.coroutine {
            Some(coroutine) => {
                println!("Task {} resuming", self.id);
                coroutine.resume();
            }
            None => {
                println!("Task {} starting", self.id);
                let mut work = self.work.take();

                // TODO pool/reuse Coroutines (although stacks are already pooled)
                let coroutine = Coroutine::spawn(move|| (work.unwrap())());

                // actually start processing on coroutine
                coroutine.resume();

                match coroutine.state() {
                    State::Normal => {
                        println!("Normal");
                        unimplemented!();
                    }
                    State::Suspended => {
                        println!("Putting suspended task {} back on the queue", self.id);
                        self.coroutine = Some(coroutine);
                        scheduler.sender.send(SchedulerMessage::Task(self)).ok().unwrap();
                    }
                    State::Running => {
                        println!("Running");
                        unimplemented!();
                    }
                    State::Finished => {
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
        sender.send(SchedulerMessage::Task(Task::new(id, move|| {
            sched();
            println!("Task {}: done", id);
        }))).ok().expect("Sending should be OK");
    }

    // launch one scheduler per thread
    for num in 0..NUM_THREADS {
        // create scheduler for thread
        let mut scheduler = Scheduler::new(
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
