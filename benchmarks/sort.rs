fn main() {
    let n: usize = 10000;
    let mut buf: Vec<u64> = (0..n).map(|i| (n - i) as u64).collect();

    let mut swapped = true;
    while swapped {
        swapped = false;
        for i in 0..n-1 {
            if buf[i] > buf[i+1] {
                buf.swap(i, i+1);
                swapped = true;
            }
        }
    }
    std::process::exit((buf[0] & 0xFF) as i32);
}
