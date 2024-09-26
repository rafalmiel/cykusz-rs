use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let pair = Arc::new((Mutex::new(0), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    let handle = thread::spawn(move || {
        let (lock, cvar) = &*pair2;

        for i in 1..100 {
            let g = lock.lock().unwrap();
            let res = cvar.wait_while(g, |l| *l != i).unwrap();
            println!("got {res}");
        }
    });

    let (lock, cvar) = &*pair;

    for i in 1..100 {
        let mut v = lock.lock().unwrap();
        *v = i;

        cvar.notify_one();

        thread::sleep(Duration::from_millis(100));
    }

    handle.join().unwrap();
}
