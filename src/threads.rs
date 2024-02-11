use std::cell::RefCell;
use std::slice::IterMut;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::stats::RawSample;

#[derive(Default)]
pub(crate) struct AuxiliaryThreads {
    inner: RefCell<Vec<BencherThread>>,
}

impl AuxiliaryThreads {
    pub fn with_threads<ThreadFn, Scope, R>(
        &self,
        thread_count: usize,
        task_fn: ThreadFn,
        scope: Scope,
    ) -> R
    where
        ThreadFn: Fn() -> RawSample,
        Scope: FnOnce(Results) -> R,
    {
        let mut threads = self.inner.borrow_mut();
        if threads.len() < thread_count {
            threads.resize_with(thread_count, BencherThread::new)
        }
        let threads = &mut threads[..thread_count];

        // SAFETY: We wait for all child task to finish executing their copy of the
        // `task_fn` within `Results::drop`. Therefore we are not holding on to the `task_fn`
        // after returning from this function / scope.
        let dyn_task_fn: &dyn Fn() -> RawSample = &task_fn;
        let dyn_task_fn: Task = unsafe { std::mem::transmute(dyn_task_fn) };

        for thread in threads.iter() {
            thread.send_workload.as_ref().unwrap().send(dyn_task_fn).unwrap();
        }

        scope(Results { threads: threads.iter_mut() })
    }
}

pub(crate) struct Results<'t> {
    threads: IterMut<'t, BencherThread>,
}

impl Iterator for Results<'_> {
    type Item = RawSample;

    fn next(&mut self) -> Option<Self::Item> {
        let thread = self.threads.next()?;
        Some(thread.expect_result())
    }
}

impl Drop for Results<'_> {
    fn drop(&mut self) {
        for _result in self {}
    }
}

type Task = &'static (dyn Fn() -> RawSample + Send + Sync);

pub struct BencherThread {
    handle: Option<JoinHandle<()>>,
    send_workload: Option<Sender<Task>>,
    receive_result: Receiver<RawSample>,
}

impl BencherThread {
    pub fn new() -> Self {
        let (send_workload, receive_workload) = channel::<Task>();
        let (send_result, receive_result) = channel();

        let handle = thread::spawn(move || {
            for task in receive_workload {
                let sample = (*task)();
                send_result.send(sample).unwrap();
            }
        });

        Self { handle: Some(handle), send_workload: Some(send_workload), receive_result }
    }

    pub fn expect_result(&mut self) -> RawSample {
        self.receive_result.recv().unwrap_or_else(|_| {
            let error = self.handle.take().unwrap().join().unwrap_err();
            std::panic::resume_unwind(error)
        })
    }
}

impl Drop for BencherThread {
    fn drop(&mut self) {
        drop(self.send_workload.take());
        self.handle.take().unwrap().join().unwrap()
    }
}
