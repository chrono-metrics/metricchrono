# Security

Report security issues privately to the maintainers before opening public
issues. Do not include exploit details in public tickets.

This public repo contains deterministic local computation and FFI bindings. The
highest-risk areas are:

- raw pointer handling in `metricchrono-ffi`;
- malformed caller-owned buffers;
- language-wrapper loading of dynamic libraries.

The FFI contract returns `MC_STATUS_NULL`, `MC_STATUS_INVALID_ARGUMENT`, or
`MC_STATUS_BUFFER_TOO_SMALL` for invalid caller inputs. Panics crossing the C ABI
are caught and returned as `MC_STATUS_PANIC`.
