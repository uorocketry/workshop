//! Multiplexer

use cortex_m::interrupt::free;
use stm32f0xx_hal::adc::Adc;
use stm32f0xx_hal::gpio::gpioa::{PA0, PA1, PA2};
use stm32f0xx_hal::gpio::{Output, PushPull};
use stm32f0xx_hal::prelude::*;

#[derive(Clone, Copy)]
pub enum Channel {
    RedLED,
    GreenLED,
    TempSensor,
}

struct Selector {
    s0: PA1<Output<PushPull>>,
    s1: PA0<Output<PushPull>>,
}

impl Selector {
    pub fn new(s0: PA1<Output<PushPull>>, s1: PA0<Output<PushPull>>) -> Selector {
        Selector { s0, s1 }
    }

    pub fn select(&mut self, channel: Channel) {
        match channel {
            Channel::RedLED => {
                // unwrap since this is an infallible error
                self.s0.set_low().unwrap();
                self.s1.set_low().unwrap();
            }
            Channel::GreenLED => {
                self.s0.set_high().unwrap();
                self.s1.set_low().unwrap();
            }
            Channel::TempSensor => {
                self.s0.set_low().unwrap();
                self.s1.set_high().unwrap();
            }
        }
    }
}

pub struct Mux {
    selector: Selector,
    io: Option<PA2<Output<PushPull>>>, // we will need to take and transform the pin, so we use an option.
    adc: Adc,
}

impl Mux {
    pub fn new(
        s0: PA1<Output<PushPull>>,
        s1: PA0<Output<PushPull>>,
        io: PA2<Output<PushPull>>,
        adc: Adc,
    ) -> Mux {
        Mux {
            selector: Selector::new(s0, s1),
            io: Some(io),
            adc,
        }
    }

    pub fn select(&mut self, channel: Channel) {
        self.selector.select(channel);
    }

    pub fn take(&mut self) -> Option<PA2<Output<PushPull>>> {
        self.io.take()
    }

    pub fn give(&mut self, io: PA2<Output<PushPull>>) {
        self.io = Some(io);
    }

    pub fn execute(&mut self, channel: Channel) -> Option<u16> {
        let mut ret_val = None;
        self.select(channel);
        match channel {
            Channel::GreenLED | Channel::RedLED => {
                if let Some(mut io) = self.take() {
                    io.toggle().unwrap();
                    self.give(io);
                }
                return None;
            }
            Channel::TempSensor => {
                // take the pin and convert to an analog pin
                if let Some(mut io) = self.take() {
                    let ret_val_ref = &mut ret_val;
                    free(move |cs| {
                        let mut adc_pin = io.into_analog(cs);
                        if let Ok(val) = self.adc.read(&mut adc_pin) as Result<u16, _> {
                            *ret_val_ref = Some(val);
                        }
                        io = adc_pin.into_push_pull_output(cs);
                        self.give(io);
                    })
                }
            }
        }
        ret_val
    }
}
