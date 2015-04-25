extern crate threadpool;

use std::sync::mpsc::{ channel };
use std::sync::mpsc::{ Sender, Receiver };

use threadpool::{ ScopedPool, ThreadPool };

type thread_num = usize;
type pool_type = ThreadPool;

const NUM_THREADS: thread_num = 2;

#[derive(Debug)]
enum SchedulerMessage {
    Done,
}

struct Scheduler {
    num: thread_num,
    sender: Sender<SchedulerMessage>,
}

impl Scheduler {
    fn run(&self) {
        for _ in 0..10 {
            print!("{}", self.num);
        };
        println!("");
        self.sender.send(SchedulerMessage::Done).ok();
    }
}

fn main() {
    let threadpool = pool_type::new(NUM_THREADS);

    let (sender, receiver) = channel();

    for num in 0..NUM_THREADS {
        let scheduler = Scheduler {
            num: num,
            sender: sender.clone(),
        };

        threadpool.execute(move|| {
            scheduler.run();
        });
    }

    // drop sender, so all senders are owned by the
    // threadpool and receiver will be done when
    // threadpool is done
    drop(sender);

    receiver.iter().map(|msg| println!("{:?}", msg)).collect::<Vec<_>>();
}

#[test]
fn it_works() {
}
