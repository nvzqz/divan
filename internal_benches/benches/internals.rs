use divan::AllocProfiler;

#[global_allocator]
static GLOBAL_ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}
