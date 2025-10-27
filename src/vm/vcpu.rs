use kvm_ioctls::{VcpuExit, VmFd};
use vm_memory::{GuestAddress, GuestMemoryMmap};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use anyhow::Result;

use super::config::VmConfig;
use super::memory::GuestMemoryManager;
use crate::io::SerialHandler;

pub struct Vcpu {
    vcpu_fd: kvm_ioctls::VcpuFd,
    serial_handler: SerialHandler,
}

impl Vcpu {
    pub fn new(vm: &VmFd, id: u64) -> Result<Self> {
        let vcpu_fd = vm.create_vcpu(id)?;
        let serial_handler = SerialHandler::new(0x3f8);
        
        Ok(Self {
            vcpu_fd,
            serial_handler,
        })
    }
    
    pub fn setup_registers(&mut self, memory: &GuestMemoryManager, kernel_size: usize, config: &VmConfig) -> Result<()> {
        let mut sregs = self.vcpu_fd.get_sregs()?;
        let mut regs = self.vcpu_fd.get_regs()?;

        // Read setup_sects from bzImage header
        let setup_sects = if kernel_size > 0x1f1 {
            let mut header_buf = [0u8; 512];
            let mut kernel_file = File::open("kernel.bin")?;
            kernel_file.read_exact(&mut header_buf)?;
            header_buf[0x1f1] as u64
        } else {
            0
        };
        
        let entry_point = config.kernel_load_addr + (setup_sects + 1) * 512;
        
        // Configure control registers
        sregs.cr0 = 0x80000001;
        sregs.cr4 = 0x00000000;
        
        // Setup segment registers
        sregs.cs.selector = 0x8;
        sregs.cs.base = 0;
        sregs.cs.limit = 0xffffffff;
        sregs.cs.g = 1;
        sregs.cs.db = 1;
        
        sregs.ds.selector = 0x10;
        sregs.ds.base = 0;
        sregs.ds.limit = 0xffffffff;
        sregs.ds.g = 1;
        sregs.ds.db = 1;
        
        sregs.es = sregs.ds.clone();
        sregs.fs = sregs.ds.clone();
        sregs.gs = sregs.ds.clone();
        sregs.ss = sregs.ds.clone();
        
        // Setup general registers
        regs.rip = entry_point;
        regs.rsp = 0x80000;
        regs.rflags = 0x2;
        regs.rdi = config.zero_page_addr;

        self.vcpu_fd.set_sregs(&sregs)?;
        self.vcpu_fd.set_regs(&regs)?;
        
        println!("VCPU registers configured (entry: 0x{:x})", entry_point);
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<()> {
        let mut io_count = 0;
        
        println!("VM started - waiting for guest output...");
        
        loop {
            match self.vcpu_fd.run()? {
                VcpuExit::Hlt => {
                    println!("\n>>> Guest halted gracefully");
                    self.serial_handler.print_output();
                    break Ok(());
                }
                VcpuExit::Shutdown => {
                    println!("\n>>> Guest requested shutdown");
                    self.serial_handler.print_output();
                    break Ok(());
                }
                VcpuExit::IoOut(port, data) => {
                    io_count += 1;
                    self.serial_handler.handle_io_out(port, &data, io_count);
                }
                VcpuExit::IoIn(port, data) => {
                    io_count += 1;
                    self.serial_handler.handle_io_in(port, data, io_count);
                }
                VcpuExit::Debug(debug_data) => {
                    println!("Debug: {:?}", debug_data);
                }
                VcpuExit::MmioWrite(addr, data) => {
                    println!("MMIO write addr=0x{:x} data={:?}", addr, data);
                }
                VcpuExit::MmioRead(addr, data) => {
                    println!("MMIO read addr=0x{:x} data_len={}", addr, data.len());
                }
                VcpuExit::InternalError => {
                    println!("Internal KVM error");
                    break Ok(());
                }
                other_exit => {
                    println!("Unhandled exit: {:?}", other_exit);
                    break Ok(());
                }
            }
            
            if io_count > 50000 {
                println!("Reached IO operation limit");
                break Ok(());
            }
        }
    }
}