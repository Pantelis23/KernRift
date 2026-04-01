fn main() {
    let n: usize = 200;
    let mut a = vec![0u64; n * n];
    let mut b = vec![0u64; n * n];
    let mut c = vec![0u64; n * n];
    for i in 0..n {
        for j in 0..n {
            a[i*n+j] = ((i + j) & 0xFF) as u64;
            b[i*n+j] = ((i + j) & 0xFF) as u64;
        }
    }
    for i in 0..n {
        for k in 0..n {
            let aik = a[i*n+k];
            for j in 0..n {
                c[i*n+j] += aik * b[k*n+j];
            }
        }
    }
    std::process::exit((c[0] & 0xFF) as i32);
}
