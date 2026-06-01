# Roadmap

> Current implementation roadmap for IdealOS.

═════════════════════════════════════════════════════════

# ◉ Phase 1 — Microkernel Foundation

⟦ ***Memory Management*** ⟧

* [x] Physical Memory Manager
* [x] Virtual Memory Manager
* [ ] Memory protection improvements
* [ ] User pointer validation

---

⟦ ***Processes*** ⟧

* [x] Process abstraction
* [x] Address spaces
* [ ] Process cleanup
* [ ] Process lifecycle management

---

⟦ ***Scheduling*** ⟧

* [x] Basic scheduler
* [ ] SMP scheduler
* [ ] CPU affinity
* [ ] Power-aware scheduling

---

⟦ ***IPC*** ⟧

* [x] Message queues
* [ ] Typed messages
* [ ] Message versioning
* [ ] Zero-copy transfers
* [ ] IPC tracing

---

⟦ ***Security*** ⟧

* [x] Capability framework
* [ ] Capability spaces
* [ ] Capability transfer
* [ ] Capability revocation trees

═════════════════════════════════════════════════════════

# ◉ Phase 2 — Userspace Foundation

⟦ ***Bootstrap*** ⟧

* [ ] ELF loader
* [ ] Init service
* [ ] Userspace drivers
* [ ] Service manager
* [ ] Runtime bootstrap

═════════════════════════════════════════════════════════

# ◉ Phase 3 — Runtime

⟦ ***Core Runtime Services*** ⟧

* [ ] Object manager
* [ ] Transaction engine
* [ ] Snapshot system
* [ ] Permission manager
* [ ] Observability framework

═════════════════════════════════════════════════════════

# ◉ Phase 4 — Storage

⟦ ***Object-Oriented Storage*** ⟧

* [ ] Object store
* [ ] Snapshotting
* [ ] POSIX compatibility layer
* [ ] Search and indexing

═════════════════════════════════════════════════════════

# ◉ Phase 5 — Compatibility

⟦ ***Application Compatibility*** ⟧

* [ ] WASM runtime
* [ ] POSIX layer
* [ ] Linux compatibility VM

═════════════════════════════════════════════════════════

# ◉ Phase 6 — Long-Term Research

⟦ ***Future Exploration*** ⟧

* [ ] Distributed objects
* [ ] GPU scheduling
* [ ] Global undo
* [ ] Persistent sessions
* [ ] System-wide time travel debugging

═════════════════════════════════════════════════════════

# ◉ Notes

⟦ ***Important*** ⟧

This roadmap represents the current implementation direction.

Research projects evolve.

Priorities may change.

Entire subsystems may be redesigned or rewritten as new ideas emerge.

The architecture remains the destination.

The roadmap is only the current path.
