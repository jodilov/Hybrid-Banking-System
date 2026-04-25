use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Result};

use crate::{
    access_control,
    audit,
    models::{Account, TransactionRecord, Transfer, User},
};

const APPROVAL_THRESHOLD: f64 = 1000.0;

fn map_account(row: &rusqlite::Row<'_>) -> Result<Account> {
    Ok(Account {
        account_id: row.get(0)?,
        user_id: row.get(1)?,
        account_type: row.get(2)?,
        balance: row.get(3)?,
        status: row.get(4)?,
    })
}

pub fn get_accessible_accounts(conn: &Connection, current_user: &User) -> Result<Vec<Account>> {
    let sql = if current_user.role == "customer" {
        "SELECT account_id, user_id, account_type, balance, status
         FROM accounts
         WHERE user_id = ?1
         ORDER BY account_id"
    } else {
        "SELECT account_id, user_id, account_type, balance, status
         FROM accounts
         ORDER BY account_id"
    };

    let mut stmt = conn.prepare(sql)?;

    if current_user.role == "customer" {
        let rows = stmt.query_map(params![current_user.user_id], map_account)?;
        rows.collect()
    } else {
        let rows = stmt.query_map([], map_account)?;
        rows.collect()
    }
}

pub fn get_account_by_id(conn: &Connection, account_id: i32) -> Result<Option<Account>> {
    conn.query_row(
        "SELECT account_id, user_id, account_type, balance, status
         FROM accounts
         WHERE account_id = ?1",
        params![account_id],
        map_account,
    )
    .optional()
}

pub fn get_transaction_history(
    conn: &Connection,
    current_user: &User,
    account_id: i32,
) -> Result<Vec<TransactionRecord>> {
    let account = match get_account_by_id(conn, account_id)? {
        Some(account) => account,
        None => return Ok(vec![]),
    };

    if !access_control::can_view_account(current_user, account.user_id) {
        return Ok(vec![]);
    }

    let mut stmt = conn.prepare(
        "SELECT transaction_id, account_id, type, amount, COALESCE(description, ''), created_at
         FROM transactions
         WHERE account_id = ?1
         ORDER BY transaction_id DESC",
    )?;

    let rows = stmt.query_map(params![account_id], |row| {
        Ok(TransactionRecord {
            transaction_id: row.get(0)?,
            account_id: row.get(1)?,
            tx_type: row.get(2)?,
            amount: row.get(3)?,
            description: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    rows.collect()
}

pub fn transfer_money(
    conn: &mut Connection,
    current_user: &User,
    from_account: i32,
    to_account: i32,
    amount: f64,
) -> Result<String> {
    if amount <= 0.0 {
        return Ok("Amount must be greater than zero.".to_string());
    }

    if from_account == to_account {
        return Ok("Source and destination accounts must be different.".to_string());
    }

    let source = match get_account_by_id(conn, from_account)? {
        Some(account) => account,
        None => return Ok("Source account not found.".to_string()),
    };

    if get_account_by_id(conn, to_account)?.is_none() {
        return Ok("Destination account not found.".to_string());
    }

    if !access_control::can_initiate_transfer(current_user, source.user_id) {
        audit::log_action(
            conn,
            Some(current_user.user_id),
            "TRANSFER_DENIED",
            &format!("User {} tried transfer from account {}", current_user.username, from_account),
        )?;
        return Ok("Access denied for this transfer.".to_string());
    }

    if source.balance < amount {
        audit::log_action(
            conn,
            Some(current_user.user_id),
            "TRANSFER_FAILED",
            &format!("Insufficient funds in account {}", from_account),
        )?;
        return Ok("Insufficient funds.".to_string());
    }

    let created_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if amount >= APPROVAL_THRESHOLD {
        conn.execute(
            "INSERT INTO transfers (from_account, to_account, amount, status, requested_by, approved_by, created_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, NULL, ?5)",
            params![from_account, to_account, amount, current_user.user_id, created_at],
        )?;

        audit::log_action(
            conn,
            Some(current_user.user_id),
            "TRANSFER_PENDING",
            &format!(
                "Pending transfer created: from {} to {} amount {:.2}",
                from_account, to_account, amount
            ),
        )?;

        return Ok("Transfer submitted. Awaiting manager approval.".to_string());
    }

    let tx = conn.transaction()?;

    tx.execute(
        "UPDATE accounts SET balance = balance - ?1 WHERE account_id = ?2",
        params![amount, from_account],
    )?;
    tx.execute(
        "UPDATE accounts SET balance = balance + ?1 WHERE account_id = ?2",
        params![amount, to_account],
    )?;

    tx.execute(
        "INSERT INTO transfers (from_account, to_account, amount, status, requested_by, approved_by, created_at)
         VALUES (?1, ?2, ?3, 'completed', ?4, ?5, ?6)",
        params![from_account, to_account, amount, current_user.user_id, current_user.user_id, created_at],
    )?;

    tx.execute(
        "INSERT INTO transactions (account_id, type, amount, description, created_at)
         VALUES (?1, 'debit', ?2, ?3, ?4)",
        params![from_account, amount, format!("Transfer to account {}", to_account), created_at],
    )?;
    tx.execute(
        "INSERT INTO transactions (account_id, type, amount, description, created_at)
         VALUES (?1, 'credit', ?2, ?3, ?4)",
        params![to_account, amount, format!("Transfer from account {}", from_account), created_at],
    )?;

    tx.commit()?;

    audit::log_action(
        conn,
        Some(current_user.user_id),
        "TRANSFER_COMPLETED",
        &format!("Transfer completed: from {} to {} amount {:.2}", from_account, to_account, amount),
    )?;

    Ok("Transfer completed successfully.".to_string())
}

pub fn list_pending_transfers(conn: &Connection) -> Result<Vec<Transfer>> {
    let mut stmt = conn.prepare(
        "SELECT transfer_id, from_account, to_account, amount, status, requested_by, approved_by, created_at
         FROM transfers
         WHERE status = 'pending'
         ORDER BY transfer_id DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Transfer {
            transfer_id: row.get(0)?,
            from_account: row.get(1)?,
            to_account: row.get(2)?,
            amount: row.get(3)?,
            status: row.get(4)?,
            requested_by: row.get(5)?,
            approved_by: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    rows.collect()
}

pub fn approve_transfer(conn: &mut Connection, current_user: &User, transfer_id: i32) -> Result<String> {
    if !access_control::can_approve_transfer(current_user) {
        audit::log_action(
            conn,
            Some(current_user.user_id),
            "APPROVAL_DENIED",
            &format!("User {} tried approving transfer {}", current_user.username, transfer_id),
        )?;
        return Ok("Only managers can approve transfers.".to_string());
    }

    let transfer = conn
        .query_row(
            "SELECT transfer_id, from_account, to_account, amount, status, requested_by, approved_by, created_at
             FROM transfers WHERE transfer_id = ?1",
            params![transfer_id],
            |row| {
                Ok(Transfer {
                    transfer_id: row.get(0)?,
                    from_account: row.get(1)?,
                    to_account: row.get(2)?,
                    amount: row.get(3)?,
                    status: row.get(4)?,
                    requested_by: row.get(5)?,
                    approved_by: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()?;

    let transfer = match transfer {
        Some(transfer) => transfer,
        None => return Ok("Transfer not found.".to_string()),
    };

    if transfer.status != "pending" {
        return Ok("This transfer is not pending anymore.".to_string());
    }

    let source = match get_account_by_id(conn, transfer.from_account)? {
        Some(account) => account,
        None => return Ok("Source account not found.".to_string()),
    };

    if source.balance < transfer.amount {
        return Ok("Cannot approve transfer because the source account has insufficient funds.".to_string());
    }

    let created_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let tx = conn.transaction()?;

    tx.execute(
        "UPDATE accounts SET balance = balance - ?1 WHERE account_id = ?2",
        params![transfer.amount, transfer.from_account],
    )?;
    tx.execute(
        "UPDATE accounts SET balance = balance + ?1 WHERE account_id = ?2",
        params![transfer.amount, transfer.to_account],
    )?;

    tx.execute(
        "UPDATE transfers SET status = 'completed', approved_by = ?1 WHERE transfer_id = ?2",
        params![current_user.user_id, transfer.transfer_id],
    )?;

    tx.execute(
        "INSERT INTO transactions (account_id, type, amount, description, created_at)
         VALUES (?1, 'debit', ?2, ?3, ?4)",
        params![
            transfer.from_account,
            transfer.amount,
            format!("Approved transfer to account {}", transfer.to_account),
            created_at
        ],
    )?;
    tx.execute(
        "INSERT INTO transactions (account_id, type, amount, description, created_at)
         VALUES (?1, 'credit', ?2, ?3, ?4)",
        params![
            transfer.to_account,
            transfer.amount,
            format!("Approved transfer from account {}", transfer.from_account),
            created_at
        ],
    )?;

    tx.commit()?;

    audit::log_action(
        conn,
        Some(current_user.user_id),
        "TRANSFER_APPROVED",
        &format!(
            "Transfer {} approved by manager {}",
            transfer.transfer_id, current_user.username
        ),
    )?;

    Ok("Transfer approved and completed.".to_string())
}
