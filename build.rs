use std::process::Command;

const ASM_DIR_PATH: &str = "asm";
const ASM_FILE: &str = "asm/context.S";
const O_FILE: &str = "asm/context.o";
const LIB_FILE: &str = "asm/libcontext.a";

fn main() {
    Command::new("cc")
        .args(&[ASM_FILE, "-c", "-fPIC", "-ggdb", "-o"])
        .arg(O_FILE)
        .status()
        .unwrap();
    Command::new("ar")
        .args(&["cruUs", LIB_FILE, O_FILE])
        .status()
        .unwrap();

    // asmをライブラリ検索パスに追加
    println!("cargo:rustc-link-search=native={}", ASM_DIR_PATH);
    // libcontext.aという静的ライブラリをリンク
    println!("cargo:rustc-link-lib=static=context");
    // asm/context.Sというファイルに依存
    println!("cargo:return-if-changed=asm/context.S");
}
