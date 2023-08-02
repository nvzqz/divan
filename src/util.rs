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
