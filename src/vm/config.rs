#[derive(Debug, Clone)]
pub struct VmConfig {
    pub guest_mem_size: u64,
    pub kernel_load_addr: u64,
    pub cmdline_addr: u64,
    pub initrd_addr: u64,
    pub zero_page_addr: u64,
    pub serial_port: u16,
}


impl Default for VmConfig {
    fn default() -> Self {
        Self {
            guest_mem_size: 64 * 1024 * 1024, // 64 MiB
            kernel_load_addr: 0x100000,
            cmdline_addr: 0x20000,
            initrd_addr: 0x800000,
            zero_page_addr: 0x10000,
            serial_port: 0x3f8,
        }
    }
}