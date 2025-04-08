use anchor_lang::prelude::*;
use crate::error::WusdError;  
use crate::state::{AuthorityState, PauseState};

/// 暂停合约
/// * `ctx` - 上下文
pub fn pause(ctx: Context<Pause>) -> Result<()> {
    require!(
        ctx.accounts.authority_state.is_admin(ctx.accounts.authority.key()),
        WusdError::Unauthorized
    );
    // 检查当前状态是否已经是暂停状态，避免不必要的状态更新
    if ctx.accounts.pause_state.paused {
        msg!("Contract is already paused");
        return Ok(());
    }
    ctx.accounts.pause_state.set_paused(true);
    Ok(())
}

/// 恢复合约
/// * `ctx` - 上下文
pub fn unpause(ctx: Context<Unpause>) -> Result<()> {
    require!(
        ctx.accounts.authority_state.is_admin(ctx.accounts.authority.key()),
        WusdError::Unauthorized
    );
    // 检查当前状态是否已经是非暂停状态，避免不必要的状态更新
    if !ctx.accounts.pause_state.paused {
        msg!("Contract is already unpaused");
        return Ok(());
    }
    ctx.accounts.pause_state.set_paused(false);
    Ok(())
}

#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        constraint = authority_state.is_admin(authority.key()) @ WusdError::Unauthorized,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,
    
    #[account(
        mut,
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump
    )]
    pub pause_state: Account<'info, PauseState>,
    
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unpause<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        constraint = authority_state.is_admin(authority.key()) @ WusdError::Unauthorized,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,
    
    #[account(
        mut,
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump
    )]
    pub pause_state: Account<'info, PauseState>,
    
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    pub system_program: Program<'info, System>,
}