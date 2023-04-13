use crossbeam::epoch::{Atomic, Guard, Owned};
use std::sync::atomic::Ordering;

type Link<T> = Atomic<Node<T>>;

struct Node<T> {
    item: T,
    next: Link<T>,
}

pub struct LinkedList<T> {
    head: Link<T>,
}

// TODO: 能否在destory的时候单独将item保留，这样就不用Copy了
impl<T: Copy> LinkedList<T> {
    pub fn new() -> Self {
        LinkedList {
            head: Atomic::null(),
        }
    }

    pub fn push(&self, item: T, guard: &Guard) {
        let mut new_node = Owned::new(Node {
            item,
            next: Atomic::null(),
        });

        loop {
            let head = self.head.load(Ordering::SeqCst, guard);

            new_node.next = Atomic::from(head.as_raw());

            match self.head.compare_exchange(
                head,
                new_node,
                Ordering::SeqCst,
                Ordering::SeqCst,
                guard,
            ) {
                Ok(_) => break,
                Err(e) => new_node = e.new,
            }
        }
    }

    pub fn pop(&self, guard: &Guard) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::SeqCst, guard);

            match unsafe { head.as_ref() } {
                Some(h) => {
                    let next = h.next.load(Ordering::SeqCst, guard);

                    if self
                        .head
                        .compare_exchange(head, next, Ordering::SeqCst, Ordering::SeqCst, guard)
                        .is_ok()
                    {
                        unsafe {
                            guard.defer_destroy(head);
                            return Some((&(*head.as_raw())).item);
                        }
                    }
                }
                None => return None,
            }
        }
    }
}

unsafe impl<T: Send> Send for LinkedList<T> {}
unsafe impl<T: Send> Sync for LinkedList<T> {}
