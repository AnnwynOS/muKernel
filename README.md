# Annwyn µKernel

> Experimental capability-based microkernel written in Rust.

═════════════════════════════════════════════════════════

## ◉ What is the µKernel?

This repository contains the core microkernel of the Annwyn operating system research project.

Responsibilities include:

✓ Thread scheduling

✓ Memory management

✓ Capability enforcement

✓ IPC primitives

✓ Synchronization

✓ Hardware abstraction

Higher-level functionality is implemented in separate repositories.

═════════════════════════════════════════════════════════

## ◉ Project Status

⟦ ***Early Development*** ⟧

The current implementation contains:

• Physical Memory Manager

• Virtual Memory Manager

• Process abstraction

• Scheduler

• Capability framework

• IPC foundations

Many components remain experimental and are expected to evolve significantly.

This repository is currently a research platform rather than a production-ready kernel.

═════════════════════════════════════════════════════════

## ◉ Current Progress

### Implemented

* [x] Physical Memory Manager
* [x] Virtual Memory Manager
* [x] Process abstraction
* [x] Scheduler
* [x] Capability framework
* [x] IPC primitives
* [x] Syscall infrastructure
* [x] UEFI boot support
* [x] BIOS boot support

### In Progress

* [ ] SMP support
* [ ] Typed IPC
* [ ] Zero-copy IPC
* [ ] Improved synchronization

### Planned

* [ ] Userspace bootstrap
* [ ] Runtime integration
* [ ] Performance improvements

═════════════════════════════════════════════════════════

## ◉ Documentation

The global architecture and project philosophy are documented in the `annwyn-docs` repository.

Useful documents include:

| Document     | Purpose                         |
| ------------ | ------------------------------- |
| Architecture | Long-term architectural vision  |
| Roadmap      | Global implementation roadmap   |
| AI Usage     | AI usage and project philosophy |
| Contributing | Contribution guidelines         |

Repository-specific documentation may be found under `docs/`.

═════════════════════════════════════════════════════════

## ◉ Building

### Install Dependencies

```bash
cargo xtask install-deps
```

### Build

```bash
cargo xtask build
```

### Run

```bash
cargo xtask run
```

═════════════════════════════════════════════════════════

## ◉ Development Commands

```bash
cargo xtask build
cargo xtask run
cargo xtask run-bios
cargo xtask run-release
cargo xtask debug
cargo xtask check
cargo xtask clippy
```

The same commands are also available through `make`.

═════════════════════════════════════════════════════════

## ◉ Debugging

```bash
cargo xtask debug

rust-gdb target/x86_64-kernel/debug/kernel

(gdb) target remote :1234
(gdb) break kernel_main
(gdb) continue
```

═════════════════════════════════════════════════════════

## ◉ Contributing

Contributions, discussions, ideas, criticism, and questions are welcome.

Please keep in mind that:

• Architecture consistency is generally more important than feature count.

• Simplicity is preferred over complexity.

• Unsafe code should be justified.

• Kernel responsibilities should remain minimal.

Before contributing, it is recommended to read the documentation available in `annwyn-docs`.

═════════════════════════════════════════════════════════

## ◉ Related Repositories

• annwyn-docs

• annwyn-runtime

• annwyn-object-store

• annwyn-posix

• annwyn-wasm

═════════════════════════════════════════════════════════

## ◉ License

Licensed under either of:

* MIT License

* Apache License 2.0

at your option.
