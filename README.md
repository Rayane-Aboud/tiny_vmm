# tiny_vmm

tiny_vmm is a small virtual machine monitor written in Rust on top of Linux KVM. I wrote it to understand
what tools like Firecracker and QEMU do when they start a virtual machine. It's about 600 lines.

## What it does

1. It creates a virtual machine and a virtual CPU through `/dev/kvm`, using
   [kvm-ioctls](https://github.com/rust-vmm/kvm-ioctls), and gives the guest 64 MiB of memory with
   [vm-memory](https://github.com/rust-vmm/vm-memory).
2. It follows the Linux boot protocol by hand. It loads a `bzImage` at 1 MiB, reads its header to find
   where the 32-bit code starts, writes the boot parameters (the "zero page") and the kernel command line,
   and loads an initramfs at 8 MiB.
3. It sets the CPU registers directly: protected mode, flat 4 GiB segments, the instruction pointer on the
   kernel entry point, and a pointer to the boot parameters.
4. It runs the virtual CPU in a loop and handles each exit: port I/O, memory-mapped I/O, `HLT` and
   shutdown.
5. It emulates a serial port at `0x3F8`, so the guest's console output shows up in your terminal.
6. It can build its own initramfs: it unpacks a base `initramfs.cpio`, adds a small test program and an
   `init` script, and packs it again as `initramfs_custom.cpio`.

| File | What it does |
|---|---|
| `src/vm/mod.rs` | the life of the VM: create it, load the images, boot |
| `src/vm/memory.rs` | guest memory, boot parameters and command line |
| `src/vm/vcpu.rs` | registers, the run loop and the VM exits |
| `src/io/mod.rs` | the serial port |
| `src/vm/initramfs.rs` | building the custom initramfs |

## Running it

You need Linux with KVM (`/dev/kvm` must be readable by your user), Rust, `gcc` and `cpio`.

The kernel and initramfs images are not in the repository. Build a kernel and put it next to
`Cargo.toml` as `kernel.bin`:

```bash
git clone --depth 1 https://github.com/torvalds/linux && cd linux
make x86_64_defconfig && make -j"$(nproc)" bzImage
cp arch/x86/boot/bzImage ../tiny_vmm/kernel.bin
```

You can also add a base `initramfs.cpio` (a BusyBox one works). Then run:

```bash
cargo run --release
```

## Status

This is a learning project. The next things I'd like to add are long mode set up by the monitor itself,
a virtio console, and support for more than one virtual CPU.
