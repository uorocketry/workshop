#![no_std]
#![no_main]

use cortex_m_rt::entry;

use stm32f0xx_hal as hal;

use hal::{pac, prelude::*, pwm};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[entry]
fn main() -> ! {
    if let Some(mut dp) = pac::Peripherals::take() {
        let mut rcc = dp.RCC.configure().sysclk(8.mhz()).freeze(&mut dp.FLASH);

        let gpioa = dp.GPIOA.split(&mut rcc);

        // let mut led: stm32f0xx_hal::gpio::gpioa::PA1<stm32f0xx_hal::gpio::Output<stm32f0xx_hal::gpio::PushPull>> = cortex_m::interrupt::free(|cs| gpioa.pa1.into_push_pull_output(cs));

        let channel = cortex_m::interrupt::free(move |cs| {
            gpioa.pa4.into_alternate_af4(cs)
        });

        let pwm = pwm::tim14(dp.TIM14, channel, &mut rcc, 20u32.khz());
        let mut ch1 = pwm;
        let max_duty = ch1.get_max_duty();
        ch1.set_duty(max_duty / 2);
        ch1.enable();
    }
    loop {
        cortex_m::asm::nop();
    }
}
