//! Communication module
//!

use crate::messages::*;

use core::cell::RefCell;
use core::ops::DerefMut;

use cortex_m::interrupt::{free, Mutex};
use heapless::Vec;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use postcard::{from_bytes_cobs, to_vec_cobs};
use serde::Deserialize;
use stm32f0xx_hal::pac;
use stm32f0xx_hal::prelude::*;
use stm32f0xx_hal::serial::{Rx, Serial, Tx};

pub static COMS_BUFFER: Mutex<RefCell<Vec<u8, 256>>> = Mutex::new(RefCell::new(Vec::new()));

pub struct ComsManager {
    packed_id: u8,
    transmitter: Tx<pac::USART1>,
    receiver: Rx<pac::USART1>,
    cobs_buf: CobsAccumulator<256>,
}

impl ComsManager {
    pub fn new(transmitter: Tx<pac::USART1>, receiver: Rx<pac::USART1>) -> ComsManager {
        ComsManager {
            packed_id: 0,
            transmitter,
            receiver,
            cobs_buf: CobsAccumulator::new(),
        }
    }

    pub fn send(&mut self, data: &[u8]) {
        let mut buffer: Vec<u8, 256> = to_vec_cobs(data).unwrap();
        buffer.insert(0, self.packed_id).unwrap();
        for byte in buffer {
            nb::block!(self.transmitter.write(byte)).unwrap();
        }
        self.packed_id = self.packed_id.wrapping_add(1);
    }

    pub fn read_byte(&mut self) {
        let byte = nb::block!(self.receiver.read());
        if let Ok(byte) = byte {
            free(|cs| {
                let mut coms_buffer = COMS_BUFFER.borrow(cs).borrow_mut();
                coms_buffer.push(byte).unwrap();
            });
        }
    }

    pub fn receive(&mut self) -> Option<Message> {
        let mut msg = None;
        let mut remaining_data: Vec<u8, 256> = Vec::new();
    
        free(|cs| {
            let mut coms_buffer = COMS_BUFFER.borrow(cs).borrow_mut();
            let buf = &mut coms_buffer[..];
            let mut window = &buf[..];
    
            'cobs: while !window.is_empty() {
                window = match self.cobs_buf.feed::<Message>(&window) {
                    FeedResult::Consumed => break 'cobs,
                    FeedResult::OverFull(new_wind) => new_wind,
                    FeedResult::DeserError(new_wind) => new_wind,
                    FeedResult::Success { data, remaining } => {
                        msg = Some(data);
                        remaining_data.extend_from_slice(remaining).unwrap();
                        break 'cobs;
                    }
                };
            }
        });
    
        // Clear and update the buffer outside the critical section
        free(|cs| {
            let mut coms_buffer = COMS_BUFFER.borrow(cs).borrow_mut();
            coms_buffer.clear();
            coms_buffer.extend_from_slice(&remaining_data).unwrap();
        });
    
        msg
    }
}
