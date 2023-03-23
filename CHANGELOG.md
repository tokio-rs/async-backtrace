<!-- next-header -->

## [Unreleased] - ReleaseDate

### Fixed
- fix error in documentation of `taskdump_tree` (#15)
- fix misplaced newline in output of `taskdump_tree(false)` (#16)

### Changed
- marked internal functions `Frame::subframes`, `Frame::prev_frame` and `Frame::next_frame` as `unsafe` (#17)
- reduced unwrapping in `Frame::fmt` (#17)
- upgrade `syn` to v2.0 (#19)

### Removed
- removed unused dev dependency on `smol` (#18)

## [0.2.2] - 2022-11-03

### Fixed
- ignore `clippy::empty_loop` in `#[framed]` macro expansion (#11)

## [0.2.1] - 2022-11-02

### Fixed
- eliminated redundant newline at end of `taskdump_tree` output (#10)

## 0.2.0 - 2022-10-25
- Initial Release

<!-- next-url -->
[Unreleased]: https://github.com/tokio-rs/async-backtrace/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/tokio-rs/async-backtrace/compare/async-backtrace-v0.2.1...v0.2.2
[0.2.1]: https://github.com/tokio-rs/async-backtrace/compare/v.2.0...async-backtrace-v0.2.1
