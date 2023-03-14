use rand::{rngs::ThreadRng, Rng};
use std::{fmt::Display, ptr::NonNull};
extern crate test;

pub struct SkipList<T>
where
    T: PartialOrd + Default,
{
    max_level: usize,
    current_level: usize,
    current_len: usize,
    ratio: usize,
    head: Link<T>,
    rng: ThreadRng,

    tmp: Vec<Link<T>>,
}

type Link<T> = Option<NonNull<Node<T>>>;

struct Node<T> {
    next: Vec<Link<T>>,
    key: T,
}

impl<T: PartialOrd + Display + Default> SkipList<T> {
    pub fn new(max_level: usize, ratio: usize) -> Self {
        Self {
            max_level,
            current_len: 0,
            current_level: 0,
            ratio,
            rng: rand::thread_rng(),
            head: NonNull::new(Box::into_raw(Box::new(Node {
                next: Vec::with_capacity(max_level),
                key: T::default(),
            }))),
            tmp: Vec::with_capacity(max_level),
        }
    }

    // TODO: tmp 能不能拿出来，不用mut
    pub fn get(&mut self, key: T) -> Option<&T> {
        let mut prev = self.head;
        let mut next = None;
        for i in (0..self.current_level).rev() {
            unsafe {
                next = prev.and_then(|prev_ptr| (*prev_ptr.as_ptr()).next[i]);
                while let Some(node) = next {
                    if (*node.as_ptr()).key >= key {
                        break;
                    }
                    prev = next;
                    next = prev.and_then(|prev_ptr| (*prev_ptr.as_ptr()).next[i]);
                }
                self.tmp[i] = prev;
            }
        }

        if let Some(next_node) = next {
            unsafe {
                if (*next_node.as_ptr()).key == key {
                    return Some(&(*next_node.as_ptr()).key);
                }
            }
        };
        None
    }

    pub fn set(&mut self, key: T) {
        let mut prev = self.head;
        let mut next = None;
        for i in (0..self.current_level).rev() {
            unsafe {
                next = prev.and_then(|prev_ptr| (*prev_ptr.as_ptr()).next[i]);
                while let Some(node) = next {
                    if (*node.as_ptr()).key >= key {
                        break;
                    }
                    prev = next;
                    next = prev.and_then(|prev_ptr| (*prev_ptr.as_ptr()).next[i]);
                }

                self.tmp[i] = prev;
            }
        }

        if let Some(next_node) = next {
            unsafe {
                if (*next_node.as_ptr()).key == key {
                    (*next_node.as_ptr()).key = key
                }
            }
            return;
        };

        let level = self.get_random_level();
        let mut new_node = Node {
            key,
            next: Vec::with_capacity(self.max_level),
        };

        for _ in 0..=level {
            new_node.next.push(None);
        }

        let new_node_ptr = NonNull::new(Box::into_raw(Box::new(new_node)));
        for _ in self.current_level..level {
            if let Some(node) = self.head {
                unsafe {
                    (*node.as_ptr()).next.push(None);
                }
            };
            self.tmp.push(self.head);
            self.current_level += 1;
        }

        println!("level: {} current_level: {}", level, self.current_level);

        for i in 0..level {
            self.tmp[i].take().map(|prev_node| unsafe {
                let new_node = &mut *new_node_ptr.unwrap().as_ptr();
                new_node.next[i] = (*prev_node.as_ptr()).next[i];
                (*prev_node.as_ptr()).next[i] = new_node_ptr;
            });
        }

        self.current_len += 1;
    }

    fn get_random_level(&mut self) -> usize {
        let mut l = 1;
        for _ in 1..self.max_level {
            let gen_v: usize = self.rng.gen();
            if gen_v % self.ratio == 0 {
                l += 1;
            }
        }
        l
    }
}

impl<T: Default + PartialOrd + Display> Display for SkipList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("");
        for i in 0..self.current_level {
            unsafe {
                let mut next = self.head.and_then(|prev_ptr| (*prev_ptr.as_ptr()).next[i]);
                while let Some(node) = next {
                    s += format!("{}=>", (*node.as_ptr()).key).as_str();
                    next = (*node.as_ptr()).next[i];
                }
                s.push('\n');
            }
        }
        f.pad(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestK {
        k: i32,
        v: i32,
    }

    impl Default for TestK {
        fn default() -> Self {
            Self { k: 0, v: 0 }
        }
    }

    impl PartialEq for TestK {
        fn eq(&self, other: &Self) -> bool {
            self.k == other.k
        }
    }

    impl PartialOrd for TestK {
        fn gt(&self, other: &Self) -> bool {
            self.k > other.k
        }

        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.k.partial_cmp(&other.k)
        }
    }

    impl Display for TestK {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "[k: {}, v: {}]", self.k, self.v)
        }
    }

    #[test]
    fn basic_test() {
        let mut l = SkipList::new(32, 4);
        l.set(TestK { k: 1, v: 1 });
        l.set(TestK { k: 2, v: 1 });
        l.set(TestK { k: 3, v: 1 });
        l.set(TestK { k: 4, v: 1 });
        println!("{}", l);
        assert_eq!(l.get(TestK { k: 1, v: 0 }).unwrap().v, 1);
    }

    #[bench]
    fn basic_bench(b: &mut test::Bencher) {
        let mut r = rand::thread_rng();
        // 274,301 / 10000 = 27ns/op
        b.iter(|| {
            let mut l = SkipList::new(32, 4);
            for _ in 0..10000 {
                l.set(TestK {
                    k: r.gen(),
                    v: r.gen(),
                });
            }
        })
    }
}
