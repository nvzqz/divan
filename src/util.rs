/// Public-in-private trait for abstracting over either `FnMut` or `()` (no-op).
pub trait ConfigFnMut {
    type Output;

    fn call_mut(&mut self) -> Self::Output;
}

impl<O, F: FnMut() -> O> ConfigFnMut for F {
    type Output = O;

    #[inline(always)]
    fn call_mut(&mut self) -> Self::Output {
        self()
    }
}

impl ConfigFnMut for () {
    type Output = ();

    #[inline(always)]
    fn call_mut(&mut self) {}
}

/// Formats an `f64` to the given number of significant figures.
pub fn format_f64(val: f64, sig_figs: usize) -> String {
    let mut str = val.to_string();

    if let Some(dot_index) = str.find('.') {
        let fract_digits = sig_figs.saturating_sub(dot_index);

        if fract_digits == 0 {
            str.truncate(dot_index);
        } else {
            let fract_start = dot_index + 1;
            let fract_end = fract_start + fract_digits;
            let fract_range = fract_start..fract_end;

            if let Some(fract_str) = str.get(fract_range) {
                // Get the offset from the end before all 0s.
                let pre_zero = fract_str.bytes().rev().enumerate().find_map(|(i, b)| {
                    if b != b'0' {
                        Some(i)
                    } else {
                        None
                    }
                });

                if let Some(pre_zero) = pre_zero {
                    str.truncate(fract_end - pre_zero);
                } else {
                    str.truncate(dot_index);
                }
            }
        }
    }

    str
}
