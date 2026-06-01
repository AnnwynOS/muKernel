# Contributing

> Thank you for your interest in IdealOS.

═════════════════════════════════════════════════════════

# ◉ Before You Start

⟦ ***Read First*** ⟧

Before contributing, please read:

1. `ARCHITECTURE.md`
2. `AI_USAGE.md`
3. `ROADMAP.md`

Understanding the project's goals and philosophy is more important than understanding its current implementation cause the codebase will evolve, where the ideas behind it should remain the same.

═════════════════════════════════════════════════════════

# ◉ Philosophy

⟦ ***Architecture First*** ⟧

IdealOS is primarily an educational and research-oriented project.

The objective is not to accumulate features as quickly as possible.

The objective is to explore operating system design and experiment with alternative architectural ideas.

◆ Priorities

✓ Architectural consistency

✓ Simplicity of concepts

✓ Long-term maintainability

✓ Learning and experimentation

Feature count is secondary.

A smaller but coherent system is preferred over a larger but inconsistent one.

═════════════════════════════════════════════════════════

# ◉ Pull Requests

⟦ ***Small Is Better*** ⟧

Small and focused pull requests are strongly preferred.

A PR should ideally:

• Solve a single problem

• Introduce a single feature

• Refactor a clearly identified area

• Remain easy to review

Large "rewrite everything" pull requests are difficult to review and will likely require discussion before merging.

═════════════════════════════════════════════════════════

# ◉ Code Style

⟦ ***General Expectations*** ⟧

◆ Rust

✓ `rustfmt` required

✓ `clippy` clean preferred

✓ Document unsafe blocks

✓ Prefer readability over cleverness

◆ Documentation

✓ Document public APIs

✓ Explain architectural decisions when relevant

✓ Keep documentation aligned with implementation

Code can always be rewritten.

Undocumented intentions are much harder to recover.

═════════════════════════════════════════════════════════

# ◉ Discussions

⟦ ***Talk Before Building*** ⟧

Major architectural changes should be discussed before implementation.

Examples include:

• Runtime design changes

• Capability model modifications

• IPC redesigns

• Object model changes

• Storage architecture changes

• Security model changes

A discussion opened early is usually cheaper than a pull request opened late.

═════════════════════════════════════════════════════════

# ◉ Final Note

⟦ ***Research Project*** ⟧

IdealOS is still in an exploratory phase.

Many ideas are incomplete.

Many subsystems will be rewritten.

Many assumptions will probably turn out to be wrong.

That is expected.

Contributions are welcome, but curiosity, discussion, and learning are just as valuable as code.
