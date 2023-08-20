use std::any::{Any, TypeId};

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

#[inline]
pub(crate) fn cast_ref<T: Any>(r: &impl Any) -> Option<&T> {
    if r.type_id() == TypeId::of::<T>() {
        // SAFETY: `r` is `&T`.
        Some(unsafe { &*(r as *const _ as *const T) })
    } else {
        None
    }
}

/// Returns the values in the middle of `slice`.
///
/// If the slice has an even length, two middle values exist.
#[inline]
pub(crate) fn slice_middle<T>(slice: &[T]) -> &[T] {
    let len = slice.len();

    if len == 0 {
        slice
    } else if len % 2 == 0 {
        &slice[(len / 2) - 1..][..2]
    } else {
        &slice[len / 2..][..1]
    }
}

/// Formats an `f64` to the given number of significant figures.
pub(crate) fn format_f64(val: f64, sig_figs: usize) -> String {
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

#[cfg(test)]
mod tests {
    #[test]
    fn slice_middle() {
        use super::slice_middle;

        assert_eq!(slice_middle::<i32>(&[]), &[]);

        assert_eq!(slice_middle(&[1]), &[1]);
        assert_eq!(slice_middle(&[1, 2]), &[1, 2]);
        assert_eq!(slice_middle(&[1, 2, 3]), &[2]);
        assert_eq!(slice_middle(&[1, 2, 3, 4]), &[2, 3]);
        assert_eq!(slice_middle(&[1, 2, 3, 4, 5]), &[3]);
    }
}
