//! Communication module 
//! 
use postcard::{from_bytes_cobs, to_vec_cobs};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use stm32f0xx_hal::serial::{Tx, Rx, Serial};
use stm32f0xx_hal::pac; 
use stm32f0xx_hal::prelude::*;
use crate::messages::*;
use heapless::Vec;
use serde::Deserialize;

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
    }

    /// Receive a message from the serial port
    /// Move this to an interrupt handler
    pub fn receive(&mut self) -> Option<Message> {
        while let Ok(byte) = nb::block!(self.receiver.read()) {
            let buf = [byte];
            let mut window = &buf[..];

            while !window.is_empty() {
                window = match self.cobs_buf.feed::<Message>(&window) {
                    FeedResult::Consumed => break,
                    FeedResult::OverFull(new_wind) => new_wind,
                    FeedResult::DeserError(new_wind) => new_wind,
                    FeedResult::Success { data, remaining } => {
                        return Some(data);
                    }
                };
            }
        }
        None
    }
}