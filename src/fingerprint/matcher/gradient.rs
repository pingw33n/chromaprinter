use super::Elem;

pub fn gradient(inp: &[Elem], out: &mut Vec<Elem>) {
    let mut inp = inp.iter();

    out.reserve(inp.len());
    let mut out = move |v| out.push(v);

    let mut f0 = if let Some(v) = inp.next() {
        v
    } else {
        return;
    };

    let mut f1 = if let Some(v) = inp.next() {
        v
    } else {
        out(0.0);
        return;
    };
    out(f1 - f0);

    let mut f2 = if let Some(v) = inp.next() {
        v
    } else {
        out(f1 - f0);
        return;
    };

    loop {
        out((f2 - f0) / 2.0);
        let next = if let Some(v) = inp.next() {
            v
        } else {
            out(f2 - f1);
            break;
        };
        f0 = f1;
        f1 = f2;
        f2 = next;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn gradient_fn() {
        let data = &[
            (&[][..], &[][..]),
            (&[1.0][..], &[0.0][..]),
            (&[1.0, 2.0][..], &[1.0, 1.0][..]),
            (&[1.0, 2.0, 4.0][..], &[1.0, 1.5, 2.0][..]),
            (&[1.0, 2.0, 4.0, 10.0][..], &[1.0, 1.5, 4.0, 6.0][..]),
        ];
        for (inp, exp) in data {
            let act = &mut Vec::new();
            gradient(inp, act);

            for (a, e) in act.iter().zip(exp.iter()) {
                assert_abs_diff_eq!(a, e, epsilon = 1e-5);
            }
        }
    }
}