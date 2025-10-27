mod vm;
mod io;

use anyhow::Result;
use vm::{VirtualMachine, VmConfig};

fn main() -> Result<()> {
    println!("=== KVM Guest VM with Custom Binary ===\n");
    
    let config: VmConfig = VmConfig::default();
    let mut vm: VirtualMachine = VirtualMachine::new(config)?;
    
    vm.run()
}