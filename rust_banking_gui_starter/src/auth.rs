use rusqlite::{params, Connection, Result};

use crate::models::User;

pub fn login(conn: &Connection, username: &str, password: &str) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, username, full_name, role
         FROM users
         WHERE username = ?1 AND password = ?2",
    )?;

    let mut rows = stmt.query(params![username, password])?;

    if let Some(row) = rows.next()? {
        Ok(Some(User {
            user_id: row.get(0)?,
            username: row.get(1)?,
            full_name: row.get(2)?,
            role: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}
