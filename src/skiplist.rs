use rand;
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
    rng: rand::ThreadRng,

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
            rng: rand::thread_rng(),
            head: NonNull::new(Box::into_raw(Box::new(Node {
                next: Vec::with_capacity(max_level),
                key: T::default(),
            }))),
            tmp: Vec::with_capacity(max_level),
        }
    }

    pub fn get(&self, key: T) -> Option<&T> {
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

        if let Some(next_node) = next {
            unsafe {
                if (*next_node.as_ptr()).key == key {
                    (*next_node.as_ptr()).key = key
                }
            }
            return;
        };

        let new_node = Node {
            key,
            next: Vec::with_capacity(self.max_level),
        };
        let new_node_ptr = NonNull::new(Box::into_raw(Box::new(new_node)));
        let level = self.get_random_level();
        for i in self.current_level..level {
            if let Some(node) = self.head {
                unsafe {
                    (*node.as_ptr()).next.push(Some(new_node_ptr));
                }
                self.tmp[i] = Some(node);
            };
            self.current_level += 1;
        }

        for i in 0..self.current_level {
            self.tmp[i]
                .take()
                .map(|prev_node| unsafe { (*prev_node.as_ptr()).next = new_node_ptr })
        }
    }

    fn get_random_level(&self) -> usize {
        0
    }
}
