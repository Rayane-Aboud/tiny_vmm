use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
use kvm_ioctls::{Kvm, VcpuExit};
use vm_memory::{GuestAddress, GuestMemoryMmap, Bytes, GuestMemory};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use vmm_sys_util::eventfd::EventFd;

const GUEST_MEM_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB
const KERNEL_LOAD_ADDR: u64 = 0x100000;
const CMDLINE_ADDR: u64 = 0x20000;
const INITRD_ADDR: u64 = 0x800000; // place initrd high in memory

fn main() -> anyhow::Result<()> {
    // 1. open KVM
    let kvm = Kvm::new()?;
    // 2. create vm fd
    let vm = kvm.create_vm()?;
    

    // 3. create guest memory with explicit type annotations
    let guest_mem: GuestMemoryMmap<()> = GuestMemoryMmap::from_ranges(&[(GuestAddress(0), GUEST_MEM_SIZE as usize)])?;
    let gm: Arc<GuestMemoryMmap<()>> = Arc::new(guest_mem);

    // register memory with KVM
    let mem_region = kvm_userspace_memory_region {
        slot: 0,
        guest_phys_addr: 0,
        memory_size: GUEST_MEM_SIZE,
        userspace_addr: gm.get_host_address(GuestAddress(0))? as u64,
        flags: 0,
    };
    unsafe { vm.set_user_memory_region(mem_region)? };

    // 4. create one VCPU - make it mutable
    let mut vcpu_fd = vm.create_vcpu(0)?;

    
    // 5. load kernel image
    let mut kernel = Vec::new();
    File::open("kernel.bin")?.read_to_end(&mut kernel)?;
    gm.write(&kernel, GuestAddress(KERNEL_LOAD_ADDR))?;

    // 6. load initramfs (initrd)
    let mut initrd = Vec::new();
    File::open("initramfs.cpio")?.read_to_end(&mut initrd)?;
    gm.write(&initrd, GuestAddress(INITRD_ADDR))?;

    // 7. write simple cmdline at known address
    let cmdline = b"console=ttyS0 root=/dev/ram0 rw panic=-1\0";
    gm.write(cmdline, GuestAddress(CMDLINE_ADDR))?;

    // 8. Setup x86_64 registers for linux boot protocol
    let mut sregs = vcpu_fd.get_sregs()?;
    // Set up basic segment registers
    sregs.cs.selector = 0x1000;
    sregs.cs.base = 0;
    sregs.ds.selector = 0x1000;
    sregs.ds.base = 0;
    sregs.es.selector = 0x1000;
    sregs.es.base = 0;
    sregs.fs.selector = 0x1000;
    sregs.fs.base = 0;
    sregs.gs.selector = 0x1000;
    sregs.ss.selector = 0x1000;

    let mut regs = vcpu_fd.get_regs()?;
    regs.rip = KERNEL_LOAD_ADDR + 0x200; // Typical bzImage entry point offset
    regs.rsp = 0x80000u64; // stack at 512KB
    regs.rsi = KERNEL_LOAD_ADDR + 0x200; // Typically the entry point for 64-bit kernel
    regs.rdi = CMDLINE_ADDR; // cmdline pointer
    regs.rflags = 0x2;

    // Pass registers
    vcpu_fd.set_sregs(&sregs)?;
    vcpu_fd.set_regs(&regs)?;

    // 9. VCPU run loop: capture serial output via KVM_EXIT_IO
    println!("Starting VCPU. Kernel entry: 0x{:x}", regs.rip);
    loop {
        match vcpu_fd.run()? {
            VcpuExit::Hlt => {
                println!("Guest halted");
                break;
            }
            VcpuExit::IoOut(port, data) => {
                if port == 0x3f8 {
                    // serial out (COM1)
                    let s = String::from_utf8_lossy(&data);
                    print!("{}", s);
                } else {
                    println!("IO out port=0x{:x} data={:?}", port, data);
                }
            }
            VcpuExit::IoIn(port, data) => {
                // Handle IO in if needed
                println!("IO in port=0x{:x} data_len={}", port, data.len());
            }
            VcpuExit::Shutdown => {
                println!("Guest shutdown");
                break;
            }
            VcpuExit::MmioWrite(addr, data) => {
                println!("MMIO write addr=0x{:x} data={:?}", addr, data);
            }
            VcpuExit::MmioRead(addr, data) => {
                println!("MMIO read addr=0x{:x} data_len={}", addr, data.len());
            }
            VcpuExit::Debug(debug_data) => {
                println!("Debug exception: {:?}", debug_data);
            }
            r => {
                println!("Unhandled vcpu exit: {:?}", r);
                break;
            }
        }
    }

    Ok(())
}