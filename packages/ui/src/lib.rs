//! Shared UI components for Budget.
//!
//! All components receive data via props — no server function calls or DB
//! access here. The `web` crate is responsible for fetching data and passing
//! it down.

pub mod format;
mod category_badge;
mod group_badge;
mod navbar;
mod stat_card;
mod transaction_list;
mod transaction_queue_card;
mod transaction_row;

pub use category_badge::{CategoryBadge, UnprocessedBadge};
pub use format::{fmt_amount, fmt_tx_amount};
pub use group_badge::GroupBadge;
pub use navbar::{NavLink, Navbar};
pub use stat_card::StatCard;
pub use transaction_list::TransactionList;
pub use transaction_queue_card::TransactionQueueCard;
pub use transaction_row::{ClassifyAction, TransactionRow};
