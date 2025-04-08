use crate::error::WusdError;
use crate::state::{AuthorityState, FreezeState, MintState, PauseState, PermitState};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, transfer_checked, Token2022};
use anchor_spl::token_interface::TokenAccount;

/// 转账WUSD代币
/// * `ctx` - 转账上下文
/// * `amount` - 转账数量
pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
    require!(amount > 0, WusdError::InvalidAmount);

    // 执行转账
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::TransferChecked {
                from: ctx.accounts.from_token.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.to_token.to_account_info(),
                authority: ctx.accounts.from.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.token_mint.decimals,
    )?;

    // 发送转账事件 - 使用更简洁的方式处理Pubkey，减少栈使用
    emit!(TransferEvent {
        from: ctx.accounts.from.key(),
        to: ctx.accounts.to.key(),
        amount,
        fee: 0,
        timestamp: Clock::get()?.unix_timestamp,
        has_memo: false,
        spender: None, // 使用Option<Pubkey>代替字节数组
    });

    Ok(())
}

pub fn transfer_from(ctx: Context<TransferFrom>, amount: u64) -> Result<()> {
    // 验证授权有效性
    let current_time = Clock::get()?.unix_timestamp;
    require!(
        ctx.accounts.permit.expiration > current_time,
        WusdError::ExpiredPermit
    );
    require!(
        ctx.accounts.permit.amount >= amount,
        WusdError::InsufficientAllowance
    );

    // 验证金额大于0
    require!(amount > 0, WusdError::InvalidAmount);

    // 直接使用spender作为authority执行转账
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::TransferChecked {
                from: ctx.accounts.from_token.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.to_token.to_account_info(),
                authority: ctx.accounts.spender.to_account_info(), // 使用spender作为authority
            },
        ),
        amount,
        ctx.accounts.token_mint.decimals, // 使用token_mint中的小数位数
    )?;

    // 更新授权额度
    ctx.accounts.permit.amount = ctx
        .accounts
        .permit
        .amount
        .checked_sub(amount)
        .ok_or(WusdError::InsufficientAllowance)?;

    // 发送转账事件 - 使用更简洁的方式处理Pubkey，减少栈使用
    emit!(TransferEvent {
        from: ctx.accounts.owner.key(),
        to: ctx.accounts.to_token.owner,
        amount,
        fee: 0,
        timestamp: Clock::get()?.unix_timestamp,
        has_memo: true,
        spender: Some(ctx.accounts.spender.key()),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct TransferFrom<'info> {
    #[account(mut)]
    pub spender: Signer<'info>,
    /// CHECK: This account is not read or written to
    #[account(mut)]
    pub owner: AccountInfo<'info>,
    #[account(
        mut,
        constraint = from_token.owner == owner.key() @ WusdError::InvalidOwner,
        constraint = from_token.mint == to_token.mint @ WusdError::InvalidMint
    )]
    pub from_token: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
    #[account(mut)]
    pub to_token: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
    #[account(
        seeds = [
            b"permit",
            owner.key().as_ref(),
            spender.key().as_ref()
        ],
        bump = permit.bump,
        has_one = owner,
        has_one = spender,
    )]
    pub permit: Account<'info, PermitState>,
    pub mint_state: Box<Account<'info, MintState>>,
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
    pub token_program: Program<'info, Token2022>,
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        seeds = [b"freeze", from_token.key().as_ref()],
        bump,
        constraint = !from_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub from_freeze_state: Account<'info, FreezeState>,
    #[account(
        seeds = [b"freeze", to_token.key().as_ref()],
        bump,
        constraint = !to_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub to_freeze_state: Account<'info, FreezeState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    /// CHECK: This account is not read or written to
    #[account(mut)]
    pub to: AccountInfo<'info>,
    #[account(
        mut,
        constraint = from_token.owner == from.key() @ WusdError::InvalidOwner,
        constraint = from_token.mint == to_token.mint @ WusdError::InvalidMint
    )]
    pub from_token: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = to_token.owner == to.key() @ WusdError::InvalidOwner
    )]
    pub to_token: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Program<'info, Token2022>,
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Box<Account<'info, AuthorityState>>,
    #[account(
        seeds = [b"pause_state", from_token.mint.as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Box<Account<'info, PauseState>>,
    #[account(
        seeds = [b"freeze", from_token.key().as_ref()],
        bump,
        constraint = !from_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub from_freeze_state: Box<Account<'info, FreezeState>>,
    #[account(
        seeds = [b"freeze", to_token.key().as_ref()],
        bump,
        constraint = !to_freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub to_freeze_state: Box<Account<'info, FreezeState>>,
}

#[event]
pub struct TransferEvent {
    // 直接使用Pubkey类型，避免不必要的转换
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    // 使用布尔值代替字符串，减少栈使用
    pub has_memo: bool,
    // 使用Option<Pubkey>代替字节数组，更符合Rust习惯且减少栈使用
    pub spender: Option<Pubkey>,
}
