// lock-free link list based on epoch

use std::{
    fmt::{self, Debug},
    sync::atomic::{AtomicBool, Ordering},
};

use crossbeam::epoch::{Atomic, Owned};

pub struct List<K, V> {
    head: Link<K, V>,
}

type Link<K, V> = Atomic<Node<K, V>>;

struct Node<K, V> {
    kv: (K, Atomic<V>),
    active: AtomicBool,
    next: Link<K, V>,
    prev: Link<K, V>,
}

impl<K, V> Node<K, V> {
    pub fn new(k: K, v: V) -> Self {
        Self {
            kv: (k, Atomic::new(v)),
            active: AtomicBool::new(true),
            next: Atomic::null(),
            prev: Atomic::null(),
        }
    }
}

impl<K, V> Default for List<K, V> {
    fn default() -> Self {
        Self {
            head: Atomic::null(),
        }
    }
}

impl<K, V> List<K, V>
where
    K: Eq,
    V: Copy, // because get return V
{
    // V is allocated in heap, so do not return v directed
    // On  CAS failure, return old value's pointer
    // if success, return None
    pub fn insert(&self, kv: (K, V)) -> Option<*const V> {
        let guard = crossbeam::epoch::pin();

        let mut curr_p = &self.head;

        loop {
            let l = curr_p.load(Ordering::SeqCst, &guard);
            if l.is_null() {
                let ins = Owned::new(Node::new(kv.0, kv.1));
                self.head.store(ins, Ordering::Release);
                return None;
            }
            let raw = l.as_raw();
            let cur = unsafe { &*raw };
            // update value
            if &cur.kv.0 == &kv.0 && cur.active.load(Ordering::Acquire) {
                let ins = Owned::new(kv.1);
                let old = cur.kv.1.load(Ordering::SeqCst, &guard);
                match cur.kv.1.compare_exchange(
                    old,
                    ins,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                    &guard,
                ) {
                    Ok(_) => {
                        return None;
                    }
                    Err(e) => return Some(e.current.as_raw() as *const V),
                };
            }

            curr_p = &cur.next;

            if cur.next.load(Ordering::SeqCst, &guard).is_null() {
                let ins = Owned::new(Node::new(kv.0, kv.1));
                ins.prev.store(l, Ordering::Release);
                cur.next.store(ins, Ordering::Release);
                return None;
            }
        }
    }

    // return &V is dangerous, for it may be freed by epoch
    pub fn get(&self, k: &K) -> Option<V> {
        let guard = crossbeam::epoch::pin();

        let mut curr_p = &self.head;

        loop {
            let l = curr_p.load(Ordering::Acquire, &guard);
            if l.is_null() {
                return None;
            }

            let raw = l.as_raw();
            let curr_node = unsafe { &*raw };

            if &curr_node.kv.0 == k && curr_node.active.load(Ordering::Acquire) {
                return unsafe { Some(*curr_node.kv.1.load(Ordering::Acquire, &guard).as_raw()) };
            }

            curr_p = &curr_node.next;
        }
    }

    pub fn remove(&self, k: &K) -> bool {
        let guard = crossbeam::epoch::pin();

        let mut curr_p = &self.head;
        loop {
            let l = curr_p.load(Ordering::SeqCst, &guard);

            if l.is_null() {
                return false;
            }

            let raw = l.as_raw();
            let curr_node = unsafe { &*raw };

            if &curr_node.kv.0 == k && curr_node.active.load(Ordering::Acquire) {
                curr_node.active.store(false, Ordering::Release);

                let next = curr_node.next.load(Ordering::Acquire, &guard);
                let prev = curr_node.prev.load(Ordering::Acquire, &guard);

                if !next.is_null() {
                    if !prev.is_null() {
                        unsafe {
                            if (*prev.as_raw())
                                .next
                                .compare_exchange(
                                    l,
                                    next,
                                    Ordering::SeqCst,
                                    Ordering::Acquire,
                                    &guard,
                                )
                                .is_err()
                            {
                                return false;
                            };

                            if (*next.as_raw())
                                .prev
                                .compare_exchange(
                                    l,
                                    prev,
                                    Ordering::SeqCst,
                                    Ordering::Acquire,
                                    &guard,
                                )
                                .is_err()
                            {
                                return false;
                            };
                        }
                    } else {
                        unsafe {
                            if (*next.as_raw())
                                .prev
                                .compare_exchange(
                                    l,
                                    prev,
                                    Ordering::SeqCst,
                                    Ordering::Acquire,
                                    &guard,
                                )
                                .is_err()
                            {
                                return false;
                            };
                        }

                        if self
                            .head
                            .compare_exchange(l, next, Ordering::SeqCst, Ordering::Acquire, &guard)
                            .is_err()
                        {
                            return false;
                        }
                    }
                } else {
                    if !prev.is_null() {
                        unsafe {
                            if (*prev.as_raw())
                                .next
                                .compare_exchange(
                                    l,
                                    next,
                                    Ordering::SeqCst,
                                    Ordering::Acquire,
                                    &guard,
                                )
                                .is_err()
                            {
                                return false;
                            }
                        }
                    } else {
                        if self
                            .head
                            .compare_exchange(l, next, Ordering::SeqCst, Ordering::Acquire, &guard)
                            .is_err()
                        {
                            return false;
                        }
                    }
                }
                unsafe { guard.defer_unchecked(move || drop(l.into_owned())) }
                return true;
            }

            curr_p = &curr_node.next;
        }
    }
}

impl<K, V> Debug for List<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let guard = crossbeam::epoch::pin();
        let mut ret = String::new();
        let mut curr_p = &self.head;

        loop {
            let l = curr_p.load(Ordering::Acquire, &guard);
            if l.is_null() {
                return write!(f, "{}", ret);
            }

            let curr_node = unsafe { &*l.as_raw() };
            if curr_node.active.load(Ordering::Acquire) {
                ret.push_str("(");
                ret.push_str(&format!("{:?}", &curr_node.kv.0));
                ret.push_str(",");
                ret.push_str(&format!("{:?}", &curr_node.kv.0));
                ret.push_str("),");
            }

            curr_p = &curr_node.next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};
    use std::{sync::Arc, thread};

    #[test]
    fn concurr() {
        let list = Arc::new(List::default());
        let mut threads = vec![];
        let nthreads = 5;
        for _ in 0..nthreads {
            let new_list = list.clone();

            threads.push(thread::spawn(move || {
                let num_iterations = 100000;
                for _ in 0..num_iterations {
                    let mut rng = thread_rng();
                    let val = rng.gen_range(0..128);
                    let two = rng.gen_range(0..3);

                    if two % 3 == 0 {
                        println!("insert {val}");
                        new_list.insert((val, val));
                    } else if two % 3 == 1 {
                        let v = new_list.get(&val);
                        println!("check {val}");
                        if v.is_some() {
                            assert_eq!(v.unwrap(), val);
                        }
                    } else {
                        println!("remove {val}");
                        new_list.remove(&val);
                    }
                }
            }));
        }
        for t in threads {
            t.join().unwrap();
        }
    }

    //     #[test]
    //     fn hashmap_delete() {
    //         let handle = Map::with_capacity(8);
    //         handle.insert(1, 3);
    //         handle.insert(2, 5);
    //         handle.insert(3, 8);
    //         handle.insert(4, 3);
    //         handle.insert(5, 4);
    //         handle.insert(6, 5);
    //         handle.insert(7, 3);
    //         handle.insert(8, 3);
    //         handle.insert(9, 3);
    //         handle.insert(10, 3);
    //         handle.insert(11, 3);
    //         handle.insert(12, 3);
    //         handle.insert(13, 3);
    //         handle.insert(14, 3);
    //         handle.insert(15, 3);
    //         handle.insert(16, 3);
    //         assert_eq!(handle.get(&1).unwrap(), 3);
    //         assert_eq!(handle.remove(&1), true);
    //         assert_eq!(handle.get(&1), None);
    //         assert_eq!(handle.remove(&2), true);
    //         assert_eq!(handle.remove(&16), true);
    //         assert_eq!(handle.get(&16), None);
    //     }

    //     #[test]
    //     fn hashmap_basics() {
    //         let new_hashmap = Map::with_capacity(8); //init with 2 buckets
    //                                                      //input values
    //         new_hashmap.insert(1, 1);
    //         new_hashmap.insert(2, 5);
    //         new_hashmap.insert(12, 5);
    //         new_hashmap.insert(13, 7);
    //         new_hashmap.insert(0, 0);

    //         new_hashmap.insert(20, 3);
    //         new_hashmap.insert(3, 2);
    //         new_hashmap.insert(4, 1);

    //         assert_eq!(new_hashmap.insert(20, 5).unwrap(), 3); //repeated new
    //         assert_eq!(new_hashmap.insert(3, 8).unwrap(), 2); //repeated new

    //         new_hashmap.insert(3, 8); //repeated

    //         assert_eq!(new_hashmap.get(&20).unwrap(), 5);
    //         assert_eq!(new_hashmap.get(&12).unwrap(), 5);
    //         assert_eq!(new_hashmap.get(&1).unwrap(), 1);
    //         assert_eq!(new_hashmap.get(&0).unwrap(), 0);
    //         assert!(new_hashmap.get(&3).unwrap() != 2); // test that it changed

    //         // try the same assert_eqs
    //         assert_eq!(new_hashmap.get(&20).unwrap(), 5);
    //         assert_eq!(new_hashmap.get(&12).unwrap(), 5);
    //         assert_eq!(new_hashmap.get(&1).unwrap(), 1);
    //         assert_eq!(new_hashmap.get(&0).unwrap(), 0);
    //         assert!(new_hashmap.get(&3).unwrap() != 2); // test that it changed
    //     }
}
