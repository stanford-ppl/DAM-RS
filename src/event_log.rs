use std::collections::LinkedList;

const ENTRIES_PER_BLOCK: usize = 64;

#[derive(Debug, Clone)]
pub struct EventLog<EventType> {
    blocks: LinkedList<LogBlock<EventType>>,
}

impl<EventType: Copy> EventLog<EventType> {
    #[inline(always)]
    fn push(&mut self, event: EventType) {
        match self.blocks.back() {
            Some(block) if !block.full() => {}
            _ => {
                self.blocks.push_back(LogBlock::new());
            }
        }
        let back_ref = self.blocks.back_mut().unwrap();
        back_ref.push(event);
    }

    fn new() -> Self {
        Self::default()
    }
}

impl<EventType> Default for EventLog<EventType> {
    fn default() -> Self {
        Self {
            blocks: Default::default(),
        }
    }
}

pub struct EventLogIterator<EventType> {
    cur_ind: usize,
    log: EventLog<EventType>,
}

impl<EventType: Copy> Iterator for EventLogIterator<EventType> {
    type Item = EventType;

    fn next(&mut self) -> Option<Self::Item> {
        match self.log.blocks.front_mut() {
            Some(block) => {
                let rv = block.data[self.cur_ind].take();
                assert!(rv.is_some());
                self.cur_ind += 1;
                if self.cur_ind >= block.ptr {
                    self.cur_ind = 0;
                    self.log.blocks.pop_front();
                }

                rv
            }
            None => None,
        }
    }
}

impl<EventType: Copy> IntoIterator for EventLog<EventType> {
    type Item = EventType;

    type IntoIter = EventLogIterator<EventType>;

    fn into_iter(self) -> Self::IntoIter {
        EventLogIterator {
            cur_ind: 0,
            log: self,
        }
    }
}

#[derive(Debug, Clone)]
struct LogBlock<EventType> {
    ptr: usize,
    data: [Option<EventType>; ENTRIES_PER_BLOCK],
}

impl<EventType: Copy> LogBlock<EventType> {
    fn new() -> Self {
        Self {
            ptr: 0,
            data: [None; ENTRIES_PER_BLOCK],
        }
    }

    fn full(&self) -> bool {
        self.ptr == ENTRIES_PER_BLOCK - 1
    }

    fn push(&mut self, event: EventType) {
        let _ = self.data[self.ptr].insert(event);
        self.ptr += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::EventLog;

    #[test]
    fn push_log_then_iter() {
        const TEST_SIZE: usize = 4096;
        let mut log = EventLog::<usize>::new();
        for i in 0..TEST_SIZE {
            log.push(i);
        }
        for (gold, evt) in log.into_iter().enumerate() {
            assert_eq!(gold, evt);
        }
    }
}
