<!-- Do not edit README.md manually. Instead, edit the module comment of `backtrace/lib.rs`. -->

# taskdump

Efficient, logical 'stack' traces of async functions.

## Usage
To use, annotate your async functions with `#[taskdump::framed]`,
like so:

```rust
#[tokio::main]
async fn main() {
    tokio::select! {
        _ = tokio::spawn(taskdump::frame!(pending())) => {}
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
```

This example program will print out something along the lines of:

```
╼ taskdump::foo::{{closure}} at backtrace/examples/taskdump.rs:20:1
  └╼ taskdump::bar::{{closure}} at backtrace/examples/taskdump.rs:25:1
     ├╼ taskdump::buz::{{closure}} at backtrace/examples/taskdump.rs:35:1
     │  └╼ taskdump::baz::{{closure}} at backtrace/examples/taskdump.rs:40:1
     └╼ taskdump::fiz::{{closure}} at backtrace/examples/taskdump.rs:30:1
╼ taskdump::pending::{{closure}} at backtrace/examples/taskdump.rs:15:1
```

## Minimizing Overhead
To minimize overhead, ensure that futures you spawn with your async runtime
are marked with `#[framed]`.

In other words, avoid doing this:
```rust
tokio::spawn(taskdump::location!().frame(async {
    foo().await;
    bar().await;
})).await;

#[taskdump::framed] async fn foo() {}
#[taskdump::framed] async fn bar() {}
```
...and prefer doing this:
```rust
tokio::spawn(async {
    foo().await;
    bar().await;
}).await;

#[taskdump::framed]
async fn foo() {
    bar().await;
    baz().await;
}

#[taskdump::framed] async fn bar() {}
#[taskdump::framed] async fn baz() {}
```

## Estimating Overhead
To estimate the overhead of adopting `#[framed]` in your application, refer
to the benchmarks and interpretive guidance in
`./backtrace/benches/frame_overhead.rs`. You can run these benchmarks with
`cargo bench`.

## License

MIT
