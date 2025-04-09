use crate::error::WusdError;
use crate::state::{AuthorityState, FreezeState, MintState, PauseState};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_2022::{self, mint_to};

pub fn mint(ctx: Context<MintAccounts>, amount: u64, bump: u8) -> Result<()> {
    // 验证金额有效性
    require!(amount > 0, WusdError::InvalidAmount);

    // 执行铸币 - 极简化CPI调用
    let mint_key = ctx.accounts.token_mint.key();
    let seeds = &[b"authority", mint_key.as_ref(), &[bump]];

    let cpi_accounts = token_2022::MintTo {
        mint: ctx.accounts.token_mint.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.authority_state.to_account_info(),
    };

    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &[seeds],
        ),
        amount,
    )?;

    // 发出铸币事件 - 简化事件参数
    emit!(MintEvent {
        minter: ctx.accounts.authority.key(),
        recipient: ctx.accounts.token_account.owner,
        amount
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, bump: u8)]
pub struct MintAccounts<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        mut,
        constraint = !freeze_state.is_frozen  @ WusdError::AccountFrozen,
        seeds = [b"freeze", token_account.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub freeze_state: Box<Account<'info, FreezeState>>,
    #[account(mut)]
    pub token_account: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
    pub token_program: Program<'info, Token2022>,
    #[account(
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_minter(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,
    #[account(
        seeds = [b"mint_state", token_mint.key().as_ref()],
        bump
    )]
    pub mint_state: Account<'info, MintState>,
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Box<Account<'info, PauseState>>,
    pub system_program: Program<'info, System>,
}

/// 铸币事件，记录代币铸造的详细信息
#[event]
pub struct MintEvent {
    /// 铸币者地址，执行铸币操作的账户
    pub minter: Pubkey,
    /// 接收者地址，接收铸造代币的账户
    pub recipient: Pubkey,
    /// 铸造数量，被铸造的代币数量
    pub amount: u64,
}
