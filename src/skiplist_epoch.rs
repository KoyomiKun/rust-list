use std::{
    alloc::{dealloc, Layout},
    mem,
    ops::{Bound, Deref, Index},
    ptr,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

use crossbeam::epoch::{Atomic, Collector, Guard, Shared};

const HEIGHT_BITS: usize = 5; // bits number for height
const MAX_HEIGHT: usize = 1 << HEIGHT_BITS;
const HEIGHT_MASK: usize = (1 << HEIGHT_BITS) - 1;

pub struct Skiplist<K, V> {
    head: Head<K, V>,
    collector: Collector,
    //hot_data:
}

unsafe impl<K: Send + Sync, V: Send + Sync> Send for Skiplist<K, V> {}
unsafe impl<K: Send + Sync, V: Send + Sync> Sync for Skiplist<K, V> {}

impl<K, V> Skiplist<K, V> {
    pub fn new() -> Self {
        Self {
            head: Head::new(),
            collector: Default::default(),
        }
    }

    fn check_guard(&self, guard: &Guard) {
        if let Some(c) = guard.collector() {
            assert!(c == &self.collector)
        }
    }
}

impl<K, V> Skiplist<K, V>
where
    K: Ord,
{
    // get the smallest key
    pub fn front<'a, 'g>(&'a self, guard: &'g Guard) -> Option<Entry<'a, 'g, K, V>> {
        self.check_guard(guard);

        let n = self.next_node(&self.head, Bound::Unbounded, guard)?;
        Some(Entry {
            parent: self,
            node: n,
            guard,
        })
    }

    fn next_node<'a>(
        &'a self,
        pred: &'a Tower<K, V>,
        lower_bound: Bound<&K>,
        guard: &'a Guard,
    ) -> Option<&'a Node<K, V>> {
        // 这里可以用consume而不是acquire TODO: WHY?
        // 1. 取出pred 0层的值
        let mut curr = pred[0].load_consume(guard);

        // 这个tag是预留给指针空位的
        // 一个指向类型T的指针p, 末尾的 (mem::align_of::<T>().trailing_zeros) 位是空的，可以放tag
        // 比如usize是按照8个字节对齐的，所以指针每次都是移动8字节的位置
        // 一开始在0b0000 走一次就在 0b1000 两次在0b1 1000 后面三位永远不会有用，可以放tag
        // 信息，当然tag也必须小于 1 << 3
        //
        // 2. 检查这个点是否已经被删除
        if curr.tag() == 1 {
            // 2.1 如果被删除，则查找第一个大于lower_bound的节点 返回该节点
            return self.search_bound(lower_bound, false, guard);
        }

        unsafe {
            // 2.2 如果没有被删除，则找下一个节点
            //
            // 遍历到链尾
            while let Some(c) = curr.as_ref() {
                // 后继节点是0层
                let succ = c.tower[0].load_consume(guard);

                if succ.tag() == 1 {
                    // 尝试交换当前位置和前一个节点的0层位置
                    if let Some(c) = self.help_unlink(&pred[0], c, succ, guard) {
                        curr = c;
                        continue;
                    } else {
                        return self.search_bound(lower_bound, false, guard);
                    }
                }

                return Some(c);
            }
        }
        None
    }

    // unlikely to invoke
    #[cold]
    /// 如果pred和curr一致，则将succ放入pred；并降低curr的引用计数，返回放入的succ地址
    /// 否则，返回None
    unsafe fn help_unlink<'a>(
        &'a self,
        pred: &'a Atomic<Node<K, V>>,
        curr: &'a Node<K, V>,
        succ: Shared<'a, Node<K, V>>,
        guard: &'a Guard,
    ) -> Option<Shared<'a, Node<K, V>>> {
        match pred.compare_exchange(
            Shared::from(curr as *const _),
            succ.with_tag(0),
            Ordering::Release,
            Ordering::Relaxed,
            guard,
        ) {
            Ok(_) => {
                curr.decrement(guard);
                Some(succ.with_tag(0))
            }
            Err(_) => None,
        }
    }

    // 查找第一个/最后一个 节点，该节点 大于/等于/小于 给定的key(bound)
    //
    // 如果 upper_bound 是true， 则查找最后一个小于等于 key的
    // 如果 upper_bound 是false，则查找第一个大于等于 key的
    fn search_bound<'a, Q>(
        &'a self,
        bound: Bound<&Q>,
        upper_bound: bool,
        guard: &'a Guard,
    ) -> Option<&'a Node<K, V>> {
        unimplemented!()
    }
}
struct Head<K, V> {
    pointers: [Atomic<Node<K, V>>; MAX_HEIGHT],
}

impl<K, V> Head<K, V> {
    pub fn new() -> Self {
        Self {
            pointers: Default::default(),
        }
    }
}

impl<K, V> Deref for Head<K, V> {
    type Target = Tower<K, V>;
    // head 可以被转换成tower，因为head本身是一个0长tower
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const Tower<K, V>) }
    }
}

struct Node<K, V> {
    value: V,
    key: K,

    refs_and_height: AtomicUsize,
    tower: Tower<K, V>,
}

impl<K, V> Node<K, V> {
    unsafe fn decrement(&self, guard: &Guard) {
        if self
            .refs_and_height
            .fetch_sub(1 << HEIGHT_BITS, Ordering::Release)
            >> HEIGHT_BITS
            == 1
        {
            fence(Ordering::Acquire);
            guard.defer_unchecked(move || Self::finalize(self))
        }
    }

    fn height(&self) -> usize {
        (self.refs_and_height.load(Ordering::Relaxed) & HEIGHT_MASK) + 1
    }

    unsafe fn dealloc(ptr: *mut Self) {
        let height = (*ptr).height();
        let layout = Self::get_layout(height);
        dealloc(ptr.cast::<u8>(), layout)
    }

    unsafe fn get_layout(height: usize) -> Layout {
        assert!((1..=MAX_HEIGHT).contains(&height));

        let size_self = mem::size_of::<Self>();
        let align_self = mem::align_of::<Self>();
        let size_pointer = mem::size_of::<Atomic<Self>>();

        Layout::from_size_align_unchecked(size_self + size_pointer * height, align_self)
    }

    unsafe fn finalize(ptr: *const Self) {
        let ptr = ptr as *mut Self;

        ptr::drop_in_place(&mut (*ptr).key);
        ptr::drop_in_place(&mut (*ptr).value);

        Node::dealloc(ptr)
    }
}

struct Tower<K, V> {
    pointers: [Atomic<Node<K, V>>; 0],
}

impl<K, V> Index<usize> for Tower<K, V> {
    type Output = Atomic<Node<K, V>>;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*(&self.pointers as *const Atomic<Node<K, V>>).add(index) }
    }
}

// parent的生命周期必然大于node/guard的生命周期
pub struct Entry<'a: 'g, 'g, K, V> {
    parent: &'a Skiplist<K, V>,
    node: &'g Node<K, V>,
    guard: &'g Guard,
}
