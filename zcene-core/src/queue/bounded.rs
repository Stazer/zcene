use alloc::{boxed::Box, vec::Vec};
use core::alloc::Allocator;
use core::mem::MaybeUninit;

use crossbeam_utils::CachePadded;

use super::sync::atomic::{AtomicUsize, Ordering};
use super::sync::cell::UnsafeCell;
#[allow(unused_imports)]
use super::sync::prelude::*;
use super::{ForcePushError, PopError, PushError, busy_wait, full_fence};

struct Slot<T> {
    stamp: AtomicUsize,
    value: UnsafeCell<MaybeUninit<T>>,
}

pub struct Bounded<T, A: Allocator + Clone> {
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
    buffer: Box<[Slot<T>], A>,
    one_lap: usize,
    mark_bit: usize,
}

impl<T, A: Allocator + Clone> Bounded<T, A> {
    pub fn new_in(cap: usize, alloc: A) -> Bounded<T, A> {
        assert!(cap > 0, "capacity must be positive");

        let head = 0;
        let tail = 0;

        let mut buffer = Vec::with_capacity_in(cap, alloc);
        for i in 0..cap {
            buffer.push(Slot {
                stamp: AtomicUsize::new(i),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }

        let mark_bit = (cap + 1).next_power_of_two();
        let one_lap = mark_bit * 2;

        Bounded {
            buffer: buffer.into(),
            one_lap,
            mark_bit,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    pub fn push(&self, value: T) -> Result<(), PushError<T>> {
        self.push_or_else(value, |value, tail, _, _| {
            let head = self.head.load(Ordering::Relaxed);

            if head.wrapping_add(self.one_lap) == tail {
                Err(PushError::Full(value))
            } else {
                Ok(value)
            }
        })
    }

    pub fn force_push(&self, value: T) -> Result<Option<T>, ForcePushError<T>> {
        let result = self.push_or_else(value, |value, tail, new_tail, slot| {
            let head = tail.wrapping_sub(self.one_lap);
            let new_head = new_tail.wrapping_sub(self.one_lap);

            if self
                .head
                .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                self.tail.store(new_tail, Ordering::SeqCst);

                let old = unsafe {
                    slot.value
                        .with_mut(|slot| slot.replace(MaybeUninit::new(value)).assume_init())
                };

                slot.stamp.store(tail + 1, Ordering::Release);

                Err(PushError::Full(old))
            } else {
                Ok(value)
            }
        });

        match result {
            Ok(()) => Ok(None),
            Err(PushError::Full(old_value)) => Ok(Some(old_value)),
            Err(PushError::Closed(value)) => Err(ForcePushError(value)),
        }
    }

    fn push_or_else<F>(&self, mut value: T, mut fail: F) -> Result<(), PushError<T>>
    where
        F: FnMut(T, usize, usize, &Slot<T>) -> Result<T, PushError<T>>,
    {
        let mut tail = self.tail.load(Ordering::Relaxed);

        loop {
            if tail & self.mark_bit != 0 {
                return Err(PushError::Closed(value));
            }

            let index = tail & (self.mark_bit - 1);
            let lap = tail & !(self.one_lap - 1);

            let new_tail = if index + 1 < self.buffer.len() {
                tail + 1
            } else {
                lap.wrapping_add(self.one_lap)
            };

            let slot = &self.buffer[index];
            let stamp = slot.stamp.load(Ordering::Acquire);

            if tail == stamp {
                match self.tail.compare_exchange_weak(
                    tail,
                    new_tail,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        slot.value.with_mut(|slot| unsafe {
                            slot.write(MaybeUninit::new(value));
                        });
                        slot.stamp.store(tail + 1, Ordering::Release);
                        return Ok(());
                    }
                    Err(t) => {
                        tail = t;
                    }
                }
            } else if stamp.wrapping_add(self.one_lap) == tail + 1 {
                full_fence();

                value = fail(value, tail, new_tail, slot)?;

                tail = self.tail.load(Ordering::Relaxed);
            } else {
                busy_wait();
                tail = self.tail.load(Ordering::Relaxed);
            }
        }
    }

    pub fn pop(&self) -> Result<T, PopError> {
        let mut head = self.head.load(Ordering::Relaxed);

        loop {
            let index = head & (self.mark_bit - 1);
            let lap = head & !(self.one_lap - 1);

            let slot = &self.buffer[index];
            let stamp = slot.stamp.load(Ordering::Acquire);

            if head + 1 == stamp {
                let new = if index + 1 < self.buffer.len() {
                    head + 1
                } else {
                    lap.wrapping_add(self.one_lap)
                };

                match self.head.compare_exchange_weak(
                    head,
                    new,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        let value = slot
                            .value
                            .with_mut(|slot| unsafe { slot.read().assume_init() });
                        slot.stamp
                            .store(head.wrapping_add(self.one_lap), Ordering::Release);
                        return Ok(value);
                    }
                    Err(h) => {
                        head = h;
                    }
                }
            } else if stamp == head {
                full_fence();
                let tail = self.tail.load(Ordering::Relaxed);

                if (tail & !self.mark_bit) == head {
                    if tail & self.mark_bit != 0 {
                        return Err(PopError::Closed);
                    } else {
                        return Err(PopError::Empty);
                    }
                }

                head = self.head.load(Ordering::Relaxed);
            } else {
                busy_wait();
                head = self.head.load(Ordering::Relaxed);
            }
        }
    }

    pub fn len(&self) -> usize {
        loop {
            let tail = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);

            if self.tail.load(Ordering::SeqCst) == tail {
                let hix = head & (self.mark_bit - 1);
                let tix = tail & (self.mark_bit - 1);

                return if hix < tix {
                    tix - hix
                } else if hix > tix {
                    self.buffer.len() - hix + tix
                } else if (tail & !self.mark_bit) == head {
                    0
                } else {
                    self.buffer.len()
                };
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        (tail & !self.mark_bit) == head
    }

    pub fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);
        head.wrapping_add(self.one_lap) == tail & !self.mark_bit
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn close(&self) -> bool {
        let tail = self.tail.fetch_or(self.mark_bit, Ordering::SeqCst);
        tail & self.mark_bit == 0
    }

    pub fn is_closed(&self) -> bool {
        self.tail.load(Ordering::SeqCst) & self.mark_bit != 0
    }
}

impl<T, A: Allocator + Clone> Drop for Bounded<T, A> {
    fn drop(&mut self) {
        let Self { head, tail, buffer, mark_bit, .. } = self;
        let mark_bit = *mark_bit;

        head.with_mut(|&mut head| {
            tail.with_mut(|&mut tail| {
                let hix = head & (mark_bit - 1);
                let tix = tail & (mark_bit - 1);

                let len = if hix < tix {
                    tix - hix
                } else if hix > tix {
                    buffer.len() - hix + tix
                } else if (tail & !mark_bit) == head {
                    0
                } else {
                    buffer.len()
                };

                for i in 0..len {
                    let index = if hix + i < buffer.len() {
                        hix + i
                    } else {
                        hix + i - buffer.len()
                    };

                    let slot = &buffer[index];
                    slot.value.with_mut(|slot| unsafe {
                        let value = &mut *slot;
                        value.as_mut_ptr().drop_in_place();
                    });
                }
            });
        });
    }
}
