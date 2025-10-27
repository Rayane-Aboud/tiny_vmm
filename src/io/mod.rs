pub struct SerialHandler {
    port: u16,
    output: String,
}

// Constants for serial port registers
const SERIAL_PORT: u16 = 0x3f8;
const SERIAL_LINE_STATUS_PORT: u16 = SERIAL_PORT + 5;
const SERIAL_MODEM_STATUS_PORT: u16 = SERIAL_PORT + 6;

impl SerialHandler {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            output: String::new(),
        }
    }
    
    pub fn handle_io_out(&mut self, port: u16, data: &[u8], io_count: usize) {
        if port == self.port {
            // Serial output - collect characters
            let s = String::from_utf8_lossy(data);
            self.output.push_str(&s);
        } else if !Self::is_ignored_port(port) && io_count < 20 {
            println!("IO out port=0x{:x} data={:?}", port, data);
        }
    }
    
    pub fn handle_io_in(&self, port: u16, data: &mut [u8], io_count: usize) {
        match port {
            0x80 => { 
                // Delay port - return 0 
                for byte in data.iter_mut() {
                    *byte = 0;
                }
            }
            0x64 => { 
                // Keyboard controller - return ready status
                if !data.is_empty() { 
                    data[0] = 0x20; 
                }
                // Zero out remaining bytes
                for byte in data.iter_mut().skip(1) {
                    *byte = 0;
                }
            }
            0x60 => { 
                // Keyboard data - return no key pressed
                for byte in data.iter_mut() {
                    *byte = 0;
                }
            }
            _ if port == SERIAL_LINE_STATUS_PORT => { 
                // Serial line status - return transmitter empty
                if !data.is_empty() { 
                    data[0] = 0x20; 
                }
                // Zero out remaining bytes
                for byte in data.iter_mut().skip(1) {
                    *byte = 0;
                }
            }
            _ if port == SERIAL_MODEM_STATUS_PORT => { 
                // Serial modem status - return all clear
                if !data.is_empty() { 
                    data[0] = 0x20; 
                }
                // Zero out remaining bytes
                for byte in data.iter_mut().skip(1) {
                    *byte = 0;
                }
            }
            _ => {
                if !Self::is_ignored_port(port) && io_count < 20 {
                    println!("IO in port=0x{:x} data_len={}", port, data.len());
                }
                // Zero out all data for unhandled ports
                for byte in data.iter_mut() {
                    *byte = 0;
                }
            }
        }
    }
    
    pub fn print_output(&self) {
        if !self.output.is_empty() {
            println!("\n=== Guest Serial Output ===");
            println!("{}", self.output);
        } else {
            println!("\n=== No serial output received ===");
        }
    }
    
    fn is_ignored_port(port: u16) -> bool {
        matches!(port, 
            0x40 | 0x43 | 0x61 |  // PIT and PC speaker
            0x80 |                 // Delay port
            0x70 | 0x71 |         // RTC
            0x20 | 0x21 | 0xa0 | 0xa1  // PIC
        )
    }
}