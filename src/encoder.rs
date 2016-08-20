use dictionary::Dictionary;
use coder::{HALF_VALUE, MAX_VALUE, QUARTER_VALUE, THREE_QUARTER_VALUE, EOF_SYMBOL};
use sum_tree::SumTree;

use std::io::{Read, Write, Result};
use std::collections::vec_deque::VecDeque;

struct Encoder<'a, Dict: Dictionary> {
    read: &'a mut Read,
    low: u32,
    high: u32,
    scale_counter: u32,
    dict: Dict,
    out_byte_buffer: VecDeque<u8>,
    out_bit_counter: u8,
    out_bit_buffer: u8,
    input_exhausted: bool,
}

impl<'a, Dict: Dictionary> Encoder<'a, Dict> {
    pub fn new(read: &mut Read) -> Encoder<Dict> {

        let n_symbols = 257; // 256 bytes + EOF marker
        Encoder {
            read: read,
            low: 0,
            high: MAX_VALUE,
            scale_counter: 0,
            dict: Dict::new(n_symbols),
            out_byte_buffer: VecDeque::new(),
            out_bit_counter: 0,
            out_bit_buffer: 0,
            input_exhausted: false,
        }
    }
    fn write_bit(&mut self, bit: bool) {
        self.out_bit_buffer = (self.out_bit_buffer << 1) | (bit as u8);
        self.out_bit_counter += 1;
        if self.out_bit_counter == 8 {
            self.out_byte_buffer.push_back(self.out_bit_buffer);
            self.out_bit_counter = 0;
            self.out_bit_buffer = 0;
        }
    }
    fn push_symbol(&mut self, symbol: u32) {
        let slice_length = (self.high - self.low + 1) / self.dict.total_frequency();
        let slice_begin = self.dict.frequency_up_to_symbol(symbol as u32);
        let slice_end = slice_begin + self.dict.symbol_frequency(symbol as u32);

        self.high = self.low + slice_length * slice_end - 1;
        self.low = self.low + slice_length * slice_begin;

        loop {
            if self.high < HALF_VALUE {
                self.write_bit(false);
                for _ in 0..self.scale_counter {
                    self.write_bit(true);
                }
                self.scale_counter = 0;
                self.low = 2 * self.low;
                self.high = 2 * self.high + 1;

                continue;
            }

            if self.low >= HALF_VALUE {
                self.write_bit(true);
                for _ in 0..self.scale_counter {
                    self.write_bit(false);
                }
                self.scale_counter = 0;
                self.low = 2 * (self.low - HALF_VALUE);
                self.high = 2 * (self.high - HALF_VALUE) + 1;

                continue;
            }
            if self.low >= QUARTER_VALUE && self.high < THREE_QUARTER_VALUE {

                self.scale_counter = self.scale_counter + 1;
                self.low = 2 * (self.low - QUARTER_VALUE);
                self.high = 2 * (self.high - QUARTER_VALUE) + 1;

                continue;
            }
            break;
        }
        self.dict.increment(symbol as u32);
    }
    fn get_byte(&mut self) -> Result<u8> {

        // push bytes from input until there is something to read..
        while self.out_byte_buffer.len() == 0 {
            let mut byte = [0u8];
            match self.read.read_exact(&mut byte) {
                Ok(_) => {
                    self.push_symbol(byte[0] as u32);
                }
                Err(x) => {
                    if !self.input_exhausted {
                        self.push_symbol(EOF_SYMBOL);

                        // make sure there are enough bits to decode the rest of the message
                        if self.low < QUARTER_VALUE {
                            self.write_bit(false);
                            for _ in 0..self.scale_counter + 1 {
                                self.write_bit(true);
                            }
                        } else {
                            self.write_bit(true);
                        }
                        if self.out_bit_counter > 0 {
                            self.out_bit_buffer <<= 8 - self.out_bit_counter;
                            self.out_byte_buffer.push_back(self.out_bit_buffer as u8);
                        }
                        self.input_exhausted = true
                    }
                    if self.out_byte_buffer.len() > 0 {
                        return Ok(self.out_byte_buffer.pop_front().unwrap());
                    }
                    return Err(x);
                }
            }
        }
        Ok(self.out_byte_buffer.pop_front().unwrap())
    }
}

impl<'a, Dict: Dictionary> Read for Encoder<'a, Dict> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {

        let mut counter: usize = 0;

        while counter < buf.len() {
            match self.get_byte() {
                Ok(byte) => {
                    buf[counter] = byte;
                    counter += 1;
                }
                Err(x) => {
                    if counter == 0 {
                        return Err(x);
                    }
                    return Ok(counter);
                }
            }
            counter += 1;
        }
        Ok(counter)
    }
}

pub fn encode(read: &mut Read, write: &mut Write) -> Result<()> {

    let enc: Encoder<SumTree<u32>> = Encoder::new(read);
    for byte in enc.bytes() {
        match byte {
            Ok(b) => {
                match write.write_all(&[b]) {
                    Ok(_) => {}
                    Err(x) => return Err(x),
                }
            }
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}
