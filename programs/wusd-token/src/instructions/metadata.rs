use anchor_lang::prelude::*;
use crate::error::WusdError;
use crate::state::{AuthorityState, MetadataState, PauseState};
use anchor_spl::token_interface::Mint;

/// 初始化代币元数据
pub fn initialize_metadata(
    ctx: Context<InitializeMetadata>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    // 验证参数
    require!(!name.is_empty(), WusdError::InvalidName);
    require!(!symbol.is_empty(), WusdError::InvalidSymbol);
    
    // 初始化元数据状态
    ctx.accounts.metadata_state.set_inner(MetadataState::initialize(
        name.clone(),
        symbol.clone(),
        uri.clone(),
        ctx.accounts.token_mint.key(),
        ctx.accounts.authority.key(),
        ctx.bumps.metadata_state
    ));
    
    // 发出元数据初始化事件
    emit!(MetadataInitializedEvent {
        authority: ctx.accounts.authority.key(),
        mint: ctx.accounts.token_mint.key(),
        name,
        symbol,
        uri,
    });
    
    Ok(())
}

/// 更新代币元数据
pub fn update_metadata(
    ctx: Context<UpdateMetadata>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    // 验证参数
    require!(!name.is_empty(), WusdError::InvalidAddress);
    require!(!symbol.is_empty(), WusdError::InvalidAddress);
    
    // 更新元数据状态
    ctx.accounts.metadata_state.update(name.clone(), symbol.clone(), uri.clone())?;
    
    // 发出元数据更新事件
    emit!(MetadataUpdatedEvent {
        authority: ctx.accounts.authority.key(),
        mint: ctx.accounts.token_mint.key(),
        name,
        symbol,
        uri,
    });
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(name: String, symbol: String, uri: String)]
pub struct InitializeMetadata<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = MetadataState::SIZE,
        seeds = [b"metadata", token_mint.key().as_ref()],
        bump
    )]
    pub metadata_state: Account<'info, MetadataState>,
    
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    #[account(
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_admin(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,
    
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String, symbol: String, uri: String)]
pub struct UpdateMetadata<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"metadata", token_mint.key().as_ref()],
        bump,
        constraint = metadata_state.update_authority == authority.key() @ WusdError::Unauthorized
    )]
    pub metadata_state: Account<'info, MetadataState>,
    
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    #[account(
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_admin(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,
    
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
    
    pub system_program: Program<'info, System>,
}

/// 元数据初始化事件
#[event]
pub struct MetadataInitializedEvent {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

/// 元数据更新事件
#[event]
pub struct MetadataUpdatedEvent {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}