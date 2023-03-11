use std::mem;

pub struct List<T> {
    head: Link<T>, // 专门有一个pub的对外数据结构, 否则rust要求pub enum的成员必须全部是pub
}

type Link<T> = Option<Box<Node<T>>>;

struct Node<T> {
    elem: T,
    nxt: Link<T>,
}

impl<T> List<T> {
    pub fn new() -> Self {
        Self { head: None }
    }

    // 头插法
    pub fn push(&mut self, elem: T) {
        let new_node = Node {
            elem,
            //nxt: self.head.clone(), // 这里没法转移所有权，只能clone，性能就很差
            nxt: self.head.take(), // 将empty放入原来head中，将原来head的值取出给nxt 取出来的值是拥有所有权的
        };
        self.head = Some(Box::new(new_node));
    }

    pub fn pop(&mut self) -> Option<T> {
        // match option { None => None, Some(x) => Some(y) } 这段代码可以直接使用 map 方法代替，map 会对 Some(x) 中的值进行映射，最终返回一个新的 Some(y) 值。
        self.head.take().map(|node| {
            self.head = node.nxt;
            node.elem
        })
    }

    pub fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.elem) // 将内部引用交给外部
    }

    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.head.as_mut().map(|node| &mut node.elem)
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut cur_link = mem::replace(&mut self.head, None); // 取出头的值
        while let Some(mut boxed_node) = cur_link {
            // 只要头部还有值, 将头部的指针换掉，就不会发生deallocate的行为
            cur_link = mem::replace(&mut boxed_node.nxt, None);
            // boxed_node现在是一个nil了，可以安全drop
        }
    }
}

impl<T> IntoIterator for List<T> {
    type Item = T;
    type IntoIter = ListIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        ListIntoIter(self)
    }
}

// 为何不直接为list实现iter
pub struct ListIntoIter<T>(List<T>);

impl<T> Iterator for ListIntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

// 这种实现只要当list 被drop以后就会失效
pub struct ListIter<'a, T>(Option<&'a Node<T>>);

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.map(|node| {
            self.0 = node.nxt.as_deref();
            &node.elem
        })
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = ListIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        match &self.head {
            None => ListIter(None),
            Some(node) => ListIter(Some(node.as_ref())),
        }
    }
}

pub struct ListMutIter<'a, T>(Option<&'a mut Node<T>>);

impl<'a, T> Iterator for ListMutIter<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        // 不可变引用不能被copy，这里需要特殊处理
        // 将引用内值取出来，然后将self的值换成None
        self.0.take().map(|node| {
            self.0 = node.nxt.as_deref_mut();
            &mut node.elem
        })
    }
}

impl<'a, T> IntoIterator for &'a mut List<T> {
    type Item = &'a mut T;
    type IntoIter = ListMutIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        match &mut self.head {
            None => ListMutIter(None),
            Some(node) => ListMutIter(Some(node.as_mut())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut list = List::<i32>::new();

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

    #[test]
    fn peak_test() {
        let mut list = List::<i32>::new();

        list.push(1);
        list.push(2);
        list.push(3);

        assert_eq!(list.peek_mut(), Some(&mut 3));

        if let Some(n) = list.peek_mut() {
            *n = 4;
        }

        assert_eq!(list.peek(), Some(&4))
    }

    #[test]
    fn into_iter_test() {
        let mut l = List::<i32>::new();
        l.push(1);
        l.push(2);
        l.push(3);

        let mut iter = l.into_iter();
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_test() {
        let mut l = List::<i32>::new();
        l.push(1);
        l.push(2);
        l.push(3);

        let mut iter = (&l).into_iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), None);

        let mut iter = (&l).into_iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_mut_test() {
        let mut list = List::new();
        list.push(1);
        list.push(2);
        list.push(3);

        let mut iter = (&mut list).into_iter();
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next(), Some(&mut 1));
    }
}
