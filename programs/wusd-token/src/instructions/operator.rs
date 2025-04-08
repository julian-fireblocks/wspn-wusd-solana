use anchor_lang::prelude::*; 
use crate::error::WusdError;  
use crate::state::{AuthorityState, AccessRegistryState};
 
/// 添加操作员
pub fn add_operator(ctx: Context<ManageOperator>, operator: Pubkey) -> Result<()> {
    let access_registry = &mut ctx.accounts.access_registry;
    require!(access_registry.initialized, WusdError::AccessRegistryNotInitialized);
    
    // 添加操作员
    access_registry.add_operator(operator) 
}  

/// 移除操作员
pub fn remove_operator(ctx: Context<ManageOperator>, operator: Pubkey) -> Result<()> {
    let access_registry = &mut ctx.accounts.access_registry;
    require!(access_registry.initialized, WusdError::AccessRegistryNotInitialized); 
    // 移除操作员
    access_registry.remove_operator(operator)
}

#[derive(Accounts)]
pub struct ManageOperator<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        constraint = authority_state.is_admin(authority.key()) @ WusdError::Unauthorized,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// CHECK: 仅用于记录地址
    pub operator: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"access_registry"],
        bump,
        constraint = access_registry.initialized @ WusdError::AccessRegistryNotInitialized
    )]
    pub access_registry: Account<'info, AccessRegistryState>,

    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    pub system_program: Program<'info, System>,
}