use async_backtrace::backtrace;
use criterion::{
    black_box, criterion_group, criterion_main, measurement::Measurement, Bencher, BenchmarkGroup,
    Criterion,
};
use futures::task;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

macro_rules! parbench {
    ($b:expr; setup { $($setup:tt)* } bench { $($bench:tt)* }) => {
        $b.iter_custom(|iters| {
            use std::sync::{Arc, Barrier};
            use std::time::{Duration, Instant};

            let core_ids = core_affinity::get_core_ids().unwrap();
            let num_cpus = core_ids.len();
            let start = &Arc::new(Barrier::new(num_cpus + 1));
            let stop = &Arc::new(Barrier::new(num_cpus + 1));
            let mut workers: Vec<_> = core_ids.into_iter().map(|core_id| {
                let (start, stop) = (start.clone(), stop.clone());
                std::thread::spawn(move || {
                    core_affinity::set_for_current(core_id);
                    $($setup)*
                    start.wait();
                    let start_time = Instant::now();
                    for _i in 0..iters {
                        $($bench)*
                    }
                    let stop_time = Instant::now();
                    stop.wait();
                    stop_time - start_time
                })
            }).collect();

            start.wait();
            stop.wait();

            let elapsed: Duration = workers.drain(..).map(|w| w.join().unwrap()).sum();

            elapsed / (num_cpus as u32)
        });
    }
}

pub struct TestFuture;

impl Future for TestFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

fn bench_poll_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("poll overhead");
    bench_poll_baseline(&mut group);
    bench_root_poll_first(&mut group);
    bench_root_poll_rest(&mut group);
    bench_leaf_poll_first(&mut group);
    bench_leaf_poll_rest(&mut group);
    group.finish();
}

fn bench_poll_baseline<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    c.bench_function("poll (baseline)", move |b| {
        parbench! {
            b;
            setup {
                let waker = task::noop_waker();
                let mut cx = Context::from_waker(&waker);
            }
            bench {
                let mut future = TestFuture;
                let pinned = unsafe { Pin::new_unchecked(&mut future) };
                let _ = black_box(pinned.poll(&mut cx));
                std::mem::forget(future);
            }
        }
    });
}

fn bench_root_poll_first<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    #[backtrace]
    async fn root() {
        TestFuture.await
    }

    c.bench_function("poll root (first)", move |b| {
        parbench! {
            b;
            setup {
                let waker = task::noop_waker();
                let mut cx = Context::from_waker(&waker);
            }
            bench {
                let mut future = root();
                let pinned = unsafe { Pin::new_unchecked(&mut future) };
                let _ = black_box(pinned.poll(&mut cx));
                // std::mem::forget(future);
            }
        }
    });
}

fn bench_root_poll_rest<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    #[backtrace]
    async fn root() {
        TestFuture.await
    }

    c.bench_function("poll root (rest)", move |b| {
        parbench! {
            b;
            setup {
                let waker = task::noop_waker();
                let mut cx = Context::from_waker(&waker);

                let mut future = root();
                let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
                let _ = pinned.as_mut().poll(&mut cx);
            }
            bench {
                let _ = black_box(pinned.as_mut().poll(&mut cx));
            }
        }
    });
}

fn bench_leaf_poll_first<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    #[backtrace]
    async fn root<'a, M: Measurement<Value = Duration>>(b: &mut Bencher<'_, M>) {
        #[backtrace]
        async fn leaf() {
            TestFuture.await
        }
        parbench! {
            b;
            setup {
                let waker = task::noop_waker();
                let mut cx = Context::from_waker(&waker);
            }
            bench {
                let mut future = leaf();
                let pinned = unsafe { Pin::new_unchecked(&mut future) };
                let _ = black_box(pinned.poll(&mut cx));
                std::mem::forget(future);
            }
        }
    }

    c.bench_function("poll leaf (first)", move |b| {
        let waker = task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let mut future = root(b);
        let pinned = unsafe { Pin::new_unchecked(&mut future) };
        let _ = pinned.poll(&mut cx);
    });
}

fn bench_leaf_poll_rest<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    #[backtrace]
    async fn root<'a, M: Measurement<Value = Duration>>(b: &mut Bencher<'_, M>) {
        #[backtrace]
        async fn leaf() {
            TestFuture.await
        }

        parbench! {
            b;
            setup {
                let waker = task::noop_waker();
                let mut cx = Context::from_waker(&waker);

                let mut future = leaf();
                let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
                let _ = pinned.as_mut().poll(&mut cx);
            }
            bench {
                let _ = black_box(pinned.as_mut().poll(&mut cx));
            }
        }
    }

    c.bench_function("poll leaf (rest)", move |b| {
        let waker = task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let mut future = root(b);
        let pinned = unsafe { Pin::new_unchecked(&mut future) };
        let _ = pinned.poll(&mut cx);
    });
}

criterion_group!(benches, bench_poll_overhead);
criterion_main!(benches);
