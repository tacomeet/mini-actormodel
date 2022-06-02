mod green;

use std::io::Write;

fn mash() {
    green::spawn(ortega, 2 * 1024 * 1024);
    for _ in 0..10 {
        std::io::stdout().write_all(b"Mash!\n").unwrap();
        green::schedule();
    }
}

fn ortega() {
    for _ in 0..10 {
        std::io::stdout().write_all(b"Ortega! \n").unwrap();
        green::schedule();
    }
}

fn gaia() {
    green::spawn(mash, 2 * 1024 * 1024);
    for _ in 0..10 {
        std::io::stdout().write_all(b"Gaia! \n").unwrap();
        green::schedule();
    }
}

fn producer() {
    let id = green::spawn(consumer, 2 * 1024 * 1024);
    for i in 0..10 {
        green::send(id, i);
    }
}

fn consumer() {
    for _ in 0..10 {
        let msg = green::recv().unwrap();
        println!("received: count = {}", msg);
    }
}

fn main() {
    // green::spawn_from_main(gaia, 2 * 1024 * 1024);
    green::spawn_from_main(producer, 2 * 1024 * 1024);
}
