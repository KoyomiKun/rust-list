// Rc 可变的情况下, 实现双端队列

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub struct List<T> {
    head: Link<T>,
    tail: Link<T>,
}

type Link<T> = Option<Rc<RefCell<Node<T>>>>;

struct Node<T> {
    elem: T,
    next: Link<T>,
    prev: Link<T>,
}

impl<T> Node<T> {
    pub fn new(elem: T) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            elem,
            next: None,
            prev: None,
        }))
    }
}

impl<T> List<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    pub fn push_front(&mut self, elem: T) {
        let new_head = Node::new(elem);
        match self.head.take() {
            Some(old_head) => {
                old_head.borrow_mut().prev = Some(new_head.clone()); // 手动deref，否则rc和refcell都有borrowmut，会调用错误
                new_head.borrow_mut().next = Some(old_head);
                self.head = Some(new_head);
            }
            None => {
                self.head = Some(new_head.clone());
                self.tail = Some(new_head);
            }
        }
    }

    pub fn push_back(&mut self, elem: T) {
        let new_tail = Node::new(elem);
        match self.tail.take() {
            Some(old_tail) => {
                old_tail.borrow_mut().next = Some(new_tail.clone());
                new_tail.borrow_mut().prev = Some(old_tail);
                self.tail = Some(new_tail);
            }
            None => {
                self.tail = Some(new_tail.clone());
                self.head = Some(new_tail);
            }
        }
    }
    pub fn pop_front(&mut self) -> Option<T> {
        // Trick：将原来head中的值全部take
        self.head.take().map(|node| {
            match node.borrow_mut().next.take() {
                Some(next) => {
                    next.borrow_mut().prev.take();
                    self.head = Some(next);
                }
                None => {
                    // 这是头里有值，但头的next没值的情况
                    // 取出后变成空表，所以tail也要take
                    self.tail.take();
                }
            }
            // into_inner确实能消费refcell，但rc不允许move内部的结构
            // 所以需要先释放rc
            // unwrap需要debug trait
            // 或者使用ok将result转成option（不需要关心error）
            // 这里没有程序bug的话node的引用就是就是1
            Rc::try_unwrap(node).ok().unwrap().into_inner().elem
        })
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.tail.take().map(|node| {
            match node.borrow_mut().prev.take() {
                Some(prev) => {
                    prev.borrow_mut().next.take();
                    self.tail = Some(prev);
                }
                None => {
                    self.head.take();
                }
            }
            Rc::try_unwrap(node).ok().unwrap().into_inner().elem
        })
    }

    pub fn peek_front(&self) -> Option<Ref<T>> {
        // borrow返回的是一个Ref，属于在&T之外又包了一层； Ref实现了Deref，基本能像&T一样使用
        // !!! Ref和RefMut相当于运行时判断的 &T和&mut T
        // 然而，这里borrow出来的Ref **生命周期和RefCell不一致了**；
        // 显然ref的生命周期就在map的闭包里，所以不能把这部分值传递出去,
        // 即使这个值的本质还是refcell
        self.head
            .as_ref()
            .map(|node| Ref::map(node.borrow(), |node| &node.elem))
    }

    pub fn peek_back(&self) -> Option<Ref<T>> {
        self.tail
            .as_ref()
            .map(|node| Ref::map(node.borrow(), |node| &node.elem))
    }

    pub fn peek_back_mut(&mut self) -> Option<RefMut<T>> {
        self.tail
            .as_ref()
            .map(|node| RefMut::map(node.borrow_mut(), |node| &mut node.elem))
    }

    pub fn peek_front_mut(&mut self) -> Option<RefMut<T>> {
        self.head
            .as_ref()
            .map(|node| RefMut::map(node.borrow_mut(), |node| &mut node.elem))
    }
}

// 循环链表的rc显然存在循环引用，所以得手动将node一个个pop出去
// 这里得保证pop的时候rc被释放了(head.prev, head.next)
impl<T> Drop for List<T> {
    fn drop(&mut self) {
        while self.pop_front().is_some() {}
    }
}

struct IntoIter<T>(List<T>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.pop_back()
    }
}

struct Iter<'a, T>(Option<Ref<'a, Node<T>>>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = Ref<'a, T>;
    // 不允许将一个ref分割成两部分，一部分(next)给self，一部分包装成ref返回
    // 然后split后外层又带了个ref...map的时候又不能map
    // 如果不用map，又不允许在闭包外拿到borrow的值，必须用引用建一个新ref...
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
        //self.0.take().map(|node| {
        //let (next, elem) = Ref::map_split(node, |node| (&node.next, &node.elem));
        //self.0 = Some(Ref::map(next, ))
        //elem
        //})
    }
}

#[cfg(test)]
mod test {
    use super::List;
    #[test]
    fn basics() {
        let mut list = List::new();

        // Check empty list behaves right
        assert_eq!(list.pop_front(), None);

        // Populate list
        list.push_front(1);
        list.push_front(2);
        list.push_front(3);

        // Check normal removal
        assert_eq!(list.pop_front(), Some(3));
        assert_eq!(list.pop_front(), Some(2));

        // Push some more just to make sure nothing's corrupted
        list.push_front(4);
        list.push_front(5);

        // Check normal removal
        assert_eq!(list.pop_front(), Some(5));
        assert_eq!(list.pop_front(), Some(4));

        // Check exhaustion
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), None);

        // ---- back -----

        // Check empty list behaves right
        assert_eq!(list.pop_back(), None);

        // Populate list
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);

        // Check normal removal
        assert_eq!(list.pop_back(), Some(3));
        assert_eq!(list.pop_back(), Some(2));

        // Push some more just to make sure nothing's corrupted
        list.push_back(4);
        list.push_back(5);

        // Check normal removal
        assert_eq!(list.pop_back(), Some(5));
        assert_eq!(list.pop_back(), Some(4));

        // Check exhaustion
        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), None);
    }

    #[test]
    fn peek() {
        let mut list = List::new();
        assert!(list.peek_front().is_none());
        assert!(list.peek_back().is_none());
        assert!(list.peek_front_mut().is_none());
        assert!(list.peek_back_mut().is_none());

        list.push_front(1);
        list.push_front(2);
        list.push_front(3);

        assert_eq!(&*list.peek_front().unwrap(), &3);
        assert_eq!(&mut *list.peek_front_mut().unwrap(), &mut 3);
        assert_eq!(&*list.peek_back().unwrap(), &1);
        assert_eq!(&mut *list.peek_back_mut().unwrap(), &mut 1);
    }
}
