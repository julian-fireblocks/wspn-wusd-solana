use anchor_lang::prelude::*;
use crate::error::WusdError;   
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_2022::{self, mint_to};
use crate::utils::require_has_access;
use crate::state::{AuthorityState, MintState, PauseState, AccessRegistryState};

pub fn mint(ctx: Context<MintAccounts>, amount: u64, bump: u8) -> Result<()> {
    // 验证Minter权限 
    require!(
        ctx.accounts.authority_state.is_minter(ctx.accounts.authority.key()), 
        WusdError::NotMinter
    );
    // 验证访问权限
    require_has_access(
        ctx.accounts.authority.key(),
        false,
        Some(amount),
        &ctx.accounts.pause_state,
        Some(&ctx.accounts.access_registry),
    )?;

    // 验证mint权限是否正确设置为authority_state PDA
    require!(
        ctx.accounts.token_mint.mint_authority.contains(&ctx.accounts.authority_state.key()),
        WusdError::Unauthorized
    );

    // 执行铸币
    let mint_key = ctx.accounts.token_mint.key();
    let seeds = &[b"authority", mint_key.as_ref(), &[bump]];
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token_2022::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.token_account.to_account_info(),
                authority: ctx.accounts.authority_state.to_account_info(),
            },
            &[&seeds[..]],
        ),
        amount
    )?;

    // 发出铸币事件
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
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(mut)]
    pub token_account: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
    pub token_program: Program<'info, Token2022>,
    #[account(
        mut,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_minter(authority.key()) @ WusdError::NotMinter
    )]
    pub authority_state: Account<'info, AuthorityState>,
    #[account(
        mut,
        seeds = [b"mint_state", token_mint.key().as_ref()],
        bump
    )]
    pub mint_state: Account<'info, MintState>,
    #[account(
        mut,
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