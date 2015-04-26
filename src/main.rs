extern crate threadpool;
extern crate deque;
extern crate coroutine;

use std::sync::mpsc::{ Sender, SendError, Receiver, RecvError, channel };
use std::sync::{ Arc, Mutex };

use threadpool::{ ScopedPool };

use coroutine::{ Coroutine, sched };
use coroutine::coroutine::State;

const NUM_THREADS: u32 = 2;

enum SchedulerMessage {
    Task(Task),
    Done,
}

#[derive(Clone)]
struct SchedulerChannel {
    sender: Sender<SchedulerMessage>,
    receiver: Arc<Mutex<Receiver<SchedulerMessage>>>,
}

impl SchedulerChannel {
    fn new() -> SchedulerChannel {
        let (sender, receiver) = channel();
        SchedulerChannel {
            sender: sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    fn send(&self, msg: SchedulerMessage) -> Result<(), SendError<SchedulerMessage>> {
        self.sender.send(msg)
    }

    fn recv(&self) -> Result<SchedulerMessage, RecvError> {
        let lock = self.receiver.lock().unwrap();
        lock.recv()
    }
}

struct Scheduler {
    num: u32,
    channel: SchedulerChannel,
}

impl Scheduler {
    fn new(num: u32, channel: SchedulerChannel) -> Scheduler {
        Scheduler {
            num: num,
            channel: channel,
        }
    }

    fn run(&mut self) {
        // until all senders hang up
        while let Ok(msg) = {
            let lock = self.channel.receiver.lock().unwrap();
            lock.recv()
        } {
            match msg {
                SchedulerMessage::Task(task) => {
                    self.spawn(task);
                }
                // or we get the message Done
                SchedulerMessage::Done => {
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

    // TODO priorities
    // channel for mpmc queue
    let channel = SchedulerChannel::new();

    // create some tasks
    for id in 0..10 {
        channel.send(SchedulerMessage::Task(Task { id: id })).ok().expect("Sending should be OK");
    }

    for _ in 0..NUM_THREADS {
        channel.send(SchedulerMessage::Done).ok().expect("Sending should be OK");
    }

    // launch one scheduler per thread
    for num in 0..NUM_THREADS {
        // create scheduler for thread
        let mut scheduler = Scheduler::new(
            num,
            // clone shared central queue
            channel.clone(),
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
