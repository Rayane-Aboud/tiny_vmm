# tiny_vmm

**A minimal virtual machine monitor in Rust on top of Linux KVM, written to understand what Firecracker and
QEMU do under the hood.** About 600 lines.

## What it does

1. **Creates a VM and a vCPU** through `/dev/kvm` ([kvm-ioctls](https://github.com/rust-vmm/kvm-ioctls)) and maps
   64 MiB of guest memory ([vm-memory](https://github.com/rust-vmm/vm-memory)).
2. **Follows the Linux x86 boot protocol by hand**: loads a `bzImage` at 1 MiB, reads `setup_sects` from its
   header to find the 32-bit entry point, writes the "zero page" boot parameters (`HdrS` signature, initrd
   address and size) and the kernel command line (`console=ttyS0 ...`), and loads an initramfs at 8 MiB.
3. **Sets the vCPU state directly**: protected mode with paging bit set in `CR0`, flat 4 GiB code and data
   segments, `RIP` at the kernel entry, `RDI` pointing to the boot parameters.
4. **Runs the vCPU loop** and handles VM exits: port I/O, MMIO, `HLT`, shutdown.
5. **Emulates a 16550 serial port** at `0x3F8`, so the guest's console output shows up in the terminal.
6. **Builds its own initramfs**: unpacks a base `initramfs.cpio`, adds a statically linked test binary and an
   `init` script, and repacks it as `initramfs_custom.cpio`.

| File | Role |
|---|---|
| `src/vm/mod.rs` | VM lifecycle: create, load images, boot |
| `src/vm/memory.rs` | guest memory, boot parameters, command line |
| `src/vm/vcpu.rs` | registers, run loop, VM exit handling |
| `src/io/mod.rs` | serial port emulation |
| `src/vm/initramfs.rs` | custom initramfs builder |

## Run it

Needs Linux with KVM (`/dev/kvm` readable by your user), Rust, `gcc` and `cpio`.

The guest images are not committed. Build a kernel and put it next to `Cargo.toml` as `kernel.bin`:

```bash
git clone --depth 1 https://github.com/torvalds/linux && cd linux
make x86_64_defconfig && make -j"$(nproc)" bzImage
cp arch/x86/boot/bzImage ../tiny_vmm/kernel.bin
```

Optionally provide a base `initramfs.cpio` (a BusyBox one works), then:

```bash
cargo run --release
```

## Status

A learning project. Next steps: long mode set up by the VMM, a virtio console, and more than one vCPU.
