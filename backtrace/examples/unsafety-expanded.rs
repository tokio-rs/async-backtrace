use async_backtrace::backtrace;

#[tokio::main]
async fn main() {
    {
        use ::tokio::macros::support::Poll::{Pending, Ready};
        use ::tokio::macros::support::{maybe_done, poll_fn, Future, Pin};
        let mut futures = (maybe_done(tokio::task::yield_now()), maybe_done(dump()));
        let mut skip_next_time: u32 = 0;
        poll_fn(move |cx| {
            const COUNT: u32 = 0 + 1 + 1;
            let mut is_pending = false;
            let mut to_run = COUNT;
            let mut skip = skip_next_time;
            skip_next_time = if skip + 1 == COUNT { 0 } else { skip + 1 };
            loop {
                if skip == 0 {
                    if to_run == 0 {
                        break;
                    }
                    to_run -= 1;
                    let (fut, ..) = &mut futures;
                    let fut = unsafe { Pin::new_unchecked(fut) };
                    if fut.poll(cx).is_pending() {
                        is_pending = true;
                    }
                } else {
                    skip -= 1;
                }
                if skip == 0 {
                    if to_run == 0 {
                        break;
                    }
                    to_run -= 1;
                    let (_, fut, ..) = &mut futures;
                    let fut = unsafe { Pin::new_unchecked(fut) };
                    if fut.poll(cx).is_pending() {
                        is_pending = true;
                    }
                } else {
                    skip -= 1;
                }
            }
            if is_pending {
                Pending
            } else {
                Ready((
                    {
                        let (fut, ..) = &mut futures;
                        let fut = unsafe { Pin::new_unchecked(fut) };
                        fut.take_output().expect("expected completed future")
                    },
                    {
                        let (_, fut, ..) = &mut futures;
                        let fut = unsafe { Pin::new_unchecked(fut) };
                        fut.take_output().expect("expected completed future")
                    },
                ))
            }
        })
        .await
    };
}

#[backtrace]
async fn dump() {
    tokio::task::spawn_blocking(async_backtrace::dump)
        .await
        .unwrap();
}