use super::{
    mpsc::{IntrusiveMpsc, IntrusiveNode},
    thread::{State, Thread},
};
use crate::arch::cpu::Local;

pub struct Scheduler {
    thread_queue: IntrusiveMpsc<Thread>,
    idle_thread: *mut Thread,
}

impl Scheduler {
    pub fn new(idle_thread: *mut Thread) -> Scheduler {
        Scheduler {
            thread_queue: IntrusiveMpsc::new(),
            idle_thread,
        }
    }

    pub fn schedule_thread(&self, thread: *mut Thread) {
        unsafe {
            self.thread_queue.push(thread);
        }
    }

    pub unsafe fn switch(&self) {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let current_thread = Thread::current();

            let next_thread = loop {
                if let Some(next_thread) = self.thread_queue.pop() {
                    assert!(!next_thread.is_on_queue());

                    let state = (*next_thread).state();
                    if state == State::Ready {
                        break next_thread;
                    } else if state == State::Killable {
                        (*next_thread).set_state(State::Dead);
                    }
                } else {
                    if (*current_thread).state() == State::Running {
                        return;
                    } else {
                        break self.idle_thread;
                    }
                }
            };

            if current_thread.state() == State::Running {
                current_thread.set_state(State::Ready);
                if current_thread as *const _ != self.idle_thread as *const _ {
                    self.thread_queue.push(current_thread);
                }
            }

            assert_eq!((*next_thread).state(), State::Ready);

            (*next_thread).set_state(State::Running);

            Local::set_current_thread(next_thread);

            current_thread.ctx.swap(&(*next_thread).ctx);
        });
    }
}
