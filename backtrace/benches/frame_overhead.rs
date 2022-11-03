use criterion::{
    black_box, criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, Criterion,
};
use std::time::Duration;

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

fn bench_frame_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("`Frame` overhead");
    bench_root_poll_first(&mut group);
    bench_root_poll_rest(&mut group);
    bench_subframe_poll_first(&mut group);
    bench_subframe_poll_rest(&mut group);
    group.finish();
}

/// BNCHMRK-0
///
/// Benchmark a root `Frame`'s initialization, first invocation of `in_scope`,
/// and invocation of `Drop`.
///
/// The results of this benchmark should be interpreted as the near-worst-case
/// overhead of spawning a `#[framed]` async function.
///
/// A root `Frame` sits at the top of its execution tree. Upon the first
/// invocation of `in_scope`, this `Frame` must insert itself into the global
/// task set. Likewise, when the root `Frame` is dropped, it must remove itself
/// from this global task set. If many tasks are being initialized
/// simultaneously, in parallel, access to this set will be highly contended.
///
/// In this near-worst-case benchmark scenario, all cores of the host
/// repeatedly simultaneously create root `Frame`s, invoke `Frame::in_scope`
/// once, and then drop them.
fn bench_root_poll_first<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    c.bench_function("Frame::in_scope + Drop (root, first)", move |b| {
        parbench! {
            b;
            setup {}
            bench {
                // initialize a `Frame`
                let frame = async_backtrace::ඞ::Frame::new(async_backtrace::location!());
                tokio::pin!(frame);
                // invoke `Frame::in_scope` once
                let _ = black_box(frame.as_mut().in_scope(|| black_box(42)));
                // drop the `Frame`
            }
        }
    });
}

/// BNCHMRK-1
///
/// Benchmark a root `Frame`'s subsequent invocations of `Frame::in_scope`.
///
/// The results of this benchmark should be interpreted as the baseline overhead
/// of polling a `#[framed]` task.
///
/// The actual overhead will be slightly higher, for each sub-`#[framed]` future
/// within the task (see "Frame::in_scope (subframe, first)" and
/// "Frame::in_scope (subframe, rest)" to estimate the cost of sub-`#[framed]`
/// functions).
///
/// The actual overhead will be significantly higher when a blocking backtrace
/// is requested.
///
/// Besides managing insertion/removal from the global task set, root `Frame`s
/// are also responsible for locking the mutex that guards their children. This
/// lock is almost always uncontended (except when a blocking backtrace is
/// requested).
fn bench_root_poll_rest<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    c.bench_function("Frame::in_scope (root, rest)", move |b| {
        parbench! {
            b;
            setup {
                // initialize a `Frame`
                let frame = async_backtrace::ඞ::Frame::new(async_backtrace::location!());
                tokio::pin!(frame);
                // invoke `Frame::in_scope` once
                let _ = black_box(frame.as_mut().in_scope(|| black_box(42)));
            }
            bench {
                // repeatedly invoke `Frame::in_scope`
                let _ = black_box(frame.as_mut().in_scope(|| black_box(42)));
            }
        }
    });
}

/// BNCHMRK-2
///
/// Benchmark a sub-`Frame`'s first invocation of `in_scope`.
///
/// The results of this benchmark reflect the worst-case cost of polling
/// sub-`#[framed]` functions. It should be *very* cheap.
///
/// Upon a sub-`#[framed]` future's first poll, the `Frame` must initialize
/// itself, identifying its parent by reading a thread-local variable, and
/// notifying its parent that it has a new child. This does not require any
/// locking.
fn bench_subframe_poll_first<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    c.bench_function("Frame::in_scope (subframe, first)", move |b| {
        let root = async_backtrace::ඞ::Frame::new(async_backtrace::location!());
        tokio::pin!(root);
        root.in_scope(|| {
            // within the scope of a root `Frame`, benchmark:
            b.iter(|| {
                // ...initializing a sub-`Frame`,
                let frame = async_backtrace::ඞ::Frame::new(async_backtrace::location!());
                tokio::pin!(frame);
                // ...and invoking `Frame::in_scope` once on it.
                let _ = black_box(frame.as_mut().in_scope(|| black_box(42)));
            })
        });
    });
}

/// BNCHMRK-3
///
/// Benchmark a sub-`Frame`'s subsequent invocations of `in_scope`.
///
/// The results of this benchmark reflect the typical cost of polling
/// sub-`#[framed]` functions. It should be virtually free.
fn bench_subframe_poll_rest<M: Measurement<Value = Duration>>(c: &mut BenchmarkGroup<'_, M>) {
    c.bench_function("Frame::in_scope (subframe, rest)", move |b| {
        let root = async_backtrace::ඞ::Frame::new(async_backtrace::location!());
        tokio::pin!(root);
        root.in_scope(|| {
            // within the scope of a root `Frame`, initialize a subframe,
            let frame = async_backtrace::ඞ::Frame::new(async_backtrace::location!());
            tokio::pin!(frame);
            // invoke `Frame::in_scope` on it
            let _ = black_box(frame.as_mut().in_scope(|| black_box(42)));
            // and benchmark subsequent invocations of `Frame::in_scope`.
            b.iter(|| {
                let _ = black_box(frame.as_mut().in_scope(|| black_box(42)));
            })
        });
    });
}

criterion_group!(benches, bench_frame_overhead);
criterion_main!(benches);
