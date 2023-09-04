// copied from: https://github.com/eterevsky/rp2040-blink/blob/main/src/usb_manager.rs
use sparkfun_pro_micro_rp2040::hal;
use usb_device::{self, UsbError};
use usb_device::{
    bus::UsbBusAllocator,
    device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_serial::SerialPort;

pub struct UsbManager {
    device: UsbDevice<'static, hal::usb::UsbBus>,
    serial: SerialPort<'static, hal::usb::UsbBus>,
}

impl UsbManager {
    pub fn new(usb_bus: &'static UsbBusAllocator<hal::usb::UsbBus>) -> Self {
        let serial = SerialPort::new(usb_bus);
        let device = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x2E8A, 0x000a))
            .manufacturer("Sparkfun")
            .product("Pro Micro RP2040")
            .serial_number("TEST")
            .device_class(2)
            .device_protocol(1)
            .build();
        UsbManager { device, serial }
    }

    // I think this triggers when sending data to the serial port..?
    // TODO only 1 write can happen at the same time.. so you may panic..?

    pub unsafe fn interrupt(&mut self) {
        if self.device.poll(&mut [&mut self.serial]) {
            // on interrupt - try to read from serial. If data is read
            // write pong..
            let mut buf = [0u8; 64];
            match self.serial.read(&mut buf) {
                Err(_e) => {}
                Ok(0) => {}
                Ok(count) => {
                    // Simple echo,
                    let data = &buf[0..count];
                    self.write_serial("read: ");
                    match self.serial.write(data) {
                        Ok(_len) => {}
                        Err(UsbError::WouldBlock) => {
                            // if the usb would block, try to flush the data..?
                            self.serial.flush().unwrap();
                            return;
                        }
                        Err(_) => {
                            return;
                        }
                    }
                    self.write_serial("\r\n");

                    // self.write_serial("pong\r\n");
                }
            }
        }
    }
    pub fn write_serial(&mut self, s: &str) {
        let write_ptr = s.as_bytes();

        match self.serial.write(write_ptr) {
            Ok(_len) => {}
            Err(UsbError::WouldBlock) => {
                // if the usb would block, try to flush the data..?
                self.serial.flush().unwrap();
                return;
            }
            Err(_) => {
                return;
            }
        }
        self.serial.flush().unwrap();
    }
}

impl core::fmt::Write for UsbManager {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // Just write the result without checking
        // TODO Might want some more elaborate serial checking
        // To see if the buffer is full, etc.
        // Not super elaborate - but maybe there's a better way to handle sending massive amounts of data..?
        // Since you can only send up to 64 bytes before filling the buffer..
        self.serial.write(s.as_bytes()).unwrap();
        self.serial.flush().unwrap();
        Ok(())
    }
}
