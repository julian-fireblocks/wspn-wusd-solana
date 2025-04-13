use crate::error::WusdError;
use crate::state::{AuthorityState, MintState, PauseState,FreezeState};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_2022::{self, burn as token_burn};

/// 销毁WUSD代币
/// * `ctx` - 销毁上下文
/// * `amount` - 销毁数量
/// * `bump` - PDA的bump值
pub fn burn(ctx: Context<Burn>, amount: u64, bump: u8) -> Result<()> {
    // 验证金额有效性
    require!(amount > 0, WusdError::InvalidAmount);
    // 验证余额充足
    require!(
        ctx.accounts.token_account.amount >= amount,
        WusdError::InsufficientBalance
    ); 

    // 执行销毁操作
    let mint_key = ctx.accounts.mint.key();
    let seeds = &[b"authority", mint_key.as_ref(), &[bump]];

    // 确保使用正确的CPI上下文和签名者
    let cpi_accounts = token_2022::Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.authority_state.to_account_info(),
    };
    
    token_burn(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &[seeds],
        ),
        amount,
    )?;

    emit!(BurnEvent {
        burner: ctx.accounts.authority.key(),
        amount
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, bump: u8)]
pub struct Burn<'info> { 
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"authority", mint.key().as_ref()],
        bump,
        constraint = authority_state.is_burner(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>, 
    #[account(mut)]
    pub mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        mut,
        constraint = token_account.mint == mint.key() @ WusdError::InvalidMint,
        constraint = token_account.owner == authority.key() @ WusdError::Unauthorized
    )]
    pub token_account: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
    pub token_program: Program<'info, Token2022>,
    pub mint_state: Account<'info, MintState>,
    #[account(
        seeds = [b"pause_state", mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
    #[account(
        mut,
        seeds = [b"freeze", token_account.key().as_ref(), mint.key().as_ref()],
        bump,
        constraint = !freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub freeze_state: Box<Account<'info, FreezeState>>,
    pub system_program: Program<'info, System>,
}

/// 销毁事件，记录代币销毁的详细信息
#[event]
pub struct BurnEvent {
    /// 销毁者地址，执行销毁操作的账户
    pub burner: Pubkey,
    /// 销毁数量，被销毁的代币数量
    pub amount: u64,
}

