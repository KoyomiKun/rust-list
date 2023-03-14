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
                    (*next_node.as_ptr()).key = key;
                    return;
                }
            }
        };

        let level = self.get_random_level();
        let mut new_node = Node {
            key,
            next: Vec::with_capacity(self.max_level),
        };

        for _ in 0..level {
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

    pub fn delete(&mut self, key: T) -> Option<T> {
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
                    let target_node = Box::from_raw(next_node.as_ptr());
                    for i in 0..target_node.next.len() {
                        match self.tmp[i].take() {
                            Some(prev_node) => {
                                println!(
                                    "delete prev {} {:?} to next {:?}",
                                    i, prev_node, target_node.next[i]
                                );
                                (*prev_node.as_ptr()).next[i] = target_node.next[i];
                                if prev_node == self.head.unwrap()
                                    && (*prev_node.as_ptr()).next[i] == None
                                {
                                    self.current_level -= 1;
                                }
                            }
                            None => {
                                println!(
                                    "BUG: tmp prev {} is None, but current length is {}",
                                    i,
                                    target_node.next.len()
                                );
                            }
                        }
                    }

                    return Some(target_node.key);
                }
            }
        };
        None
    }

    fn get_random_level(&mut self) -> usize {
        let mut l = 0;
        for _ in 0..self.max_level {
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

    struct Fib(i32, i32);

    impl Iterator for Fib {
        type Item = i32;
        fn next(&mut self) -> Option<Self::Item> {
            Some(self.0 + self.1)
        }
    }

    #[derive(Debug)]
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

        // modify value
        l.set(TestK { k: 4, v: 7 });
        println!("{}", l);
        assert_eq!(l.get(TestK { k: 4, v: 0 }).unwrap().v, 7);

        // delete key
        assert_eq!(l.delete(TestK { k: 3, v: 0 }), Some(TestK { k: 3, v: 1 }));
        println!("{}", l);
        assert_eq!(l.delete(TestK { k: 7, v: 0 }), None);
        println!("{}", l);
    }

    //test skiplist::tests::delete_bench ... bench:           2 ns/iter (+/- 0)
    //test skiplist::tests::get_bench    ... bench:           8 ns/iter (+/- 0)
    //test skiplist::tests::set_bench    ... bench:      85,710 ns/iter (+/- 5,533)
    #[bench]
    fn set_bench(b: &mut test::Bencher) {
        let mut f = Fib(0, 1).into_iter();
        let mut l = SkipList::new(32, 4);

        b.iter(|| {
            let v = f.next().unwrap();
            for _ in 0..10000 {
                l.set(TestK { k: v, v });
            }
        })
    }

    #[bench]
    fn get_bench(b: &mut test::Bencher) {
        let mut f = Fib(0, 1).into_iter();
        let l = &mut SkipList::new(32, 4);
        for _ in 0..10000 {
            let v = f.next().unwrap();
            l.set(TestK { k: v, v });
        }
        let mut f = Fib(0, 1).into_iter();
        b.iter(move || {
            let v = f.next().unwrap();
            l.get(TestK { k: v, v });
        })
    }

    #[bench]
    fn delete_bench(b: &mut test::Bencher) {
        let mut f = Fib(0, 1).into_iter();
        let l = &mut SkipList::new(32, 4);
        for _ in 0..10000 {
            let v = f.next().unwrap();
            l.set(TestK { k: v, v });
        }
        b.iter(move || {
            let v = f.next().unwrap();
            l.delete(TestK { k: v, v });
        })
    }
}
