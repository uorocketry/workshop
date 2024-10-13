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
use messages::Temperature;
use stm32f0xx_hal::{self as hal, adc::Adc};

// What is a Mutex? It is a mutual exclusion primitive that can be used to protect shared data from being accessed by multiple threads at the same time.
// What is a RefCell? It is a mutable memory location with dynamically checked borrow rules.
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

        let adc = Adc::new(dp.ADC, &mut rcc);

        let mut mux = cortex_m::interrupt::free(move |cs| {
            mux::Mux::new(
                gpioa.pa1.into_push_pull_output(cs),
                gpioa.pa0.into_push_pull_output(cs),
                gpioa.pa2.into_push_pull_output(cs),
                adc,
            )
        });

        mux.execute(mux::Channel::RedLED);

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

            eeprom_manager.write_16_byte_key(priv_key_bytes, eeprom::RSA_PRIV_KEY_ADDRESS);

            // store the public key in the EEPROM
            let pub_key_bytes = pub_key.to_bytes();

            eeprom_manager.write_16_byte_key(pub_key_bytes, eeprom::RSA_PUB_KEY_ADDRESS);

            // generate the AES key
            let seed = [0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];
            let aes_key = crypt::generate_aes_key(&seed);

            // store the AES key in the EEPROM
            eeprom_manager.write_8_byte_key(aes_key, eeprom::AES_KEY_ADDRESS);
        }

        let tx = cortex_m::interrupt::free(move |cs| gpioa.pa9.into_alternate_af1(cs));
        let rx = cortex_m::interrupt::free(move |cs| gpioa.pa10.into_alternate_af1(cs));

        let serial = Serial::usart1(dp.USART1, (tx, rx), 115_200.bps(), &mut rcc);

        let (tx, rx) = serial.split();

        let coms_manager = coms_manager::ComsManager::new(tx, rx);

        cortex_m::interrupt::free(|cs| {
            COMS.borrow(cs).replace(Some(coms_manager));
        });

        let channel = cortex_m::interrupt::free(move |cs| gpioa.pa4.into_alternate_af4(cs));

        let pwm = pwm::tim14(dp.TIM14, channel, &mut rcc, 20u32.khz());
        let mut ch1 = pwm;
        let max_duty = ch1.get_max_duty();
        ch1.set_duty(max_duty / 2);
        ch1.enable();

        // init complete
        mux.execute(mux::Channel::GreenLED);

        // we can now enter the main loop and start brodcasting
        loop {
            let msg = cortex_m::interrupt::free(|cs| {
                if let Some(ref mut coms_manager) = COMS.borrow(cs).borrow_mut().deref_mut() {
                    if coms_manager.has_new_message() {
                        coms_manager.receive()
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            if let Some(msg) = msg {
                cortex_m::interrupt::free(|cs| {
                    match msg.data {
                        messages::Data::Command(cmd) => match cmd {
                            messages::Command::DeleteAESKey => {
                                eeprom_manager.write_8_byte_key(
                                    [0, 0, 0, 0, 0, 0, 0, 0],
                                    eeprom::AES_KEY_ADDRESS,
                                );
                            }
                        },
                        messages::Data::RSAPublicKey(key) => {
                            // take the key and write it to memory.
                            let key_bytes = key.to_bytes();
                            for i in 0..16 {
                                eeprom_manager.write_memory(i + 48, key_bytes[i as usize]);
                            }
                        }
                        messages::Data::Status(status) => match status {
                            messages::Status::UnkownAESKey => {
                                // Send out our AES key.
                                if let Some(ref mut coms_manager) =
                                    COMS.borrow(cs).borrow_mut().deref_mut()
                                {
                                    let key =
                                        eeprom_manager.read_8_byte_key(eeprom::AES_KEY_ADDRESS);
                                    let msg = messages::Data::AESKey(key);
                                    coms_manager.send(&msg);
                                }
                            }
                            messages::Status::UnkownPublicKey => {
                                // Send out our public key.
                                if let Some(ref mut coms_manager) =
                                    COMS.borrow(cs).borrow_mut().deref_mut()
                                {
                                    let mut key_bytes = [0; 16];
                                    for i in 0..16 {
                                        // casting here cannot panic because a u8 will always fit into a usize.
                                        key_bytes[i as usize] = eeprom_manager.read_memory(i + 16);
                                    }
                                    // create our message packet to send.
                                    let msg = messages::Data::RSAPublicKey(
                                        crypt::RSAPublicKey::from_bytes(&key_bytes),
                                    );
                                    coms_manager.send(&msg);
                                }
                            }
                        },
                        messages::Data::Temperature(data) => {
                            // get the AES key from memory
                            let mut aes_key: [u8; 8] = [0; 8];
                            for i in 0..8 {
                                aes_key[i] = eeprom_manager.read_memory(i as u8 + 32);
                            }

                            // unstuff the message
                            let unstuffed = coms_manager::ComsManager::unstuff_message(&data);

                            // decrypt the message
                            let mut decrypted = crypt::aes_decrypt(&aes_key, &unstuffed);

                            // deserialize the message
                            if let Ok(msg) =
                                postcard::from_bytes_cobs::<Temperature>(&mut decrypted.as_mut())
                            {
                                // check if the temperature is too high
                                if msg.temp > 100.0 {
                                    // turn on the red LED
                                    mux.execute(mux::Channel::RedLED);
                                } else {
                                    // turn on the green LED
                                    mux.execute(mux::Channel::GreenLED);
                                }
                            } else {
                                // we cannot decrypt the message, delete the AES key and send a message to the other device to delete the AES key.
                                for i in 0..16 {
                                    eeprom_manager.write_memory(i + 32, 0);
                                }
                                if let Some(ref mut coms_manager) =
                                    COMS.borrow(cs).borrow_mut().deref_mut()
                                {
                                    let msg =
                                        messages::Data::Command(messages::Command::DeleteAESKey);
                                    coms_manager.send(&msg);
                                }
                            }
                        }
                        messages::Data::AESKey(key) => {
                            // use our private key to decrypt the AES key
                            // get the key from eeprom
                            let key_bytes =
                                eeprom_manager.read_16_byte_key(eeprom::RSA_PRIV_KEY_ADDRESS);

                            // recreat the private key structure
                            let priv_key = crypt::RSAPrivateKey::from_bytes(&key_bytes);

                            // decrypt the AES key
                            let decrypted = crypt::decrypt(&priv_key, &key);

                            eeprom_manager
                                .write_8_byte_key(decrypted, eeprom::FORIEGN_AES_KEY_ADDRESS);
                        }
                    }
                })
            }

            // try and send a temperature message
            let aes_key = eeprom_manager.read_8_byte_key(eeprom::AES_KEY_ADDRESS);

            let foriegn_aes_key = eeprom_manager.read_8_byte_key(eeprom::FORIEGN_AES_KEY_ADDRESS);

            if aes_key == foriegn_aes_key {
                // we have the same AES key, we can send the message
            } else {
                // we do not have the same AES key, we need to request the AES key from the other device.
                cortex_m::interrupt::free(|cs| {
                    if let Some(ref mut coms_manager) = COMS.borrow(cs).borrow_mut().deref_mut() {
                        let msg = messages::Data::Status(messages::Status::UnkownAESKey);
                        coms_manager.send(&msg);
                    }
                });
            }
        }
    }

    panic!()
}
