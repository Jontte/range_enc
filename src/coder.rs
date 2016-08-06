use sum_tree::SumTree;

use std::io::{Read, Write};

// most significant bit kept free to avoid overflows
const MAX_VALUE: u32 =           0x80000000u32;
const THREE_QUARTER_VALUE: u32 = 0x60000000u32;
const HALF_VALUE: u32 =          0x40000000u32;
const QUARTER_VALUE: u32 =       0x20000000u32;

const SYMBOL_MAX_FREQ: u32 = 2 << 16;
const EOF_SYMBOL: usize = 256;

pub fn encode<R, W>(read: R, mut write: W) where R: Read, W: Write {

	let n_symbols = 257; // 256 bytes + EOF marker
	let mut tree = SumTree::new(n_symbols);

	for i in 0..n_symbols {
		tree.increment(i as u32, 1);
	}

	struct Ctx {
		low: u32,
		high: u32,
		scale_counter: u32,
		out_bit_counter: u8,
		out_bit_buffer: u32,
		tree: SumTree<u32>
	};

	let mut ctx = Ctx {
		low: 0,
		high: MAX_VALUE,
		scale_counter: 0,
		out_bit_counter: 0,
		out_bit_buffer: 0,
		tree: tree
	};

	{
		let mut write_bit = |ctx: &mut Ctx, bit: bool| {
			ctx.out_bit_buffer = (ctx.out_bit_buffer << 1) | (bit as u32);
			ctx.out_bit_counter += 1;
			if ctx.out_bit_counter == 8 {
				write.write_all(&[ctx.out_bit_buffer as u8]).expect("Error writing to file");
				ctx.out_bit_counter = 0;
				ctx.out_bit_buffer = 0;
			}
		};

		{
			let mut push_symbol = |ctx: &mut Ctx, symbol: usize| {

				let slice_length = (ctx.high - ctx.low + 1) / ctx.tree.get_total();
				let slice_begin  = ctx.tree.get_before(symbol);
				let slice_end    = slice_begin + ctx.tree.get(symbol);

				ctx.high = ctx.low + slice_length * slice_end-1;
				ctx.low  = ctx.low + slice_length * slice_begin;

				let mut done: bool = false;
				while !done {

					done = true;
					while ctx.high < HALF_VALUE {
						write_bit(ctx, false);
						for _ in 0..ctx.scale_counter {
							write_bit(ctx, true);
						}
						done = false;
						ctx.scale_counter = 0;
						ctx.low  = 2 * ctx.low;
						ctx.high = 2 * ctx.high + 1;
					}
					while ctx.low >= HALF_VALUE {
						write_bit(ctx, true);
						for _ in 0..ctx.scale_counter {
							write_bit(ctx, false);
						}
						done = false;
						ctx.scale_counter = 0;
						ctx.low  = 2 * (ctx.low  - HALF_VALUE);
						ctx.high = 2 * (ctx.high - HALF_VALUE) + 1;
					}
					while ctx.low >= QUARTER_VALUE && ctx.high < THREE_QUARTER_VALUE {
						done = false;
						ctx.scale_counter = ctx.scale_counter + 1;
						ctx.low  = 2 * (ctx.low  - QUARTER_VALUE);
						ctx.high = 2 * (ctx.high - QUARTER_VALUE) + 1;
					}
				}
				if ctx.tree.get(symbol) < SYMBOL_MAX_FREQ {
					ctx.tree.increment(symbol as u32, 1);
				}
			};

			for byte in read.bytes() {
				push_symbol(&mut ctx, byte.unwrap() as usize);
			}
			// Push EOF
			push_symbol(&mut ctx, EOF_SYMBOL);
		}
		if ctx.low < QUARTER_VALUE {
			write_bit(&mut ctx, false);
			for _ in 0..ctx.scale_counter+1 {
				write_bit(&mut ctx, true);
			}
		}
		else {
			write_bit(&mut ctx, true);
		}
	}

	if ctx.out_bit_counter > 0 {
		ctx.out_bit_buffer <<= 8 - ctx.out_bit_counter;
		write.write_all(&[ctx.out_bit_buffer as u8]).expect("Error writing to file");
	}
}

pub fn decode<R, W>(read: R, mut write: W) where R: Read, W: Write {

	struct Ctx {
		low: u32,
		high: u32,
		buffer: u32,
		prepend_bit_counter: u32,
		tree: SumTree<u32>,
	};

	let n_symbols = 257; // 256 bytes + EOF marker
	let mut tree = SumTree::new(n_symbols);
	for i in 0..n_symbols {
		tree.increment(i as u32, 1);
	}

	let mut ctx = Ctx {
		low: 0,
		high: MAX_VALUE,
		buffer: 0,
		prepend_bit_counter: 31,
		tree: tree
	};

	let mut push_bit = |ctx: &mut Ctx, bit: bool| -> bool {

		if ctx.prepend_bit_counter > 0 {
			ctx.buffer = (ctx.buffer << 1) | bit as u32;
			ctx.prepend_bit_counter -= 1;
			return true;
		}

		loop
		{
			if ctx.high < HALF_VALUE {
				ctx.low    = ctx.low    * 2;
				ctx.high   = ctx.high   * 2 + 1;
				ctx.buffer = ctx.buffer * 2 + bit as u32;
				break;
			}
			else if ctx.low >= HALF_VALUE {
				ctx.low    = 2 * (ctx.low    - HALF_VALUE);
				ctx.high   = 2 * (ctx.high   - HALF_VALUE) + 1;
				ctx.buffer = 2 * (ctx.buffer - HALF_VALUE) + bit as u32;
				break;
			}
			else if (QUARTER_VALUE <= ctx.low) && (ctx.high < THREE_QUARTER_VALUE) {
				ctx.low    = 2 * (ctx.low    - QUARTER_VALUE);
				ctx.high   = 2 * (ctx.high   - QUARTER_VALUE) + 1;
				ctx.buffer = 2 * (ctx.buffer - QUARTER_VALUE) + bit as u32;
				break;
			}
			else {
				let slice_length = (ctx.high - ctx.low + 1) / ctx.tree.get_total();
				let value = (ctx.buffer - ctx.low) / slice_length;
				let symbol = ctx.tree.get_index(value);

				let range_low = ctx.tree.get_before(symbol);
				let range_high = range_low + ctx.tree.get(symbol);

				ctx.high = ctx.low + slice_length * range_high - 1;
				ctx.low  = ctx.low + slice_length * range_low;

				if ctx.tree.get(symbol) < SYMBOL_MAX_FREQ {
					ctx.tree.increment(symbol as u32, 1);
				}

				println!("decoded sym {}", symbol as u32);
				if symbol == EOF_SYMBOL {
					println!("hit eof!");
					return false;
				}
				else {
					write.write_all(&[symbol as u8]).expect("Unable to write to stream!");
				}
			}
		}
		return true;
	};

	let mut push_byte = |ctx: &mut Ctx, byte: u8| -> bool {
		let mut mask = 128;
		for _ in 0..8 {
			if !push_bit(ctx, (byte & mask) != 0) {
				return false;
			}
			mask >>= 1;
		};
		return true;
	};

	for byte in read.bytes() {
		if !push_byte(&mut ctx, byte.unwrap()) {
			// eof reached, file finished
			return;
		}
	}
	// push zeros in hopes of finding the EOF
	for _ in 0..20 {
		if !push_byte(&mut ctx, 0) {
			return;
		}
	}
	// reading failed...?
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

	encode(buf.as_slice(), encoded.as_mut_slice());
	decode(encoded.as_slice(), buf2.as_mut_slice());

	buf2.resize(buf.len(), 0);

	assert!(buf == buf2);
}

#[test]
fn test_coder_file() {

	use std::fs::{File};
	let mut buf: Vec<u8> = Vec::new();
	let mut buf2: Vec<u8> = Vec::new();

	for i in 0..10000 {
		buf.push((i % 100) as u8);
	}

	encode(buf.as_slice(), File::create("tmp.cargotest").expect("Unable to open file"));
	decode(File::open("tmp.cargotest").expect("Unable to open file"), File::create("tmp.cargotest2").expect("Unable to open file"));

	buf2.clear();
	File::open("tmp.cargotest2").expect("Unable to open file").read_to_end(&mut buf2).unwrap();
	println!("size1: {}, size2: {}", buf.len(), buf2.len());
	assert!(buf == buf2);
}
