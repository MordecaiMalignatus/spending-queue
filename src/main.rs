use ansi_term::Style;
use chrono::prelude::*;
use clap::{App, AppSettings, Arg};
use fraction::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Result;
use std::path::PathBuf;

type M = GenericDecimal<u64, u8>;

fn main() -> Result<()> {
    let args = App::new("sq")
        .version("0.1")
        .author("Mordecai Malignatus <mordecai@malignat.us>")
        .about("The tiniest spending queue")
        .subcommand(App::new("status").about("Report the current state"))
        .subcommand(
            App::new("budget")
                .about("Change budget.")
                .arg(
                    Arg::with_name("amount")
                        .help("Amount of money budgeted by interval")
                        .short("a")
                        .long("amount")
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::with_name("interval")
                        .help("Interval of money budget, measured in days.")
                        .short("i")
                        .long("interval")
                        .takes_value(true)
                        .default_value("30")
                        .required(false),
                ),
        )
        .subcommand(App::new("buy").about("Marks the top item as bought if it can be."))
        .subcommand(
            App::new("add")
                .about("Add an item to the queue")
                .setting(AppSettings::TrailingVarArg)
                .arg(Arg::from_usage("<words>... 'Description of thing to buy'")),
        )
        .get_matches();

    match args.subcommand() {
        ("status", _) => display_status(),
        ("budget", Some(matches)) => {
            let income = matches
                .value_of("amount")
                .unwrap()
                .parse::<u64>()
                .expect("can't parse amount");
            let interval = matches
                .value_of("interval")
                .unwrap()
                .parse()
                .expect("can't parse interval");

            update_budget(M::from(income), interval)
        }
        ("buy", _) => buy_item(),
        ("add", Some(matches)) => {
            let to_add = matches
                .values_of("words")
                .unwrap()
                .collect::<Vec<&str>>()
                .join(" ");
            add_to_purchase_queue(to_add)
        }
        _ => display_status(),
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    name: String,
    amount: M,
}

#[derive(Serialize, Deserialize, Debug)]
struct Income {
    amount: M,
    interval_in_days: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    income: Income,
    /// unix timestamp, because lazy
    last_calculation: String,
    current_amount: M,
    future_purchases: Vec<Item>,
    past_purchases: Vec<Item>,
}

fn buy_item() -> Result<()> {
    let mut state = read_file();

    match state.future_purchases.first() {
        Some(item) => {
            if item.amount < state.current_amount {
                let item_amount = format!("{:.2}", item.amount);
                let current_amount = format!("{:.2}", state.current_amount - item.amount);
                println!(
                    "Bought {} for {}. Remaining: {}",
                    Style::new().bold().paint(&item.name),
                    Style::new().bold().paint(item_amount),
                    Style::new().bold().paint(current_amount)
                );
                state.current_amount = state.current_amount - item.amount;
                let last = state.future_purchases.pop().unwrap();
                state.past_purchases.append(&mut vec![last]);
            }
        }
        None => {
            eprintln!("No item in the queue, can't buy it!");
        }
    }

    write_file(state)
}

fn add_to_purchase_queue(thing_to_add: String) -> Result<()> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    let parsed: u64;

    loop {
        println!("What does this cost?: ");
        match stdin.read_line(&mut line) {
            Ok(_) => {
                line = line.trim().to_string();
                match line.parse() {
                    Ok(float) => {
                        parsed = float;
                        break;
                    }
                    Err(e) => {
                        eprintln!("Can't parse amount, try again: {}", e);
                    }
                }
            }
            Err(e) => panic!("Can't read from stdin: {}", e),
        }
    }

    let amount = format!("{:.2}", M::from(parsed));
    println!("Adding \"{}\" for ${} to the list.", &thing_to_add, amount);
    let mut state = read_file();
    let item = Item {
        name: thing_to_add,
        amount: M::from(parsed),
    };
    state.future_purchases.append(&mut vec![item]);

    write_file(state)
}

fn display_status() -> Result<()> {
    let mut state = read_file();

    let (new_timestamp, new_amount) = calculate_current_amount(&state);
    state.last_calculation = new_timestamp.to_rfc2822();
    state.current_amount = new_amount;

    let amount = format!("{:.2}", state.current_amount);
    println!(
        "Currently available free budget: {}",
        Style::new().bold().paint(amount)
    );

    match state.future_purchases.first() {
        Some(item) => {
            let amount = format!("{:.2}", item.amount);
            println!(
                "The next item in the queue is {} for ${}",
                Style::new().bold().paint(&item.name),
                Style::new().bold().paint(amount)
            )
        }
        None => println!("There's no next item in the queue, add one!"),
    };

    println!("");
    write_file(state)
}

fn calculate_current_amount(state: &State) -> (DateTime<Local>, M) {
    let now = Local::now();
    let then = DateTime::parse_from_rfc2822(&state.last_calculation)
        .expect("Can't parse date from last calculation, check the statefile");
    let time_between = now.signed_duration_since(then).num_seconds();
    let time_between = M::from(time_between as u64);

    let interval = M::from(state.income.interval_in_days as u64);
    let current_balance = M::from(state.current_amount);

    let seconds_in_interval = M::from(24u64 * 60 * 60) * interval;
    let money_per_second = state.income.amount / seconds_in_interval;
    let since_last_calc = time_between * money_per_second;

    let subtotal = current_balance + since_last_calc;

    (now, subtotal)
}

fn update_budget(amount: M, interval: u8) -> Result<()> {
    let mut state = read_file();

    println!("Updated income to ${:.2} per {} days.", amount, interval);
    state.income = Income {
        amount,
        interval_in_days: interval,
    };
    write_file(state)
}

fn file() -> PathBuf {
    let home = env::var("HOME").expect("$HOME is not set, aborting.");
    let mut home = PathBuf::from(home);
    home.push(".config");
    home.push("sq");
    home.push("state.json");

    home
}

// TODO: This could be a lot better
fn read_file() -> State {
    let statepath = file();

    if !statepath.exists() {
        let mut t = file();
        let _ = t.pop();
        std::fs::create_dir_all(t).expect("can't create config dir")
    }

    match fs::read_to_string(statepath) {
        Ok(string) => {
            serde_json::from_str(&string).expect("Can't parse statefile, check the formatting")
        }
        Err(_) => {
            eprintln!("Can't read statefile, continuing with default");
            eprintln!("You're going to want to adjust the income, currently $1/mo.");
            State {
                income: Income {
                    amount: M::from(1),
                    interval_in_days: 30,
                },
                current_amount: M::from(0),
                last_calculation: Local::now().to_rfc2822(),
                future_purchases: Vec::new(),
                past_purchases: Vec::new(),
            }
        }
    }
}

fn write_file(state: State) -> Result<()> {
    fs::write(file(), serde_json::to_string_pretty(&state).unwrap())
}
