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

#[taskdump::framed]
async fn pending() {
    std::future::pending::<()>().await
}

#[taskdump::framed]
async fn foo() {
    bar().await;
}

#[taskdump::framed]
async fn bar() {
    futures::join!(fiz(), buz());
}

#[taskdump::framed]
async fn fiz() {
    tokio::task::yield_now().await;
}

#[taskdump::framed]
async fn buz() {
    println!("{}", baz().await);
}

#[taskdump::framed]
async fn baz() -> String {
    taskdump::taskdump(true)
}
