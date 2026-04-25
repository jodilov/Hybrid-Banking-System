#[derive(Debug, Clone)]
pub struct User {
    pub user_id: i32,
    pub username: String,
    pub full_name: String,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub account_id: i32,
    pub user_id: i32,
    pub account_type: String,
    pub balance: f64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Transfer {
    pub transfer_id: i32,
    pub from_account: i32,
    pub to_account: i32,
    pub amount: f64,
    pub status: String,
    pub requested_by: i32,
    pub approved_by: Option<i32>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct TransactionRecord {
    pub transaction_id: i32,
    pub account_id: i32,
    pub tx_type: String,
    pub amount: f64,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub log_id: i32,
    pub user_id: Option<i32>,
    pub action: String,
    pub details: String,
    pub created_at: String,
    pub prev_hash: String,
    pub current_hash: String,
}
