```bash
cargo xtask install-deps
```

## Commandes

```bash
cargo xtask build # compile kernel + génère images UEFI et BIOS
cargo xtask run # build + QEMU UEFI  ← commande principale
cargo xtask run-bios # build + QEMU BIOS legacy
cargo xtask run-release # build release + QEMU UEFI
cargo xtask debug # build + QEMU + stub GDB sur :1234
cargo xtask check # vérification rapide sans linker
cargo xtask clippy # linter
```

Egalement disponiblkes via `make`.

## Debug GDB

```bash
cargo xtask debug


# Dans un autre terminal :
rust-gdb target/x86_64-kernel/debug/kernel
(gdb) target remote :1234
(gdb) break kernel_main
(gdb) continue
```