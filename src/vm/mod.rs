pub mod config;
pub mod memory;
pub mod vcpu;
pub mod initramfs;


pub use config::VmConfig;
pub use memory::GuestMemoryManager;
pub use vcpu::Vcpu;
pub use initramfs::InitramfsBuilder;


use kvm_ioctls::Kvm;
use anyhow::Result;

pub struct VirtualMachine {
    config: VmConfig,
    kvm: Kvm,
    vm: kvm_ioctls::VmFd,
    memory: GuestMemoryManager,
    vcpu: Vcpu,
}


impl VirtualMachine {

    pub fn new(config: VmConfig) -> Result<Self> {
        // Initialize KVM
        let kvm = Kvm::new()?;
        //println!("KVM initialized (version: {})", kvm.get_api_version()?);
        
        let vm = kvm.create_vm()?;
        println!("VM created");
        
        // Setup guest memory
        let memory = GuestMemoryManager::new(&vm, config.guest_mem_size)?;
        
        // Create VCPU
        let vcpu = Vcpu::new(&vm, 0)?;
        println!("VCPU created");
        
        Ok(Self {
            config,
            kvm,
            vm,
            memory,
            vcpu,
        })
    }


    pub fn run(&mut self) -> Result<()> {
        // Build custom initramfs if needed
        if !std::path::Path::new("initramfs_custom.cpio").exists() {
            InitramfsBuilder::create_custom_initramfs()?;
        }
        
        // Load kernel
        let kernel_size = self.memory.load_file("kernel.bin", self.config.kernel_load_addr)?;
        
        // Load initramfs
        let initrd_file = if std::path::Path::new("initramfs_custom.cpio").exists() {
            "initramfs_custom.cpio"
        } else {
            "initramfs.cpio"
        };
        
        let initrd_size = self.memory.load_file(initrd_file, self.config.initrd_addr)?;
        println!("Using initramfs: {}", initrd_file);
        
        // Setup command line
        self.memory.setup_command_line(self.config.cmdline_addr)?;
        
        // Setup Linux boot parameters
        self.memory.setup_boot_params(initrd_size, self.config.zero_page_addr)?;
        
        // Setup VCPU registers
        self.vcpu.setup_registers(&self.memory, kernel_size, &self.config)?;
        
        // Start the VM
        println!("\n=== Starting VM Execution ===");
        self.vcpu.run()
    }


}