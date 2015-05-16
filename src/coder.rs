use sum_tree::SumTree;

// most significant bit kept free to avoid overflows
const MAX_VALUE: u32 =           0x80000000u32;
const THREE_QUARTER_VALUE: u32 = 0x60000000u32;
const HALF_VALUE: u32 =          0x40000000u32;
const QUARTER_VALUE: u32 =       0x20000000u32;

pub struct Encoder {
	low: u32,
	high: u32,
	scale_counter: u32,
	tree: SumTree<u32>
}

pub struct Decoder {
	low: u32,
	high: u32,
	buffer: u32,
	prepend_bit_counter: u32,
	tree: SumTree<u32>
}

const SYMBOL_MAX_FREQ: u32 = 2 << 16;
const EOF_SYMBOL: usize = 256;

impl Encoder {

	pub fn new() -> Encoder {
		let n_symbols = 257; // 256 bytes + EOF marker
		let mut tree = SumTree::new(n_symbols);
		for i in 0..n_symbols {
			tree.increment(i as u32, 1);
		}

		Encoder {
			low: 0,
			high: MAX_VALUE,
			scale_counter: 0,
			tree: tree
		}
	}

	pub fn push_byte<F: FnMut(bool) -> ()> (&mut self, byte: u8, pull_bit: &mut F) {
		self.push_symbol(byte as usize, pull_bit);
	}

	pub fn push_eof<F: FnMut(bool) -> ()> (&mut self, pull_bit: &mut F) {
		self.push_symbol(EOF_SYMBOL, pull_bit);

		if self.low < QUARTER_VALUE {
			pull_bit(false);
			for _ in 0..self.scale_counter {
				pull_bit(true);
			}
		}
		else {
			pull_bit(true);
		}
	}

	fn push_symbol<F: FnMut(bool) -> ()> (&mut self, symbol: usize, pull_bit: &mut F) {

		let slice_length = (self.high - self.low + 1) / self.tree.get_total();
		let slice_begin  = self.tree.get_before(symbol);
		let slice_end    = slice_begin + self.tree.get(symbol);

		self.high = self.low + slice_length * slice_end-1;
		self.low  = self.low + slice_length * slice_begin;


		let mut done: bool = false;
		while !done {

			done = true;
			while self.high < HALF_VALUE {
				pull_bit(false);
				for _ in 0..self.scale_counter {
					pull_bit(true);
				}
				done = false;
				self.scale_counter = 0;
				self.low  = 2 * self.low;
				self.high = 2 * self.high + 1;
			}
			while self.low >= HALF_VALUE {
				pull_bit(true);
				for _ in 0..self.scale_counter {
					pull_bit(false);
				}
				done = false;
				self.scale_counter = 0;
				self.low  = 2 * (self.low  - HALF_VALUE);
				self.high = 2 * (self.high - HALF_VALUE) + 1;
			}
			while self.low >= QUARTER_VALUE && self.high < THREE_QUARTER_VALUE {
				done = false;
				self.scale_counter = self.scale_counter + 1;
				self.low  = 2 * (self.low  - QUARTER_VALUE);
				self.high = 2 * (self.high - QUARTER_VALUE) + 1;
			}
		}
		if self.tree.get(symbol) < SYMBOL_MAX_FREQ {
			self.tree.increment(symbol as u32, 1);
		}
	}
}

impl Decoder {

	pub fn new() -> Decoder {
		let n_symbols = 257; // 256 bytes + EOF marker
		let mut tree = SumTree::new(n_symbols);
		for i in 0..n_symbols {
			tree.increment(i as u32, 1);
		}

		Decoder {
			low: 0,
			high: MAX_VALUE,
			buffer: 0,
			prepend_bit_counter: 31,
			tree: tree
		}
	}

	pub fn push_byte<F: FnMut(u8) -> ()> (&mut self, byte: u8, pull_byte: &mut F) -> bool{
		let mut mask = 128;
		for _ in 0..8 {
			if !self.push_bit((byte & mask) != 0, pull_byte) {
				return false;
			}
			mask >>= 1;
		};
		return true;
	}

	fn push_bit<F: FnMut(u8) -> ()> (&mut self, bit: bool, pull_byte: &mut F) -> bool{

		if self.prepend_bit_counter > 0 {
			self.buffer = (self.buffer << 1) | bit as u32;
			self.prepend_bit_counter -= 1;
			return true;
		}

		let mut ret = true;

		loop
		{
			if self.high < HALF_VALUE {
				self.low    = self.low    * 2;
				self.high   = self.high   * 2 + 1;
				self.buffer = self.buffer * 2 + bit as u32;
				break;
			}
			else if self.low >= HALF_VALUE {
				self.low    = 2 * (self.low    - HALF_VALUE);
				self.high   = 2 * (self.high   - HALF_VALUE) + 1;
				self.buffer = 2 * (self.buffer - HALF_VALUE) + bit as u32;
				break;
			}
			else if (QUARTER_VALUE <= self.low) && (self.high < THREE_QUARTER_VALUE) {
				self.low    = 2 * (self.low    - QUARTER_VALUE);
				self.high   = 2 * (self.high   - QUARTER_VALUE) + 1;
				self.buffer = 2 * (self.buffer - QUARTER_VALUE) + bit as u32;
				break;
			}
			else {
				let slice_length = (self.high - self.low + 1) / self.tree.get_total();
				let value = (self.buffer - self.low) / slice_length;
				let symbol = self.tree.get_index(value);

				let range_low = self.tree.get_before(symbol);
				let range_high = range_low + self.tree.get(symbol);

				self.high = self.low + slice_length * range_high - 1;
				self.low  = self.low + slice_length * range_low;

				if self.tree.get(symbol) < SYMBOL_MAX_FREQ {
					self.tree.increment(symbol as u32, 1);
				}

				if symbol == EOF_SYMBOL {
					ret = false;
				}
				else if symbol < 256 {
					pull_byte(symbol as u8);
				}
			}
		}
		return ret;
	}
}
