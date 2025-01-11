use core::cell::RefCell;
use cortex_m::interrupt::{free, Mutex};
use rp_pico::hal::Watchdog;

pub struct Context {
    pub watchdog: RefCell<&'static mut Watchdog>,
}

pub static CONTEXT: Mutex<RefCell<Option<Context>>> = Mutex::new(RefCell::new(None));

pub fn init(watchdog: &'static mut Watchdog) {
    cortex_m::interrupt::free(|cs| {
        CONTEXT.borrow(cs).replace(Some(Context {
            watchdog: RefCell::new(watchdog),
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
