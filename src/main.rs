mod green;

use std::io::Write;

fn mash() {
    green::spawn(ortega, 2 * 1024 * 1024);
    for _ in 0..10 {
        std::io::stdout().write(b"Mash!\n").unwrap();
        green::schedule();
    }
}

fn ortega() {
    for _ in 0..10 {
        std::io::stdout().write(b"Gaia! \n").unwrap();
        green::schedule();
    }
}

fn gaia() {
    green::spawn(mash, 2 * 1024 * 1024);
    for _ in 0..10 {
        std::io::stdout().write(b"Gaia! \n").unwrap();
        green::schedule();
    }
}

fn main() {
    green::spawn_from_main(gaia, 2 * 1024 * 1024);
}
