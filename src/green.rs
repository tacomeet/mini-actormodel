use nix::sys::mman::{mprotect, ProtFlags};
use nix::unistd::sysconf;
use rand;
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
// const PAGE_SIZE: usize = 4 * 1024;
const PAGE_SIZE: usize = sysconf(_SC_PAGESIZE) as usize;

struct Context {
    regs: Registers,
    stack: *mut u8,
    stack_layout: Layout,
    entry: Entry,
    // スレッドID
    id: u64,
}

impl Context {
    fn get_regs_mut(&mut self) -> &mut Registers {
        &mut self.regs as *mut Registers
    }

    fn get_regs(&self) -> &Registers {
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
            regs: regs,
            stack: stack,
            stack_layout: layout,
            entry: func,
            id: id,
        }
    }
}
