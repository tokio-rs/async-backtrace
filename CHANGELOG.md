<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.2.7] - 2024-02-19

### Changed
- removed dependency on `itertools` (#36)
- stopped sending `rust-version` field (#37)

## [0.2.6] - 2023-06-21

### Fixed
- suppress clippy warnings in generated code (#26)

## [0.2.5] - 2023-04-13

### Added
- make `tasks()`, `Task`, and related methods public (#23)

## [0.2.4] - 2023-03-28

### Changed
- Consolidate consecutive identical sub-traces in output of `taskdump_tree` (#21)

## [0.2.3] - 2023-03-23

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
[Unreleased]: https://github.com/tokio-rs/async-backtrace/compare/v0.2.7...HEAD
[0.2.7]: https://github.com/tokio-rs/async-backtrace/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/tokio-rs/async-backtrace/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/tokio-rs/async-backtrace/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/tokio-rs/async-backtrace/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/tokio-rs/async-backtrace/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/tokio-rs/async-backtrace/compare/async-backtrace-v0.2.1...v0.2.2
[0.2.1]: https://github.com/tokio-rs/async-backtrace/compare/v.2.0...async-backtrace-v0.2.1
