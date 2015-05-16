extern crate getopts;
extern crate num;

use getopts::Options;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;

mod sum_tree;
mod coder;

fn encode(input: &str, output: &str) {

	let mut k = coder::Encoder::new();

	let mut f_in = File::open(input).ok().expect("Unable to open file");
	let mut f_out = File::create(output).ok().expect("Unable to open file");

	let mut buf_in = vec![0; 4096];
	let mut buf_out: u8 = 0;
	let mut buf_out_counter: u8 = 0;

	{
		let mut write_bit = |bit| {

			buf_out = (buf_out << 1) | (bit as u8);
			buf_out_counter += 1;

			if buf_out_counter >= 8 {
				buf_out_counter = 0;

				f_out.write_all(&[buf_out]).ok().expect("Error writing to file");
				buf_out = 0;
			}
		};

		loop {
			let n = f_in.read(&mut buf_in[..]).ok().expect("I/O error");

			for byte in buf_in[0..n].iter() {
				k.push_byte(*byte, &mut write_bit);
			}

			if n <= 0 {
				break
			}
		}
		k.push_eof(&mut write_bit);
	}

	if buf_out_counter > 0 {
		buf_out <<= 8 - buf_out_counter;
		f_out.write(&[buf_out]).ok().expect("Error writing to file");
	}
}
fn decode(input: &str, output: &str) {

	let mut k = coder::Decoder::new();

	let mut f_in = File::open(input).ok().expect("Unable to open file");
	let mut f_out = File::create(output).ok().expect("Unable to open file");

	let mut buf_in = vec![0; 4096];

	let mut write_byte = |byte: u8| {
		f_out.write_all(&[byte]).ok().expect("Error writing to file");
	};

	let mut finished = false;

	loop {
		let n = f_in.read(&mut buf_in[..]).ok().expect("I/O error");

		for byte in buf_in[0..n].iter() {
			if !k.push_byte(*byte, &mut write_byte) {
				finished = true;
				break;
			}
		}
		if n <= 0 {
			break
		}
	}

	if !finished {
		for _ in 0..32 {
			if !k.push_byte(0u8, &mut write_byte) {
				break;
			}
		}
	}
}

fn print_usage(program: &str, opts: Options) {
	let brief = format!("Usage: {} [options]", program);
	print!("{}", opts.usage(&brief));
}

fn main() {
	let args: Vec<String> = env::args().collect();
	let program = args[0].clone();

	let mut opts = Options::new();
	opts.optopt("o", "", "set output file name", "NAME");
	opts.optopt("m", "mode", "encode or decode", "MODE");
	opts.optflag("h", "help", "print this help menu");

	let matches = match opts.parse(&args[1..]) {
		Ok(m) => { m }
		Err(f) => { panic!(f.to_string()) }
	};

	if matches.opt_present("h") {
		print_usage(&program, opts);
		return;
	}

	let output = match matches.opt_str("o") {
		Some(x) => x,
		None => "out.bin".into()
	};

	let mode = match matches.opt_str("mode") {
		Some(x) => x,
		None => "encode".into()
	};
	if mode != "encode" && mode != "decode" {
		print_usage(&program, opts);
		return;
	}

	let input = if !matches.free.is_empty() {
		matches.free[0].clone()
	} else {
		print_usage(&program, opts);
		return;
	};

	if mode == "encode" {
		encode(&input, &output);
	}
	else {
		decode(&input, &output);
	}
}
