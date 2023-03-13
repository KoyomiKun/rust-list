// unsafe实现 双端 队列链表

use std::{
    fmt::{Debug, Display},
    ptr::NonNull,
};

// 一旦开始使用裸指针，就要尝试着只使用它; 安全指针会引入额外的约束，但是裸指针并不会遵守这些约束
// 1. 在开始时，将输入参数中的引用转换成裸指针
// 2. 在函数体中只使用裸指针
// 3. 返回之前，将裸指针转换成安全的指针
pub struct List<T: Debug> {
    length: usize,
    head: Link<T>,
    tail: Link<T>,
}

// NonNull 为 Rust 中的一个内置类型，其是裸指针 *mut T 的一个包装
// NonNull类型即使从未解引用指针，指针也必须始终为非 null; 可以触发option空指针优化
// 支持协变类型、保证指针非空、空指针优化: 协变，即子类型可以当成父类型使用
// 协变（covariance）：M<Cat>: M<Animal> 它们维持内部参数的关系不变；协变规则只有在容器只读的情况下才能生效，否则函数内可能把容器内容换成其他具体类型
// 逆变（contravariance）：M<Animal>: M<Cat> 它们的关系被反转了；对于只写的类型可以用逆变；
// 不变（invariance）：两者没有任何子类型关系；那么对于可读又可写的类型，当然就是不变了：我们不能做出任何假定，不然有可能爆炸;
type Link<T> = Option<NonNull<Node<T>>>; // 将空指针转成null，保证里面是nonnull的

#[derive(Debug)]
struct Node<T: Debug> {
    elem: T,
    next: Link<T>,
    prev: Link<T>,
}

impl<T: Debug> Node<T> {
    pub fn new(elem: T) -> Node<T> {
        Node {
            elem,
            next: None,
            prev: None,
        }
    }

    pub fn into_val(self: Box<Self>) -> T {
        self.elem
    }
}

impl<T: Debug> List<T> {
    pub fn new() -> Self {
        Self {
            length: 0,
            head: None,
            tail: None,
        }
    }

    pub fn push_front(&mut self, elem: T) {
        let mut new_head = Box::new(Node::new(elem));

        new_head.next = self.head;
        new_head.prev = None;
        let new_head = NonNull::new(Box::into_raw(new_head));

        match self.head {
            None => {
                self.tail = new_head;
            }
            Some(old_head) => unsafe {
                (*old_head.as_ptr()).prev = new_head;
            },
        }
        self.head = new_head;
        self.length += 1;
    }

    pub fn push_back(&mut self, elem: T) {
        let mut new_tail = Box::new(Node::new(elem));
        new_tail.prev = self.tail;
        new_tail.next = None;
        let new_tail = NonNull::new(Box::into_raw(new_tail));

        match self.tail {
            Some(old_tail) => unsafe {
                (*old_tail.as_ptr()).next = new_tail;
            },
            None => {
                self.head = new_tail;
            }
        }

        self.tail = new_tail;
        self.length += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.head.map(|old_head| unsafe {
            let old_head = Box::from_raw(old_head.as_ptr()); // 必须换成Box，否则还要自己处理后事

            match old_head.next {
                Some(new_head) => {
                    (*new_head.as_ptr()).prev = None;
                }
                None => {
                    self.tail = None;
                }
            }

            self.head = old_head.next;
            self.length -= 1;
            old_head.into_val()
        })
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.tail.map(|old_tail| unsafe {
            let old_tail = Box::from_raw(old_tail.as_ptr());

            match old_tail.prev {
                Some(new_tail) => {
                    (*new_tail.as_ptr()).next = None;
                }
                None => {
                    self.head = None;
                }
            }

            self.tail = old_tail.next;
            self.length -= 1;
            old_tail.into_val()
        })
    }

    pub fn peek_front(&self) -> Option<&T> {
        // 两个as_ref不一样
        // 第一个as_ref将head所有权转化成引用，不然就试图把所有权返回了
        // 第二个as_ref将node（指针的引用）解引用后返回里面值的引用
        unsafe { self.head.as_ref().map(|node| &node.as_ref().elem) }
    }

    pub fn peek_back(&self) -> Option<&T> {
        unsafe { self.tail.as_ref().map(|node| &node.as_ref().elem) }
    }

    pub fn peek_front_mut(&mut self) -> Option<&mut T> {
        unsafe { self.head.as_mut().map(|node| &mut node.as_mut().elem) }
    }

    pub fn peek_back_mut(&mut self) -> Option<&mut T> {
        unsafe { self.tail.as_mut().map(|node| &mut node.as_mut().elem) }
    }

    // idx in range [0, len)
    pub fn get_by_idx(&self, idx: usize) -> Option<&T> {
        if idx >= self.length {
            return None;
        }

        if idx > self.length >> 1 {
            self.get_by_idx_from_tail(self.length - idx - 1)
        } else {
            self.get_by_idx_from_head(idx)
        }
    }

    fn get_by_idx_from_tail(&self, idx: usize) -> Option<&T> {
        let mut cur_p = self.tail.as_ref();
        let mut cur_idx = 0;
        while let Some(node) = cur_p {
            unsafe {
                if cur_idx == idx {
                    return Some(&(*node.as_ptr()).elem);
                }
                cur_p = (*node.as_ptr()).prev.as_ref();
            }
            cur_idx += 1;
        }
        None
    }

    fn get_by_idx_from_head(&self, idx: usize) -> Option<&T> {
        let mut cur_p = self.head.as_ref();
        let mut cur_idx = 0;
        while let Some(node) = cur_p {
            unsafe {
                if cur_idx == idx {
                    return Some(&(*node.as_ptr()).elem);
                }
                cur_p = (*node.as_ptr()).next.as_ref();
            }
            cur_idx += 1;
        }
        None
    }

    // idx in range [0, len)
    pub fn get_mut_by_idx(&self, idx: usize) -> Option<&mut T> {
        if idx >= self.length {
            return None;
        }

        if idx > self.length >> 1 {
            self.get_mut_by_idx_from_tail(self.length - idx - 1)
        } else {
            self.get_mut_by_idx_from_head(idx)
        }
    }

    fn get_mut_by_idx_from_tail(&self, idx: usize) -> Option<&mut T> {
        let mut cur_p = self.tail.as_ref();
        let mut cur_idx = 0;
        while let Some(node) = cur_p {
            unsafe {
                if cur_idx == idx {
                    return Some(&mut (*node.as_ptr()).elem);
                }
                cur_p = (*node.as_ptr()).prev.as_ref();
            }
            cur_idx += 1;
        }
        None
    }

    fn get_mut_by_idx_from_head(&self, idx: usize) -> Option<&mut T> {
        let mut cur_p = self.head.as_ref();
        let mut cur_idx = 0;
        while let Some(node) = cur_p {
            unsafe {
                if cur_idx == idx {
                    return Some(&mut (*node.as_ptr()).elem);
                }
                cur_p = (*node.as_ptr()).next.as_ref();
            }
            cur_idx += 1;
        }
        None
    }

    pub fn insert_by_index(&mut self, idx: usize, elem: T) {
        if idx > self.length {
            return;
        }

        if idx == 0 {
            self.push_front(elem);
            return;
        }

        if idx == self.length {
            self.push_back(elem);
            return;
        }

        let mut new_node = Box::new(Node::new(elem)); // 放在堆上

        let mut cur_p = self.head;
        let mut cur_idx = 0;
        while let Some(cur_node) = cur_p {
            if cur_idx + 1 == idx {
                new_node.prev = cur_p;
                unsafe {
                    new_node.next = (*cur_node.as_ptr()).next;
                    let new_node_ptr = NonNull::new(Box::into_raw(new_node)); // !!!这里不能把into_raw写成&mut *new_node, 否则boxdrop后指针依然存在，会导致野指针
                    if let Some(next_node) = (*cur_node.as_ptr()).next {
                        // 因为去掉了插最后一个点的情况，所以这里一定是Some
                        (*next_node.as_ptr()).prev = new_node_ptr;
                    }
                    (*cur_node.as_ptr()).next = new_node_ptr;
                }
                self.length += 1;
                break;
            }
            unsafe { cur_p = (*cur_node.as_ptr()).next }
            cur_idx += 1;
        }
    }
}

impl<T: Debug> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug> Display for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut cur_p = self.head;
        let mut dis_str = String::from("");
        while let Some(cur_node) = cur_p {
            unsafe {
                cur_p = (*cur_node.as_ptr()).next;
                dis_str += format!("{:?}=>", (*cur_node.as_ptr()).elem).as_str();
            }
        }

        let mut cur_p = self.tail;
        while let Some(cur_node) = cur_p {
            unsafe {
                cur_p = (*cur_node.as_ptr()).prev;
                dis_str += format!("{:?}<=", (*cur_node.as_ptr()).elem).as_str();
            }
        }
        f.pad(&dis_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_index_test() {
        let mut l = List::new();

        // 1 -> 2 -> 3
        l.push_front(3);
        l.push_front(2);
        l.push_front(1);

        assert_eq!(l.get_by_idx(0), Some(&1));
        assert_eq!(l.get_by_idx(1), Some(&2));
        assert_eq!(l.get_by_idx(2), Some(&3));
        assert_eq!(l.get_by_idx(3), None);
    }

    #[test]
    fn get_mut_index_test() {
        let mut l = List::new();

        // 1 -> 2 -> 3
        l.push_front(3);
        l.push_front(2);
        l.push_front(1);

        assert_eq!(l.get_by_idx(3), None);

        let elem = l.get_mut_by_idx(0);
        assert_eq!(elem, Some(&mut 1));
        if let Some(n) = elem {
            *n = 4;
        }
        assert_eq!(l.get_by_idx(0), Some(&4));
    }

    #[test]
    fn insert_by_idx_test() {
        let mut l = List::new();

        // 1 -> 2 -> 3
        l.push_front(3);
        l.push_front(2);
        l.push_front(1);

        // -1 -> 1 -> 2 -> 3
        l.insert_by_index(0, -1);

        // -1 -> 1 -> 2 -> 3 -> 4
        l.insert_by_index(4, 4);

        // -1 -> 1 -> 7 -> 2 -> 3 -> 4
        l.insert_by_index(2, 7);

        // -1 -> 1 -> 7 -> 2 -> 3 -> 4
        l.insert_by_index(7, 4);

        assert_eq!(l.get_by_idx(0), Some(&-1));
        assert_eq!(l.get_by_idx(1), Some(&1));
        assert_eq!(l.get_by_idx(2), Some(&7));
        assert_eq!(l.get_by_idx(3), Some(&2));
        assert_eq!(l.get_by_idx(4), Some(&3));
        assert_eq!(l.get_by_idx(5), Some(&4));
        assert_eq!(l.get_by_idx(6), None);
    }

    #[test]
    fn same_address_test() {
        let mut new_node = Box::new(Node::new(3));
        let new_node_ptr2 = NonNull::new(&mut *new_node);
        let new_node_ptr = NonNull::new(Box::into_raw(new_node));
        assert_eq!(new_node_ptr, new_node_ptr2);
    }
}
