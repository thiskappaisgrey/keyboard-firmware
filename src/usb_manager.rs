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
    pub unsafe fn interrupt(&mut self) {
        if self.device.poll(&mut [&mut self.serial]) {
            // on interrupt - try to read from serial. If data is read
            // write pong..
            let mut buf = [0u8; 64];
            match self.serial.read(&mut buf) {
                Err(_e) => {}
                Ok(0) => {}
                Ok(_count) => {
                    self.write_serial("pong\r\n");
                }
            }
        }
    }
    pub fn write_serial(&mut self, s: &str) {
        let write_ptr = s.as_bytes();

        self.serial.write(write_ptr).unwrap();
        self.serial.flush().unwrap();
    }
}

impl core::fmt::Write for UsbManager {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let write_ptr = s.as_bytes();
        let mut index = 0;

        // Because the buffer is of constant size and initialized to zero (0) we here
        // add a test to determine the size that's really occupied by the str that we
        // wan't to send. From index zero to first byte that is as the zero byte value.
        while index < write_ptr.len() && write_ptr[index] != 0 {
            index += 1;
        }
        let mut write_ptr = &write_ptr[0..index];
        while !write_ptr.is_empty() {
            match self.serial.write(write_ptr) {
                Ok(len) => write_ptr = &write_ptr[len..],
                Err(UsbError::WouldBlock) => {
                    break;
                }
                Err(_) => break,
            }
        }
        let _ = self.serial.flush();
        // self.serial.write(s.as_bytes()).unwrap();

        Ok(())
    }
}
