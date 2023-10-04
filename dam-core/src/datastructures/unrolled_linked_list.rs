#![allow(unused)]

use std::{collections::LinkedList, mem::MaybeUninit};

//TODO: After generic constants (https://github.com/rust-lang/rust/issues/76560) gets resolved, should change to bytes-per-block instead.
const ENTRIES_PER_BLOCK: usize = 4096;

#[derive(Debug)]
pub struct UnrolledLinkedList<ElemType> {
    blocks: LinkedList<LogBlock<ElemType>>,
}

impl<ElemType> UnrolledLinkedList<ElemType> {
    #[inline(always)]
    pub fn push(&mut self, event: ElemType) {
        match self.blocks.back() {
            Some(block) if !block.full() => {}
            _ => {
                self.blocks.push_back(LogBlock::new());
            }
        }
        let back_ref = self.blocks.back_mut().unwrap();
        back_ref.push(event);
    }

    pub fn new() -> Self {
        Self::default()
    }
}

impl<ElemType> Default for UnrolledLinkedList<ElemType> {
    fn default() -> Self {
        Self {
            blocks: Default::default(),
        }
    }
}

// This only happens if we haven't started iterating yet
impl<T> Drop for UnrolledLinkedList<T> {
    fn drop(&mut self) {
        for block in self.blocks.iter_mut() {
            for i in 0..block.ptr {
                unsafe {
                    block.data[i].assume_init_drop();
                }
            }
        }
    }
}

pub struct UnrolledLinkedListIterator<ElemType> {
    cur_ind: usize,
    log: UnrolledLinkedList<ElemType>,
}

impl<ElemType> Iterator for UnrolledLinkedListIterator<ElemType> {
    type Item = ElemType;

    fn next(&mut self) -> Option<Self::Item> {
        match self.log.blocks.front_mut() {
            Some(block) => {
                let rv = unsafe {
                    std::mem::replace(&mut block.data[self.cur_ind], MaybeUninit::uninit())
                        .assume_init()
                };
                self.cur_ind += 1;
                if self.cur_ind >= block.ptr {
                    self.cur_ind = 0;
                    self.log.blocks.pop_front();
                }

                Some(rv)
            }
            None => None,
        }
    }
}

impl<ElemType> IntoIterator for UnrolledLinkedList<ElemType> {
    type Item = ElemType;

    type IntoIter = UnrolledLinkedListIterator<ElemType>;

    fn into_iter(self) -> Self::IntoIter {
        UnrolledLinkedListIterator {
            cur_ind: 0,
            log: self,
        }
    }
}

impl<ElemType> Drop for UnrolledLinkedListIterator<ElemType> {
    // When we're dropped, drain the iterator first to drop everyone.
    fn drop(&mut self) {
        loop {
            match self.next() {
                None => return,
                Some(_) => {}
            }
        }
    }
}

#[derive(Debug)]
struct LogBlock<ElemType> {
    data: [MaybeUninit<ElemType>; ENTRIES_PER_BLOCK],
    ptr: usize,
}

impl<ElemType> LogBlock<ElemType> {
    fn new() -> Self {
        Self {
            data: std::array::from_fn(|_| MaybeUninit::uninit()),
            ptr: 0,
        }
    }

    fn push(&mut self, event: ElemType) {
        assert!(!self.full());
        let _ = self.data[self.ptr].write(event);
        self.ptr += 1;
    }

    fn full(&self) -> bool {
        self.ptr == ENTRIES_PER_BLOCK
    }
}

#[cfg(test)]
mod tests {
    use super::UnrolledLinkedList;

    struct DropTest {
        id: usize,
    }

    impl Drop for DropTest {
        fn drop(&mut self) {
            println!("Dropping {}", self.id);
        }
    }

    #[test]
    fn push_log_then_iter() {
        const TEST_SIZE: usize = 4096;
        let mut log = UnrolledLinkedList::<usize>::new();
        for i in 0..TEST_SIZE {
            log.push(i);
        }
        for (gold, evt) in log.into_iter().enumerate() {
            assert_eq!(gold, evt);
        }
    }

    #[test]
    fn push_log_drop() {
        const TEST_ITERATIONS: usize = 1024;
        const TEST_SIZE: usize = 1024;
        for iter in 0..TEST_ITERATIONS {
            let mut log = UnrolledLinkedList::<Box<DropTest>>::new();
            for i in 0..TEST_SIZE {
                log.push(Box::new(DropTest {
                    id: iter * TEST_SIZE + i,
                }));
            }
            drop(log);
        }
    }

    #[test]
    fn push_log_iter_drop() {
        const TEST_ITERATIONS: usize = 1024;
        const TEST_SIZE: usize = 4096;
        for iter in 0..TEST_ITERATIONS {
            let mut log = UnrolledLinkedList::<Box<DropTest>>::new();
            for i in 0..TEST_SIZE {
                log.push(Box::new(DropTest {
                    id: iter * TEST_SIZE + i,
                }));
            }
            let v = log.into_iter();
            drop(v);
        }
    }
}
