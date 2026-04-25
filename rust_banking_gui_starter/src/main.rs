mod access_control;
mod audit;
mod auth;
mod banking;
mod db;
mod models;
mod blockchain;

use blockchain::BlockchainClient;
use eframe::egui;
use models::{Account, AuditRecord, TransactionRecord, Transfer, User};
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Login,
    Dashboard,
    Accounts,
    Transfer,
    History,
    PendingTransfers,
    AuditLogs,
}

struct AppState {
    conn: Connection,
    current_user: Option<User>,
    screen: Screen,
    status_message: String,

    username_input: String,
    password_input: String,
    from_account_input: String,
    to_account_input: String,
    amount_input: String,
    history_account_input: String,
    approve_transfer_input: String,
    approve_transfer_pubkey_input: String,
    accounts: Vec<Account>,
    transactions: Vec<TransactionRecord>,
    pending_transfers: Vec<Transfer>,
    audit_logs: Vec<AuditRecord>,
    blockchain: BlockchainClient,
}

impl AppState {
    fn new() -> Self {
        let conn = db::connect().expect("Failed to connect to database");
        db::init_db(&conn).expect("Failed to initialize database");

        Self {
            conn,
            current_user: None,
            screen: Screen::Login,
            status_message: String::new(),
            username_input: String::new(),
            password_input: String::new(),
            from_account_input: String::new(),
            to_account_input: String::new(),
            amount_input: String::new(),
            history_account_input: String::new(),
            approve_transfer_input: String::new(),
            accounts: vec![],
            transactions: vec![],
            pending_transfers: vec![],
            audit_logs: vec![],
            blockchain: BlockchainClient::new().unwrap(),
            approve_transfer_pubkey_input: String::new(),
        }
    }

    fn login(&mut self) {
        let username = self.username_input.trim();
        let password = self.password_input.trim();

        if username.is_empty() || password.is_empty() {
            self.status_message = "Please enter both username and password.".to_string();
            return;
        }

        match auth::login(&self.conn, username, password) {
            Ok(Some(user)) => {
                let details = format!("{} logged in as {}", user.username, user.role);
                let _ = audit::log_action(&self.conn, Some(user.user_id), "LOGIN_SUCCESS", &details);

                self.current_user = Some(user);
                self.screen = Screen::Dashboard;
                self.password_input.clear();
                self.status_message = "Login successful.".to_string();
                self.refresh_all_lists();
            }
            Ok(None) => {
                self.status_message = "Invalid username or password.".to_string();
            }
            Err(_) => {
                self.status_message = "Could not log in right now.".to_string();
            }
        }
    }

    fn logout(&mut self) {
        if let Some(user) = &self.current_user {
            let details = format!("{} logged out", user.username);
            let _ = audit::log_action(&self.conn, Some(user.user_id), "LOGOUT", &details);
        }

        self.current_user = None;
        self.screen = Screen::Login;
        self.status_message = "Logged out.".to_string();
        self.username_input.clear();
        self.password_input.clear();
        self.accounts.clear();
        self.transactions.clear();
        self.pending_transfers.clear();
        self.audit_logs.clear();
    }

    fn refresh_all_lists(&mut self) {
        self.load_accounts();
        self.load_pending_transfers();
        self.load_audit_logs();
    }

    fn load_accounts(&mut self) {
        self.accounts.clear();

        if let Some(user) = &self.current_user {
            if let Ok(accounts) = banking::get_accessible_accounts(&self.conn, user) {
                self.accounts = accounts;
            }
        }
    }

    fn load_history(&mut self) {
        self.transactions.clear();

        let account_id = match self.history_account_input.trim().parse::<i32>() {
            Ok(value) => value,
            Err(_) => {
                self.status_message = "Account ID must be a number.".to_string();
                return;
            }
        };

        let Some(user) = &self.current_user else {
            return;
        };

        match banking::get_transaction_history(&self.conn, user, account_id) {
            Ok(records) => {
                self.transactions = records;
                if self.transactions.is_empty() {
                    self.status_message = "No history found, or access was denied.".to_string();
                } else {
                    self.status_message = format!("Loaded history for account {}.", account_id);
                }
            }
            Err(_) => {
                self.status_message = "Could not load transaction history.".to_string();
            }
        }
    }

    fn load_pending_transfers(&mut self) {
        self.pending_transfers.clear();

        if let Some(user) = &self.current_user {
            if user.role == "manager" {
                if let Ok(transfers) = banking::list_pending_transfers(&self.conn) {
                    self.pending_transfers = transfers;
                }
            }
        }
    }

    fn load_audit_logs(&mut self) {
        self.audit_logs.clear();

        if let Some(user) = &self.current_user {
            if user.role == "manager" || user.role == "auditor" {
                if let Ok(logs) = audit::list_audit_logs(&self.conn, 20) {
                    self.audit_logs = logs;
                }
            }
        }
    }

    fn submit_transfer(&mut self) {
    let from_account = match self.from_account_input.trim().parse::<i32>() {
        Ok(value) => value,
        Err(_) => {
            self.status_message = "From account ID must be a number.".to_string();
            return;
        }
    };

    let to_account = match self.to_account_input.trim().parse::<i32>() {
        Ok(value) => value,
        Err(_) => {
            self.status_message = "To account ID must be a number.".to_string();
            return;
        }
    };

    let amount = match self.amount_input.trim().parse::<f64>() {
        Ok(value) => value,
        Err(_) => {
            self.status_message = "Amount must be a valid number.".to_string();
            return;
        }
    };

    let Some(user) = self.current_user.clone() else {
        return;
    };

    match banking::transfer_money(&mut self.conn, &user, from_account, to_account, amount) {
        Ok(message) => {
            let amount_in_cents = (amount * 100.0).round() as u64;

            match self.blockchain.submit_transfer_on_chain(
                from_account as u64,
                to_account as u64,
                amount_in_cents,
            ) {
                Ok(result) => {
                    println!(
                        "Saved on-chain transfer request: {}",
                        result.transfer_request_pubkey
                    );
                    println!("Blockchain tx signature: {}", result.signature);
                }
                Err(e) => {
                    println!("Blockchain submit failed: {}", e);
                }
            }

            self.status_message = message;
            self.from_account_input.clear();
            self.to_account_input.clear();
            self.amount_input.clear();
            self.refresh_all_lists();
        }
        Err(_) => {
            self.status_message = "Could not submit transfer.".to_string();
        }
    }
}

    fn approve_selected_transfer(&mut self) {
    let transfer_id = match self.approve_transfer_input.trim().parse::<i32>() {
        Ok(value) => value,
        Err(_) => {
            self.status_message = "Transfer ID must be a number.".to_string();
            return;
        }
    };

    let transfer_request_pubkey = match self.approve_transfer_pubkey_input.trim().parse() {
        Ok(value) => value,
        Err(_) => {
            self.status_message = "Transfer request pubkey is invalid.".to_string();
            return;
        }
    };

    let Some(user) = self.current_user.clone() else {
        return;
    };

    match self.blockchain.approve_transfer_on_chain(transfer_request_pubkey) {
        Ok(signature) => {
            println!("On-chain approval signature: {}", signature);

            match banking::approve_transfer(&mut self.conn, &user, transfer_id) {
                Ok(message) => {
                    self.status_message =
                        format!("{} | On-chain approval successful.", message);
                    self.approve_transfer_input.clear();
                    self.approve_transfer_pubkey_input.clear();
                    self.refresh_all_lists();
                }
                Err(_) => {
                    self.status_message =
                        "On-chain approval worked, but local DB approval failed.".to_string();
                }
            }
        }
        Err(e) => {
            println!("Blockchain approval failed: {}", e);
            self.status_message = format!("Blockchain approval failed: {}", e);
        }
    }
}
    fn show_login_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(70.0);
                ui.heading("Beginner-Friendly Banking App");
                ui.label("Login first, then use the menu on the left.");
                ui.add_space(20.0);

                ui.set_max_width(300.0);
                ui.label("Username");
                ui.text_edit_singleline(&mut self.username_input);

                ui.label("Password");
                ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));

                ui.add_space(10.0);
                if ui.button("Login").clicked() {
                    self.login();
                }

                if !self.status_message.is_empty() {
                    ui.add_space(10.0);
                    ui.label(&self.status_message);
                }

                ui.add_space(20.0);
                ui.separator();
                ui.label("Test accounts: cust1, teller1, manager1, auditor1");
                ui.label("Password for all of them: 1234");
            });
        });
    }

    fn show_sidebar(&mut self, ctx: &egui::Context) {
        let Some(user) = &self.current_user else {
            return;
        };

        let full_name = user.full_name.clone();
        let role = user.role.clone();

        egui::SidePanel::left("sidebar").min_width(220.0).show(ctx, |ui| {
            ui.heading("Menu");
            ui.separator();
            ui.label(format!("Name: {}", full_name));
            ui.label(format!("Role: {}", role));
            ui.add_space(10.0);

            if ui.button("Dashboard").clicked() {
                self.screen = Screen::Dashboard;
            }
            if ui.button("Accounts").clicked() {
                self.load_accounts();
                self.screen = Screen::Accounts;
            }
            if role != "auditor" && ui.button("Transfer Money").clicked() {
                self.screen = Screen::Transfer;
            }
            if ui.button("Transaction History").clicked() {
                self.screen = Screen::History;
            }
            if role == "manager" && ui.button("Pending Transfers").clicked() {
                self.load_pending_transfers();
                self.screen = Screen::PendingTransfers;
            }
            if (role == "manager" || role == "auditor") && ui.button("Audit Logs").clicked() {
                self.load_audit_logs();
                self.screen = Screen::AuditLogs;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                if ui.button("Logout").clicked() {
                    self.logout();
                }
            });
        });
    }

    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        let role = self
            .current_user
            .as_ref()
            .map(|user| user.role.clone())
            .unwrap_or_default();

        ui.heading("Dashboard");
        ui.add_space(10.0);
        ui.label("This screen gives a quick summary of what you can see right now.");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(format!("Accounts you can see: {}", self.accounts.len()));
            if role == "manager" {
                ui.label(format!("Pending transfers: {}", self.pending_transfers.len()));
            }
            if role == "manager" || role == "auditor" {
                ui.label(format!("Recent audit logs loaded: {}", self.audit_logs.len()));
            }
        });
    }

    fn show_accounts(&mut self, ui: &mut egui::Ui) {
        ui.heading("Accounts");
        ui.add_space(10.0);

        if self.accounts.is_empty() {
            ui.label("No accounts to show.");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for account in &self.accounts {
                ui.group(|ui| {
                    ui.label(format!("Account ID: {}", account.account_id));
                    ui.label(format!("Owner User ID: {}", account.user_id));
                    ui.label(format!("Type: {}", account.account_type));
                    ui.label(format!("Balance: ${:.2}", account.balance));
                    ui.label(format!("Status: {}", account.status));
                });
                ui.add_space(8.0);
            }
        });
    }

    fn show_transfer_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transfer Money");
        ui.add_space(10.0);
        ui.label("Transfers below $1000 complete immediately.");
        ui.label("Transfers of $1000 or more become pending and need manager approval.");
        ui.add_space(10.0);

        ui.label("From Account ID");
        ui.text_edit_singleline(&mut self.from_account_input);
        ui.label("To Account ID");
        ui.text_edit_singleline(&mut self.to_account_input);
        ui.label("Amount");
        ui.text_edit_singleline(&mut self.amount_input);

        ui.add_space(10.0);
        if ui.button("Submit Transfer").clicked() {
            self.submit_transfer();
        }
    }

    fn show_history_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transaction History");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Account ID:");
            ui.text_edit_singleline(&mut self.history_account_input);
            if ui.button("Load History").clicked() {
                self.load_history();
            }
        });

        ui.add_space(10.0);
        if self.transactions.is_empty() {
            ui.label("No transactions loaded.");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for record in &self.transactions {
                ui.group(|ui| {
                    ui.label(format!("Transaction ID: {}", record.transaction_id));
                    ui.label(format!("Account ID: {}", record.account_id));
                    ui.label(format!("Type: {}", record.tx_type));
                    ui.label(format!("Amount: ${:.2}", record.amount));
                    ui.label(format!("Description: {}", record.description));
                    ui.label(format!("Time: {}", record.created_at));
                });
                ui.add_space(8.0);
            }
        });
    }

    fn show_pending_transfers_screen(&mut self, ui: &mut egui::Ui) {
    ui.heading("Pending Transfers");
    ui.add_space(10.0);

    ui.label("Approve a transfer using both the local transfer ID and the on-chain transfer request pubkey.");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Local Transfer ID:");
        ui.text_edit_singleline(&mut self.approve_transfer_input);
    });

    ui.horizontal(|ui| {
        ui.label("On-chain Transfer Pubkey:");
        ui.text_edit_singleline(&mut self.approve_transfer_pubkey_input);
    });

    ui.add_space(8.0);

    if ui.button("Approve Transfer").clicked() {
        self.approve_selected_transfer();
    }

    ui.add_space(12.0);

    if self.pending_transfers.is_empty() {
        ui.label("No pending transfers.");
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for transfer in &self.pending_transfers {
            ui.group(|ui| {
                ui.label(format!("Transfer ID: {}", transfer.transfer_id));
                ui.label(format!("From Account: {}", transfer.from_account));
                ui.label(format!("To Account: {}", transfer.to_account));
                ui.label(format!("Amount: ${:.2}", transfer.amount));
                ui.label(format!("Status: {}", transfer.status));
                ui.label(format!("Requested By User ID: {}", transfer.requested_by));
                ui.label(format!("Approved By: {:?}", transfer.approved_by));
                ui.label(format!("Created At: {}", transfer.created_at));
            });
            ui.add_space(8.0);
        }
    });
}

    fn show_audit_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Audit Logs");
        ui.add_space(10.0);

        if self.audit_logs.is_empty() {
            ui.label("No audit logs to show.");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for log in &self.audit_logs {
                let prev_hash = if log.prev_hash.len() > 20 {
                    format!("{}...", &log.prev_hash[..20])
                } else {
                    log.prev_hash.clone()
                };

                let current_hash = if log.current_hash.len() > 20 {
                    format!("{}...", &log.current_hash[..20])
                } else {
                    log.current_hash.clone()
                };

                ui.group(|ui| {
                    ui.label(format!("Log ID: {}", log.log_id));
                    ui.label(format!("User ID: {:?}", log.user_id));
                    ui.label(format!("Action: {}", log.action));
                    ui.label(format!("Details: {}", log.details));
                    ui.label(format!("Time: {}", log.created_at));
                    ui.label(format!("Previous Hash: {}", prev_hash));
                    ui.label(format!("Current Hash: {}", current_hash));
                });
                ui.add_space(8.0);
            }
        });
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Rust Banking App");
                ui.separator();
                ui.label("Beginner-friendly version");
            });
        });

        if self.screen == Screen::Login {
            self.show_login_screen(ctx);
            return;
        }

        self.show_sidebar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.status_message.is_empty() {
                ui.label(format!("Status: {}", self.status_message));
                ui.add_space(10.0);
            }

            match self.screen {
                Screen::Login => {}
                Screen::Dashboard => self.show_dashboard(ui),
                Screen::Accounts => self.show_accounts(ui),
                Screen::Transfer => self.show_transfer_screen(ui),
                Screen::History => self.show_history_screen(ui),
                Screen::PendingTransfers => self.show_pending_transfers_screen(ui),
                Screen::AuditLogs => self.show_audit_screen(ui),
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Rust Banking App",
        options,
        Box::new(|_cc| Box::new(AppState::new())),
    )
}