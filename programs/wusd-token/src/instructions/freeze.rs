use crate::error::WusdError;
use crate::state::{AuthorityState, FreezeState};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, transfer_checked};
use anchor_spl::token_interface::{Token2022, TokenAccount};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum FreezeOperation {
    Freeze,
    Unfreeze,
}
pub fn initialize_freeze_state(ctx: Context<InitializeFreezeState>) -> Result<()> {
    ctx.accounts.freeze_state.is_frozen = false;
    Ok(())
}
/// 统一处理冻结/解冻账户操作
pub fn handle_freeze_operation(ctx: Context<FreezeOperationAccounts>, operation: FreezeOperation) -> Result<()> {
    match operation {
        FreezeOperation::Freeze => {
            // 验证账户未被冻结
            require!(
                !ctx.accounts.freeze_state.is_frozen,
                WusdError::AccountAlreadyFrozen
            );

            // 冻结账户
            ctx.accounts.freeze_state.freeze()?;

            // 发出冻结事件
            emit!(FreezeAccountEvent {
                authority: ctx.accounts.authority.key(),
                freeze_state: ctx.accounts.freeze_state.key(),
                timestamp: Clock::get()?.unix_timestamp,
            });
        },
        FreezeOperation::Unfreeze => {
            // 验证账户已被冻结
            require!(
                ctx.accounts.freeze_state.is_frozen,
                WusdError::AccountNotFrozen
            );

            // 解冻账户
            ctx.accounts.freeze_state.unfreeze();

            // 发出解冻事件
            emit!(UnfreezeAccountEvent {
                authority: ctx.accounts.authority.key(),
                freeze_state: ctx.accounts.freeze_state.key(),
                timestamp: Clock::get()?.unix_timestamp,
            });
        }
    }

    Ok(())
} 

/// 从被冻结的账户中回收资产
pub fn recover_frozen_assets(ctx: Context<RecoverFrozenAssets>, amount: u64) -> Result<()> {
    // 验证账户已被冻结
    require!(
        ctx.accounts.freeze_state.is_frozen,
        WusdError::AccountNotFrozen
    );

    // 验证金额大于0
    require!(amount > 0, WusdError::InvalidAmount);

    // 执行转账
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::TransferChecked {
                from: ctx.accounts.frozen_token.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.freezer_token.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
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
pub struct InitializeFreezeState<'info> {
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = FreezeState::SIZE,
        seeds = [b"freeze", token_account.key().as_ref()],
        bump
    )]
    pub freeze_state: Account<'info, FreezeState>,
    /// CHECK: Token account being frozen/unfrozen
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

/// 操作员管理账户结构体
#[derive(Accounts)]
pub struct FreezeOperationAccounts<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init_if_needed,
        payer = authority,
        space = FreezeState::SIZE,
        seeds = [b"freeze", account.key().as_ref()],
        bump
    )]
    pub freeze_state: Account<'info, FreezeState>, 
   
    /// CHECK: 这个账户仅用于生成PDA种子
    pub account: AccountInfo<'info>,

    #[account(
        constraint = authority_state.is_freezer(authority.key()) @ WusdError::Unauthorized,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RecoverFrozenAssets<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"freeze", frozen_token.key().as_ref()],
        bump,
        constraint = freeze_state.is_frozen @ WusdError::AccountNotFrozen
    )]
    pub freeze_state: Account<'info, FreezeState>,

    #[account(
        mut,
        constraint = authority_state.is_freezer(authority.key()) @ WusdError::Unauthorized
    )]
    pub frozen_token: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub freezer_token: InterfaceAccount<'info, TokenAccount>,

    #[account(
        constraint = authority_state.is_freezer(authority.key()) @ WusdError::Unauthorized,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct FreezeAccountEvent {
    pub authority: Pubkey,
    pub freeze_state: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct UnfreezeAccountEvent {
    pub authority: Pubkey,
    pub freeze_state: Pubkey,
    pub timestamp: i64,
} 

#[event]
pub struct RecoverFrozenAssetsEvent {
    pub authority: Pubkey,
    pub frozen_account: Pubkey,
    pub freezer_account: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}
