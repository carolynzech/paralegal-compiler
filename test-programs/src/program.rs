#[paralegal::analyze]
#[paralegal::marker(a, arguments = [0])]
fn f(a: u32) -> u32 {
    g(a)
}

#[paralegal::marker(b, arguments = [0])]
fn g(b: u32) -> u32 {
    b
}

fn main() {
    f(7);
}
