

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