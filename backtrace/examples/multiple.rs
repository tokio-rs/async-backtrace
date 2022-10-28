//! Run this example to see how taskdumps appear with multiple tasks.

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tokio::select! {
        // run the following branches in order of their appearance
        biased;

        // spawn task #1
        _ = tokio::spawn(foo()) => { unreachable!() }

        // spawn task #2
        _ = tokio::spawn(foo()) => { unreachable!() }

        // print the running tasks
        _ = tokio::spawn(async {}) => {
            println!("{}", async_backtrace::taskdump_tree(true));
        }
    };
}

#[async_backtrace::framed]
async fn foo() {
    bar().await;
}

#[async_backtrace::framed]
async fn bar() {
    baz().await;
}

#[async_backtrace::framed]
async fn baz() {
    std::future::pending::<()>().await
}
