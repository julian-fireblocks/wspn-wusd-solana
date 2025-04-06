use crate::error::WusdError;
use crate::state::{AccessRegistryState, FreezeState, MintState, PauseState, PermitState};
use crate::access::AccessLevel;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, transfer_checked, Token2022};
use anchor_spl::token_interface::TokenAccount;

/// 转账WUSD代币
/// * `ctx` - 转账上下文
/// * `amount` - 转账数量
pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
    // 验证系统未被暂停
    ctx.accounts.pause_state.validate_not_paused()?;
    require!(amount > 0, WusdError::InvalidAmount);
    
    // 检查冻结状态 - 通过账户约束已经验证
    // 验证from_token和to_token的所有者 - 通过账户约束已经验证
    
    // 验证访问权限
    require!(
        ctx.accounts.access_registry.has_access(
            ctx.accounts.from.key(),
            AccessLevel::Debit
        ),
        WusdError::AccessDenied
    );

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
        ctx.accounts.token_mint.decimals, // 使用token_mint中的小数位数
    )?;

    // 发送转账事件 - 直接使用i64::now()代替Clock::get()?.unix_timestamp减少栈使用
    emit!(TransferEvent {
        from_bytes: ctx.accounts.from.key().to_bytes(),
        to_bytes: ctx.accounts.to.key().to_bytes(),
        amount,
        fee: 0, 
        timestamp: Clock::get()?.unix_timestamp,
        has_memo: false,
        spender_bytes: [0; 32], // 使用全0字节数组表示没有spender
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

    // 验证 token account 所有权 - 通过账户约束已经验证
    
    // 验证系统未暂停状态
    ctx.accounts.pause_state.validate_not_paused()?;

    // 验证金额大于0
    require!(amount > 0, WusdError::InvalidAmount);

    // 检查冻结状态 - 通过账户约束已经验证
    
    // 验证访问权限
    require!(
        ctx.accounts.access_registry.has_access(
            ctx.accounts.spender.key(),
            AccessLevel::Debit
        ),
        WusdError::AccessDenied
    );

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
        
    // 发送转账事件 - 直接使用Clock::get()减少栈使用
    // 直接在TransferEvent初始化时调用to_bytes()，避免创建局部变量
    emit!(TransferEvent {
        from_bytes: ctx.accounts.owner.key().to_bytes(),
        to_bytes: ctx.accounts.to_token.owner.to_bytes(),
        amount,
        fee: 0, 
        timestamp: Clock::get()?.unix_timestamp,
        has_memo: true,
        spender_bytes: ctx.accounts.spender.key().to_bytes(),
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
    pub from_token: Box<InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>>,
    #[account(mut)]
    pub to_token: Box<InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>>,
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
    #[account(mut)]
    pub mint_state: Box<Account<'info, MintState>>,
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
    #[account(
        seeds = [b"access_registry"],
        bump,
        constraint = access_registry.initialized @ WusdError::AccessRegistryNotInitialized
    )]
    pub access_registry: Account<'info, AccessRegistryState>,
    pub token_program: Program<'info, Token2022>,
    #[account(mut)]
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
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        seeds = [b"pause_state", from_token.mint.as_ref()],
        bump,
    )]
    pub pause_state: Account<'info, PauseState>,
    #[account(
        seeds = [b"access_registry"],
        bump,
    )]
    pub access_registry: Account<'info, AccessRegistryState>,
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
}

#[event]
pub struct TransferEvent {
    // 使用字节数组代替Pubkey，减少栈使用
    pub from_bytes: [u8; 32],
    pub to_bytes: [u8; 32],
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    // 使用布尔值代替字符串，减少栈使用
    pub has_memo: bool,
    // 使用[u8; 32]代替Option<Pubkey>，减少栈使用
    // 当不需要spender时，使用[0; 32]表示
    pub spender_bytes: [u8; 32],
}
