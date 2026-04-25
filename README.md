# 🏦 Hybrid Banking System (Solana + Rust GUI)

## 🚀 Overview

This project is a **hybrid banking application** that combines a **Rust desktop GUI** with a **Solana smart contract**.

* 🖥️ GUI handles user interaction and local data (SQLite)
* 🔗 Blockchain handles **transfer requests and approvals**
* ⚡ Result: fast + secure system (best of both worlds)

## 🏗️ Project Structure

```
Hybrid-Banking-System/
├── banking-solana-program/      # Solana smart contract (Anchor)
├── rust_banking_gui_starter/    # Rust GUI application
└── README.md
---

## ⚙️ Tech Stack

* Rust (GUI + backend)
* egui (desktop interface)
* SQLite (local database)
* Solana (blockchain)
* Anchor (smart contract framework)

## 🔄 How It Works

1. User submits a transfer from GUI
2. Transfer is stored locally (SQLite)
3. Same request is sent to blockchain (`submit_transfer`)
4. Manager reviews pending transfers
5. Manager approves transfer (`approve_transfer`)
6. Blockchain records approval permanently
7. GUI updates status

👉 Only **critical actions** are on-chain
👉 Everything else stays fast and local

---

## ▶️ How to Run

### 1. Start local blockchain

```bash
solana-test-validator --reset
```

---

### 2. Configure Solana

```bash
solana config set --url localhost
```

---

### 3. Fund wallet

```bash
solana airdrop 100
```

---

### 4. Deploy smart contract

```bash
cd banking-solana-program
anchor build
anchor deploy
```

---

### 5. Run GUI

```bash
cd rust_banking_gui_starter
cargo run
```

---


## ⚠️ Common Issues

### Airdrop stuck

```bash
solana-test-validator --reset
```

---

### InstructionFallbackNotFound

```bash
anchor build
anchor deploy
```

---

### Wrong network

```bash
solana config set --url localhost
```

---

## 🔐 Design Idea

This project uses a **hybrid model**:

| Off-chain (SQLite) | On-chain (Solana) |
| ------------------ | ----------------- |
| users, accounts    | transfer requests |
| balances           | approvals         |
| UI state           | proof of action   |

👉 Blockchain = trust layer
👉 Database = performance layer

---
