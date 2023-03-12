// unsafe实现 双端 队列链表

use std::ptr;

pub struct List<T> {
    head: Link<T>,
    tail: *mut Node<T>,
}

type Link<T> = Option<Box<Node<T>>>;

struct Node<T> {
    elem: T,
    next: Link<T>,
}

impl<T> Node<T> {
    pub fn new(elem: T) -> Box<Self> {
        Box::new(Node { elem, next: None })
    }
}

impl<T> List<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: ptr::null_mut(),
        }
    }

    pub fn enqueue(&mut self, elem: T) {
        let mut new_tail = Node::new(elem);
        let raw_new_tail: *mut _ = &mut *new_tail;
        if self.tail.is_null() {
            self.head = Some(new_tail);
        } else {
            unsafe { (*self.tail).next = Some(new_tail) }
        }
        self.tail = raw_new_tail;
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.head.take().map(|old_head| {
            match old_head.next {
                Some(new_head) => {
                    self.head = Some(new_head);
                }
                None => {
                    self.tail = ptr::null_mut();
                }
            };

            old_head.elem
        })
    }
}

#[cfg(test)]
mod test {
    use super::List;
    #[test]
    fn basics() {
        let mut list = List::new();

        // Check empty list behaves right
        assert_eq!(list.dequeue(), None);

        // dequeueulate list
        list.enqueue(1);
        list.enqueue(2);
        list.enqueue(3);

        // Check normal removal
        assert_eq!(list.dequeue(), Some(1));
        assert_eq!(list.dequeue(), Some(2));

        // enqueue some more just to make sure nothing's corrupted
        list.enqueue(4);
        list.enqueue(5);

        // Check normal removal
        assert_eq!(list.dequeue(), Some(3));
        assert_eq!(list.dequeue(), Some(4));

        // Check exhaustion
        assert_eq!(list.dequeue(), Some(5));
        assert_eq!(list.dequeue(), None);

        // Check the exhaustion case fixed the pointer right
        list.enqueue(6);
        list.enqueue(7);

        // Check normal removal
        assert_eq!(list.dequeue(), Some(6));
        assert_eq!(list.dequeue(), Some(7));
        assert_eq!(list.dequeue(), None);
    }
}
