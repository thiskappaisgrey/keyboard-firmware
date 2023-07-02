#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

#[rtic::app(
    device = sparkfun_pro_micro_rp2040::hal::pac,
    dispatchers = [TIMER_IRQ_1]
)]
mod app {
    use sparkfun_pro_micro_rp2040::hal::{
        clocks, gpio,
        gpio::pin::bank0::{Gpio2, Gpio25, Gpio3},
        pac,
        pio::PIOExt,
        sio::Sio,
        watchdog::Watchdog,
        Clock, I2C,
    };
    use sparkfun_pro_micro_rp2040::XOSC_CRYSTAL_FREQ;
    use ws2812_pio::Ws2812Direct;

    use core::{iter::once, mem::MaybeUninit};
    use fugit::RateExtU32;
    use rtic_monotonics::rp2040::*;

    use panic_probe as _;
    use smart_leds::{brightness, SmartLedsWrite, RGB8};

    type I2CBus = I2C<
        pac::I2C1,
        (
            gpio::Pin<Gpio2, gpio::FunctionI2C>,
            gpio::Pin<Gpio3, gpio::FunctionI2C>,
        ),
    >;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Ws2812Direct<rp2040_pac::PIO0, rp_pico::hal::pio::SM0, Gpio25>,
        i2c: &'static mut I2CBus,
    }

    #[init(local=[
        // Task local initialized resources are static
        // Here we use MaybeUninit to allow for initialization in init()
        // This enables its usage in driver initialization
        i2c_ctx: MaybeUninit<I2CBus> = MaybeUninit::uninit()
    ])]
    fn init(mut ctx: init::Context) -> (Shared, Local) {
        // Initialize the interrupt for the RP2040 timer and obtain the token
        // proving that we have.
        let rp2040_timer_token = rtic_monotonics::create_rp2040_monotonic_token!();
        // Configure the clocks, watchdog - The default is to generate a 125 MHz system clock
        Timer::start(ctx.device.TIMER, &mut ctx.device.RESETS, rp2040_timer_token); // default rp2040 clock-rate is 125MHz
        let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);
        let clocks = clocks::init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            ctx.device.XOSC,
            ctx.device.CLOCKS,
            ctx.device.PLL_SYS,
            ctx.device.PLL_USB,
            &mut ctx.device.RESETS,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        // Init LED pin
        let sio = Sio::new(ctx.device.SIO); // Single-cycle IO
        let gpioa = sparkfun_pro_micro_rp2040::Pins::new(
            // interesting that the context stores
            // peripherals and stuff.
            // unlike the no-rtic case,
            // rtic initializes the device for us..?
            // device = pac.
            ctx.device.IO_BANK0,
            ctx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut ctx.device.RESETS,
        );
        // let mut delay = timer.count_down();

        // configure led
        let (mut pio, sm0, _, _, _) = ctx.device.PIO0.split(&mut ctx.device.RESETS);
        let mut ws = Ws2812Direct::new(
            gpioa.led.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );

        // let mut led = gpioa.led.into_push_pull_output();
        // led.set_low().unwrap();

        // Init I2C pins
        let sda_pin = gpioa.gpio2.into_mode::<gpio::FunctionI2C>();
        let scl_pin = gpioa.gpio3.into_mode::<gpio::FunctionI2C>();

        // Init I2C itself, using MaybeUninit to overwrite the previously
        // uninitialized i2c_ctx variable without dropping its value
        // (i2c_ctx definined in init local resources above)
        let i2c_tmp: &'static mut _ = ctx.local.i2c_ctx.write(I2C::i2c1(
            ctx.device.I2C1,
            sda_pin,
            scl_pin,
            100.kHz(),
            &mut ctx.device.RESETS,
            &clocks.system_clock,
        ));

        // Spawn heartbeat task
        heartbeat::spawn().ok();

        // Return resources and timer
        (
            Shared {},
            Local {
                led: ws,
                i2c: i2c_tmp,
            },
        )
    }

    #[task(local = [i2c, led])]
    async fn heartbeat(ctx: heartbeat::Context) {
        // Loop forever.
        //
        // It is important to remember that tasks that loop
        // forever should have an `await` somewhere in that loop.
        //
        // Without the await, the task will never yield back to
        // the async executor, which means that no other lower or
        // equal  priority task will be able to run.
        let mut n: u8 = 128;

        loop {
            // Flicker the built-in LED
            // TODO important: the ws2812 needs at least a 60 microsecond delay

            ctx.local.led.write(brightness(once(wheel(n)), 32)).unwrap();
            n = n.wrapping_add(1);

            // Congrats, you can use your i2c and have access to it here,
            // now to do something with it!

            // Delay for 1 second
            Timer::delay(25.millis()).await;
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
}
