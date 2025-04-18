use crate::error::WusdError;
use crate::state::{AuthorityState, FreezeState, PauseState};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, freeze_account, thaw_account};
use anchor_spl::token_interface::{Token2022, TokenAccount, Mint};

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
pub fn handle_freeze_operation(
    ctx: Context<FreezeOperationAccounts>,
    operation: FreezeOperation,
) -> Result<()> {
    let authority_state = &ctx.accounts.authority_state;
    let token_mint = &ctx.accounts.token_mint;
    
    // 生成权限PDA签名
    let mint_key = token_mint.key();
    let seeds = &[
        b"authority",
        mint_key.as_ref(),
        &[ctx.bumps.authority_state],
    ];
    
    match operation {
        FreezeOperation::Freeze => {
            // 验证账户未被冻结
            require!(!ctx.accounts.freeze_state.is_frozen, WusdError::AccountAlreadyFrozen);

            // 执行代币账户冻结
            freeze_account(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token_2022::FreezeAccount {
                        account: ctx.accounts.token_account.to_account_info(),
                        mint: token_mint.to_account_info(),
                        authority: authority_state.to_account_info(),
                    },
                    &[seeds],
                ),
            )?;

            // 更新冻结状态
            ctx.accounts.freeze_state.freeze();

            emit!(FreezeAccountEvent {
                authority: ctx.accounts.authority.key(),
                token_account: ctx.accounts.token_account.key(),
                timestamp: Clock::get()?.unix_timestamp,
            });
        }
        FreezeOperation::Unfreeze => {
            // 验证账户已被冻结
            require!(ctx.accounts.freeze_state.is_frozen, WusdError::AccountNotFrozen);

            // 解冻代币账户
            thaw_account(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token_2022::ThawAccount {
                        account: ctx.accounts.token_account.to_account_info(),
                        mint: token_mint.to_account_info(),
                        authority: authority_state.to_account_info(),
                    },
                    &[seeds],
                ),
            )?;

            // 更新冻结状态
            ctx.accounts.freeze_state.unfreeze();

            emit!(UnfreezeAccountEvent {
                authority: ctx.accounts.authority.key(),
                token_account: ctx.accounts.token_account.key(),
                timestamp: Clock::get()?.unix_timestamp,
            });
        }
    }
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeFreezeState<'info> {
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = FreezeState::SIZE,
        seeds = [b"freeze", token_account.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub freeze_state: Account<'info, FreezeState>,
    /// CHECK: Token account being frozen/unfrozen
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        constraint = token_mint.key() == token_account.mint @ WusdError::InvalidMint
    )]
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_admin(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,
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
        seeds = [b"freeze", token_account.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub freeze_state: Account<'info, FreezeState>,
    
    #[account(mut)]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_freezer(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,
    
    pub token_mint: InterfaceAccount<'info, Mint>,
    
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
pub struct FreezeAccountEvent {
    pub authority: Pubkey,
    pub token_account: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct UnfreezeAccountEvent {
    pub authority: Pubkey,
    pub token_account: Pubkey,
    pub timestamp: i64,
}


