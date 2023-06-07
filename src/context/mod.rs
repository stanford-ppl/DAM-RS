use crate::time::Time;
use crossbeam::channel;
use rayon::prelude::*;
use std::sync::{Arc, RwLock};

trait Context<'a>: Send + Sync {
    fn init(&mut self);
    fn run(&mut self);
    fn cleanup(&mut self);
    fn set_parent(&mut self, parent: &mut Arc<dyn ParentContext>);
}

pub trait ContextView: Clone {
    fn tick_lower_bound(&self) -> Time;
    fn signal_when(&self, when: Time) -> channel::Receiver<Time>;
}

struct SignalElement {
    when: Time,
    how: channel::Sender<Time>,
}

// Encapsulates the callback backlog and the current tick info to make ContextView work.
struct TimeInfoUnderlying {
    time: Time,
    signal_buffer: Vec<SignalElement>,
}
pub struct TimeInfo {
    under: RwLock<TimeInfoUnderlying>,
}

impl TimeInfo {
    fn tick_lower_bound(&self) -> Time {
        return self.under.read().unwrap().time;
    }

    fn signal_when(self, when: Time) -> channel::Receiver<Time> {
        let (tx, rx) = channel::bounded::<Time>(1);

        // Check time first. Since time is non-decreasing, if this cond is true, then it's always true.
        let cur_time = self.tick_lower_bound();
        if cur_time >= when {
            tx.send(cur_time).unwrap();
            drop(tx);
            rx
        } else {
            let mut write = self.under.write().unwrap();
            if write.time >= when {
                tx.send(cur_time).unwrap();
                drop(tx);
            } else {
                write.signal_buffer.push(SignalElement { when, how: tx })
            }
            rx
        }
    }

    fn incr_cycles<T>(&mut self, incr: T)
    where
        Time: std::ops::AddAssign<T>,
    {
        self.under.write().unwrap().time += incr;
        self.scan_and_write_signals();
    }

    fn scan_and_write_signals(&mut self) {
        let tlb = self.tick_lower_bound();
        let mut write = self.under.write().unwrap();
        write.signal_buffer.retain(|signal| {
            if signal.when <= tlb {
                // If the signal time is in the present or past,
                signal.how.send(tlb).unwrap();
                false
            } else {
                true
            }
        })
    }
}

type ChildType<'a> = Arc<RwLock<dyn Context<'a>>>;
trait ParentContext<'a>: Context<'a> {
    fn current_id(&mut self) -> &mut usize;
    fn new_child_id(&mut self) -> usize {
        let x = self.current_id();
        *x += 1;
        *x - 1
    }

    fn children(&mut self) -> &mut Vec<ChildType<'a>>;
    fn add_child(&mut self, child: &mut ChildType<'a>) {
        self.children().push(child.clone());
    }

    fn for_each_child(&mut self, map_f: fn(child: &ChildType<'a>)) {
        self.children()
            .into_par_iter()
            .for_each(|child| map_f(child))
    }

    fn init(&mut self) {
        self.for_each_child(|child| {
            child.write().unwrap().init();
        })
    }

    fn run(&mut self) {
        self.for_each_child(|child| {
            child.write().unwrap().run();
        })
    }

    fn cleanup(&mut self) {
        self.for_each_child(|child| {
            child.write().unwrap().cleanup();
        })
    }
}
