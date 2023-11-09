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

fn gen_inputs(len: usize) -> impl FnMut() -> [Box<[u8]>; 2] {
    let mut rng = Rng::default();
    move || {
        // Very buffers by length rather than adhere to nice numbers.
        let max_len = len + (len / 8);
        let lens = [rng.usize(len..=max_len), rng.usize(len..=max_len)];

        lens.map(|len| (0..len).map(|_| rng.u8(..)).collect())
    }
}

#[divan::bench(consts = LENS)]
fn memcpy<const N: usize>(bencher: Bencher) {
    bencher.counter(BytesCount::new(N)).with_inputs(gen_inputs(N)).bench_local_refs(
        |[src, dst]| unsafe {
            let src = src.as_ptr().cast();
            let dst = dst.as_mut_ptr().cast();
            libc::memcpy(dst, src, N);
        },
    )
}

#[divan::bench(consts = LENS)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn movsb<const N: usize>(bencher: Bencher) {
    use std::arch::asm;

    bencher.counter(BytesCount::new(N)).with_inputs(gen_inputs(N)).bench_local_refs(
        |[src, dst]| unsafe {
            let src = src.as_ptr();
            let dst = dst.as_mut_ptr();

            #[cfg(target_arch = "x86")]
            asm!(
                "rep movsb",
                inout("ecx") N => _,
                inout("esi") src => _,
                inout("edi") dst => _,
                options(nostack, preserves_flags),
            );

            #[cfg(target_arch = "x86_64")]
            asm!(
                "rep movsb",
                inout("rcx") N => _,
                inout("rsi") src => _,
                inout("rdi") dst => _,
                options(nostack, preserves_flags),
            );
        },
    )
}
