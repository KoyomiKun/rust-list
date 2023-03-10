use std::mem;

pub struct List {
    head: Link, // 专门有一个pub的对外数据结构, 否则rust要求pub enum的成员必须全部是pub
}

enum Link {
    Nil,
    More(Box<Node>), // 用指针来触发空指针优化
}

struct Node {
    elem: i32,
    nxt: Link,
}

impl List {
    pub fn new() -> Self {
        Self { head: Link::Nil }
    }

    // 头插法
    pub fn push(&mut self, elem: i32) {
        let new_node = Node {
            elem,
            //nxt: self.head.clone(), // 这里没法转移所有权，只能clone，性能就很差
            nxt: std::mem::replace(&mut self.head, Link::Nil), // 将empty放入原来head中，将原来head的值取出给nxt 取出来的值是拥有所有权的
        };
        self.head = Link::More(Box::new(new_node));
    }

    pub fn pop(&mut self) -> Option<i32> {
        match std::mem::replace(&mut self.head, Link::Nil) {
            Link::Nil => None,
            Link::More(node) => {
                self.head = node.nxt; // 这里如果不用replace，node只有引用，没法赋值给head
                Some(node.elem)
            }
        }
    }
}

impl Drop for List {
    fn drop(&mut self) {
        let mut cur_link = mem::replace(&mut self.head, Link::Nil); // 取出头的值
        while let Link::More(mut boxed_node) = cur_link {
            // 只要头部还有值, 将头部的指针换掉，就不会发生deallocate的行为
            cur_link = mem::replace(&mut boxed_node.nxt, Link::Nil);
            // boxed_node现在是一个nil了，可以安全drop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut list = List::new();

        assert_eq!(list.pop(), None);

        list.push(1);
        list.push(2);
        list.push(3);

        assert_eq!(list.pop(), Some(3));
        assert_eq!(list.pop(), Some(2));

        list.push(4);
        list.push(5);

        assert_eq!(list.pop(), Some(5));
        assert_eq!(list.pop(), Some(4));
        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), None);
    }

    #[test]
    fn long_list_test() {
        let mut l = List::new();
        for i in 0..100000 {
            l.push(i);
        }
        drop(l) // 不做特殊处理，由于指针的关系，不是尾递归，会爆栈
    }
}
