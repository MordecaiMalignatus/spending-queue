use std::io::Result;
use std::{
    io::{Error, ErrorKind},
    process::Command,
};

pub fn read_stdin_line(prompt: &str) -> String {
    let stdin = std::io::stdin();
    let mut buf = String::new();
    println!("{}", prompt);
    match stdin.read_line(&mut buf) {
        Ok(_bytes_read) => buf.trim().to_string(),
        Err(x) => {
            panic!("Can't parse purchase url because {}, aborting...", x);
        }
    }
}

pub fn yes_no_predicate(prompt: &str) -> bool {
    loop {
        match read_stdin_line(&format!("{} (y/n)", prompt)).as_ref() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            &_ => eprintln!("Please enter either yes/y or no/n"),
        }
    }
}

pub fn parse_float_from_stdin(prompt: &str) -> f64 {
    let stdin = std::io::stdin();
    let mut line = String::new();

    loop {
        println!("{}", prompt);
        match stdin.read_line(&mut line) {
            Ok(_) => {
                line = line.trim().to_string();
                match line.parse() {
                    Ok(float) => {
                        return float;
                    }
                    Err(e) => {
                        eprintln!("Can't parse amount, try again: {}", e);
                        line.clear();
                    }
                }
            }
            Err(e) => panic!("Can't read from stdin: {}", e),
        }
    }
}

pub fn open_url(url: &Option<String>) -> Result<()> {
    match url {
        Some(purchase_url) => {
            match Command::new("open").arg(purchase_url).output() {
                Ok(_) => Ok(()), // Everything worked as intended.
                Err(_) => Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Can't open purchase URL",
                )),
            }
        }
        None => {
            eprintln!("Would open purchase link, none present.");
            Ok(())
        }
    }
}
