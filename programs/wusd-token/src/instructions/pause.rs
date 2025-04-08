use anchor_lang::prelude::*;
use crate::error::WusdError;  
use crate::state::{AuthorityState, PauseState};

/// 合约暂停事件，记录合约暂停状态变更
#[event]
pub struct ContractPausedEvent {
    /// 执行暂停操作的账户地址
    pub authority: Pubkey,
}

/// 合约恢复事件，记录合约恢复状态变更
#[event]
pub struct ContractUnpausedEvent {
    /// 执行恢复操作的账户地址
    pub authority: Pubkey,
}

/// 暂停合约
/// * `ctx` - 上下文
pub fn pause(ctx: Context<Pause>) -> Result<()> { 
    // 检查当前状态是否已经是暂停状态，避免不必要的状态更新
    if ctx.accounts.pause_state.paused {
        msg!("Contract is already paused");
        return Ok(());
    }
    ctx.accounts.pause_state.set_paused(true);
    
    // 发出合约暂停事件
    emit!(ContractPausedEvent {
        authority: ctx.accounts.authority.key(),
    });
    
    Ok(())
}

/// 恢复合约
/// * `ctx` - 上下文
pub fn unpause(ctx: Context<Unpause>) -> Result<()> { 
    // 检查当前状态是否已经是非暂停状态，避免不必要的状态更新
    if !ctx.accounts.pause_state.paused {
        msg!("Contract is already unpaused");
        return Ok(());
    }
    ctx.accounts.pause_state.set_paused(false);
    
    // 发出合约恢复事件
    emit!(ContractUnpausedEvent {
        authority: ctx.accounts.authority.key(),
    });
    
    Ok(())
}

#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        constraint = authority_state.is_pauser(authority.key()) @ WusdError::Unauthorized,
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
        constraint = authority_state.is_pauser(authority.key()) @ WusdError::Unauthorized,
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