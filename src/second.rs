// 不可变队列 并发安全
use std::sync::Arc;

pub struct List<T> {
    head: Link<T>,
}

type Link<T> = Option<Arc<Node<T>>>;

struct Node<T> {
    elem: T,
    next: Link<T>,
}

impl<T> List<T> {
    pub fn new() -> Self {
        List { head: None }
    }

    // 将elem加到链表的第一个元素
    // 不改变内部结构，返回一个新的链表
    pub fn prepend(&self, elem: T) -> List<T> {
        List {
            head: Some(Arc::new(Node {
                elem,
                next: self.head.clone(), // 引用计数加一
            })),
        }
    }

    // 移除第一个元素
    pub fn tail(&self) -> List<T> {
        List {
            // head 不能给move，所以得用ref, ref相当于将外面的引用放到option里面
            // 所以这里自动将head变成&head了
            // and_then是处理option的有值状态的map，之所以不用map是因为map会再包一层option
            head: self.head.as_ref().and_then(|node| node.next.clone()),
        }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut cur_ref = self.head.take();
        while let Some(node) = cur_ref {
            // 释放到第一个被多个引用的节点
            if let Ok(mut node) = Arc::try_unwrap(node) {
                cur_ref = node.next.take()
            } else {
                break;
            }
        }
    }
}

pub struct ListIter<'a, T>(Option<&'a Node<T>>);

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.map(|node| {
            self.0 = node.next.as_deref();
            &node.elem
        })
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = ListIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        ListIter(self.head.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter_test() {
        let mut l = List::<i32>::new();
        l = l.prepend(1).prepend(2).prepend(3);

        let mut iter = l.into_iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), None);
    }
}
