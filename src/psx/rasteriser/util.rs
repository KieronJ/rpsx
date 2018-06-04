pub fn f32_cmp_3(a: f32, b: f32, c: f32) -> (f32, f32) {
    let mut a = a;
    let mut b = b;
    let mut c = c;

    if a > b {
        let tmp = a;
        a = b;
        b = tmp;
    }

    if b > c {
        let tmp = b;
        b = c;
        c = tmp;  
    }

    if a > b {
        a = b;
    }

    (a, c)
}