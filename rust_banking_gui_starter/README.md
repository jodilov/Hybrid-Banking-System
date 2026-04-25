# Zero-Trust Banking GUI Starter

This is a native Rust desktop-style starter project for your capstone banking system.
It keeps the original Rust + SQLite backend logic and adds an `egui/eframe` GUI layer.

## Features
- Login screen
- Role-based dashboards
- View accessible accounts
- Transfer money
- Large-transfer manager approval
- Transaction history
- Audit log with hash chain

## Seeded users
- `cust1`
- `teller1`
- `manager1`
- `auditor1`

Password for all seeded users:
- `1234`

## Run
```bash
cargo run
```

## Notes
- SQLite file: `banking.db`
- Large transfers (`>= 1000`) are created as pending and require manager approval.
- The GUI is intentionally simple so you can demo separate screens and workflows quickly.
