#![warn(clippy::pedantic, clippy::all)]
mod io;
mod legacy;
mod types;
mod queues;

use ansi_term::Color;
use ansi_term::Style;
use chrono::prelude::*;
use clap::{App, AppSettings, Arg};
use prettytable::cell;
use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR;
use prettytable::row;
use prettytable::Table;
use rand::Rng;
use std::env;
use std::fs;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::path::PathBuf;

use crate::types::Income;
use crate::types::Item;
use crate::types::Queue;
use crate::types::State;
use crate::types::M;

use crate::io::open_url;
use crate::io::parse_float_from_stdin;
use crate::io::read_stdin_line;
use crate::io::yes_no_predicate;

fn main() {
    let args = parse_args();
    let res = match args.subcommand() {
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

            cmd_budget(income, interval)
        }
        ("buy", Some(m)) => {
            let no_open = m.is_present("no_open");
            let peek = m.is_present("peek");
            let force = m.is_present("force");
            match m.value_of("new_price") {
                Some(p) => {
                    match p.parse::<f64>() {
                        Ok(new_price) => {
                            let price: M = new_price.into();
                            cmd_buy(no_open, Some(price), peek, force)},
                        Err(_) => Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Can't parse specified price to float.\n(Did you accidentally specify `-peek` instead of `--peek`?)")),
                    }
                },
                None => cmd_buy(no_open, None, peek, force),
            }
        }
        ("list", _) => cmd_list(),
        ("delete", _) => cmd_delete(),
        ("past", _) => cmd_past(),
        ("bump", _) => cmd_bump(),
        ("pause", _) => cmd_pause(),
        ("unpause", _) => cmd_unpause(),
        ("add", Some(m)) => {
            let prepend = m.is_present("prepend");
            let to_add = m
                .values_of("words")
                .unwrap()
                .collect::<Vec<&str>>()
                .join(" ");
            cmd_add(to_add, prepend)
        }
        ("queue", Some(m)) => queues::cmd_queue(m),
        _ => cmd_status(),
    };

    if let Err(e) = res {
        let f = Style::new().bold().fg(Color::Red);
        eprintln!("{}", f.paint(e.to_string()));
    }
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
                    Arg::with_name("no_open")
                        .help("Suppress opening purchase URL, if present.")
                        .short("n")
                        .long("no_open")
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
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::with_name("force")
                        .help("Force purchase despite not enough budget being accrued. This will push the balance into the negative.")
                        .short("f")
                        .long("force")
                        .takes_value(false)
                        .required(false)
                )
        )
        .subcommand(App::new("delete").about("Delete item at head at queue."))
        .subcommand(App::new("list").about("Print items remaining to be bought."))
        .subcommand(App::new("past").about("Print items that were already marked as bought."))
        .subcommand(App::new("bump").about("Move current head of queue back 1-3 spots."))
        .subcommand(App::new("pause").about("Pause accumulation of the queue."))
        .subcommand(App::new("unpause").about("Unpause accumulation of the queue."))
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
        .subcommand(queues::subcommand())
        .get_matches()
}

fn cmd_buy(
    suppress_opening_url: bool,
    new_price: Option<M>,
    peek: bool,
    force: bool,
) -> Result<()> {
    let mut q = currently_selected_queue();

    match q.future_purchases.front_mut() {
        Some(item) => {
            let cost = match new_price {
                Some(x) => x,
                None => item.amount,
            };

            if peek {
                open_url(&item.purchase_link)?;
            } else if cost < q.current_balance || force {
                if !suppress_opening_url {
                    open_url(&item.purchase_link.clone())?;
                }

                if yes_no_predicate(&format!("Did the item cost {}?", cost)) {
                    purchase_next(cost, &mut q);
                } else {
                    let cost = parse_float_from_stdin("What did it cost?").into();
                    purchase_next(cost, &mut q);
                }
            } else {
                eprintln!("Can't buy item, not enough money accumulated.");
            }
        }
        None => {
            eprintln!("No item in the queue, can't buy it!");
        }
    }

    write_current_queue(q)
}

fn purchase_next(cost: M, queue: &mut Queue) {
    let now = Local::now().to_rfc2822();
    let mut item = queue.future_purchases.pop_front().unwrap();

    item.time_purchased = Some(now);
    item.amount = cost;
    let current_amount_string = format!("{:#.2}", queue.current_balance - cost);
    let item_amount_string = format!("{:#.2}", cost);

    println!(
        "Bought {} for ${}. Remaining: ${}",
        Style::new().bold().paint(&item.name),
        Style::new().bold().paint(item_amount_string),
        Style::new().bold().paint(current_amount_string)
    );

    queue.current_balance -= cost;
    queue.past_purchases.push_back(item);
}

/// Move current head of queue back 1-3 spots. This is essentially a "not right
/// now" button for reordering the queue.
fn cmd_bump() -> Result<()> {
    let mut queue = currently_selected_queue();
    let bold = Style::new().bold();

    match queue.future_purchases.len() {
        0 => eprintln!("No items in the queue, can't bump anything."),
        1 => eprintln!("One item in the queue, can't bump anything."),
        _ => {
            let head = queue.future_purchases.pop_front().expect(
                "We already checked for queue length, thus must succeed in any sane universe",
            );
            let head_name = head.name.clone();
            // Subtract 1 to go from length of list to valid, zero-based indices.
            let upper_bound = queue.future_purchases.len() - 1;
            // The x..=y syntax is an inclusive range, the x..y syntax by
            // default is *exclusive*, meaning that a range like 1..2 is empty,
            // and causes `rand` to panic.
            let new_position = rand::thread_rng().gen_range(1..=upper_bound);
            queue.future_purchases.insert(new_position, head);
            println!(
                "Moved {} from head of queue to position {}. Next item is now {}.",
                bold.paint(&head_name),
                bold.paint((new_position + 1).to_string()),
                bold.paint(queue.future_purchases.front().unwrap().name.clone())
            );
            write_current_queue(queue)?;
            cmd_status()?;
        }
    }

    Ok(())
}

fn cmd_pause() -> Result<()> {
    let mut queue = currently_selected_queue();
    queue.paused = true;
    println!("Paused accumulation. Run `sq unpause` to resume.");
    write_current_queue(queue)
}

fn cmd_unpause() -> Result<()> {
    let mut queue = currently_selected_queue();
    queue.paused = false;
    println!("Unpaused accumulation, welcome back.");
    write_current_queue(queue)
}

fn cmd_delete() -> Result<()> {
    let mut queue = currently_selected_queue();
    if let Some(item) = queue.future_purchases.pop_front() {
        println!("Deleted item at head of queue: {}", item.name);
        write_current_queue(queue)?;
        cmd_status()
    } else {
        eprintln!("No item in queue, can't remove any.");
        Ok(())
    }
}

// We return a result to make main have a uniform return type for subcommands,
// even if it is not needed here.
#[allow(clippy::unnecessary_wraps)]
/// Print the list as it is right now.
fn cmd_list() -> Result<()> {
    let queue = currently_selected_queue();
    let mut table = Table::new();
    table.set_titles(row!("Name", "Cost"));
    table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
    queue.future_purchases.iter().for_each(|item| {
        let cost = format!("${:#.2}", item.amount);
        if item.purchase_link.is_some() {
            table.add_row(row!(bi->item.name, cost));
        } else {
            table.add_row(row!(b->item.name, cost));
        };
    });

    table.printstd();
    println!();
    Ok(())
}

// We return a result to make main have a uniform return type for subcommands,
// even if it is not needed here.
#[allow(clippy::unnecessary_wraps)]
/// Print list of past purchases, the things already bought.
fn cmd_past() -> Result<()> {
    let queue = currently_selected_queue();

    let mut table = Table::new();
    table.set_titles(row!("Name", "Cost", "Purchased"));
    table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
    queue.past_purchases.iter().for_each(|item| {
        let cost = format!("${:#.2}", item.amount);
        let ts = item.time_purchased.clone().unwrap_or_default();
        table.add_row(row!(b->item.name, cost, ts));
    });

    table.printstd();
    println!();
    Ok(())
}

fn cmd_add(thing_to_add: String, prepend: bool) -> Result<()> {
    let parsed = parse_float_from_stdin("What does this cost?: ");
    let purchase_url = read_stdin_line("Do you have a purchase URL? (Leave empty for no)");

    let purchase_link = match purchase_url.as_ref() {
        "" => None,
        _ => Some(purchase_url),
    };

    let amount = format!("{:#.2}", M::from(parsed));
    println!("Adding \"{}\" for ${} to the list.", &thing_to_add, amount);
    let mut queue = currently_selected_queue();
    let item = Item {
        name: thing_to_add,
        amount: M::from(parsed),
        purchase_link,
        time_purchased: None,
    };

    if prepend {
        queue.future_purchases.push_front(item);
    } else {
        queue.future_purchases.push_back(item);
    }

    write_current_queue(queue)
}

fn cmd_status() -> Result<()> {
    let state = read_state_file();
    let bold = Style::new().bold();

    if state.globally_paused {
        println!(
            "{}",
            Style::new()
                .italic()
                .bold()
                .paint("SQ is currently paused. To unpause, run `sq unpause`.")
        );
        Ok(())
    } else {
        let mut queue = currently_selected_queue();
        update_accumulation(&mut queue);

        let available_amount = format!("{:#.2}", queue.current_balance);
        println!(
            "Currently available free budget: ${}",
            Style::new().bold().paint(&available_amount)
        );

        match queue.future_purchases.front() {
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
                if queue.current_balance >= item.amount {
                    println!("{}", bold.paint("*** NEXT ITEM PURCHASEABLE ***"));
                }
            }
            None => println!("There's no next item in the queue, add one!"),
        };

        println!();
        write_file(&state)
    }
}

fn update_accumulation(queue: &mut Queue) {
    let (new_timestamp, new_amount) = calculate_current_amount(queue);
    queue.last_calculation = new_timestamp.to_rfc2822();
    queue.current_balance = if queue.paused {
        queue.current_balance
    } else {
        new_amount
    };
}

fn calculate_current_amount(queue: &Queue) -> (DateTime<Local>, M) {
    let now = Local::now();
    let then = DateTime::parse_from_rfc2822(&queue.last_calculation)
        .expect("Can't parse date from last calculation, check the statefile");
    let time_between = now.signed_duration_since(then).num_seconds();
    let time_between = M::from(time_between);
    let income = M::from(queue.income.amount);

    let interval = M::from(queue.income.interval_in_days as u64);
    let current_balance = queue.current_balance;

    let seconds_in_interval = M::from(24_u64 * 60 * 60) * interval;
    let money_per_second = income / seconds_in_interval;
    let since_last_calc = time_between * money_per_second;

    let subtotal = current_balance + since_last_calc;

    (now, subtotal)
}

fn cmd_budget(amount: f64, interval: u64) -> Result<()> {
    let mut queue = currently_selected_queue();

    println!("Updated income to ${:.2} per {} days.", amount, interval);
    queue.income = Income {
        amount,
        interval_in_days: interval,
    };
    write_current_queue(queue)
}

#[must_use]
pub fn config_file_path() -> PathBuf {
    let home = env::var("HOME").expect("$HOME is not set, aborting.");
    let mut home = PathBuf::from(home);
    home.push(".config");
    home.push("sq");
    home.push("state.json");

    home
}

fn read_state_file() -> State {
    let statepath = config_file_path();
    if !statepath.exists() {
        let mut t = config_file_path();
        let _ = t.pop();
        std::fs::create_dir_all(t).expect("can't create config dir");
    }

    let content = match fs::read_to_string(statepath) {
        Ok(s) => s,
        Err(err) => {
            eprintln!(
                "ERROR: Can't read config file contents: {}",
                err.to_string()
            );
            std::process::exit(1)
        }
    };

    match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(_) => if let Ok(s) = legacy::migrate_statefile() { s } else {
            eprintln!("Can neither read nor migrate statefile, continuing with default");
            eprintln!("You're going to want to adjust the income, currently $1 per day.");
            State::default()
        },
    }
}

fn write_file(state: &State) -> Result<()> {
    fs::write(
        config_file_path(),
        serde_json::to_string_pretty(state).unwrap(),
    )
}

fn write_current_queue(queue: Queue) -> Result<()> {
    let mut state = read_state_file();
    let mut nq: Vec<Queue> = state
        .queues
        .into_iter()
        .filter(|q| q.name != queue.name)
        .collect();
    nq.push(queue);
    state.queues = nq;

    write_file(&state)
}

fn currently_selected_queue() -> Queue {
    let state = read_state_file();
    let current_name = state.currently_selected.clone();
    state.queues
        .into_iter()
        .find(|q| q.name == current_name)
        .expect("Currently selected queue does not match any of the actual queues, present, you will need to fix this manually.")
}
