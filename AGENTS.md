# Repository Rules

## File size

- No source file may reach 300 lines. Split a file that grows past roughly
  250 lines into focused modules before it hits the limit.
- A "source file" is any tracked `.rs` file, including integration tests under
  `tests/`. Documentation and binary assets are out of scope.
- Split by responsibility, not mechanically: keep one module per clear
  concern, re-export the public surface, and keep each new file cohesive.
- `scripts/check-file-size.sh` enforces this; the pre-commit hook runs it.
