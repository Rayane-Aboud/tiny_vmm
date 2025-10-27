use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::VmFd;
use vm_memory::{Bytes, GuestAddress, GuestMemory, GuestMemoryMmap};
use std::sync::Arc;
use std::fs::File;
use std::io::Read;
use anyhow::Result;

use super::config::VmConfig;

pub struct GuestMemoryManager {
    pub guest_mem: Arc<GuestMemoryMmap<()>>,
    config: VmConfig,
}

impl GuestMemoryManager {
    pub fn new(vm: &VmFd, mem_size: u64) -> Result<Self> {
        let guest_mem = Arc::new(
            GuestMemoryMmap::from_ranges(&[(GuestAddress(0), mem_size as usize)])?
        );

        let mem_region = kvm_userspace_memory_region {
            slot: 0,
            guest_phys_addr: 0,
            memory_size: mem_size,
            userspace_addr: guest_mem.get_host_address(GuestAddress(0))? as u64,
            flags: 0,
        };
        
        unsafe { vm.set_user_memory_region(mem_region)? };
        println!("Guest memory configured: {} MiB", mem_size / 1024 / 1024);
        
        Ok(Self {
            guest_mem,
            config: VmConfig::default(),
        })
    }
    
    pub fn load_file(&self, filename: &str, address: u64) -> Result<usize> {
        let mut file_content = Vec::new();
        let file_size = File::open(filename)?.read_to_end(&mut file_content)?;
        
        println!("Loading {}: {} bytes at 0x{:x}", filename, file_size, address);
        
        if filename == "kernel.bin" {
            if file_size > 0x202 && &file_content[0x202..0x206] == b"HdrS" {
                println!("  Detected bzImage format");
                let setup_sects = file_content[0x1f1] as usize;
                println!("  Setup sectors: {}", setup_sects);
                
                let loadflags = file_content[0x211];
                let is_64bit = (loadflags & 0x1) != 0;
                println!("  64-bit kernel: {}", is_64bit);
            }
        }
        
        self.guest_mem.write(&file_content, GuestAddress(address))?;
        Ok(file_size)
    }
    
    pub fn setup_command_line(&self, address: u64) -> Result<()> {
        let cmdline = b"console=ttyS0 earlyprintk=ttyS0 root=/dev/ram0 rw panic=1 debug\0";
        self.guest_mem.write(cmdline, GuestAddress(address))?;
        println!("Command line configured at 0x{:x}", address);
        Ok(())
    }
    
    pub fn setup_boot_params(&self, initrd_size: usize, zero_page_addr: u64) -> Result<()> {
        // Zero out the boot params area
        let zero_page = vec![0u8; 0x1000];
        self.guest_mem.write(&zero_page, GuestAddress(zero_page_addr))?;
        
        // Set up basic boot parameters
        let hdr_signature = b"HdrS";
        self.guest_mem.write(hdr_signature, GuestAddress(zero_page_addr + 0x202))?;
        
        self.guest_mem.write_obj(0x212u16, GuestAddress(zero_page_addr + 0x206))?;
        self.guest_mem.write_obj(0xffu8, GuestAddress(zero_page_addr + 0x210))?;
        
        // Command line pointer
        self.guest_mem.write_obj(self.config.cmdline_addr as u64, GuestAddress(zero_page_addr + 0x228))?;
        
        // Command line size
        let cmdline = b"console=ttyS0 earlyprintk=ttyS0 root=/dev/ram0 rw panic=1 debug\0";
        self.guest_mem.write_obj(cmdline.len() as u32, GuestAddress(zero_page_addr + 0x238))?;
        
        // Initrd address and size
        self.guest_mem.write_obj(self.config.initrd_addr as u64, GuestAddress(zero_page_addr + 0x218))?;
        self.guest_mem.write_obj(initrd_size as u64, GuestAddress(zero_page_addr + 0x220))?;
        
        // E820 memory map
        let e820_entries = [
            (0, 0x9fc00, 1),
            (0x9fc00, 0x400, 2),
            (0x100000, self.config.guest_mem_size - 0x100000, 1),
        ];
        
        self.guest_mem.write_obj(3u8, GuestAddress(zero_page_addr + 0x1e8))?;
        for (i, (addr, size, typ)) in e820_entries.iter().enumerate() {
            let base = zero_page_addr + 0x2d0 + (i * 20) as u64;
            self.guest_mem.write_obj(*addr as u64, GuestAddress(base))?;
            self.guest_mem.write_obj(*size as u64, GuestAddress(base + 8))?;
            self.guest_mem.write_obj(*typ as u32, GuestAddress(base + 16))?;
        }
        
        println!("Linux boot parameters configured at 0x{:x}", zero_page_addr);
        Ok(())
    }
    
    pub fn guest_mem(&self) -> &Arc<GuestMemoryMmap<()>> {
        &self.guest_mem
    }
}