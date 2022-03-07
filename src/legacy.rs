use std::collections::VecDeque;
use std::io::Result;

use serde::Deserialize;
use serde::Serialize;

use crate::config_file_path;
use crate::types::Income;
use crate::types::Item;
use crate::types::Queue;
use crate::types::State;
use crate::types::M;
use crate::write_file;

/// Attempt to read state file in the old format, and if successful, write it
/// back in the new format. No semantic changes should be made in the migration.
pub fn migrate_statefile() -> Result<State> {
    let file = std::fs::read_to_string(config_file_path())?;
    let parsed: LegacyState = serde_json::from_str(&file)?;

    let new_state = State {
        queues: vec![Queue {
            income: parsed.income,
            name: "default".into(),
            last_calculation: parsed.last_calculation,
            current_balance: parsed.current_amount,
            future_purchases: parsed.future_purchases,
            past_purchases: parsed.past_purchases,
            paused: parsed.paused.unwrap_or(false),
        }],
        currently_selected: "default".into(),
        globally_paused: false,
    };

    println!("Migrated config file to current format, continuing...");
    write_file(&new_state)?;
    Ok(new_state)
}

/// Legacy format of the state file. This is going to get interesting when I
/// need to add another migration...
#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyState {
    income: Income,
    last_calculation: String,
    current_amount: M,
    future_purchases: VecDeque<Item>,
    past_purchases: VecDeque<Item>,
    paused: Option<bool>,
}
