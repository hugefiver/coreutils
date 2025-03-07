// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
//  For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::fs::File;
use std::io::{self, stdin, BufReader, Read};
use std::path::Path;
use uucore::display::Quotable;
use uucore::InvalidEncodingHandling;

// NOTE: CRC_TABLE_LEN *must* be <= 256 as we cast 0..CRC_TABLE_LEN to u8
const CRC_TABLE_LEN: usize = 256;
const CRC_TABLE: [u32; CRC_TABLE_LEN] = generate_crc_table();

const NAME: &str = "cksum";
const SYNTAX: &str = "[OPTIONS] [FILE]...";
const SUMMARY: &str = "Print CRC and size for each file";

const fn generate_crc_table() -> [u32; CRC_TABLE_LEN] {
    let mut table = [0; CRC_TABLE_LEN];

    let mut i = 0;
    while i < CRC_TABLE_LEN {
        table[i] = crc_entry(i as u8) as u32;

        i += 1;
    }

    table
}

const fn crc_entry(input: u8) -> u32 {
    let mut crc = (input as u32) << 24;

    let mut i = 0;
    while i < 8 {
        let if_condition = crc & 0x8000_0000;
        let if_body = (crc << 1) ^ 0x04c1_1db7;
        let else_body = crc << 1;

        // NOTE: i feel like this is easier to understand than emulating an if statement in bitwise
        //       ops
        let condition_table = [else_body, if_body];

        crc = condition_table[(if_condition != 0) as usize];
        i += 1;
    }

    crc
}

#[inline]
fn crc_update(crc: u32, input: u8) -> u32 {
    (crc << 8) ^ CRC_TABLE[((crc >> 24) as usize ^ input as usize) & 0xFF]
}

#[inline]
fn crc_final(mut crc: u32, mut length: usize) -> u32 {
    while length != 0 {
        crc = crc_update(crc, length as u8);
        length >>= 8;
    }

    !crc
}

fn init_byte_array() -> Vec<u8> {
    vec![0; 1024 * 1024]
}

#[inline]
fn cksum(fname: &str) -> io::Result<(u32, usize)> {
    let mut crc = 0u32;
    let mut size = 0usize;

    let file;
    let mut rd: Box<dyn Read> = match fname {
        "-" => Box::new(stdin()),
        _ => {
            let path = &Path::new(fname);
            if path.is_dir() {
                return Err(std::io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Is a directory",
                ));
            };
            // Silent the warning as we want to the error message
            #[allow(clippy::question_mark)]
            if path.metadata().is_err() {
                return Err(std::io::Error::new(
                    io::ErrorKind::NotFound,
                    "No such file or directory",
                ));
            };
            file = File::open(&path)?;
            Box::new(BufReader::new(file))
        }
    };

    let mut bytes = init_byte_array();
    loop {
        let num_bytes = rd.read(&mut bytes)?;
        if num_bytes == 0 {
            return Ok((crc_final(crc, size), size));
        }
        for &b in bytes[..num_bytes].iter() {
            crc = crc_update(crc, b);
        }
        size += num_bytes;
    }
}

mod options {
    pub static FILE: &str = "file";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let files: Vec<String> = match matches.values_of(options::FILE) {
        Some(v) => v.clone().map(|v| v.to_owned()).collect(),
        None => vec![],
    };

    if files.is_empty() {
        match cksum("-") {
            Ok((crc, size)) => println!("{} {}", crc, size),
            Err(err) => {
                show_error!("-: {}", err);
                return 2;
            }
        }
        return 0;
    }

    let mut exit_code = 0;
    for fname in &files {
        match cksum(fname.as_ref()) {
            Ok((crc, size)) => println!("{} {} {}", crc, size, fname),
            Err(err) => {
                show_error!("{}: {}", fname.maybe_quote(), err);
                exit_code = 2;
            }
        }
    }

    exit_code
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .about(SUMMARY)
        .usage(SYNTAX)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
}
