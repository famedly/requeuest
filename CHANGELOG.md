# Changelog

## [0.2.2] - 2021-09-20

### Bug Fixes

- Disable job ordering to prevent triggering bug in upstream ([!31])

  Ordered jobs have been forcibly disabled to avoid triggering a constraint violation in the postgres database. This is a temporary workaround, and a more proper fix will be developed later.

### Internal

- Correct URL for documentation badge in readme ([!30])
- Upgrade `sqlxmq` and `tokio` dependencies

[!30]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/30
[!31]: https://gitlab.com/famedly/company/backend/libraries/requeuest/-/merge_requests/31

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

- Delegate futher to CI template
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
