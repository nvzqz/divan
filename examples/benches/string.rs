use divan::Bencher;

fn main() {
    divan::main();
}

#[derive(Default)]
struct Ascii {
    rng: fastrand::Rng,
}

#[derive(Default)]
struct Unicode {
    rng: fastrand::Rng,
}

trait GenString: Default {
    fn gen_string(&mut self) -> String;
}

impl GenString for Ascii {
    fn gen_string(&mut self) -> String {
        let len = 100;
        (0..len).map(|_| self.rng.alphanumeric()).collect()
    }
}

impl GenString for Unicode {
    fn gen_string(&mut self) -> String {
        let len = 100;
        (0..len).map(|_| self.rng.char(..)).collect()
    }
}

#[divan::bench(types = [Ascii, Unicode])]
fn clear<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_refs(String::clear);
}

#[divan::bench(types = [Ascii, Unicode])]
fn drop<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_values(std::mem::drop);
}

#[divan::bench(types = [Ascii, Unicode])]
fn make_ascii_lowercase<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_refs(|s| s.make_ascii_lowercase());
}

#[divan::bench(types = [Ascii, Unicode])]
fn make_ascii_uppercase<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_refs(|s| s.make_ascii_uppercase());
}

#[divan::bench(types = [Ascii, Unicode])]
fn to_ascii_lowercase<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_refs(|s| s.to_ascii_lowercase());
}

#[divan::bench(types = [Ascii, Unicode])]
fn to_ascii_uppercase<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_refs(|s| s.to_ascii_uppercase());
}

#[divan::bench(types = [Ascii, Unicode])]
fn to_lowercase<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_refs(|s| s.to_lowercase());
}

#[divan::bench(types = [Ascii, Unicode])]
fn to_uppercase<G: GenString>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string()).bench_refs(|s| s.to_uppercase());
}
