use coder::{EOF_SYMBOL, HALF_VALUE, MAX_VALUE, QUARTER_VALUE, THREE_QUARTER_VALUE};
use dictionary::Dictionary;
use sum_tree::SumTree;

use std::collections::vec_deque::VecDeque;
use std::io::{Read, Result, Write};

struct Decoder<'a, Dict: Dictionary> {
    read: &'a mut dyn Read,
    out_byte_buffer: VecDeque<u8>,
    low: u32,
    high: u32,
    buffer: u32,
    prepend_bit_counter: u32,
    dict: Dict,
    hit_eof: bool,
    got_eof_symbol: bool,
}

impl<'a, Dict: Dictionary> Decoder<'a, Dict> {
    fn new(read: &mut dyn Read) -> Decoder<Dict> {
        let n_symbols = 257; // 256 bytes + EOF marker

        Decoder {
            read: read,
            out_byte_buffer: VecDeque::new(),
            low: 0,
            high: MAX_VALUE,
            buffer: 0,
            prepend_bit_counter: 31,
            dict: Dict::new(n_symbols),
            hit_eof: false,
            got_eof_symbol: false,
        }
    }
    fn push_bit(&mut self, bit: bool) {
        // consume bit by building buffer in the beginning...
        if self.prepend_bit_counter > 0 {
            self.buffer = (self.buffer << 1) | bit as u32;
            self.prepend_bit_counter -= 1;
            return;
        }

        // loop until bit is consumed..
        loop {
            if self.high < HALF_VALUE {
                self.low = self.low * 2;
                self.high = self.high * 2 + 1;
                self.buffer = self.buffer * 2 + bit as u32;
                break;
            } else if self.low >= HALF_VALUE {
                self.low = 2 * (self.low - HALF_VALUE);
                self.high = 2 * (self.high - HALF_VALUE) + 1;
                self.buffer = 2 * (self.buffer - HALF_VALUE) + bit as u32;
                break;
            } else if (QUARTER_VALUE <= self.low) && (self.high < THREE_QUARTER_VALUE) {
                self.low = 2 * (self.low - QUARTER_VALUE);
                self.high = 2 * (self.high - QUARTER_VALUE) + 1;
                self.buffer = 2 * (self.buffer - QUARTER_VALUE) + bit as u32;
                break;
            } else {
                let slice_length = (self.high - self.low + 1) / self.dict.total_frequency();
                let value = (self.buffer - self.low) / slice_length;
                let symbol = self.dict.symbol_lookup(value);

                let range_low = self.dict.frequency_up_to_symbol(symbol);
                let range_high = range_low + self.dict.symbol_frequency(symbol);

                self.high = self.low + slice_length * range_high - 1;
                self.low = self.low + slice_length * range_low;

                self.dict.increment(symbol as u32);

                if symbol == EOF_SYMBOL {
                    self.got_eof_symbol = true;
                } else {
                    if !self.got_eof_symbol {
                        self.out_byte_buffer.push_back(symbol as u8);
                    }
                }
            }
        }
    }
    fn push_byte(&mut self, byte: u8) {
        for b in 0..8 {
            self.push_bit(byte & (1 << (7 - b)) > 0);
        }
    }
    fn get_byte(&mut self) -> Result<u8> {
        // push bytes from input until there is something to read..
        while self.out_byte_buffer.len() == 0 {
            let mut byte = [0u8];
            match self.read.read_exact(&mut byte) {
                Ok(_) => {
                    self.push_byte(byte[0]);
                }
                Err(x) => {
                    if !self.hit_eof {
                        self.hit_eof = true;

                        // push zeros in hopes of finding the EOF
                        for _ in 0..20 {
                            self.push_byte(0);
                        }
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

impl<'a, Dict: Dictionary> Read for Decoder<'a, Dict> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut counter: usize = 0;
        while counter < buf.len() {
            match self.get_byte() {
                Ok(byte) => {
                    buf[counter] = byte;
                }
                Err(x) => {
                    if counter > 0 {
                        return Ok(counter);
                    }
                    return Err(x);
                }
            }
            counter += 1;
        }

        Ok(counter)
    }
}

pub fn decode(read: &mut dyn Read, write: &mut dyn Write) -> Result<()> {
    let dec: Decoder<SumTree<u32>> = Decoder::new(read);

    for byte in dec.bytes() {
        match byte {
            Ok(b) => match write.write_all(&[b]) {
                Ok(_) => {}
                Err(x) => return Err(x),
            },
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}
