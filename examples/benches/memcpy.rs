use divan::{counter::BytesCount, Bencher};
use fastrand::Rng;

fn main() {
    divan::main();
}

const LENS: &[usize] = &[
    1,
    2,
    8,
    16,
    64,
    512,
    1024 * 4,
    1024 * 16,
    1024 * 64,
    1024 * 256,
    1024 * 1024,
    1024 * 1024 * 4,
];

#[divan::bench(consts = LENS)]
fn memcpy<const N: usize>(bencher: Bencher) {
    bencher.counter(BytesCount::new(N)).with_inputs(Input::gen(N)).bench_local_refs(
        |input| unsafe {
            let src_ptr = input.src_ptr();
            let dst_ptr = input.dst_ptr();
            libc::memcpy(dst_ptr.cast(), src_ptr.cast(), divan::black_box(N));
        },
    )
}

#[divan::bench(consts = LENS)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn movsb<const N: usize>(bencher: Bencher) {
    use std::arch::asm;

    bencher.counter(BytesCount::new(N)).with_inputs(Input::gen(N)).bench_local_refs(
        |input| unsafe {
            #[cfg(target_arch = "x86")]
            asm!(
                "rep movsb",
                inout("ecx") divan::black_box(N) => _,
                inout("esi") input.src_ptr() => _,
                inout("edi") input.dst_ptr() => _,
                options(nostack, preserves_flags),
            );

            #[cfg(target_arch = "x86_64")]
            asm!(
                "rep movsb",
                inout("rcx") divan::black_box(N) => _,
                inout("rsi") input.src_ptr() => _,
                inout("rdi") input.dst_ptr() => _,
                options(nostack, preserves_flags),
            );
        },
    )
}

/// Self-referential input.
///
/// It stores random offsets into the buffers, which are allowed to reference up
/// to the provided length. This enables us to benchmark unaligned writes. We
/// generate these as part of the input to not add benchmark time.
struct Input {
    src_buf: Box<[u8]>,
    dst_buf: Box<[u8]>,
    src_offset: usize,
    dst_offset: usize,
}

impl Input {
    fn gen(len: usize) -> impl FnMut() -> Self {
        let mut rng = Rng::default();
        move || {
            // Very buffers by length rather than adhere to nice numbers.
            let max_len = len + (len / 8);

            let src_len = rng.usize(len..=max_len);
            let dst_len = rng.usize(len..=max_len);

            let src_buf: Box<[u8]> = (0..src_len).map(|_| rng.u8(..)).collect();
            let dst_buf: Box<[u8]> = (0..dst_len).map(|_| rng.u8(..)).collect();

            // 50% chance of the copy being aligned. Aligned writes are
            // potentially must faster.
            let is_aligned = rng.bool();
            let (src_offset, dst_offset) = if is_aligned {
                (0, 0)
            } else {
                (rng.usize(..=src_len - len), rng.usize(..=dst_len - len))
            };

            Input { src_buf, dst_buf, src_offset, dst_offset }
        }
    }

    fn src_ptr(&self) -> *const u8 {
        self.src_buf.as_ptr().wrapping_add(self.src_offset)
    }

    fn dst_ptr(&mut self) -> *mut u8 {
        self.dst_buf.as_mut_ptr().wrapping_add(self.dst_offset)
    }
}
