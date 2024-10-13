//! Communication module
//!

use crate::messages::*;

use heapless::Vec;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use stm32f0xx_hal::pac;
use stm32f0xx_hal::prelude::*;
use stm32f0xx_hal::serial::{Rx, Tx};

pub struct ComsManager {
    packet_id: u8,
    transmitter: Tx<pac::USART1>,
    receiver: Rx<pac::USART1>,
    buffer: Vec<u8, 256>,
    cobs_buf: CobsAccumulator<256>,
    new_message: bool,
}

impl ComsManager {
    pub fn new(transmitter: Tx<pac::USART1>, receiver: Rx<pac::USART1>) -> ComsManager {
        ComsManager {
            packet_id: 0,
            transmitter,
            receiver,
            buffer: Vec::new(),
            cobs_buf: CobsAccumulator::new(),
            new_message: false,
        }
    }

    pub fn has_new_message(&self) -> bool {
        self.new_message
    }

    pub fn unstuff_message(stuffed: &[u8]) -> Vec<u8, 256> {
        let mut unstuffed = Vec::<u8, 256>::new();
        let mut escape = false;

        for &byte in stuffed {
            if escape {
                unstuffed.push(byte).unwrap();
                escape = false;
            } else if byte == 0xFE {
                escape = true;
            } else {
                unstuffed.push(byte).unwrap();
            }
        }

        unstuffed
    }

    pub fn send(&mut self, data: &Data) {
        // Construct the message
        let msg = Message {
            id: self.packet_id,
            data: data.clone(),
        };
        // Serialize the message
        let mut buffer = [0; 32];
        let payload = postcard::to_slice_cobs(&msg, &mut buffer).unwrap();

        // Byte stuffing
        let mut stuffed_payload = Vec::<u8, 64>::new();
        for &mut byte in payload {
            if byte == 0xFF {
                stuffed_payload.push(0xFE).unwrap();
            }
            stuffed_payload.push(byte).unwrap();
        }
        // Add stop frame
        stuffed_payload.push(0xFF).unwrap();

        // Send the stuffed payload
        for byte in stuffed_payload {
            nb::block!(self.transmitter.write(byte)).unwrap();
        }
        self.packet_id = self.packet_id.wrapping_add(1);
    }

    pub fn read_byte(&mut self) {
        let byte = nb::block!(self.receiver.read());
        if let Ok(byte) = byte {
            if self.buffer.push(byte).is_err() || byte == 0xFF {
                self.new_message = true;
            }
            // let msg: FeedResult<'_, Message> = self.cobs_buf.feed::<Message>(&[byte]);
        }
    }

    pub fn receive(&mut self) -> Option<Message> {
        let mut msg = None;
        let mut remaining_data: Vec<u8, 256> = Vec::new();

        let buf = &mut self.buffer[..];
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

        self.buffer.clear();
        self.buffer.extend_from_slice(&remaining_data).unwrap();
        self.new_message = false;
        msg
    }
}
