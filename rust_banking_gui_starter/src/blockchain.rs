use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature, Signer},
    system_program,
    transaction::Transaction,
};

pub struct OnChainTransferResult {
    pub transfer_request_pubkey: Pubkey,
    pub signature: Signature,
}

pub struct BlockchainClient {
    rpc: RpcClient,
    program_id: Pubkey,
    payer: Keypair,
}

impl BlockchainClient {
    pub fn new() -> Result<Self> {
        let rpc = RpcClient::new("http://localhost:8899".to_string());

        let program_id: Pubkey =
            "6BWzBZHkuVgHew2mo4Uqf87csVVXJcbt2QZzreMUifQK".parse()?;

        let payer = read_keypair_file(r"C:\Users\unitu\.config\solana\id.json")
            .map_err(|e| anyhow!("Failed to load wallet file: {}", e))?;

        Ok(Self {
            rpc,
            program_id,
            payer,
        })
    }

    pub fn test_call(&self) -> Result<()> {
        println!("Connected to program: {}", self.program_id);
        println!("Wallet address: {}", self.payer.pubkey());
        println!("Ready to send on-chain transfer records.");
        Ok(())
    }

    pub fn submit_transfer_on_chain(
        &self,
        from_user_id: u64,
        to_user_id: u64,
        amount: u64,
    ) -> Result<OnChainTransferResult> {
        if amount == 0 {
            return Err(anyhow!("Amount must be greater than zero"));
        }

        let transfer_request = Keypair::new();
        let recent_blockhash = self.rpc.get_latest_blockhash()?;

        let mut data = Vec::new();

        // Exact discriminator from the generated IDL for submit_transfer
        data.extend_from_slice(&[131, 57, 253, 234, 98, 101, 37, 157]);

        // Arguments in exact order:
        // submit_transfer(from_user_id, to_user_id, amount)
        data.extend_from_slice(&from_user_id.to_le_bytes());
        data.extend_from_slice(&to_user_id.to_le_bytes());
        data.extend_from_slice(&amount.to_le_bytes());

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(transfer_request.pubkey(), true),
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer, &transfer_request],
            recent_blockhash,
        );

        let simulation = self.rpc.simulate_transaction(&transaction)?;
        println!("Simulation logs: {:?}", simulation.value.logs);

        let signature = self.rpc.send_and_confirm_transaction(&transaction)?;

        println!("On-chain transfer request created.");
        println!("Transfer request account: {}", transfer_request.pubkey());
        println!("Transaction signature: {}", signature);

        Ok(OnChainTransferResult {
            transfer_request_pubkey: transfer_request.pubkey(),
            signature,
        })
    }

    pub fn approve_transfer_on_chain(
        &self,
        transfer_request_pubkey: Pubkey,
    ) -> Result<Signature> {
        let recent_blockhash = self.rpc.get_latest_blockhash()?;

        let mut data = Vec::new();

        // Exact discriminator from the generated IDL for approve_transfer
        data.extend_from_slice(&[198, 217, 247, 150, 208, 60, 169, 244]);

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(transfer_request_pubkey, false),
                AccountMeta::new(self.payer.pubkey(), true),
            ],
            data,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );

        let simulation = self.rpc.simulate_transaction(&transaction)?;
        println!("Approval simulation logs: {:?}", simulation.value.logs);

        let signature = self.rpc.send_and_confirm_transaction(&transaction)?;

        println!("On-chain transfer approved.");
        println!("Transaction signature: {}", signature);

        Ok(signature)
    }
}