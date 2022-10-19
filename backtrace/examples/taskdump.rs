// This example outputs something like:
// ╼ taskdump::foo::{{closure}} at backtrace/examples/taskdump.rs:20:1
//   └╼ taskdump::bar::{{closure}} at backtrace/examples/taskdump.rs:25:1
//      ├╼ taskdump::buz::{{closure}} at backtrace/examples/taskdump.rs:35:1
//      │  └╼ taskdump::baz::{{closure}} at backtrace/examples/taskdump.rs:40:1
//      └╼ taskdump::fiz::{{closure}} at backtrace/examples/taskdump.rs:30:1
// ╼ taskdump::pending::{{closure}} at backtrace/examples/taskdump.rs:15:1

#[tokio::main]
async fn main() {
    tokio::select! {
        _ = tokio::spawn(pending()) => {}
        _ = foo() => {}
    };
}

#[async_backtrace::framed]
async fn pending() {
    std::future::pending::<()>().await
}

#[async_backtrace::framed]
async fn foo() {
    bar().await;
}

#[async_backtrace::framed]
async fn bar() {
    tokio::join!(Box::pin(fiz()), Box::pin(buz()));
}

#[async_backtrace::framed]
async fn fiz() {
    tokio::task::yield_now().await;
}

#[async_backtrace::framed]
async fn buz() {
    println!("{}", baz().await);
}

#[async_backtrace::framed]
async fn baz() -> String {
    async_backtrace::taskdump(true)
}
