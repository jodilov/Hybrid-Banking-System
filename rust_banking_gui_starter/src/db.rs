use rusqlite::{params, Connection, Result};

pub fn connect() -> Result<Connection> {
    let conn = Connection::open("banking.db")?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            user_id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            full_name TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('customer', 'teller', 'manager', 'auditor'))
        );

        CREATE TABLE IF NOT EXISTS accounts (
            account_id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            account_type TEXT NOT NULL,
            balance REAL NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active',
            FOREIGN KEY (user_id) REFERENCES users(user_id)
        );

        CREATE TABLE IF NOT EXISTS transfers (
            transfer_id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_account INTEGER NOT NULL,
            to_account INTEGER NOT NULL,
            amount REAL NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('pending', 'approved', 'rejected', 'completed')),
            requested_by INTEGER NOT NULL,
            approved_by INTEGER,
            created_at TEXT NOT NULL,
            FOREIGN KEY (from_account) REFERENCES accounts(account_id),
            FOREIGN KEY (to_account) REFERENCES accounts(account_id),
            FOREIGN KEY (requested_by) REFERENCES users(user_id),
            FOREIGN KEY (approved_by) REFERENCES users(user_id)
        );

        CREATE TABLE IF NOT EXISTS transactions (
            transaction_id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id INTEGER NOT NULL,
            type TEXT NOT NULL,
            amount REAL NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (account_id) REFERENCES accounts(account_id)
        );

        CREATE TABLE IF NOT EXISTS audit_log (
            log_id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER,
            action TEXT NOT NULL,
            details TEXT NOT NULL,
            created_at TEXT NOT NULL,
            prev_hash TEXT NOT NULL,
            current_hash TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(user_id)
        );
        "#,
    )?;

    seed_data(conn)?;
    Ok(())
}

fn seed_data(conn: &Connection) -> Result<()> {
    let user_count: i32 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    if user_count == 0 {
        conn.execute(
            "INSERT INTO users (username, password, full_name, role) VALUES (?1, ?2, ?3, ?4)",
            params!["cust1", "1234", "Ali Customer", "customer"],
        )?;
        conn.execute(
            "INSERT INTO users (username, password, full_name, role) VALUES (?1, ?2, ?3, ?4)",
            params!["teller1", "1234", "Sara Teller", "teller"],
        )?;
        conn.execute(
            "INSERT INTO users (username, password, full_name, role) VALUES (?1, ?2, ?3, ?4)",
            params!["manager1", "1234", "Omar Manager", "manager"],
        )?;
        conn.execute(
            "INSERT INTO users (username, password, full_name, role) VALUES (?1, ?2, ?3, ?4)",
            params!["auditor1", "1234", "Lina Auditor", "auditor"],
        )?;
    }

    let account_count: i32 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))?;
    if account_count == 0 {
        conn.execute(
            "INSERT INTO accounts (user_id, account_type, balance, status) VALUES (?1, ?2, ?3, ?4)",
            params![1, "checking", 2500.00_f64, "active"],
        )?;
        conn.execute(
            "INSERT INTO accounts (user_id, account_type, balance, status) VALUES (?1, ?2, ?3, ?4)",
            params![1, "savings", 5000.00_f64, "active"],
        )?;
        conn.execute(
            "INSERT INTO accounts (user_id, account_type, balance, status) VALUES (?1, ?2, ?3, ?4)",
            params![2, "checking", 10000.00_f64, "active"],
        )?;
        conn.execute(
            "INSERT INTO accounts (user_id, account_type, balance, status) VALUES (?1, ?2, ?3, ?4)",
            params![3, "operations", 20000.00_f64, "active"],
        )?;
    }

    Ok(())
}
