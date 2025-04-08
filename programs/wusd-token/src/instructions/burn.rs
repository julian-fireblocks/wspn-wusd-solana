use crate::error::WusdError;
use crate::state::{AuthorityState, FreezeState, MintState, PauseState};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_2022::{self, burn as token_burn};

/// 销毁WUSD代币
/// * `ctx` - 销毁上下文
/// * `amount` - 销毁数量
pub fn burn(ctx: Context<Burn>, amount: u64) -> Result<()> {
    // 验证合约未暂停
    ctx.accounts.pause_state.validate_not_paused()?;
    // 验证金额有效性
    require!(amount > 0, WusdError::InvalidAmount);
    // 验证余额充足
    require!(
        ctx.accounts.token_account.amount >= amount,
        WusdError::InsufficientBalance
    ); 

    // 执行销毁操作
    token_burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.token_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
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
pub struct Burn<'info> {
    #[account(
        mut,
        seeds = [b"authority", mint.key().as_ref()],
        bump,
        constraint = authority_state.is_burner(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        mut,
        constraint = token_account.mint == mint.key() @ WusdError::InvalidMint
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
        seeds = [b"freeze", token_account.key().as_ref()],
        bump,
        constraint = !freeze_state.is_frozen @ WusdError::AccountFrozen
    )]
    pub freeze_state: Account<'info, FreezeState>,
}

/// 销毁事件，记录代币销毁的详细信息
#[event]
pub struct BurnEvent {
    /// 销毁者地址，执行销毁操作的账户
    pub burner: Pubkey,
    /// 销毁数量，被销毁的代币数量
    pub amount: u64,
}
