extern crate threadpool;
extern crate deque;

use std::sync::mpsc::{ channel };
use std::sync::mpsc::{ Sender, Receiver };

use threadpool::{ ScopedPool };
use deque::BufferPool;

const NUM_THREADS: u32 = 2;

#[derive(Debug)]
enum SchedulerMessage {
    Done,
}

struct Scheduler {
    num: u32,
    sender: Sender<SchedulerMessage>,
    worker: deque::Worker<Task>,
    task_receiver: Receiver<Task>,
}

impl Scheduler {
    fn run(&mut self) {

        loop {
            while let Some(task) = self.worker.pop() {
                println!("{:?}", task);
            };

            match self.task_receiver.recv() {
                Ok(task) => {
                    println!("{:?}", task);
                }
                Err(err) => {
                    break;
                }
            }
        }

        self.sender.send(SchedulerMessage::Done).ok();
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

    let mut task_senders = Vec::new();

    let (scheduler_sender, scheduler_receiver) = channel();

    for num in 0..NUM_THREADS {
        let (mut worker, mut stealer) = deque_pool.deque();
        stealers.push(stealer);

        let (task_sender, task_receiver) = channel();
        task_sender.send(Task { id: task_id });
        task_id += 1;
        task_senders.push(task_sender);

        let mut scheduler = Scheduler {
            num: num,
            sender: scheduler_sender.clone(),
            worker: worker,
            task_receiver: task_receiver,
        };

        threadpool.execute(move|| {
            scheduler.run();
        });
    }

    // drop sender, so all senders are owned by the
    // threadpool and receiver will be done when
    // threadpool is done
    drop(scheduler_sender);

    // for debugging purposes drop task_sender so
    // that threads end
    drop(task_senders);

    while let Ok(msg) = scheduler_receiver.recv() {
        println!("{:?}", msg);
    }
}

#[test]
fn it_works() {
}
