use std::ptr::NonNull;

pub struct SkipList<T>
where
    T: PartialOrd + Eq + Default,
{
    max_level: usize,
    current_level: usize,
    current_len: usize,
    ratio: u8,
    head: Link<T>,

    tmp: Vec<Link<T>>,
}

type Link<T> = Option<NonNull<Node<T>>>;

struct Node<T> {
    next: Vec<Link<T>>,
    key: T,
}

impl<T: PartialOrd + Eq + Default> SkipList<T> {
    pub fn new(max_level: usize, ratio: u8) -> Self {
        Self {
            max_level,
            current_len: 0,
            current_level: 0,
            ratio,
            head: NonNull::new(Box::into_raw(Box::new(Node {
                next: Vec::with_capacity(max_level),
                key: T::default(),
            }))),
            tmp: Vec::with_capacity(max_level),
        }
    }

    pub fn get(key: T) -> T {
        unimplemented!()
    }

    pub fn set(&mut self, key: T) {
        let mut prev = self.head;
        let mut next;
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

        let level = self.get_random_level();
        if level > self.current_level {}
    }

    fn get_random_level(&self) -> usize {
        0
    }
}
