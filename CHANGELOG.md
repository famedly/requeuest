# Changelog

## [Unreleased]

### Added

- Builder interface added to `Request`. Available via `Request::builder()`. ([!42])

### Changed

- The methods `Request::{get, head, delete, put, post}` now return a builder
  with the corresponding method. Additionally, they now expect only a url or a url and a body for `post` and `put`.  ([!42])

[!42]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/42

## [0.3.0] - 2021-09-21

### Bug Fixes

- Fix constraint violation issues with ordered jobs ([!33])

### Removed

- Remove functions for spawning jobs via transactions ([!33]).

### Miscellaneous Tasks

- Correct url for documentation badge in readme ([!30])

[!30]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/30
[!33]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/33


## [0.2.1] - 2021-09-15

### Bug Fixes

- Fix configured request headers not being set on sent requests

### Documentation

- Document integration test setup in readme
- Misc readme improvements

### Internal

- Add missing metadata to Cargo.toml
- Add logo
- Add edition to rustfmt.toml
- Add .env to .gitignore

### Testing

- Add integration tests

### Ci

- Delegate further to CI template
- Add postgres service to the `check` job for the integration tests

## [0.2.0] - 2021-08-23

### Added

- Add conversion from request types from the `http` crate ([!20])

  Available via `Request::from_http_builder`, `Request::from_http_empty`, and `Request::from_http_body`.

### Changed

- Moved `Request::spawn_with` and related functions to `Client` ([!18])

### Internal

- Remove remaining global static, use `sqlxmq` context mechanism instead ([!18])
- Add .editorconfig ([!19])

[!18]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/18
[!19]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/19
[!20]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/20

## [0.1.1] - 2021-08-16
First release! 🎉
