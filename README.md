<!-- Do not edit README.md manually. Instead, edit the module comment of `backtrace/lib.rs`. -->

# async-backtrace

Efficient, logical 'stack' traces of async functions.

## Usage
To use, annotate your async functions with `#[async_backtrace::framed]`,
like so:

```rust
#[tokio::main]
async fn main() {
    tokio::select! {
        _ = tokio::spawn(async_backtrace::frame!(pending())) => {}
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
    futures::join!(fiz(), buz());
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
tokio::spawn(async_backtrace::location!().frame(async {
    foo().await;
    bar().await;
})).await;

#[async_backtrace::framed] async fn foo() {}
#[async_backtrace::framed] async fn bar() {}
```
...and prefer doing this:
```rust
tokio::spawn(async {
    foo().await;
    bar().await;
}).await;

#[async_backtrace::framed]
async fn foo() {
    bar().await;
    baz().await;
}

#[async_backtrace::framed] async fn bar() {}
#[async_backtrace::framed] async fn baz() {}
```

## Estimating Overhead
To estimate the overhead of adopting `#[framed]` in your application, refer
to the benchmarks and interpretive guidance in
`./backtrace/benches/frame_overhead.rs`. You can run these benchmarks with
`cargo bench`.

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/tokio-rs/async-backtrace/blob/main/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in taskdump by you, shall be licensed as MIT, without any
additional terms or conditions.
