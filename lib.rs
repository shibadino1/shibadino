use anchor_lang::prelude::*;
use anchor_spl::token::{ self, Mint, Token, TokenAccount, Transfer };
use anchor_lang::Result;
use chainlink_solana as chainlink;
declare_id!("2tFfBvRfgyXXRo6eaRijAmpvC69zCdqTzGy7z8B8cTX3");
const TIME: u64 = 604800;

// 604800 for 7 days
// 600 for 10 minutes
#[program]
pub mod shibadino_presale {
    use super::*;

    pub fn initialize_presale(
        ctx: Context<InitializePresale>,
        bump: u8,
        stages: Vec<Stage>,
        usdt_decimals: u8,
        decimals: u8
    ) -> Result<()> {
        let presale_account = &mut ctx.accounts.presale_account;
        presale_account.bump = bump;
        presale_account.active_stage = 0;
        presale_account.vesting = 0;
        presale_account.stage_end_time = (Clock::get().unwrap().unix_timestamp as u64) + TIME;
        presale_account.owner = ctx.accounts.owner.key();
        presale_account.admin = ctx.accounts.admin.key();
        presale_account.presale_token_vault = ctx.accounts.presale_token_vault.key();
        presale_account.owner_usdt_account = ctx.accounts.owner_usdt_account.key();
        presale_account.mint = ctx.accounts.mint.key();
        presale_account.usdt_mint = ctx.accounts.usdt_mint.key();
        presale_account.usdt_decimals = usdt_decimals;
        presale_account.decimals = decimals;
        // presale_account.current_stage_sold_tokens = 0;
        presale_account.sold_tokens = 0;
        presale_account.sol_raised = 0;
        presale_account.usdt_raised = 0;
        presale_account.stages = stages;
        Ok(())
    }
    pub fn add_stages(ctx: Context<AddStages>, new_stages: Vec<Stage>) -> Result<()> {
        if ctx.accounts.admin.key.ne(&ctx.accounts.presale_account.admin) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        let presale_account = &mut ctx.accounts.presale_account;
        presale_account.stages.extend(new_stages);
        Ok(())
    }
    pub fn transfer_token(ctx: Context<TransferTokens>, tokens: u128, decimals: u8) -> Result<()> {
        let final_tokens = tokens * (10u128).pow(decimals as u32);
        token::transfer(ctx.accounts.transfer_token(), final_tokens as u64)?;
        Ok(())
    }

    pub fn buy(ctx: Context<Buy>, lamports: u128, token_type: u8) -> Result<()> {
        let index = ctx.accounts.presale_account.active_stage;
        let stage_end_time = ctx.accounts.presale_account.stage_end_time;
        let from_lamports = lamports;
        let now = Clock::get().unwrap().unix_timestamp as u64;
        let usdt_decimals = ctx.accounts.presale_account.usdt_decimals;
        let decimals = ctx.accounts.presale_account.decimals;

        if now > stage_end_time {
            return Err(error!(ErrorCode::StageTimeUp));
        }
        if ctx.accounts.owner.key.ne(&ctx.accounts.presale_account.owner) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        if ctx.accounts.owner.key.ne(&ctx.accounts.presale_account.owner) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        if ctx.accounts.owner_usdt_account.key() != ctx.accounts.presale_account.owner_usdt_account {
            return Err(error!(ErrorCode::InvalidVault));
        }
        let tokens_per_usd =
            ctx.accounts.presale_account.stages[index as usize].price *
            (10u128).pow(decimals as u32);
        msg!("Lamports: {} {} {}", stage_end_time, now, ctx.accounts.presale_account.active_stage);
        let allocation = ctx.accounts.presale_account.stages[index as usize].allocation;
        let sold_tokens =
            ctx.accounts.presale_account.stages[index as usize].sold_tokens /
            (10u128).pow(decimals as u32);
        msg!("allocation: {}", allocation);
        if token_type == 1 {
            let round = chainlink::latest_round_data(
                ctx.accounts.chainlink_program.to_account_info(),
                ctx.accounts.chainlink_feed.to_account_info()
            )?;

            // Calculate the number of Bebe tokens based on the fetched Solana price
            let sol_price = round.answer;
            // let sol_price = 150;
            msg!("sol_price: {}", sol_price);
            let tokens =
                (lamports * (sol_price as u128) * tokens_per_usd) / 100_000_000_000_000_000;
            if
                (ctx.accounts.presale_account.stages[index as usize].sold_tokens + tokens) /
                    (10u128).pow(decimals as u32) > allocation
            {
                return Err(error!(ErrorCode::AllocationReached));
            }
            solana_program::program::invoke(
                &solana_program::system_instruction::transfer(
                    &ctx.accounts.user.key,
                    &ctx.accounts.owner.key,
                    from_lamports as u64
                ),
                &[
                    ctx.accounts.user.to_account_info().clone(),
                    ctx.accounts.owner.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ]
            )?;

            let presale_account = &mut ctx.accounts.presale_account;
            msg!("before {}", presale_account.stages[index as usize].sold_tokens);
            presale_account.stages[index as usize].sold_tokens += tokens;
            msg!(
                "after {} {} {}",
                presale_account.stages[index as usize].sold_tokens,
                now > stage_end_time || sold_tokens >= allocation,
                now > stage_end_time
            );
            presale_account.stages[index as usize].sol_raised += lamports;
            presale_account.sol_raised += lamports as u128;
            presale_account.sold_tokens += tokens as u128;
            let user_account = &mut ctx.accounts.user_account;
            user_account.user = ctx.accounts.user.key();
            user_account.total_tokens = tokens;
            user_account.lamports = lamports;
            user_account.usdt_token = 0;
        } else {
            let tokens = (lamports * tokens_per_usd) / (10u128).pow(usdt_decimals as u32);
            if
                (ctx.accounts.presale_account.stages[index as usize].sold_tokens + tokens) /
                    (10u128).pow(decimals as u32) > allocation
            {
                return Err(error!(ErrorCode::AllocationReached));
            }

            token::transfer(ctx.accounts.transfer_usdt_token(), from_lamports as u64)?;
            let presale_account = &mut ctx.accounts.presale_account;
            msg!("before {}", presale_account.stages[index as usize].sold_tokens);
            presale_account.stages[index as usize].sold_tokens += tokens;
            msg!(
                "after {} {} {} {} {} {}",
                presale_account.stages[index as usize].sold_tokens,
                now > stage_end_time || sold_tokens >= allocation,
                sold_tokens >= allocation,
                allocation,
                sold_tokens,
                (10u128).pow(decimals as u32)
            );
            presale_account.stages[index as usize].usdt_raised += lamports;

            presale_account.usdt_raised += lamports as u128;
            presale_account.sold_tokens += tokens as u128;
            let user_account = &mut ctx.accounts.user_account;
            user_account.user = ctx.accounts.user.key();
            user_account.total_tokens = tokens;
            user_account.lamports = 0;
            user_account.usdt_token = lamports;
            msg!("allocation end: {}", allocation);
        }
        // msg!("Price: {}", ctx.accounts.presale_account.token_price);
        msg!("Lamports: {}", lamports);

        Ok(())
    }
    pub fn existing_buy(ctx: Context<ExistingBuy>, lamports: u128, token_type: u8) -> Result<()> {
        let index = ctx.accounts.presale_account.active_stage;
        let stage_end_time = ctx.accounts.presale_account.stage_end_time;
        let from_lamports = lamports;
        let now = Clock::get().unwrap().unix_timestamp as u64;
        let usdt_decimals = ctx.accounts.presale_account.usdt_decimals;
        let decimals = ctx.accounts.presale_account.decimals;

        if now > stage_end_time {
            return Err(error!(ErrorCode::StageTimeUp));
        }
        if ctx.accounts.owner.key.ne(&ctx.accounts.presale_account.owner) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        if ctx.accounts.owner_usdt_account.key() != ctx.accounts.presale_account.owner_usdt_account {
            return Err(error!(ErrorCode::InvalidVault));
        }

        if now > stage_end_time {
            ctx.accounts.presale_account.active_stage += 1;
            ctx.accounts.presale_account.stage_end_time = now + TIME;
        }
        let tokens_per_usd =
            ctx.accounts.presale_account.stages[index as usize].price *
            (10u128).pow(decimals as u32);
        msg!("Lamports: {} {} {}", stage_end_time, now, ctx.accounts.presale_account.active_stage);
        let allocation = ctx.accounts.presale_account.stages[index as usize].allocation;
        let sold_tokens =
            ctx.accounts.presale_account.stages[index as usize].sold_tokens /
            (10u128).pow(decimals as u32);
        if token_type == 1 {
            let round = chainlink::latest_round_data(
                ctx.accounts.chainlink_program.to_account_info(),
                ctx.accounts.chainlink_feed.to_account_info()
            )?;

            // Calculate the number of Bebe tokens based on the fetched Solana price
            let sol_price = round.answer;
            // let sol_price = 150;
            msg!("sol_price: {}", sol_price);
            let tokens =
                (lamports * (sol_price as u128) * tokens_per_usd) / 100_000_000_000_000_000;
            msg!("Lamports: {} {}", tokens_per_usd, from_lamports);
            if
                (ctx.accounts.presale_account.stages[index as usize].sold_tokens + tokens) /
                    (10u128).pow(decimals as u32) > allocation
            {
                return Err(error!(ErrorCode::AllocationReached));
            }
            solana_program::program::invoke(
                &solana_program::system_instruction::transfer(
                    &ctx.accounts.user.key,
                    &ctx.accounts.owner.key,
                    from_lamports as u64
                ),
                &[
                    ctx.accounts.user.to_account_info().clone(),
                    ctx.accounts.owner.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ]
            )?;

            let presale_account = &mut ctx.accounts.presale_account;

            msg!("before {}", presale_account.stages[index as usize].sold_tokens);
            presale_account.stages[index as usize].sold_tokens += tokens;
            msg!(
                "after {} {} {}",
                presale_account.stages[index as usize].sold_tokens,
                now > stage_end_time || sold_tokens >= allocation,
                now > stage_end_time
            );
            presale_account.stages[index as usize].sol_raised += lamports;
            presale_account.sol_raised += lamports as u128;
            presale_account.sold_tokens += tokens as u128;
            let user_account = &mut ctx.accounts.user_account;
            user_account.user = ctx.accounts.user.key();
            user_account.total_tokens += tokens;
            user_account.lamports += lamports;
        } else {
            let tokens = (lamports * tokens_per_usd) / (10u128).pow(usdt_decimals as u32);
            if
                (ctx.accounts.presale_account.stages[index as usize].sold_tokens + tokens) /
                    (10u128).pow(decimals as u32) > allocation
            {
                return Err(error!(ErrorCode::AllocationReached));
            }

            msg!("Lamports: {}", tokens);
            token::transfer(ctx.accounts.transfer_usdt_token(), from_lamports as u64)?;
            let presale_account = &mut ctx.accounts.presale_account;
            msg!("before {}", presale_account.stages[index as usize].sold_tokens);
            presale_account.stages[index as usize].sold_tokens += tokens;
            msg!(
                "after {} {} {} {} {} {} {}",
                presale_account.stages[index as usize].sold_tokens,
                now > stage_end_time || sold_tokens >= allocation,
                sold_tokens >= allocation,
                allocation,
                sold_tokens,
                (10u128).pow(decimals as u32),
                decimals
            );
            presale_account.stages[index as usize].usdt_raised += lamports;

            presale_account.usdt_raised += lamports as u128;
            presale_account.sold_tokens += tokens as u128;
            let user_account = &mut ctx.accounts.user_account;
            user_account.total_tokens += tokens;
            user_account.usdt_token += lamports;
        }
        // msg!("Price: {}", ctx.accounts.presale_account.token_price);
        msg!("Lamports: {}", lamports);

        Ok(())
    }
    pub fn change_vesting(ctx: Context<ChangeVesting>, state: u8) -> Result<()> {
        let presale_account = &mut ctx.accounts.presale_account;
        if ctx.accounts.owner.key.ne(&presale_account.owner) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        presale_account.vesting = state;
        Ok(())
    }
    pub fn delete_presale(ctx: Context<DeletePresale>) -> Result<()> {
        if ctx.accounts.admin.key.ne(&ctx.accounts.presale_account.admin) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        ctx.accounts.presale_account.close(ctx.accounts.admin.to_account_info())?;
        Ok(())
    }
    pub fn change_stage(
        ctx: Context<ChangeStage>,
        active_stage: u8,
        stage_end_time: u64
    ) -> Result<()> {
        let presale_account = &mut ctx.accounts.presale_account;
        if ctx.accounts.admin.key.ne(&presale_account.admin) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        presale_account.active_stage = active_stage;
        presale_account.stage_end_time = stage_end_time;
        Ok(())
    }
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        if ctx.accounts.user_account.claimed_tokens >= ctx.accounts.user_account.total_tokens {
            return Err(error!(ErrorCode::InvalidClaimAmount));
        }
        if ctx.accounts.owner.key.ne(&ctx.accounts.presale_account.owner) {
            return Err(error!(ErrorCode::InvalidOwner));
        }
        if ctx.accounts.user.key.ne(&ctx.accounts.user_account.user) {
            return Err(error!(ErrorCode::InvalidAccountOwner));
        }
        if
            ctx.accounts.presale_token_vault.key() !=
            ctx.accounts.presale_account.presale_token_vault
        {
            return Err(error!(ErrorCode::InvalidVault));
        }
        if ctx.accounts.presale_account.vesting == 0 {
            return Err(error!(ErrorCode::ClaimTimeError));
        }
        let authority_seeds = &[&b"presale_authority"[..], &[ctx.accounts.presale_account.bump]];

        let claimable = ctx.accounts.user_account.total_tokens;
        if
            (ctx.accounts.presale_token_vault.amount as u128) <
            claimable / (10u128).pow(ctx.accounts.presale_account.decimals as u32)
        {
            return Err(error!(ErrorCode::NotEnoughAmount));
        }

        token::transfer(
            ctx.accounts.transfer_token().with_signer(&[&authority_seeds[..]]),
            claimable as u64
        )?;
        ctx.accounts.user_account.claimed_tokens = claimable;

        Ok(())
    }
}
#[derive(Accounts)]
pub struct DeletePresale<'info> {
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    pub admin: Signer<'info>,
}
#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(mut, seeds = [b"presale_authority"], bump = presale_account.bump)]
    pub presale_pda: AccountInfo<'info>,
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub presale_token_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(mut)]
    pub owner: AccountInfo<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
impl<'info> Claim<'info> {
    pub fn transfer_token(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.presale_token_vault.to_account_info().clone(),
            to: self.user_token_account.to_account_info().clone(),
            authority: self.presale_pda.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}
#[derive(Accounts)]
pub struct AddStages<'info> {
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    #[account(mut)]
    pub admin: Signer<'info>,
}
#[derive(Accounts)]
pub struct ChangeStage<'info> {
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    #[account(mut)]
    pub admin: Signer<'info>,
}
#[derive(Accounts)]
pub struct ChangeVesting<'info> {
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
}
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Stage {
    pub allocation: u128,
    pub price: u128,
    pub sold_tokens: u128,
    pub sol_raised: u128,
    pub usdt_raised: u128,
}

#[derive(Accounts)]
#[instruction(bump: u8,stages: Vec<Stage>)]
pub struct InitializePresale<'info> {
    #[account(init, payer = admin, space = 3000)]
    pub presale_account: Account<'info, PresaleAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(seeds = [b"presale_authority"], bump = bump)]
    pub presale_pda: AccountInfo<'info>,
    #[account(
        init,
        payer = admin,
        // Remove 'token::mint = mint' from the following line
        token::mint = mint,
        token::authority = presale_pda
    )]
    pub presale_token_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub owner_usdt_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK:
    pub owner: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(seeds = [b"presale_authority"], bump = presale_account.bump)]
    pub presale_pda: AccountInfo<'info>,

    #[account(mut)]
    pub presale_token_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub owner_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub mint: Account<'info, Mint>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> TransferTokens<'info> {
    pub fn transfer_token(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.owner_token_account.to_account_info().clone(),
            to: self.presale_token_vault.to_account_info().clone(),
            authority: self.owner.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(init, payer = user, space = 50 + 116)]
    pub user_account: Account<'info, UserAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(mut, seeds = [b"presale_authority"], bump = presale_account.bump)]
    pub presale_pda: AccountInfo<'info>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    pub chainlink_feed: AccountInfo<'info>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    pub chainlink_program: AccountInfo<'info>,
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    #[account(mut)]
    pub owner_usdt_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_usdt_account: Account<'info, TokenAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(mut)]
    pub owner: AccountInfo<'info>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
impl<'info> Buy<'info> {
    pub fn transfer_usdt_token(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_usdt_account.to_account_info().clone(),
            to: self.owner_usdt_account.to_account_info().clone(),
            authority: self.user.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}
#[derive(Accounts)]
pub struct ExistingBuy<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(mut, seeds = [b"presale_authority"], bump = presale_account.bump)]
    pub presale_pda: AccountInfo<'info>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    pub chainlink_feed: AccountInfo<'info>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    pub chainlink_program: AccountInfo<'info>,
    #[account(mut)]
    pub presale_account: Account<'info, PresaleAccount>,
    #[account(mut)]
    pub owner_usdt_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_usdt_account: Account<'info, TokenAccount>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(mut)]
    pub owner: AccountInfo<'info>,
    /// CHECK: No checks are necessary because the presale_pda account is created internally and guaranteed to exist
    #[account(mut)]
    pub user: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
impl<'info> ExistingBuy<'info> {
    pub fn transfer_usdt_token(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_usdt_account.to_account_info().clone(),
            to: self.owner_usdt_account.to_account_info().clone(),
            authority: self.user.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[account]
pub struct PresaleAccount {
    pub bump: u8,
    pub active_stage: u8,
    pub vesting: u8,
    pub stage_end_time: u64,
    pub owner: Pubkey,
    pub admin: Pubkey,
    pub presale_token_vault: Pubkey,
    pub owner_usdt_account: Pubkey,
    pub mint: Pubkey,
    pub usdt_mint: Pubkey,
    pub usdt_decimals: u8,
    pub decimals: u8,
    pub sold_tokens: u128,
    pub sol_raised: u128,
    pub usdt_raised: u128,

    pub stages: Vec<Stage>,
}
#[account]
pub struct UserAccount {
    user: Pubkey,
    total_tokens: u128,
    claimed_tokens: u128,
    lamports: u128,
    usdt_token: u128,
}
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid owner.")]
    InvalidOwner,
    #[msg("Invalid account owner.")]
    InvalidAccountOwner,
    #[msg("The vault given does not match the vault expected.")]
    InvalidVault,
    #[msg("You dont have enough tokens to claim!")]
    InvalidClaimAmount,
    #[msg("Claiming is not yet enabled. Please wait.")]
    NotEnoughAmount,
    #[msg("Claiming not started yet!")]
    ClaimTimeError,
    #[msg("The current stage time is up. Please wait for the next stage to start.")]
    StageTimeUp,
    #[msg("You can not buy more than the current stage allocated tokens!")]
    AllocationReached,
}
