use std::sync::Arc;

use crossbeam::queue::SegQueue;

use super::{executed::Executed, programdata::ProgramData, RunMode};

pub struct Initialized<'a> {
    pub(super) data: ProgramData<'a>,
}

impl<'a> Initialized<'a> {
    pub fn run(mut self, mode: RunMode) -> Executed<'a> {
        let (priority, policy) = match mode {
            RunMode::Simple => (
                thread_priority::get_current_thread_priority().unwrap(),
                thread_priority::thread_schedule_policy().unwrap(),
            ),
            RunMode::FIFO => {
                let priority =
                    thread_priority::ThreadPriority::Crossplatform(10u8.try_into().unwrap());
                let policy = thread_priority::unix::ThreadSchedulePolicy::Realtime(
                    thread_priority::RealtimeThreadSchedulePolicy::Fifo,
                );
                (priority, policy)
            }
        };

        let summaries = Arc::new(SegQueue::new());

        std::thread::scope(|s| {
            self.data.nodes.drain(..).for_each(|mut child| {
                let id = child.id();
                let name = child.name();
                let builder = thread_priority::ThreadBuilder::default().name(format!(
                    "{}({})",
                    child.id(),
                    child.name()
                ));

                let builder = builder.priority(priority).policy(policy);
                let summary_queue = summaries.clone();

                builder
                    .spawn_scoped_careless(s, move || {
                        child.run();
                        summary_queue.push(child.summarize());
                    })
                    .unwrap_or_else(|_| panic!("Failed to spawn child {name:?} {id:?}"));
            });
        });
        Executed {
            nodes: Arc::into_inner(summaries).unwrap().into_iter().collect(),
            edges: self.data.edges,
        }
    }
}
