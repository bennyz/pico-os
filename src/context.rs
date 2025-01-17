use core::cell::RefCell;
use cortex_m::delay::Delay;
use cortex_m::interrupt::{free, Mutex};
use rp_pico::hal::gpio::PullDown;
use rp_pico::hal::{
    gpio::{bank0::Gpio25, FunctionSioOutput, Pin},
    Adc, Watchdog,
};

pub struct Context {
    pub watchdog: RefCell<&'static mut Watchdog>,
    pub led: RefCell<Pin<Gpio25, FunctionSioOutput, PullDown>>,
    pub delay: RefCell<Delay>,
    pub adc: RefCell<Adc>,
}

pub static CONTEXT: Mutex<RefCell<Option<Context>>> = Mutex::new(RefCell::new(None));

pub fn init(
    watchdog: &'static mut Watchdog,
    led: Pin<Gpio25, FunctionSioOutput, PullDown>,
    delay: Delay,
    adc: Adc,
) {
    cortex_m::interrupt::free(|cs| {
        CONTEXT.borrow(cs).replace(Some(Context {
            watchdog: RefCell::new(watchdog),
            led: RefCell::new(led),
            delay: RefCell::new(delay),
            adc: RefCell::new(adc),
        }));
    });
}

pub fn with_context<R>(f: impl FnOnce(&Context) -> R) -> R {
    free(|cs| {
        let context_ref = CONTEXT.borrow(cs);
        let context = context_ref.borrow();
        f(context.as_ref().unwrap())
    })
}
