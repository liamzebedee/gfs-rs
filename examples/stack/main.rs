
fn main() {
    // ulimit -s - stack size in KB
    // 8176 KB
    // to bytes
    // 8176 * 1024 = 8371712
    let x = [0; ((8176 - 7000) * 1024)];
}