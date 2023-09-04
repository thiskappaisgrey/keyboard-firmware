//! # Rainbow Example for the Pro Micro RP2040
//!
//! Runs a rainbow-effect colour wheel on the on-board LED.
//!
//! Uses the `ws2812_pio` driver to control the LED, which in turns uses the
//! RP2040's PIO block.

#![no_std]
#![no_main]
mod layout;
mod usb_manager;

use core::fmt::Write as _;
use core::iter::once;
use core::panic::PanicInfo;
use embedded_hal::timer::CountDown;
use fugit::ExtU32;
use keyberon::debounce::Debouncer;
use keyberon::key_code::KbHidReport;
use keyberon::layout::Layout;
use keyberon::matrix::Matrix;
use smart_leds::{brightness, SmartLedsWrite, RGB8};
use sparkfun_pro_micro_rp2040::entry;
use sparkfun_pro_micro_rp2040::hal::gpio::DynPin;
use sparkfun_pro_micro_rp2040::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        pac,
        pac::interrupt,
        pio::PIOExt,
        rom_data,
        timer::Timer,
        usb::UsbBus,
        watchdog::Watchdog,
        Sio,
    },
    XOSC_CRYSTAL_FREQ,
};
use usb_device::class_prelude::{UsbBusAllocator, UsbClass};
use usb_device::prelude::UsbDevice;
use ws2812_pio::Ws2812;

use crate::layout::LAYERS;
use crate::usb_manager::*;

static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_MANAGER: Option<UsbManager> = None;

#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    match USB_MANAGER.as_mut() {
        Some(manager) => manager.interrupt(),
        None => (),
    };
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    if let Some(usb) = unsafe { USB_MANAGER.as_mut() } {
        usb.write_serial("Panicked!\r\n");
    }
    // reset into usb boot on panic
    rom_data::reset_to_usb_boot(0, 0);
    loop {}
}

/// Entry point to our bare-metal application.
///
/// The `#[entry]` macro ensures the Cortex-M start-up code calls this
/// function as soon as all global variables are initialised.
///
/// The function configures the RP2040 peripherals, then the LED, then runs
/// the colour wheel in an infinite loop.
#[entry]
fn main() -> ! {
    // Configure the RP2040 peripherals

    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);

    let pins = sparkfun_pro_micro_rp2040::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // TODO Read stuff from serial

    // USB stuff
    // TODO This is in an unsafe block for now(I think because it changes a global), but I can/will move it
    // out after integrating stuff.
    // TODO Both splits won't be connected to usb.. This imght cause an panic..?
    let usb = unsafe {
        USB_BUS = Some(UsbBusAllocator::new(UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        )));
        USB_MANAGER = Some(UsbManager::new(USB_BUS.as_ref().unwrap()));
        // Enable the USB interrupt
        pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
        USB_MANAGER.as_mut().unwrap()
    };

    // let mut usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, ());
    // let mut usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

    // // The debouncer in keyberon creates the events
    // let mut debouncer = Debouncer::new([[false; 7]; 5], [[false; 7]; 5], 20);
    // let mut layout = Layout::new(&LAYERS);

    // TODO Enable UART as well - I need the rx pin
    // to be doing tx/rx depending on whether it's the master or slave.

    // TODO I need to figure out how to read keycodes and if I need to actually handle the reset button.
    // I think - without having to code anything, pressing reset twice SHOULD reset the board? We'll see though.

    // TODO

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut delay = timer.count_down();

    // Configure the addressable LED
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut ws = Ws2812::new(
        pins.led.into_mode(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    // write to the color wheel
    ws.write(brightness(once(wheel(10)), 32)).unwrap();

    // write to the usb..?
    // usb.write_serial("Hello\r\n");
    loop {
        // have to use "\r\n" as a newline character
        // b/c tio won't work otherwise..
        // let keys_pressed = matrix.get().unwrap();
        // let deb_events = debouncer.events(keys_pressed);
        // if deb_events.count() > 1 {
        //     usb.write_serial("Events greater than 1");
        // }

        // TODO I need some sort of lock for this
        // Since it will panic if I try to write from multiple places at the same time..
        usb.write_serial("LED\r\n");

        // delay for 1 millisecond to allow the
        delay.start(3000.millis());
        let _ = nb::block!(delay.wait());
    }
}

/// Convert a number from `0..=255` to an RGB color triplet.
///
/// The colours are a transition from red, to green, to blue and back to red.
fn wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        // No green in this sector - red and blue only
        (255 - (wheel_pos * 3), 0, wheel_pos * 3).into()
    } else if wheel_pos < 170 {
        // No red in this sector - green and blue only
        wheel_pos -= 85;
        (0, wheel_pos * 3, 255 - (wheel_pos * 3)).into()
    } else {
        // No blue in this sector - red and green only
        wheel_pos -= 170;
        (wheel_pos * 3, 255 - (wheel_pos * 3), 0).into()
    }
}
