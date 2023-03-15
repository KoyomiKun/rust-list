// 基于CAS的无锁skiplist

use std::{marker::PhantomData, mem, sync::atomic::AtomicPtr};

// 用一个word可以指针的数据结构
trait Pointable {
    // 对齐类型：
    // P 的 align_of 的计算过程:

    //i: 如果一次读取完毕, 需读取 4 个字节, 所以对齐到 4 可以很方便的读取 i.
    //s: 如果一次读取完毕, 需读取 1 个字节, 所以对齐到 1 就可以, 但 i 需要对齐到 4, 所以对齐仍为 4.
    //l: 如果一次读取完毕, 需读取 8 个字节, 所以对齐到 8 可以很方便的读取 l. 于是 P 的对齐就是 8.
    //P 的 align_of = max({size_of(f)|f∈P}) = max(1, 4, 8) = 8

    //P 的 size_of 的计算过程:

    //P 的 align_of = 8, 意味着读取 P 的时候, 每次读取 8 个字节.

    //第一次读取 8 个字节, 其中包含了 i:i32 和 s:i8
    //第二次读取 8 个字节, 包含了 l:i64
    //P 的 size_of = 对齐 * 读取次数 = 8 + 8 = 16
    //
    // 所以，对齐就是能保证一个数据结构一次性读入的是完整的成员的所需bytes数
    const ALIGN: usize;

    // 自定义初始化函数
    type Init;

    // 传入初始化函数，返回一个空元组结构体?的裸指针
    unsafe fn init(init: Self::Init) -> *mut ();

    unsafe fn deref<'a>(ptr: *mut ()) -> &'a Self;

    unsafe fn deref_mut<'a>(ptr: *mut ()) -> &'a mut Self;

    unsafe fn drop(ptr: *mut ());
}

impl<T> Pointable for T {
    const ALIGN: usize = mem::align_of::<T>();

    type Init = T;

    // 将init放到堆上，并将堆指针强制转换为匿名空元组指针
    unsafe fn init(init: Self::Init) -> *mut () {
        Box::into_raw(Box::new(init)).cast::<()>()
    }

    // 将ptr强制转换为T类型指针后解引用返回
    unsafe fn deref<'a>(ptr: *mut ()) -> &'a Self {
        &*(ptr as *const T)
    }

    unsafe fn deref_mut<'a>(ptr: *mut ()) -> &'a mut Self {
        &mut *ptr.cast::<T>()
    }

    unsafe fn drop(ptr: *mut ()) {
        drop(Box::from_raw(ptr.cast::<T>()))
    }
}

//PhantomData是告诉编译器：请以PhantomData中泛型T的样子来看待我，尽管我内部的设计与实现并不符合对应类型的约束。
struct Owned<T: ?Sized + Pointable> {
    data: *mut (),
    _marker: PhantomData<Box<T>>,
}

// T是一个编译时不确定大小（堆上）的对象，并且可以被转为堆指针
struct Atomic<T: ?Sized + Pointable> {
    data: AtomicPtr<()>,
}

impl<T: ?Sized + Pointable> Atomic<T> {
    pub fn init(init: T::Init) -> Atomic<T> {
        Self::from(Owned)
    }
}
