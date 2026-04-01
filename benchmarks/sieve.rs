fn main() {
    let n: usize = 1000000;
    let mut sieve = vec![true; n];
    let mut p = 2;
    while p * p < n {
        if sieve[p] {
            let mut j = p * p;
            while j < n {
                sieve[j] = false;
                j += p;
            }
        }
        p += 1;
    }
    let count = sieve[2..].iter().filter(|&&x| x).count();
    std::process::exit((count & 0xFF) as i32);
}
