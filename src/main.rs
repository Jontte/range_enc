extern crate getopts;
extern crate num;

use getopts::Options;
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

mod coder;
mod decoder;
mod dictionary;
mod encoder;
mod sum_tree;

fn encode(input: &str, output: &str) {
    let f_in = File::open(input).expect("Unable to open file");
    let f_out = File::create(output).expect("Unable to open file");

    let mut f_in = BufReader::new(f_in);
    let mut f_out = BufWriter::new(f_out);

    encoder::encode(&mut f_in, &mut f_out).unwrap();
}

fn decode(input: &str, output: &str) {
    let f_in = File::open(input).expect("Unable to open file");
    let f_out = File::create(output).expect("Unable to open file");

    let mut f_in = BufReader::new(f_in);
    let mut f_out = BufWriter::new(f_out);

    decoder::decode(&mut f_in, &mut f_out).unwrap();
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
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let output = match matches.opt_str("o") {
        Some(x) => x,
        None => "out.bin".into(),
    };

    let mode = match matches.opt_str("mode") {
        Some(x) => x,
        None => "encode".into(),
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
    } else {
        decode(&input, &output);
    }
}
