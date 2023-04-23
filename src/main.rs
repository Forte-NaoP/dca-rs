use ogg::PacketReader;
use std::{fs::File, io::{Read, Write}, io::Cursor};
use tokio;
use clap::{arg, Command as ClapCommand};
use std::{
    process::{Command, Stdio},
};

mod metadata;
mod dca;

use metadata::Metadata;

#[tokio::main]
async fn main() {

    let matches = ClapCommand::new("MyApp")
        .version("1.0")
        .author("Forte-NaoP")
        .about("Ogg to DCA converter")
        .arg(arg!(-i --input <VALUE> "ogg file path to convert").required(true))
        .arg(arg!(-o --output <VALUE> "dca file path to save").required(true))
        .arg(arg!(-j --json <VALUE> "metadata json path from yt-dlp or songbird").required(true))
        .arg(arg!(-t --time <VALUE> "total duration of audio").required(false))
        .arg(arg!(-s --start <VALUE> "set the start time offset").required(false))
        .get_matches();

    let input = matches.get_one::<String>("input").expect("-i required");
    let output = matches.get_one::<String>("output").expect("-o required");
    let json_path = matches.get_one::<String>("json").expect("-j required");
    let duration = matches.get_one::<String>("time").expect("-t invalid integer").parse::<u64>().unwrap();
    let start = matches.get_one::<String>("start").expect("-s invalid integer").parse::<u64>().unwrap();

    let mut file = File::open(json_path).unwrap();
    let json: Metadata = serde_json::from_reader(file).unwrap();

    let FFMPEG_ARGS = vec![
        "-ac",
        "2",
        "-ar",
        "48000",
        "-ab",
        "64000",
        "-acodec",
        "libopus",
        "-f",
        "opus"
    ];

    let mut ffmpeg = Command::new("ffmpeg")
        .args(&["-ss", start.to_string().as_str()])
        .args(&["-i", input.as_str()])
        .args(&["-t", duration.to_string().as_str()])
        .args(&FFMPEG_ARGS)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .arg("pipe:1")
        .spawn()
        .unwrap();

    let mut stdout = ffmpeg.stdout.take().unwrap();
    let mut o_vec = vec![];
    stdout.read_to_end(&mut o_vec).unwrap();
    ffmpeg.wait().unwrap();

    let mut container = dca::DcaWrapper::new(json);
    container.write_dca_header();

    let mut cursor = Cursor::new(o_vec);
    let mut reader = PacketReader::new(cursor);
    let mut skip = 2;
    while let Ok(packet) = reader.read_packet() {
        if let None = packet {
            break;
        } else if skip > 0 {
            skip -= 1;
            continue;
        }
        let packet = packet.unwrap();

        container.write_audio_data(packet.data.as_slice());
    }

    let mut dca_output = File::create(output).unwrap();
    dca_output.write_all(container.raw().as_slice()).unwrap();

}