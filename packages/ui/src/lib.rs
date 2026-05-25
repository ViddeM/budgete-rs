//! Shared UI components for Budget.
//!
//! All components receive data via props — no server function calls or DB
//! access here. The `web` crate is responsible for fetching data and passing
//! it down.

mod category_badge;
pub mod format;
mod group_badge;
mod navbar;
mod project_transaction_row;
mod stat_card;
mod transaction_list;
mod transaction_queue_card;
mod transaction_row;

pub use category_badge::{CategoryBadge, UnprocessedBadge};
pub use format::{
    contrast_text, fmt_amount, fmt_date, fmt_tx_amount, hover_filter, tx_amount_color,
};
pub use group_badge::GroupBadge;
pub use navbar::{NavLink, Navbar};
pub use project_transaction_row::ProjectTransactionRow;
pub use stat_card::StatCard;
pub use transaction_list::TransactionList;
pub use transaction_queue_card::TransactionQueueCard;
pub use transaction_row::{ClassifyAction, TransactionRow};
