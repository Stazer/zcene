/* Copyright (c) 2010-2011 Dmitry Vyukov. All rights reserved.
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 *    1. Redistributions of source code must retain the above copyright notice,
 *       this list of conditions and the following disclaimer.
 *
 *    2. Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in the
 *       documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY DMITRY VYUKOV "AS IS" AND ANY EXPRESS OR IMPLIED
 * WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT
 * SHALL DMITRY VYUKOV OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE
 * OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation are
 * those of the authors and should not be interpreted as representing official
 * policies, either expressed or implied, of Dmitry Vyukov.
 */

pub(super) use self::PopResult::*;

use alloc::boxed::Box;
use core::alloc::Allocator;
use core::cell::UnsafeCell;
use core::hint;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

pub(super) enum PopResult<T> {
    Data(T),
    Empty,
    Inconsistent,
}

struct Node<T> {
    next: AtomicPtr<Self>,
    value: Option<T>,
}

pub(super) struct Queue<T, A: Allocator + Clone> {
    head: AtomicPtr<Node<T>>,
    tail: UnsafeCell<*mut Node<T>>,
    alloc: A,
}

unsafe impl<T: Send, A: Allocator + Clone + Send> Send for Queue<T, A> {}
unsafe impl<T: Send, A: Allocator + Clone + Sync> Sync for Queue<T, A> {}

impl<T, A: Allocator + Clone> Queue<T, A> {
    unsafe fn alloc_node(v: Option<T>, alloc: A) -> *mut Node<T> {
        let (pointer, alloc) = Box::into_raw_with_allocator(Box::new_in(
            Node { next: AtomicPtr::new(ptr::null_mut()), value: v },
            alloc,
        ));
        drop(alloc);
        pointer
    }

    pub(super) fn new_in(alloc: A) -> Self {
        let stub = unsafe { Self::alloc_node(None, alloc.clone()) };
        Self { head: AtomicPtr::new(stub), tail: UnsafeCell::new(stub), alloc }
    }

    pub(super) fn push(&self, t: T) {
        unsafe {
            let n = Self::alloc_node(Some(t), self.alloc.clone());
            let prev = self.head.swap(n, Ordering::AcqRel);
            (*prev).next.store(n, Ordering::Release);
        }
    }

    pub(super) unsafe fn pop(&self) -> PopResult<T> {
        unsafe {
            let tail = *self.tail.get();
            let next = (*tail).next.load(Ordering::Acquire);

            if !next.is_null() {
                *self.tail.get() = next;
                assert!((*tail).value.is_none());
                assert!((*next).value.is_some());
                let ret = (*next).value.take().unwrap();
                drop(Box::from_raw_in(tail, self.alloc.clone()));
                return Data(ret);
            }

            if self.head.load(Ordering::Acquire) == tail {
                Empty
            } else {
                Inconsistent
            }
        }
    }

    pub(super) unsafe fn pop_spin(&self) -> Option<T> {
        loop {
            match unsafe { self.pop() } {
                Empty => return None,
                Data(t) => return Some(t),
                Inconsistent => {
                    hint::spin_loop();
                }
            }
        }
    }
}

impl<T, A: Allocator + Clone> Drop for Queue<T, A> {
    fn drop(&mut self) {
        unsafe {
            let mut cur = *self.tail.get();
            while !cur.is_null() {
                let next = (*cur).next.load(Ordering::Relaxed);
                drop(Box::from_raw_in(cur, self.alloc.clone()));
                cur = next;
            }
        }
    }
}
