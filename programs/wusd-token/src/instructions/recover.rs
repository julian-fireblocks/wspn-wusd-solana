use crate::error::WusdError;
use crate::state::{AuthorityState, FreezeState, PauseState};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, transfer_checked};
use anchor_spl::token_interface::{Token2022, TokenAccount};

/// 从被冻结的账户中回收资产
pub fn recover_frozen_assets(ctx: Context<RecoverAssets>, amount: u64) -> Result<()> {
    // 验证金额大于0
    require!(amount > 0, WusdError::InvalidAmount);  

    // 验证冻结账户余额充足
    require!(
        ctx.accounts.frozen_token.amount >= amount,
        WusdError::InsufficientBalance
    );
    
    // 从冻结账户转移资产到freezer账户  
    let token_mint_key = ctx.accounts.token_mint.key(); 
    let seeds = &[b"authority", token_mint_key.as_ref(), &[ctx.bumps.authority_state]];
    let signer = &[&seeds[..]];  
    
    // 执行转账 - 使用PDA作为权限，无需被冻结账户持有者授权
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token_2022::TransferChecked {
                from: ctx.accounts.frozen_token.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.freezer_token.to_account_info(),
                authority: ctx.accounts.authority_state.to_account_info(),
            },
            signer,
        ),
        amount,
        ctx.accounts.token_mint.decimals,
    )?; 

    // 发出资产回收事件
    emit!(RecoverFrozenAssetsEvent {
        authority: ctx.accounts.authority.key(),
        frozen_account: ctx.accounts.frozen_token.key(),
        freezer_account: ctx.accounts.freezer_token.key(),
        amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct RecoverAssets<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        constraint = authority_state.is_freezer(authority.key()) @ WusdError::Unauthorized,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump 
    )]
    pub authority_state: Account<'info, AuthorityState>, 
    #[account(
        mut,
        seeds = [b"freeze", frozen_token.key().as_ref(), token_mint.key().as_ref()],
        bump,
        constraint = freeze_state.is_frozen @ WusdError::AccountNotFrozen
    )]
    pub freeze_state: Account<'info, FreezeState>,
    #[account(
        mut,
        constraint = frozen_token.mint == token_mint.key() @ WusdError::InvalidMint
    )]
    pub frozen_token: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        constraint = freezer_token.mint == token_mint.key() @ WusdError::InvalidMint
    )]
    pub freezer_token: InterfaceAccount<'info, TokenAccount>,
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct RecoverFrozenAssetsEvent {
    pub authority: Pubkey,
    pub frozen_account: Pubkey,
    pub freezer_account: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}