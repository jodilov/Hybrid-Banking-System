use anchor_lang::prelude::*;

declare_id!("6BWzBZHkuVgHew2mo4Uqf87csVVXJcbt2QZzreMUifQK");

#[program]
pub mod banking_solana_program {
    use super::*;

    pub fn submit_transfer(
        ctx: Context<SubmitTransfer>,
        from_user_id: u64,
        to_user_id: u64,
        amount: u64,
    ) -> Result<()> {
        let transfer_request = &mut ctx.accounts.transfer_request;
        let user = &ctx.accounts.user;

        if amount == 0 {
            return err!(BankingError::InvalidAmount);
        }

        transfer_request.requested_by = user.key();
        transfer_request.from_user_id = from_user_id;
        transfer_request.to_user_id = to_user_id;
        transfer_request.amount = amount;
        transfer_request.approved = false;
        transfer_request.completed = false;
        transfer_request.approved_by = None;

        Ok(())
    }

    pub fn approve_transfer(ctx: Context<ApproveTransfer>) -> Result<()> {
        let transfer_request = &mut ctx.accounts.transfer_request;
        let manager = &ctx.accounts.manager;

        if transfer_request.approved {
            return err!(BankingError::AlreadyApproved);
        }

        if transfer_request.completed {
            return err!(BankingError::AlreadyCompleted);
        }

        transfer_request.approved = true;
        transfer_request.completed = true;
        transfer_request.approved_by = Some(manager.key());

        Ok(())
    }
}

#[derive(Accounts)]
pub struct SubmitTransfer<'info> {
    #[account(init, payer = user, space = 8 + TransferRequest::INIT_SPACE)]
    pub transfer_request: Account<'info, TransferRequest>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveTransfer<'info> {
    #[account(mut)]
    pub transfer_request: Account<'info, TransferRequest>,

    #[account(mut)]
    pub manager: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct TransferRequest {
    pub requested_by: Pubkey,
    pub from_user_id: u64,
    pub to_user_id: u64,
    pub amount: u64,
    pub approved: bool,
    pub completed: bool,
    pub approved_by: Option<Pubkey>,
}

#[error_code]
pub enum BankingError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,

    #[msg("Transfer already approved")]
    AlreadyApproved,

    #[msg("Transfer already completed")]
    AlreadyCompleted,
}
