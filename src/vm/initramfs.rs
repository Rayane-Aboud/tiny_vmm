use std::fs::File;
use std::process::Command;
use tempfile::tempdir;
use anyhow::Result;

pub struct InitramfsBuilder;

impl InitramfsBuilder {
    pub fn create_custom_initramfs() -> Result<()> {
        println!("Creating custom initramfs...");
        
        let temp_dir = tempdir()?;
        
        // Extract existing initramfs if it exists
        if std::path::Path::new("initramfs.cpio").exists() {
            let input = File::open("initramfs.cpio")?;
            Command::new("cpio")
                .args(["-i", "--make-directories"])
                .stdin(input)
                .current_dir(&temp_dir)
                .status()?;
        }
        
        // Create and compile custom binary
        Self::create_custom_binary(&temp_dir)?;
        
        // Create init script
        Self::create_init_script(&temp_dir)?;
        
        // Create device nodes
        Self::create_device_nodes(&temp_dir)?;
        
        // Repackage initramfs
        Self::repackage_initramfs(&temp_dir)?;
        
        println!("Custom initramfs created: initramfs_custom.cpio");
        Ok(())
    }
    
    fn create_custom_binary(temp_dir: &tempfile::TempDir) -> Result<()> {
        let simple_c = r#"
#include <stdio.h>
#include <unistd.h>

int main() {
    printf("\n");
    printf("=== Hello from the Guest VM! ===\n");
    printf("Custom binary executed successfully!\n");
    printf("This message is printed from inside the virtual machine\n");
    printf("VM is about to shut down gracefully...\n");
    printf("\n");
    return 0;
}
"#;
        
        let source_path = temp_dir.path().join("myapp.c");
        std::fs::write(&source_path, simple_c)?;
        
        let binary_path = temp_dir.path().join("myapp");
        
        if Command::new("which").arg("gcc").output().is_ok() {
            let status = Command::new("gcc")
                .args(["-static", "-o", binary_path.to_str().unwrap(), source_path.to_str().unwrap()])
                .status();
                
            if status.is_ok() {
                println!("Successfully compiled custom binary");
                return Ok(());
            }
        }
        
        // Fallback to shell script
        println!("GCC not available, creating shell script instead");
        let shell_script = r#"#!/bin/sh
echo ""
echo "=== Hello from the Guest VM! ==="
echo "Custom script executed successfully!"
echo "This message is printed from inside the virtual machine"
echo "VM is about to shut down gracefully..."
echo ""
"#;
        std::fs::write(&binary_path, shell_script)?;
        Command::new("chmod")
            .args(["+x", binary_path.to_str().unwrap()])
            .status()?;
            
        Ok(())
    }
    
    fn create_init_script(temp_dir: &tempfile::TempDir) -> Result<()> {
        let init_script = r#"#!/bin/sh
# Mount necessary filesystems
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
mount -t devtmpfs devtmpfs /dev 2>/dev/null

echo "Starting custom initialization..."
echo ""

# Run our custom binary/script
if [ -x /myapp ]; then
    /myapp
else
    echo "Custom binary not found, running fallback message"
    echo "=== Hello from Guest VM ==="
    echo "Fallback message: VM is running!"
fi

echo ""
echo "Initialization complete. Shutting down..."
poweroff -f
"#;
        
        std::fs::write(temp_dir.path().join("init"), init_script)?;
        Command::new("chmod")
            .args(["+x", temp_dir.path().join("init").to_str().unwrap()])
            .status()?;
            
        Ok(())
    }
    
    fn create_device_nodes(temp_dir: &tempfile::TempDir) -> Result<()> {
        std::fs::create_dir_all(temp_dir.path().join("dev"))?;
        Ok(())
    }
    
    fn repackage_initramfs(temp_dir: &tempfile::TempDir) -> Result<()> {
        let output = File::create("initramfs_custom.cpio")?;
        
        let status = Command::new("sh")
            .args(["-c", "find . -print0 | cpio -0 -o -H newc"])
            .current_dir(&temp_dir)
            .stdout(output)
            .status()?;
        
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to create cpio archive"));
        }
        
        Ok(())
    }
}