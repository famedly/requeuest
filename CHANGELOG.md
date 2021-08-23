# Changelog
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
