use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use numerologist_core::{bazi, qimen, ziwei};
use serde_json::Value;

fn usage() {
    println!(
        "numerologist {version}\n\
         Native Rust chart engines (local-only; no server or GUI)\n\
         Usage:\n\
           numerologist <qimen|ziwei|bazi> --input FILE [--json|--pretty]\n\
           numerologist doctor [--json]\n\
           numerologist --help | --version\n\
         Use --input - for stdin. JSON is always written to stdout;\n\
         diagnostics go to stderr. Times are local civil times.",
        version = env!("CARGO_PKG_VERSION")
    );
}

fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        usage();
        return Ok(());
    }
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let command = args.first().ok_or("missing command; run --help")?;
    let pretty = args.iter().any(|argument| argument == "--pretty");
    if command == "doctor" {
        let report = serde_json::json!({
            "engine": "numerologist-core",
            "version": env!("CARGO_PKG_VERSION"),
            "runtime": "native-rust",
            "commands": ["qimen", "ziwei", "bazi", "doctor"],
            "transport": "stdin/file JSON",
            "network_server": false,
            "publication_ready": false
        });
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    if !["qimen", "ziwei", "bazi"].contains(&command.as_str()) {
        return Err(format!("unknown command: {command}").into());
    }
    let mut file = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--input" => {
                index += 1;
                file = Some(args.get(index).ok_or("--input needs a path")?.clone());
            }
            "--json" | "--pretty" => {}
            flag => return Err(format!("unknown argument: {flag}").into()),
        }
        index += 1;
    }
    let file = file.ok_or("required --input FILE (or --input - for stdin)")?;
    let input = if file == "-" {
        let mut data = String::new();
        io::stdin().read_to_string(&mut data)?;
        data
    } else {
        fs::read_to_string(file)?
    };
    let output: Value = match command.as_str() {
        "qimen" => serde_json::to_value(qimen::calculate(&serde_json::from_str(&input)?)?)?,
        "ziwei" => serde_json::to_value(ziwei::calculate(&serde_json::from_str(&input)?)?)?,
        "bazi" => serde_json::to_value(bazi::calculate(&serde_json::from_str(&input)?)?)?,
        _ => unreachable!(),
    };
    if pretty {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", serde_json::to_string(&output)?);
    }
    Ok(())
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("numerologist: {error}");
            ExitCode::FAILURE
        }
    }
}
