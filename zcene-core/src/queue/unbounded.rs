use alloc::boxed::Box;
use core::alloc::Allocator;
use core::mem::MaybeUninit;
use core::ptr;

use crossbeam_utils::CachePadded;

use super::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use super::sync::cell::UnsafeCell;
#[allow(unused_imports)]
use super::sync::prelude::*;
use super::{PopError, PushError, busy_wait, full_fence};

const WRITE: usize = 1;
const READ: usize = 2;
const DESTROY: usize = 4;

const LAP: usize = 32;
const BLOCK_CAP: usize = LAP - 1;
const SHIFT: usize = 1;
const MARK_BIT: usize = 1;

struct Slot<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    state: AtomicUsize,
}

impl<T> Slot<T> {
    const UNINIT: Slot<T> = Slot {
        value: UnsafeCell::new(MaybeUninit::uninit()),
        state: AtomicUsize::new(0),
    };

    fn uninit_block() -> [Slot<T>; BLOCK_CAP] {
        [Self::UNINIT; BLOCK_CAP]
    }

    fn wait_write(&self) {
        while self.state.load(Ordering::Acquire) & WRITE == 0 {
            busy_wait();
        }
    }
}

struct Block<T> {
    next: AtomicPtr<Block<T>>,
    slots: [Slot<T>; BLOCK_CAP],
}

impl<T> Block<T> {
    fn new() -> Block<T> {
        Block {
            next: AtomicPtr::new(ptr::null_mut()),
            slots: Slot::uninit_block(),
        }
    }

    fn wait_next(&self) -> *mut Block<T> {
        loop {
            let next = self.next.load(Ordering::Acquire);
            if !next.is_null() {
                return next;
            }
            busy_wait();
        }
    }

    unsafe fn destroy<A: Allocator + Clone>(this: *mut Block<T>, start: usize, alloc: A) {
        for i in start..BLOCK_CAP - 1 {
            let slot = (*this).slots.get_unchecked(i);

            if slot.state.load(Ordering::Acquire) & READ == 0
                && slot.state.fetch_or(DESTROY, Ordering::AcqRel) & READ == 0
            {
                return;
            }
        }

        drop(Box::from_raw_in(this, alloc));
    }
}

struct Position<T> {
    index: AtomicUsize,
    block: AtomicPtr<Block<T>>,
}

pub struct Unbounded<T, A: Allocator + Clone> {
    head: CachePadded<Position<T>>,
    tail: CachePadded<Position<T>>,
    alloc: A,
}

impl<T, A: Allocator + Clone> Unbounded<T, A> {
    pub fn new_in(alloc: A) -> Unbounded<T, A> {
        Unbounded {
            head: CachePadded::new(Position {
                block: AtomicPtr::new(ptr::null_mut()),
                index: AtomicUsize::new(0),
            }),
            tail: CachePadded::new(Position {
                block: AtomicPtr::new(ptr::null_mut()),
                index: AtomicUsize::new(0),
            }),
            alloc,
        }
    }

    fn alloc_block(&self) -> *mut Block<T> {
        let (pointer, alloc) =
            Box::into_raw_with_allocator(Box::new_in(Block::<T>::new(), self.alloc.clone()));
        drop(alloc);
        pointer
    }

    unsafe fn reclaim_block(&self, pointer: *mut Block<T>) -> Box<Block<T>, A> {
        Box::from_raw_in(pointer, self.alloc.clone())
    }

    pub fn push(&self, value: T) -> Result<(), PushError<T>> {
        let mut tail = self.tail.index.load(Ordering::Acquire);
        let mut block = self.tail.block.load(Ordering::Acquire);
        let mut next_block = None;

        loop {
            if tail & MARK_BIT != 0 {
                return Err(PushError::Closed(value));
            }

            let offset = (tail >> SHIFT) % LAP;

            if offset == BLOCK_CAP {
                busy_wait();
                tail = self.tail.index.load(Ordering::Acquire);
                block = self.tail.block.load(Ordering::Acquire);
                continue;
            }

            if offset + 1 == BLOCK_CAP && next_block.is_none() {
                next_block = Some(Box::new_in(Block::<T>::new(), self.alloc.clone()));
            }

            if block.is_null() {
                let new = self.alloc_block();

                if self
                    .tail
                    .block
                    .compare_exchange(block, new, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    self.head.block.store(new, Ordering::Release);
                    block = new;
                } else {
                    next_block = unsafe { Some(self.reclaim_block(new)) };
                    tail = self.tail.index.load(Ordering::Acquire);
                    block = self.tail.block.load(Ordering::Acquire);
                    continue;
                }
            }

            let new_tail = tail + (1 << SHIFT);

            match self.tail.index.compare_exchange_weak(
                tail,
                new_tail,
                Ordering::SeqCst,
                Ordering::Acquire,
            ) {
                Ok(_) => unsafe {
                    if offset + 1 == BLOCK_CAP {
                        let (next_block, alloc) =
                            Box::into_raw_with_allocator(next_block.unwrap());
                        drop(alloc);
                        self.tail.block.store(next_block, Ordering::Release);
                        self.tail.index.fetch_add(1 << SHIFT, Ordering::Release);
                        (*block).next.store(next_block, Ordering::Release);
                    }

                    let slot = (*block).slots.get_unchecked(offset);
                    slot.value.with_mut(|slot| {
                        slot.write(MaybeUninit::new(value));
                    });
                    slot.state.fetch_or(WRITE, Ordering::Release);
                    return Ok(());
                },
                Err(t) => {
                    tail = t;
                    block = self.tail.block.load(Ordering::Acquire);
                }
            }
        }
    }

    pub fn pop(&self) -> Result<T, PopError> {
        let mut head = self.head.index.load(Ordering::Acquire);
        let mut block = self.head.block.load(Ordering::Acquire);

        loop {
            let offset = (head >> SHIFT) % LAP;

            if offset == BLOCK_CAP {
                busy_wait();
                head = self.head.index.load(Ordering::Acquire);
                block = self.head.block.load(Ordering::Acquire);
                continue;
            }

            let mut new_head = head + (1 << SHIFT);

            if new_head & MARK_BIT == 0 {
                full_fence();
                let tail = self.tail.index.load(Ordering::Relaxed);

                if head >> SHIFT == tail >> SHIFT {
                    if tail & MARK_BIT != 0 {
                        return Err(PopError::Closed);
                    } else {
                        return Err(PopError::Empty);
                    }
                }

                if (head >> SHIFT) / LAP != (tail >> SHIFT) / LAP {
                    new_head |= MARK_BIT;
                }
            }

            if block.is_null() {
                busy_wait();
                head = self.head.index.load(Ordering::Acquire);
                block = self.head.block.load(Ordering::Acquire);
                continue;
            }

            match self.head.index.compare_exchange_weak(
                head,
                new_head,
                Ordering::SeqCst,
                Ordering::Acquire,
            ) {
                Ok(_) => unsafe {
                    if offset + 1 == BLOCK_CAP {
                        let next = (*block).wait_next();
                        let mut next_index = (new_head & !MARK_BIT).wrapping_add(1 << SHIFT);
                        if !(*next).next.load(Ordering::Relaxed).is_null() {
                            next_index |= MARK_BIT;
                        }

                        self.head.block.store(next, Ordering::Release);
                        self.head.index.store(next_index, Ordering::Release);
                    }

                    let slot = (*block).slots.get_unchecked(offset);
                    slot.wait_write();
                    let value = slot.value.with_mut(|slot| slot.read().assume_init());

                    if offset + 1 == BLOCK_CAP {
                        Block::destroy(block, 0, self.alloc.clone());
                    } else if slot.state.fetch_or(READ, Ordering::AcqRel) & DESTROY != 0 {
                        Block::destroy(block, offset + 1, self.alloc.clone());
                    }

                    return Ok(value);
                },
                Err(h) => {
                    head = h;
                    block = self.head.block.load(Ordering::Acquire);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        loop {
            let mut tail = self.tail.index.load(Ordering::SeqCst);
            let mut head = self.head.index.load(Ordering::SeqCst);

            if self.tail.index.load(Ordering::SeqCst) == tail {
                tail &= !((1 << SHIFT) - 1);
                head &= !((1 << SHIFT) - 1);

                if (tail >> SHIFT) & (LAP - 1) == LAP - 1 {
                    tail = tail.wrapping_add(1 << SHIFT);
                }
                if (head >> SHIFT) & (LAP - 1) == LAP - 1 {
                    head = head.wrapping_add(1 << SHIFT);
                }

                let lap = (head >> SHIFT) / LAP;
                tail = tail.wrapping_sub((lap * LAP) << SHIFT);
                head = head.wrapping_sub((lap * LAP) << SHIFT);

                tail >>= SHIFT;
                head >>= SHIFT;

                return tail - head - tail / LAP;
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.index.load(Ordering::SeqCst);
        let tail = self.tail.index.load(Ordering::SeqCst);
        head >> SHIFT == tail >> SHIFT
    }

    pub fn is_full(&self) -> bool {
        false
    }

    pub fn close(&self) -> bool {
        let tail = self.tail.index.fetch_or(MARK_BIT, Ordering::SeqCst);
        tail & MARK_BIT == 0
    }

    pub fn is_closed(&self) -> bool {
        self.tail.index.load(Ordering::SeqCst) & MARK_BIT != 0
    }
}

impl<T, A: Allocator + Clone> Drop for Unbounded<T, A> {
    fn drop(&mut self) {
        let Self { head, tail, alloc } = self;
        let Position { index: head, block } = &mut **head;

        head.with_mut(|&mut mut head| {
            tail.index.with_mut(|&mut mut tail| {
                head &= !((1 << SHIFT) - 1);
                tail &= !((1 << SHIFT) - 1);

                unsafe {
                    while head != tail {
                        let offset = (head >> SHIFT) % LAP;

                        if offset < BLOCK_CAP {
                            block.with_mut(|block| {
                                let slot = (**block).slots.get_unchecked(offset);
                                slot.value.with_mut(|slot| {
                                    let value = &mut *slot;
                                    value.as_mut_ptr().drop_in_place();
                                });
                            });
                        } else {
                            block.with_mut(|block| {
                                let next_block = (**block).next.with_mut(|next| *next);
                                drop(Box::from_raw_in(*block, alloc.clone()));
                                *block = next_block;
                            });
                        }

                        head = head.wrapping_add(1 << SHIFT);
                    }

                    block.with_mut(|block| {
                        if !block.is_null() {
                            drop(Box::from_raw_in(*block, alloc.clone()));
                        }
                    });
                }
            });
        });
    }
}
