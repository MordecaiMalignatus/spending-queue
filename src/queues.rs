use std::{collections::VecDeque, io::Result};

use clap::{App, Arg};

use crate::types::{Income, Queue};
use crate::{read_state_file, write_file};

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    App::new("queue")
        .about("Manage various queues and their settings")
        .subcommand(
            App::new("new").about("Create a new queue").arg(
                Arg::with_name("name")
                    .help("What to name the new queue, ie 'books'")
                    .short("n")
                    .long("name")
                    .required(false)
                    .takes_value(true),
            ),
        ).subcommand(App::new("select").about("Select a queue as active"))
}

pub fn cmd_queue(matches: &clap::ArgMatches) -> Result<()> {
    match matches.subcommand() {
        ("new", Some(m)) => {
            let name = m.value_of("name").unwrap().to_string();
            cmd_queue_create(name)
        }
        _ => {
            eprintln!("{}", matches.usage());
            Ok(())
        }
    }
}

pub fn cmd_queue_select(name: String) -> Result<()> {
    Ok(())
}

pub fn cmd_queue_create(name: String) -> Result<()> {
    let mut state = read_state_file();
    let nq = Queue {
        income: Income {
            amount: 1.0,
            interval_in_days: 1,
        },
        name,
        last_calculation: chrono::Local::now().to_rfc2822(),
        current_balance: 0.into(),
        future_purchases: VecDeque::new(),
        past_purchases: VecDeque::new(),
        paused: false,
    };

    state.queues.push(nq);

    write_file(&state)
}
