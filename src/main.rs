#![feature(core)]
extern crate threadpool;
extern crate coroutine;

use std::cell::{ UnsafeCell };
use std::boxed::{ FnBox };
use std::mem::transmute;
use std::sync::mpsc::{ Sender, SendError, Receiver, channel };
use std::sync::{ Arc, Mutex };
use std::sync::atomic::{ AtomicUsize, Ordering };

use threadpool::{ ScopedPool };

#[allow(unused_imports)]
use coroutine::sched;

use coroutine::{ Coroutine };
use coroutine::coroutine::{ Handle, State };

struct Environment {
    scheduler: Option<Scheduler>,
}

impl Environment {
    fn new() -> Environment {
        Environment {
            scheduler: None,
        }
    }

    fn init(scheduler: Scheduler) {
        ENV.with(|env| {
            let env: &mut Environment = unsafe { transmute(env.get()) };
            env.scheduler = Some(scheduler);
        });
    }
}

struct Scheduler {
    id: u32,
    sender: Sender<Task>,
    receiver: Arc<Mutex<Receiver<Task>>>,
    current: Option<Task>,
}

impl Scheduler {
    fn new(id: u32, sender: Sender<Task>, receiver: Arc<Mutex<Receiver<Task>>>) -> Scheduler {
        Scheduler {
            id: id,
            sender: sender,
            receiver: receiver,
            current: None,
        }
    }

    fn id() -> u32 {
        ENV.with(|env| {
            let env: &mut Environment = unsafe { transmute(env.get()) };
            let scheduler = env.scheduler.as_mut().unwrap();

            scheduler.id
        })
    }

    fn start() {
        ENV.with(|env| {
            let env: &mut Environment = unsafe { transmute(env.get()) };
            let scheduler = env.scheduler.as_mut().unwrap();

            println!("Scheduler {} run()", scheduler.id);
            // until all senders hang up
            while let Ok(task) = {
                let lock = scheduler.receiver.lock().unwrap();
                // TODO FIXME temporary fix to have threads end
                // needs reworking so that if the queue is empty
                // right now, but a running task will produce more
                // tasks, this thread doesn't exit
                // lock.try_recv()
                lock.recv()
            } {
                scheduler.current = Some(task);
                let state = scheduler.current.as_mut().unwrap().run();

                // when control returns to the scheduler
                // check what needs to happen to the task
                match state {
                    // coroutine waiting for child coroutines
                    // shouldn't occur with the scheduler
                    // and coroutine running on the same thread
                    State::Normal => {
                        println!("Normal");
                        unimplemented!();
                    }
                    // coroutine suspended
                    // treat this as just yielding?
                    // or treat this as waiting for child tasks?
                    // or distinguish these two somehow
                    State::Suspended => {
                        // send this task to the back of the queue
                        // Note that the back of the queue means might not be very efficient,
                        // since it could mean accumulating lots of coroutines
                        let task = scheduler.current.take().unwrap();
                        if task.dependencies.load(Ordering::Relaxed) == 0 {
                            scheduler.sender.send(task).ok().unwrap();
                        } else {
                            // TODO optionally put task in wait_queue if dependencies > 0
                        }
                    }
                    // coroutine running
                    // shouldn't occur with the scheduler
                    // and coroutine running on the same thread
                    State::Running => {
                        println!("Running");
                        unimplemented!();
                    }
                    // coroutine is done, let control pass back
                    // to the scheduler
                    State::Finished => {
                        // decrement dependencies of parent
                        if let Some(mut parent) = scheduler.current.as_mut().unwrap().parent.take() {
                            let dependencies = parent.dependencies.fetch_sub(1, Ordering::Relaxed);

                            if dependencies == 1 {
                                (*parent).run();
                            }
                        }
                    }
                    // probably fine to panic here for now
                    State::Panicked => {
                        println!("Panicked");
                        unimplemented!();
                    }
                }

            }
        });
    }

    fn schedule(mut task: Task) {
        ENV.with(|env| {
            let env: &Environment = unsafe { transmute(env.get()) };
            let scheduler = env.scheduler.as_ref().unwrap();

            let parent = scheduler.current.clone().unwrap();
            let _ = parent.dependencies.fetch_add(1, Ordering::Relaxed);
            task.parent = Some(Box::new(parent));

            scheduler.sender.send(task).ok().unwrap();
        });
    }
}

thread_local!(static ENV: UnsafeCell<Environment> = UnsafeCell::new(Environment::new()));

struct Task {
    coroutine: Option<Handle>,
    work: Option<Box<FnBox() + Send + 'static>>,
    parent: Option<Box<Task>>,
    dependencies: Arc<AtomicUsize>,
}

impl Task {
    fn new<F: FnOnce() + Send + 'static>(work: F) -> Task {
        Task {
            coroutine: None,
            work: Some(Box::new(work)),
            parent: None,
            dependencies: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn run(&mut self) -> State {
        if self.coroutine.is_none() {
            // println!("Task {} on Scheduler {}: starting", self.id, scheduler.id);
            let work = self.work.take().unwrap();

            // TODO pool/reuse Coroutines (although stacks are already pooled)
            self.coroutine = Some(Coroutine::spawn(move|| work()));
        }

        // println!("Task {} on Scheduler {}: resuming", self.id, scheduler.id);
        let coroutine = self.coroutine.as_mut().unwrap();
        coroutine.resume().ok().unwrap();
        coroutine.state()
    }
}

impl Clone for Task {
    fn clone(&self) -> Task {
        if self.work.is_some() {
            panic!("No cloning scheduled tasks");
        }

        Task {
            work: None,
            coroutine: self.coroutine.clone(),
            parent: None,
            dependencies: self.dependencies.clone(),
        }
    }
}

struct FiberPool<'a> {
    threadpool: ScopedPool<'a>,
    sender: Sender<Task>,
}

impl<'a> FiberPool<'a> {
    fn new(num_threads: u32) -> FiberPool<'a> {
        let (sender, receiver) = channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let threadpool = ScopedPool::new(num_threads);

        // launch one scheduler per thread
        for num in 0..num_threads {
            // create scheduler for thread
            let scheduler = Scheduler::new(
                num,
                // clone shared central queue
                sender.clone(),
                receiver.clone(),
            );

            // launch scheduler
            threadpool.execute(move|| {
                Environment::init(scheduler);

                Scheduler::start();
            });
        }

        FiberPool {
            threadpool: threadpool,
            sender: sender,
        }
    }

    fn execute(&mut self, task: Task) -> Result<(), SendError<Task>> {
        self.sender.send(task)
    }
}

const NUM_THREADS: u32 = 4;

fn main() {
    let mut pool = FiberPool::new(NUM_THREADS);

    // create some tasks
    for id in 0..6 {
        pool.execute(Task::new(move|| {
            // sched();

            for sub_id in 0..6 {
                Scheduler::schedule(Task::new(move|| {
                    println!("Inner Task {}.{} on {}: done", id, sub_id, Scheduler::id());
                }));
            }
            sched();

            println!("Task {} on {}: done", id, Scheduler::id());
        })).ok().expect("Sending should be OK");
    }
}

#[test]
fn it_works() {
}