#![warn(clippy::pedantic, clippy::all)]
mod io;

use ansi_term::Style;
use chrono::prelude::*;
use clap::{App, AppSettings, Arg};
use fraction::prelude::*;
use io::open_url;
use prettytable::cell;
use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR;
use prettytable::row;
use prettytable::Table;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::Result;
use std::path::PathBuf;

use crate::io::parse_float_from_stdin;
use crate::io::read_stdin_line;
use crate::io::yes_no_predicate;

type M = GenericDecimal<u64, u8>;

fn main() -> Result<()> {
    let args = parse_args();
    match args.subcommand() {
        ("budget", Some(m)) => {
            // This is a bug waiting to happen, I think. Needs to be converted
            // to pass Options to `update_budget`.
            let income = m
                .value_of("amount")
                .unwrap()
                .parse()
                .expect("can't parse amount");
            let interval = m
                .value_of("interval")
                .unwrap()
                .parse()
                .expect("can't parse interval");

            update_budget(income, interval)?;
        }
        ("buy", Some(m)) => {
            let no_open = m.is_present("no-open");
            let peek = m.is_present("peek");
            let new_price = m
                .value_of("new_price")
                .map(|s| s.parse::<f64>().expect("Can't parse specified new price"))
                .map(M::from);
            buy_item(no_open, new_price, peek)?;
        }
        ("list", _) => display_queue(),
        ("delete", _) => delete_head_item()?,
        ("past", _) => display_prior_purchases(),
        ("bump", _) => bump_head()?,
        ("add", Some(m)) => {
            let prepend = m.is_present("prepend");
            let to_add = m
                .values_of("words")
                .unwrap()
                .collect::<Vec<&str>>()
                .join(" ");
            add_to_purchase_queue(to_add, prepend)?;
        }
        _ => display_status()?,
    }

    Ok(())
}

fn parse_args() -> clap::ArgMatches<'static> {
    App::new("sq")
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
                        .required(true),
                )
                .arg(
                    Arg::with_name("interval")
                        .help("Interval of money budget, measured in days.")
                        .short("i")
                        .long("interval")
                        .takes_value(true)
                        .default_value("30")
                        .required(true),
                ),
        )
        .subcommand(
            App::new("buy")
                .about("Marks the top item as bought if it can be.")
                .arg(
                    Arg::with_name("no-open")
                        .help("Suppress opening purchase URL, if present.")
                        .short("n")
                        .long("no-open")
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::with_name("peek")
                        .help("Open the purchase URL without buying anything")
                        .long("--peek")
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::with_name("new_price")
                        .help("Set price explicitly if it no longer matches what's in the list")
                        .short("p")
                        .long("price")
                        .takes_value(false)
                        .required(false),
                ),
        )
        .subcommand(App::new("delete").about("Delete item at head at queue."))
        .subcommand(App::new("list").about("Print items remaining to be bought."))
        .subcommand(App::new("past").about("Print items that were already marked as bought."))
        .subcommand(App::new("bump").about("Move current head of queue back 1-3 spots."))
        .subcommand(
            App::new("add")
                .about("Add an item to the queue")
                .setting(AppSettings::TrailingVarArg)
                .arg(
                    Arg::with_name("prepend")
                        .long("--prepend")
                        .short("p")
                        .help("Push new item to the head of the queue instead of the last spot")
                        .takes_value(false)
                        .required(false),
                )
                .arg(Arg::from_usage("<words>... 'Description of thing to buy'")),
        )
        .get_matches()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Item {
    name: String,
    amount: M,
    purchase_link: Option<String>,
    time_purchased: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Income {
    amount: f64,
    interval_in_days: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct State {
    income: Income,
    last_calculation: String,
    current_amount: M,
    future_purchases: VecDeque<Item>,
    past_purchases: VecDeque<Item>,
}

fn buy_item(suppress_opening_url: bool, new_price: Option<M>, peek: bool) -> Result<()> {
    let mut state = read_file();

    match state.clone().future_purchases.front_mut() {
        Some(item) => {
            if peek {
                open_url(&item.purchase_link)?;
            } else if item.amount < state.current_amount {
                let cost = match new_price {
                    Some(x) => x,
                    None => item.amount,
                };

                if !suppress_opening_url {
                    open_url(&item.purchase_link.clone())?;
                }

                if yes_no_predicate(&format!("Did the item cost {}?", cost)) {
                    purchase(item, cost, &mut state);
                } else {
                    let cost = parse_float_from_stdin("What did it cost?").into();
                    purchase(item, cost, &mut state);
                }
            } else {
                eprintln!("Can't buy item, not enough money accumulated.");
            }
        }
        None => {
            eprintln!("No item in the queue, can't buy it!");
        }
    }

    write_file(&state)
}

fn purchase(item: &mut Item, cost: M, state: &mut State) {
    let now = Local::now().to_rfc2822();
    item.time_purchased = Some(now);
    let current_amount_string = format!("{:#.2}", state.current_amount - cost);
    let item_amount_string = format!("{:#.2}", cost);

    println!(
        "Bought {} for ${}. Remaining: ${}",
        Style::new().bold().paint(&item.name),
        Style::new().bold().paint(item_amount_string),
        Style::new().bold().paint(current_amount_string)
    );

    state.current_amount -= cost;
    let last = state.future_purchases.pop_front().unwrap();
    state.past_purchases.push_back(last);
}

/// Move current head of queue back 1-3 spots. This is essentially a "not right
/// now" button for reordering the queue.
fn bump_head() -> Result<()> {
    let mut state = read_file();
    let bold = Style::new().bold();

    match state.future_purchases.len() {
        0 => eprintln!("No items in the queue, can't bump anything."),
        1 => eprintln!("One item in the queue, can't bump anything."),
        _ => {
            let head = state.future_purchases.pop_front().expect(
                "We already checked for queue length, thus must succeed in any sane universe",
            );
            let head_name = head.name.clone();
            let upper_bound = state.future_purchases.len() - 1;
            let new_position = rand::thread_rng().gen_range(1..upper_bound);
            state.future_purchases.insert(new_position, head);
            println!(
                "Moved {} from head of queue to position {}. Next item is now {}.",
                bold.paint(&head_name),
                bold.paint((new_position + 1).to_string()),
                bold.paint(state.future_purchases.front().unwrap().name.clone())
            );
            write_file(&state)?;
            display_status()?;
        }
    }

    Ok(())
}

fn delete_head_item() -> Result<()> {
    let mut state = read_file();
    if let Some(item) = state.future_purchases.pop_front() {
        println!("Deleted item at head of queue: {}", item.name);
        write_file(&state)?;
        display_status()
    } else {
        eprintln!("No item in queue, can't remove any.");
        Ok(())
    }
}

/// Print the list as it is right now.
fn display_queue() {
    let state = read_file();
    let mut table = Table::new();
    table.set_titles(row!("Name", "Cost"));
    table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
    state.future_purchases.iter().for_each(|item| {
        let cost = format!("${:#.2}", item.amount);
        if item.purchase_link.is_some() {
            table.add_row(row!(bi->item.name, cost));
        } else {
            table.add_row(row!(b->item.name, cost));
        };
    });

    table.printstd();
    println!();
}

/// Print list of past purchases, the things already bought.
fn display_prior_purchases() {
    let state = read_file();
    let mut table = Table::new();
    table.set_titles(row!("Name", "Cost", "Purchased"));
    table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
    state.past_purchases.iter().for_each(|item| {
        let cost = format!("${:#.2}", item.amount);
        let ts = item.time_purchased.clone().unwrap_or_default();
        table.add_row(row!(b->item.name, cost, ts));
    });

    table.printstd();
    println!();
}

fn add_to_purchase_queue(thing_to_add: String, prepend: bool) -> Result<()> {
    let parsed = parse_float_from_stdin("What does this cost?: ");
    let purchase_url = read_stdin_line("Do you have a purchase URL? (Leave empty for no)");

    let purchase_link = match purchase_url.as_ref() {
        "" => None,
        _ => Some(purchase_url),
    };

    let amount = format!("{:#.2}", M::from(parsed));
    println!("Adding \"{}\" for ${} to the list.", &thing_to_add, amount);
    let mut state = read_file();
    let item = Item {
        name: thing_to_add,
        amount: M::from(parsed),
        purchase_link,
        time_purchased: None,
    };

    if prepend {
        state.future_purchases.push_front(item);
    } else {
        state.future_purchases.push_back(item);
    }

    write_file(&state)
}

fn display_status() -> Result<()> {
    let mut state = read_file();
    let bold = Style::new().bold();

    let (new_timestamp, new_amount) = calculate_current_amount(&state);
    state.last_calculation = new_timestamp.to_rfc2822();
    state.current_amount = new_amount;

    let available_amount = format!("{:#.2}", state.current_amount);
    println!(
        "Currently available free budget: ${}",
        Style::new().bold().paint(&available_amount)
    );

    match state.future_purchases.front() {
        Some(item) => {
            let amount = format!("{:#.2}", item.amount);
            let name = match &item.purchase_link {
                Some(_) => Style::new().bold().italic().paint(item.name.clone()),
                None => bold.paint(item.name.clone()),
            };

            println!(
                "The next item in the queue is {} for ${}",
                name,
                bold.paint(&amount)
            );
            if state.current_amount >= item.amount {
                println!("{}", bold.paint("*** NEXT ITEM PURCHASEABLE ***"));
            }
        }
        None => println!("There's no next item in the queue, add one!"),
    };

    println!();
    write_file(&state)
}

fn calculate_current_amount(state: &State) -> (DateTime<Local>, M) {
    let now = Local::now();
    let then = DateTime::parse_from_rfc2822(&state.last_calculation)
        .expect("Can't parse date from last calculation, check the statefile");
    let time_between = now.signed_duration_since(then).num_seconds();
    let time_between = M::from(time_between);
    let income = M::from(state.income.amount);

    let interval = M::from(state.income.interval_in_days as u64);
    let current_balance = state.current_amount;

    let seconds_in_interval = M::from(24_u64 * 60 * 60) * interval;
    let money_per_second = income / seconds_in_interval;
    let since_last_calc = time_between * money_per_second;

    let subtotal = current_balance + since_last_calc;

    (now, subtotal)
}

fn update_budget(amount: f64, interval: u64) -> Result<()> {
    let mut state = read_file();

    println!("Updated income to ${:.2} per {} days.", amount, interval);
    state.income = Income {
        amount,
        interval_in_days: interval,
    };
    write_file(&state)
}

fn file() -> PathBuf {
    let home = env::var("HOME").expect("$HOME is not set, aborting.");
    let mut home = PathBuf::from(home);
    home.push(".config");
    home.push("sq");
    home.push("state.json");

    home
}

fn read_file() -> State {
    let statepath = file();

    if !statepath.exists() {
        let mut t = file();
        let _ = t.pop();
        std::fs::create_dir_all(t).expect("can't create config dir");
    }

    if let Ok(string) = fs::read_to_string(statepath) {
        serde_json::from_str(&string).expect("Can't parse statefile, check the formatting")
    } else {
        eprintln!("Can't read statefile, continuing with default");
        eprintln!("You're going to want to adjust the income, currently $1/mo.");
        State {
            income: Income {
                amount: 1.0,
                interval_in_days: 30,
            },
            current_amount: M::from(0),
            last_calculation: Local::now().to_rfc2822(),
            future_purchases: VecDeque::new(),
            past_purchases: VecDeque::new(),
        }
    }
}

fn write_file(state: &State) -> Result<()> {
    fs::write(file(), serde_json::to_string_pretty(state).unwrap())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_calculate_accumulation() {
        let now = Local::now();
        let yesterday = now - chrono::Duration::days(1);
        let demo_state = State {
            income: Income {
                amount: 100.0,
                interval_in_days: 1,
            },
            last_calculation: yesterday.to_rfc2822(),
            current_amount: M::from(0),
            future_purchases: VecDeque::new(),
            past_purchases: VecDeque::new(),
        };
        let (_last_update, balance) = calculate_current_amount(&demo_state);

        assert_eq!(balance, M::from(100));
    }
}
