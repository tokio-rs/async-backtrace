// This example prints something like:
// ╼ taskdump::foo::{{closure}} at backtrace/examples/taskdump.rs:12:1
//   └╼ taskdump::bar::{{closure}} at backtrace/examples/taskdump.rs:17:1
//      └╼ taskdump::buz::{{closure}} at backtrace/examples/taskdump.rs:27:1
// ╼ taskdump::baz::{{closure}} at backtrace/examples/taskdump.rs:32:1

#[tokio::main]
async fn main() {
    foo().await;
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
    println!("{}", tokio::spawn(baz()).await.unwrap());
}

#[async_backtrace::framed]
async fn baz() -> String {
    async_backtrace::taskdump(true)
}
