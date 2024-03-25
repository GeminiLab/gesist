use std::string::FromUtf8Error;
use std::fs;
use std::io::{self, Read, stdin, stdout, Write};
use clap::{Args, CommandFactory, Parser, error::ErrorKind};
use gesist::{decode_from_base64, encode_to_base64};
use gesist::padder::PaddingValidationError;

#[derive(Parser)]
#[command(name = "gesist", arg_required_else_help = true)]
#[command(version, about, long_about = None)]
struct GesistCli {
    #[command(flatten)]
    main: MainActions,
    #[arg(help = "File to be encoded or decoded, if not provided, stdin will be used.")]
    file: Option<String>,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct MainActions {
    #[arg(short = 'e', long, help = "Encode the file or input from stdin.")]
    encode: bool,
    #[arg(short = 'd', long, help = "Decode the file or input from stdin.")]
    decode: bool,
}

fn main() {
    let args = GesistCli::parse();


    match (args.main.encode, args.main.decode) {
        (true, false) => encode_once(args.file),
        (false, true) => decode_once(args.file),
        _ => unreachable!(),
    }
}

fn exit_on_io_error(error: io::Error) -> ! {
    GesistCli::command().error(ErrorKind::Io, format!("IO Error: {}", error)).exit()
}

fn exit_on_from_utf8_error(error: FromUtf8Error) -> ! {
    GesistCli::command().error(ErrorKind::InvalidUtf8, format!("FromUtf8 Error: {}", error)).exit()
}

fn exit_on_base64_error(error: base64::DecodeError) -> ! {
    GesistCli::command().error(ErrorKind::InvalidValue, format!("Base64 Error: {}", error)).exit()
}

fn exit_on_decode_error(error: PaddingValidationError) -> ! {
    GesistCli::command().error(ErrorKind::InvalidValue, format!("Decode Error: {:?}", error)).exit()
}

fn read_all_from_file_or_stdin(file: Option<String>) -> Vec<u8> {
    (match file {
        None => {
            let mut buf = vec![];
            stdin().read_to_end(&mut buf).map(|_| buf)
        },
        Some(file) => {
            fs::read(file)
        }
    }).unwrap_or_else(|e| exit_on_io_error(e))
}

fn whitespace_removed(mut input: String) -> String {
    input.retain(|c| !c.is_whitespace());
    input
}

fn encode_once(file: Option<String>) {
    println!("{}", encode_to_base64(read_all_from_file_or_stdin(file)))
}

fn decode_once(file: Option<String>) {
    let content = read_all_from_file_or_stdin(file);
    let stripped = String::from_utf8(content).map(whitespace_removed).unwrap_or_else(|e| exit_on_from_utf8_error(e));
    let data = decode_from_base64(stripped).unwrap_or_else(|e| exit_on_base64_error(e)).unwrap_or_else(|e| exit_on_decode_error(e));
    stdout().write_all(&data).unwrap_or_else(|e| exit_on_io_error(e))
}
