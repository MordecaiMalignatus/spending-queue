use std::collections::VecDeque;

use fraction::GenericDecimal;
use serde::{Deserialize, Serialize};

/// Type used for money, abstracting over an arbitrary-precision number. This is
/// important, as sq has to work correctly on potentially very small fractions
/// of currency without loss of precision, as the accrual window has to be kept
/// to.
pub type M = GenericDecimal<u64, u8>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Item {
    pub name: String,
    pub amount: M,
    pub purchase_link: Option<String>,
    pub time_purchased: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Income {
    pub amount: f64,
    pub interval_in_days: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Queue {
    pub income: Income,
    pub name: String,
    pub last_calculation: String,
    pub current_balance: M,
    pub future_purchases: VecDeque<Item>,
    pub past_purchases: VecDeque<Item>,
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub queues: Vec<Queue>,
    /// Identify queue by its name. Not foolproof, good enough here.
    pub currently_selected: String,
    pub globally_paused: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            queues: vec![Queue {
                income: Income{ amount: 1.0, interval_in_days: 1 },
                name: "default".into(),
                last_calculation: chrono::Local::now().to_rfc2822(),
                current_balance: 0.into(),
                future_purchases: VecDeque::new(),
                past_purchases: VecDeque::new(),
                paused: false,
            }],
            currently_selected: "default".into(),
            globally_paused: false,
        }
    }
}
