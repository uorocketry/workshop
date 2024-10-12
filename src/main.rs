#![no_std]
#![no_main]

mod coms_manager;
mod crypt;
mod eeprom;
mod messages;
mod mux; 

use core::cell::RefCell;
use core::ops::DerefMut;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;

use hal::{pac, pac::interrupt, prelude::*, pwm, serial::Serial};
use stm32f0xx_hal::{self as hal, adc::Adc};

static COMS: Mutex<RefCell<Option<coms_manager::ComsManager>>> = Mutex::new(RefCell::new(None));

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[interrupt]
fn USART1() {
    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut coms_manager) = COMS.borrow(cs).borrow_mut().deref_mut() {
            coms_manager.read_byte();
        }
    });
}

#[entry]
fn main() -> ! {
    if let Some(mut dp) = pac::Peripherals::take() {
        let mut rcc = dp.RCC.configure().sysclk(8.mhz()).freeze(&mut dp.FLASH);

        let gpioa = dp.GPIOA.split(&mut rcc);

        let mut eeprom_manager = cortex_m::interrupt::free(|cs| {
            eeprom::EepromManager::new(
                gpioa.pa6.into_alternate_af0(cs),
                gpioa.pa7.into_alternate_af0(cs),
                gpioa.pa5.into_alternate_af0(cs),
                &mut rcc,
                dp.SPI1,
            )
        });

        // create a scope to free the memory used by the keys.
        {
            // generate an RSA key pair. Never do this in production code.
            let pub_key = crypt::RSAPublicKey::new(0x10001, 0x10001);
            let priv_key = crypt::RSAPrivateKey::new(0x10001, 0x12345);

            // store the private key in the EEPROM
            let priv_key_bytes = priv_key.to_bytes();

            for i in 0..16 {
                eeprom_manager.write_memory(i, priv_key_bytes[i as usize]);
            }

            // store the public key in the EEPROM
            let pub_key_bytes = pub_key.to_bytes();

            for i in 0..16 {
                eeprom_manager.write_memory(i + 16, pub_key_bytes[i as usize]);
            }

            // generate the AES key
            let seed = [
                0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10,
            ];
            let aes_key = crypt::generate_aes_key(&seed);

            // store the AES key in the EEPROM
            for i in 0..16 {
                eeprom_manager.write_memory(i + 32, aes_key[i as usize]);
            }
        }

        let tx = cortex_m::interrupt::free(move |cs| gpioa.pa9.into_alternate_af1(cs));
        let rx = cortex_m::interrupt::free(move |cs| gpioa.pa10.into_alternate_af1(cs));

        let serial = Serial::usart1(dp.USART1, (tx, rx), 115_200.bps(), &mut rcc);

        let (tx, rx) = serial.split();

        let coms_manager = coms_manager::ComsManager::new(tx, rx);

        cortex_m::interrupt::free(|cs| {
            COMS.borrow(cs).replace(Some(coms_manager));
        });

        let adc = Adc::new(dp.ADC, &mut rcc); 

        let mux = cortex_m::interrupt::free(move |cs| {
            mux::Mux::new(
                gpioa.pa1.into_push_pull_output(cs),
                gpioa.pa0.into_push_pull_output(cs),
                gpioa.pa2.into_push_pull_output(cs),
                adc
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
