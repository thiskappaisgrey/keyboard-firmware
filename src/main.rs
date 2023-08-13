#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod layout;

#[rtic::app(
    device = sparkfun_pro_micro_rp2040::hal::pac,
    peripherals = true,
    dispatchers = [PIO0_IRQ_0]
)]
mod app {
    use core::iter::once;
    // use embedded_time::duration::units::*;
    use cortex_m::prelude::{
        _embedded_hal_watchdog_Watchdog, _embedded_hal_watchdog_WatchdogEnable,
    };
    use fugit::ExtU32;
    use hal::timer::*;
    use rtic_monotonics::rp2040::*;
    use sparkfun_pro_micro_rp2040::hal;
    use sparkfun_pro_micro_rp2040::hal::{
        clocks, gpio, gpio::pin::bank0::Gpio25, pio::PIOExt, sio::Sio, watchdog::Watchdog, Clock,
    };
    use sparkfun_pro_micro_rp2040::XOSC_CRYSTAL_FREQ;
    use ws2812_pio::Ws2812Direct;

    use panic_probe as _;
    use smart_leds::{brightness, SmartLedsWrite, RGB8};

    use usb_device::class_prelude::*;

    static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<rp2040_hal::usb::UsbBus>> = None;
    const SCAN_TIME_US: u32 = 1000;

    // shared reference is when you have immutable data.
    #[shared]
    struct Shared {
        usb_dev: usb_device::device::UsbDevice<'static, rp2040_hal::usb::UsbBus>,
        usb_class: keyberon::hid::HidClass<
            'static,
            rp2040_hal::usb::UsbBus,
            keyberon::keyboard::Keyboard<()>,
        >,
        // uart: rp2040_hal::pac::UART0,
        layout: keyberon::layout::Layout<7, 5, 1, crate::layout::Action>,
    }

    // Local reference is when you need locks
    #[local]
    struct Local {
        led: Ws2812Direct<rp2040_pac::PIO0, sparkfun_pro_micro_rp2040::hal::pio::SM0, Gpio25>,
        watchdog: hal::watchdog::Watchdog,
        // chording: keyberon::chording::Chording<4>,
        matrix: keyberon::matrix::Matrix<gpio::DynPin, gpio::DynPin, 7, 5>,
        debouncer: keyberon::debounce::Debouncer<[[bool; 7]; 5]>,
        alarm: hal::timer::Alarm0,
        // transform: fn(layout::Event) -> layout::Event,
        // is_right: bool,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local) {
        // Initialize the interrupt for the RP2040 timer and obtain the token
        // proving that we have.
        // Configure the clocks, watchdog - The default is to generate a 125 MHz system clock
        // Timer::start(ctx.device.TIMER, &mut ctx.device.RESETS, rp2040_timer_token); // default rp2040 clock-rate is 125MHz
        let mut resets = ctx.device.RESETS;
        let mut timer = hal::Timer::new(ctx.device.TIMER, &mut resets);
        let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);
        let clocks = clocks::init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            ctx.device.XOSC,
            ctx.device.CLOCKS,
            ctx.device.PLL_SYS,
            ctx.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        // Init LED pin
        let sio = Sio::new(ctx.device.SIO); // Single-cycle IO
        let pins = sparkfun_pro_micro_rp2040::Pins::new(
            // interesting that the context stores
            // peripherals and stuff.
            // unlike the no-rtic case,
            // rtic initializes the device for us..?
            // device = pac.
            ctx.device.IO_BANK0,
            ctx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        // configure led
        let (mut pio, sm0, _, _, _) = ctx.device.PIO0.split(&mut resets);
        let ws = Ws2812Direct::new(
            pins.led.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );
        // FIXME this is somehow wrong..?
        let matrix = keyberon::matrix::Matrix::new(
            [
                pins.adc3.into_pull_up_input().into(),
                pins.adc2.into_pull_up_input().into(),
                pins.adc1.into_pull_up_input().into(),
                pins.adc0.into_pull_up_input().into(),
                pins.copi.into_pull_up_input().into(),
                pins.cipo.into_pull_up_input().into(),
                pins.sck.into_pull_up_input().into(),
            ],
            [
                pins.gpio4.into_push_pull_output().into(),
                pins.gpio5.into_push_pull_output().into(),
                pins.gpio6.into_push_pull_output().into(),
                pins.gpio7.into_push_pull_output().into(),
                pins.tx1.into_push_pull_output().into(),
            ],
        )
        .unwrap();
        let layout = keyberon::layout::Layout::new(&crate::layout::LAYERS);
        let debouncer = keyberon::debounce::Debouncer::new([[false; 7]; 5], [[false; 7]; 5], 20);

        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US.micros());
        alarm.enable_interrupt();

        let usb_bus = UsbBusAllocator::new(rp2040_hal::usb::UsbBus::new(
            ctx.device.USBCTRL_REGS,
            ctx.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        ));
        unsafe {
            USB_BUS = Some(usb_bus);
        }
        let usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, ());
        let usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

        // TODO for a split keyboard, w/ TRRS cable, there's only a 1-way communication
        // master needs to communicate with slave, essentially.

        // TODO need to figure out which microcontroller is connected to the computer
        // so that I can know which one should be the reciever and which one should be sender.

        // TODO implement a ValidUartPinout for only TX / RX enabled

        // TODO Usb communication needs to be handled as well..

        // let mut led = gpioa.led.into_push_pull_output();
        // led.set_low().unwrap();

        // Spawn heartbeat task
        // heartbeat::spawn().ok();
        watchdog.start(fugit::ExtU32::micros(10_000));

        // Return resources and timer
        (
            Shared {
                usb_dev,
                usb_class,
                layout,
            },
            Local {
                led: ws,
                matrix,
                watchdog,
                alarm,
                debouncer,
            },
        )
    }

    #[task(binds = USBCTRL_IRQ, priority = 4, shared = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        (usb, kb).lock(|usb, kb| {
            if usb.poll(&mut [kb]) {
                kb.poll();
            }
        });
    }

    #[task(
        binds = TIMER_IRQ_0,
        priority = 1,
        local = [matrix, watchdog, alarm, debouncer, led],
        shared=[usb_dev, usb_class, layout]
    )]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let alarm = c.local.alarm;
        alarm.clear_interrupt();
        let _ = alarm.schedule(SCAN_TIME_US.micros());

        c.local.watchdog.feed();
        let keys_pressed = c.local.matrix.get().unwrap();

        for keycol in keys_pressed {
            for key_row in keycol {
                if key_row {
                    c.local.led.write(brightness(once(wheel(25)), 32)).unwrap();
                }
            }
        }

        let events = c.local.debouncer.events(keys_pressed);
        let _ = c.shared.layout.lock(|l| l.tick());
        let mut n: u8 = 128;
        for event in events {
            n = n.wrapping_add(n);
            // TODO
            c.local.led.write(brightness(once(wheel(n)), 32)).unwrap();
            c.shared.layout.lock(|l| l.event(event));
            return;
        }
        let report: keyberon::key_code::KbHidReport =
            c.shared.layout.lock(|l| l.keycodes().collect());
        if !c
            .shared
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        if c.shared.usb_dev.lock(|d| d.state()) != usb_device::prelude::UsbDeviceState::Configured {
            return;
        }
        while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    // #[task(local = [led])]
    // async fn heartbeat(ctx: heartbeat::Context) {
    //     // Loop forever.
    //     //
    //     // It is important to remember that tasks that loop
    //     // forever should have an `await` somewhere in that loop.
    //     //
    //     // Without the await, the task will never yield back to
    //     // the async executor, which means that no other lower or
    //     // equal  priority task will be able to run.
    //     let mut n: u8 = 128;

    //     loop {
    //         // Flicker the built-in LED
    //         // TODO important: the ws2812 needs at least a 60 microsecond delay

    //         ctx.local.led.write(brightness(once(wheel(n)), 32)).unwrap();
    //         n = n.wrapping_add(1);

    //         // Congrats, you can use your i2c and have access to it here,
    //         // now to do something with it!
    //         // Delay for 1 second
    //         //Timer::delay(25.millis()).await;
    //     }
    // }

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
}
