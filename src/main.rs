#![no_std]
#![no_main]

mod eeprom;
mod temperature;
mod crypt;
mod coms_manager;
mod messages;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;

use stm32f0xx_hal as hal;
use crate::hal::spi::{Mode, Phase, Polarity, Spi};
use hal::{pac, pac::interrupt, prelude::*, pwm};

static COMS_MANAGER: Mutex<RefCell<Option<coms_manager::ComsManager>>> = Mutex::new(RefCell::new(None));

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[interrupt]
fn USART1 () {
    // Handle the interrupt
}

#[entry]
fn main() -> ! {
    if let Some(mut dp) = pac::Peripherals::take() {
        let mut rcc = dp.RCC.configure().sysclk(8.mhz()).freeze(&mut dp.FLASH);

        let gpioa = dp.GPIOA.split(&mut rcc);

        // let mut led = cortex_m::interrupt::free(|cs| gpioa.pa1.into_push_pull_output(cs));

        let eeprom_manager = cortex_m::interrupt::free(|cs| {
            eeprom::EepromManager::new(
                gpioa.pa6.into_alternate_af0(cs),
                gpioa.pa7.into_alternate_af0(cs),
                gpioa.pa5.into_alternate_af0(cs),
                &mut rcc,
                dp.SPI1,
            )
        });

        let channel = cortex_m::interrupt::free(move |cs| gpioa.pa4.into_alternate_af4(cs));

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

