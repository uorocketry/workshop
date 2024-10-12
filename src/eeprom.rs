//! EEPROM driver for AT25010B
//!

use stm32f0xx_hal::pac;
use stm32f0xx_hal::prelude::*;
use stm32f0xx_hal::spi::{Mode, Phase, Polarity, Spi};

pub struct EepromManager {
    spi: Spi<
        pac::SPI1,
        stm32f0xx_hal::gpio::gpioa::PA5<stm32f0xx_hal::gpio::Alternate<stm32f0xx_hal::gpio::AF0>>,
        stm32f0xx_hal::gpio::gpioa::PA6<stm32f0xx_hal::gpio::Alternate<stm32f0xx_hal::gpio::AF0>>,
        stm32f0xx_hal::gpio::gpioa::PA7<stm32f0xx_hal::gpio::Alternate<stm32f0xx_hal::gpio::AF0>>,
        stm32f0xx_hal::spi::EightBit,
    >,
    status: Option<StatusRegister>, // Cache the status register, on boot we do not know the status, so we wrap in an option.
}

#[derive(Debug, Clone)]
pub struct StatusRegister {
    pub bp1: bool,
    pub bp0: bool,
    pub wel: bool,
    pub rdy_bsy: bool,
}

impl StatusRegister {
    pub fn from_byte(byte: u8) -> Self {
        StatusRegister {
            bp1: (byte & 0b0000_0100) != 0,
            bp0: (byte & 0b0000_0010) != 0,
            wel: (byte & 0b0000_0001) != 0,
            rdy_bsy: (byte & 0b0000_0001) != 0,
        }
    }

    pub fn to_byte(&self) -> u8 {
        (if self.bp1 { 0b0000_0100 } else { 0 })
            | (if self.bp0 { 0b0000_0010 } else { 0 })
            | (if self.wel { 0b0000_0001 } else { 0 })
            | (if self.rdy_bsy { 0b0000_0001 } else { 0 })
    }
}

impl EepromManager {
    const WREN: u8 = 0b0000_0110;
    const WRDI: u8 = 0b0000_0100;
    const RDSR: u8 = 0b0000_0101;
    const WRSR: u8 = 0b0000_0001;
    const READ: u8 = 0b0000_0011;
    const WRITE: u8 = 0b0000_0010;

    pub fn new(
        miso: stm32f0xx_hal::gpio::gpioa::PA6<
            stm32f0xx_hal::gpio::Alternate<stm32f0xx_hal::gpio::AF0>,
        >,
        mosi: stm32f0xx_hal::gpio::gpioa::PA7<
            stm32f0xx_hal::gpio::Alternate<stm32f0xx_hal::gpio::AF0>,
        >,
        sck: stm32f0xx_hal::gpio::gpioa::PA5<
            stm32f0xx_hal::gpio::Alternate<stm32f0xx_hal::gpio::AF0>,
        >,
        rcc: &mut stm32f0xx_hal::rcc::Rcc,
        spi_peripheral: pac::SPI1,
    ) -> EepromManager {
        let spi = Spi::spi1(
            spi_peripheral,
            (sck, miso, mosi),
            Mode {
                polarity: Polarity::IdleHigh,
                phase: Phase::CaptureOnSecondTransition,
            },
            1.mhz(),
            rcc,
        );
        EepromManager { spi, status: None }
    }

    pub fn write_enable(&mut self) {
        self.spi.write(&[Self::WREN]).unwrap();
    }

    pub fn write_disable(&mut self) {
        self.spi.write(&[Self::WRDI]).unwrap();
    }

    pub fn read_status(&mut self) -> StatusRegister {
        let mut status = [0];
        self.spi.write(&[Self::RDSR]).unwrap();
        self.spi.transfer(&mut status).unwrap();
        let status_reg = StatusRegister::from_byte(status[0]);
        self.status = Some(status_reg.clone());
        status_reg
    }

    pub fn write_status(&mut self, status: StatusRegister) {
        self.spi.write(&[Self::WRSR, status.to_byte()]).unwrap();
    }

    pub fn read_memory(&mut self, address: u8) -> u8 {
        let mut data = [0];
        self.spi.write(&[Self::READ, address]).unwrap();
        self.spi.transfer(&mut data).unwrap();
        data[0]
    }

    pub fn write_memory(&mut self, address: u8, data: u8) {
        self.write_enable();
        self.spi.write(&[Self::WRITE, address, data]).unwrap();
        self.wait_until_ready();
        self.write_disable();
    }

    pub fn wait_until_ready(&mut self) {
        loop {
            let status = self.read_status();
            if !status.rdy_bsy {
                break;
            }
        }
    }
}
