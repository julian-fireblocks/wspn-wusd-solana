use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, approve};
use anchor_spl::token_interface::{Mint, TokenAccount};
use crate::error::WusdError;  
use crate::state::{MintState, PermitState, PauseState}; 
/// 处理代表津贴请求，允许代币持有者授权其他账户使用其代币 
/// 
/// # 参数
/// * `ctx` - 包含所有必要账户的上下文
/// * `amount` - 授权金额
/// * `expiry_time` - 授权过期时间（Unix时间戳）
/// 
/// # 返回值
/// * `Result<()>` - 操作成功返回Ok(()), 失败返回错误
pub fn approve_set(ctx: Context<Approve>, amount: u64, expiry_time: i64) -> Result<()> { 
    // 验证基本参数
    require!(amount > 0, WusdError::InvalidAmount);
    
    // 验证过期时间有效性
    let current_time = Clock::get()?.unix_timestamp;
    require!(expiry_time > current_time, WusdError::ExpiredPermit);
    
    // 验证mint状态
    require!(
        ctx.accounts.mint_state.mint == ctx.accounts.token_mint.key(),
        WusdError::InvalidMint
    );
    
    // 使用token-2022的approve指令授权代表
    approve(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::Approve {
                to: ctx.accounts.token_account.to_account_info(),
                delegate: ctx.accounts.delegate.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;
    
    // 初始化 permit_state 用于兼容现有的transfer_from功能
    ctx.accounts.permit_state.set_inner(PermitState::initialize(
        ctx.accounts.owner.key(),
        ctx.accounts.delegate.key(),
        amount,
        expiry_time,
        ctx.bumps.permit_state
    ));
    
    // 发出授权代表事件
    emit!(ApproveSetEvent { 
        delegator: ctx.accounts.owner.key(),
        delegate: ctx.accounts.delegate.key(),
        amount,
        authority_type: 0 // 0 表示 AuthorityType::AccountOwner
    });
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, expiry_time: i64)]
pub struct Approve<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: This is the delegate account that will be granted permission
    pub delegate: AccountInfo<'info>,

    #[account(
        mut,
        constraint = token_account.owner == owner.key() @ WusdError::InvalidOwner,
        constraint = token_account.mint == token_mint.key() @ WusdError::InvalidMint
    )]
    pub token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = owner,
        space = PermitState::SIZE,
        seeds = [
            b"permit",
            owner.key().as_ref(),
            delegate.key().as_ref()
        ],
        bump,
    )]
    pub permit_state: Account<'info, PermitState>,

    pub token_mint: InterfaceAccount<'info, Mint>,
    pub mint_state: Box<Account<'info, MintState>>,

    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>
} 

/// 代表津贴设置事件，记录代表津贴授权信息
#[event]
pub struct ApproveSetEvent {
    /// 代币所有者地址
    pub delegator: Pubkey,
    /// 被授权者地址
    pub delegate: Pubkey,
    /// 授权金额
    pub amount: u64,
    /// 授权类型（使用u8代替AuthorityType以支持序列化）
    /// 0 = AccountOwner, 1 = MintTokens, 2 = FreezeAccount, 3 = CloseAccount, 
    /// 4 = TransferFeeConfig, 5 = WithheldWithdraw, 6 = ConfidentialTransferMint,
    ///  7 = TransferHookProgramId
    pub authority_type: u8,
}