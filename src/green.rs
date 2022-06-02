use nix::sys::mman::{mprotect, ProtFlags};
use std::alloc::{alloc, dealloc, Layout};
use std::collections::{HashMap, HashSet, LinkedList};
use std::ffi::c_void;
use std::ptr;

// 全てのスレッドが終了時に戻ってくる先
static mut CTX_MAIN: Option<Box<Registers>> = None;

// 不要なスタック領域
static mut UNUSED_STACK: (*mut u8, Layout) = (ptr::null_mut(), Layout::new::<u8>());

// スレッドの実行キュー
static mut CONTEXTS: LinkedList<Box<Context>> = LinkedList::new();

// スレッドIDの集合
static mut ID: *mut HashSet<u64> = ptr::null_mut();

// message queue ('mail box' in Erlang)
static mut MESSAGES: *mut MappedList<u64> = ptr::null_mut();

// waiting thread set
static mut WAITING: *mut HashMap<u64, Box<Context>> = ptr::null_mut();

// 構造体の内部メモリ表現がC言語と同じであることを指定
// レジスタの値を保存する構造体
#[repr(C)]
struct Registers {
    rbx: u64,
    rbp: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rsp: u64,
    rdx: u64,
}

impl Registers {
    fn new(rsp: u64) -> Registers {
        Registers {
            rbx: 0,
            rbp: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rsp,
            // スレッド開始のエントリポイントとなる関数のアドレスを保存
            rdx: entry_point as u64,
        }
    }
}

extern "C" {
    fn set_context(ctx: *mut Registers) -> u64;
    fn switch_context(ctx: *const Registers) -> !;
}

// スレッド開始時に実行する関数の型
type Entry = fn();

// ページサイズ Linuxだと4KiB
const PAGE_SIZE: usize = 4 * 1024;
// const PAGE_SIZE: usize = sysconf() as usize;

struct MappedList<T> {
    map: HashMap<u64, LinkedList<T>>,
}

impl<T> MappedList<T> {
    fn new() -> Self {
        MappedList {
            map: HashMap::new(),
        }
    }

    fn push_back(&mut self, key: u64, value: T) {
        if let Some(list) = self.map.get_mut(&key) {
            list.push_back(value);
        } else {
            let mut list = LinkedList::new();
            list.push_back(value);
            self.map.insert(key, list);
        }
    }

    fn popo_front(&mut self, key: u64) -> Option<T> {
        if let Some(list) = self.map.get_mut(&key) {
            let val = list.pop_front();
            if list.is_empty() {
                self.map.remove(&key);
            }
            val
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.map.clear();
    }
}

struct Context {
    regs: Registers,
    stack: *mut u8,
    stack_layout: Layout,
    entry: Entry,
    // スレッドID
    id: u64,
}

impl Context {
    fn get_regs_mut(&mut self) -> *mut Registers {
        &mut self.regs as *mut Registers
    }

    fn get_regs(&self) -> *const Registers {
        &self.regs as *const Registers
    }

    #[inline(never)]
    fn new(func: Entry, stack_size: usize, id: u64) -> Self {
        // スタック領域の確保
        let layout = Layout::from_size_align(stack_size, PAGE_SIZE).unwrap();
        let stack = unsafe { alloc(layout) };

        // スタック用のガードページ設定
        unsafe { mprotect(stack as *mut c_void, PAGE_SIZE, ProtFlags::PROT_NONE).unwrap() };
        // レジスタの初期化（stackは高アドレス -> 低アドレス）
        let regs = Registers::new(stack as u64 + stack_size as u64);

        // コンテキストの初期化とリターン
        Context {
            regs,
            stack,
            stack_layout: layout,
            entry: func,
            id,
        }
    }
}

fn get_id() -> u64 {
    loop {
        let rnd = rand::random::<u64>();
        unsafe {
            // check if ID is already used
            if !(*ID).contains(&rnd) {
                (*ID).insert(rnd);
                return rnd;
            }
        }
    }
}

pub fn spawn(func: Entry, stack_size: usize) -> u64 {
    unsafe {
        // create new thread id
        let id = get_id();
        // create context and add it to the queue
        CONTEXTS.push_back(Box::new(Context::new(func, stack_size, id)));
        schedule();
        id
    }
}

pub fn schedule() {
    unsafe {
        // if there is only itself in the queue, return
        if CONTEXTS.len() == 1 {
            return;
        }

        // get current context
        let mut ctx = CONTEXTS.pop_front().unwrap();
        // get pointer to registers
        let regs = ctx.get_regs_mut();
        CONTEXTS.push_back(ctx);

        // save current registers
        if set_context(regs) == 0 {
            // context switch to the new thread
            let next = CONTEXTS.front().unwrap();
            switch_context((**next).get_regs());
        }

        // remove unused stack
        rm_unused_stack();
    }
}

#[no_mangle]
pub extern "C" fn entry_point() {
    unsafe {
        // call specified entry function
        let ctx = CONTEXTS.front().unwrap();
        ((**ctx).entry)();

        let ctx = CONTEXTS.pop_front().unwrap();

        (*ID).remove(&ctx.id);

        // set pointer to unused stack to the global variable
        UNUSED_STACK = ((*ctx).stack, (*ctx).stack_layout);

        match CONTEXTS.front() {
            Some(c) => {
                // context switch to the next thread
                switch_context((**c).get_regs());
            }
            None => {
                // if there is no thread, context switch to main thread
                if let Some(c) = &CTX_MAIN {
                    switch_context(&**c as *const Registers);
                }
            }
        }
    }
    panic!("entry_point");
}

pub fn spawn_from_main(func: Entry, stack_size: usize) {
    unsafe {
        // if already initialized, panic
        if let Some(_) = &CTX_MAIN {
            panic!("spawn_from_main is called twice");
        }

        // create context for main function
        CTX_MAIN = Some(Box::new(Registers::new(0)));
        if let Some(ctx) = &mut CTX_MAIN {
            // initialize global variable
            // let mut msgs = MappedList::new();
            // MESSAGES = &mut msgs as *mut MappedList<u64>;

            let mut waiting = HashMap::new();
            WAITING = &mut waiting as *mut HashMap<u64, Box<Context>>;

            let mut ids = HashSet::new();
            ID = &mut ids as *mut HashSet<u64>;

            // save itsself
            if set_context(&mut **ctx as *mut Registers) == 0 {
                // create and execute first thread
                CONTEXTS.push_back(Box::new(Context::new(func, stack_size, 0)));
                let first = CONTEXTS.front().unwrap();
                switch_context(first.get_regs());
            }

            // after all the threads are finished
            rm_unused_stack();

            CTX_MAIN = None;
            CONTEXTS.clear();
            MESSAGES = ptr::null_mut();
            WAITING = ptr::null_mut();
            ID = ptr::null_mut();

            // guarantee lifetime
            // msgs.clear();
            waiting.clear();
            ids.clear();
        }
    }
}

unsafe fn rm_unused_stack() {
    if UNUSED_STACK.0 != ptr::null_mut() {
        // remove guard page
        mprotect(
            UNUSED_STACK.0 as *mut c_void,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
        )
        .unwrap();
        dealloc(UNUSED_STACK.0, UNUSED_STACK.1);
        // prevent double free
        UNUSED_STACK = (ptr::null_mut(), Layout::new::<u8>());
    }
}
