use sum_tree::SumTree;
use std::io::{Read, Write, Result};
use std::collections::vec_deque::VecDeque;

// most significant bit kept free to avoid overflows
const MAX_VALUE: u32 = 0x80000000u32;
const THREE_QUARTER_VALUE: u32 = 0x60000000u32;
const HALF_VALUE: u32 = 0x40000000u32;
const QUARTER_VALUE: u32 = 0x20000000u32;

const SYMBOL_MAX_FREQ: u32 = 1 << 16;
const EOF_SYMBOL: usize = 256;
const INCREMENT_STEP: u32 = 10;

struct Encoder<'a> {
    read: &'a mut Read,
    low: u32,
    high: u32,
    scale_counter: u32,
    tree: SumTree<u32>,
    out_byte_buffer: VecDeque<u8>,
    out_bit_counter: u8,
    out_bit_buffer: u8,
    input_exhausted: bool,
}

impl<'a> Encoder<'a> {
    pub fn new(read: &mut Read) -> Encoder {

        let n_symbols = 257; // 256 bytes + EOF marker
        let mut tree = SumTree::new(n_symbols);

        for i in 0..n_symbols {
            tree.increment(i as u32, 1);
        }
        Encoder {
            read: read,
            low: 0,
            high: MAX_VALUE,
            scale_counter: 0,
            tree: tree,
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
    fn push_symbol(&mut self, symbol: usize) {
        let slice_length = (self.high - self.low + 1) / self.tree.get_total();
        let slice_begin = self.tree.get_before(symbol);
        let slice_end = slice_begin + self.tree.get(symbol);

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
        if self.tree.get(symbol) < SYMBOL_MAX_FREQ {
            self.tree.increment(symbol as u32, INCREMENT_STEP);
        }
    }
    fn get_byte(&mut self) -> Result<u8> {

        // push bytes from input until there is something to read..
        while self.out_byte_buffer.len() == 0 {
            let mut byte = [0u8];
            match self.read.read_exact(&mut byte) {
                Ok(_) => {
                    self.push_symbol(byte[0] as usize);
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

impl<'a> Read for Encoder<'a> {
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

struct Decoder<'a> {
    read: &'a mut Read,
    out_byte_buffer: VecDeque<u8>,
    low: u32,
    high: u32,
    buffer: u32,
    prepend_bit_counter: u32,
    tree: SumTree<u32>,
    hit_eof: bool,
    got_eof_symbol: bool,
}

impl<'a> Decoder<'a> {
    fn new(read: &mut Read) -> Decoder {

        let n_symbols = 257; // 256 bytes + EOF marker
        let mut tree = SumTree::new(n_symbols);
        for i in 0..n_symbols {
            tree.increment(i as u32, 1);
        }

        Decoder {
            read: read,
            out_byte_buffer: VecDeque::new(),
            low: 0,
            high: MAX_VALUE,
            buffer: 0,
            prepend_bit_counter: 31,
            tree: tree,
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
                let slice_length = (self.high - self.low + 1) / self.tree.get_total();
                let value = (self.buffer - self.low) / slice_length;
                let symbol = self.tree.get_index(value);

                let range_low = self.tree.get_before(symbol);
                let range_high = range_low + self.tree.get(symbol);

                self.high = self.low + slice_length * range_high - 1;
                self.low = self.low + slice_length * range_low;

                if self.tree.get(symbol) < SYMBOL_MAX_FREQ {
                    self.tree.increment(symbol as u32, INCREMENT_STEP);
                }

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

impl<'a> Read for Decoder<'a> {
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

pub fn encode(read: &mut Read, write: &mut Write) -> Result<()> {

    let enc = Encoder::new(read);
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

pub fn decode(read: &mut Read, write: &mut Write) -> Result<()> {

    let dec = Decoder::new(read);

    for byte in dec.bytes() {
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

#[test]
fn test_coder_vec() {

    let mut buf: Vec<u8> = Vec::new();
    let mut buf2: Vec<u8> = Vec::new();
    let mut encoded: Vec<u8> = Vec::new();

    for i in 0..1024 {
        buf.push((i % 255) as u8);
    }

    encoded.resize(10000, 0);
    buf2.resize(10000, 0);

    encode(&mut buf.as_slice(), &mut encoded.as_mut_slice()).unwrap();
    decode(&mut encoded.as_slice(), &mut buf2.as_mut_slice()).unwrap();

    buf2.resize(buf.len(), 0);

    assert!(buf == buf2);
}

#[test]
fn test_coder_file() {

    use std::fs::File;
    let mut buf: Vec<u8> = Vec::new();
    let mut buf2: Vec<u8> = Vec::new();

    for i in 0..10000 {
        buf.push((i % 100) as u8);
    }

    encode(&mut buf.as_slice(),
           &mut File::create("tmp.cargotest").expect("Unable to open file"))
        .unwrap();
    decode(&mut File::open("tmp.cargotest").expect("Unable to open file"),
           &mut File::create("tmp.cargotest2").expect("Unable to open file"))
        .unwrap();

    buf2.clear();
    File::open("tmp.cargotest2").expect("Unable to open file").read_to_end(&mut buf2).unwrap();
    println!("size1: {}, size2: {}", buf.len(), buf2.len());
    assert!(buf == buf2);
}
