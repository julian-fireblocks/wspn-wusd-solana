//! WUSD Token 程序  
mod error;
mod state; 
mod utils;
mod access;
mod instructions; 

use crate::error::WusdError;
use anchor_lang::prelude::*;
use anchor_spl::token_2022; 
use anchor_spl::token_interface::Mint;
use anchor_spl::token_2022::ID as TOKEN_PROGRAM_ID;
use spl_token_2022::instruction::AuthorityType;    

use state::{AuthorityState, MintState, PauseState, AccessRegistryState};

use instructions::mint::*; 
use instructions::burn::*;
use instructions::transfer::*;
use instructions::permit::*;
use instructions::operator::*;
use instructions::pause::*;
use instructions::freeze::*; 

declare_id!("8nBbkdsTkqbrnrbVTUxyciQNvT6Q5B3pZkPQmP3nnuwU");

#[program]
pub mod wusd_token {
    use super::*; 
    pub fn initialize_access_registry(ctx: Context<InitializeAccessRegistry>) -> Result<()> {
        // 确保访问注册表尚未初始化
        require!(!ctx.accounts.access_registry.initialized, WusdError::Unauthorized);
        
        let access_registry = &mut ctx.accounts.access_registry;
        access_registry.authority = ctx.accounts.authority.key();
        access_registry.operator_count = 0;
        access_registry.operators = [Pubkey::default(); 10];
        access_registry.initialized = true;
        
        // 发出初始化事件
        emit!(AccessRegistryInitializeEvent {
            authority: ctx.accounts.authority.key()
        });
        
        Ok(())
    }

    pub fn initialize(ctx: Context<Initialize>, decimals: u8) -> Result<()> {
        msg!("Starting initialization...");
        msg!("Authority: {}", ctx.accounts.authority.key());
        msg!("Mint: {}", ctx.accounts.token_mint.key());

        // 验证访问注册表已初始化
        require!(ctx.accounts.access_registry.initialized, WusdError::AccessRegistryNotInitialized);
        
        // 验证调用者是否有权限初始化
        require!(
            ctx.accounts.access_registry.authority == ctx.accounts.authority.key() || 
            ctx.accounts.access_registry.has_access(ctx.accounts.authority.key(), crate::access::AccessLevel::Debit),
            WusdError::Unauthorized
        );

        // 1. 初始化状态账户
        let authority_state = &mut ctx.accounts.authority_state;
        authority_state.admin = ctx.accounts.authority.key();
        authority_state.minter = ctx.accounts.authority.key();
        authority_state.pauser = ctx.accounts.authority.key();

        let mint_state = &mut ctx.accounts.mint_state;
        mint_state.mint = ctx.accounts.token_mint.key();
        mint_state.decimals = decimals;

        let pause_state = &mut ctx.accounts.pause_state;
        pause_state.paused = false;

        // 2. 转移mint的authority给authority_state PDA
        let mint_key = ctx.accounts.token_mint.key();
        let seeds = &[b"authority", mint_key.as_ref()]; 
        let (_authority_pda, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
        let _bump_bytes = &[bump]; 
        token_2022::set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_2022::SetAuthority {
                    current_authority: ctx.accounts.authority.to_account_info(),
                    account_or_mint: ctx.accounts.token_mint.to_account_info(),
                }
            ),
            AuthorityType::MintTokens,
            Some(ctx.accounts.authority_state.key()),
        )?;
        
        // 转移freeze_account权限给authority_state PDA
        token_2022::set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_2022::SetAuthority {
                    current_authority: ctx.accounts.authority.to_account_info(),
                    account_or_mint: ctx.accounts.token_mint.to_account_info(),
                }
            ),
            AuthorityType::FreezeAccount,
            Some(ctx.accounts.authority_state.key()),
        )?;
        
        // 转移freeze_account权限给authority_state PDA
        token_2022::set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_2022::SetAuthority {
                    current_authority: ctx.accounts.authority.to_account_info(),
                    account_or_mint: ctx.accounts.token_mint.to_account_info(),
                }
            ),
            AuthorityType::FreezeAccount,
            Some(ctx.accounts.authority_state.key()),
        )?;

        // 3. 发出初始化事件
        emit!(InitializeEvent {
            authority: ctx.accounts.authority.key(),
            mint: ctx.accounts.token_mint.key(),
            decimals
        });

        msg!("Initialization completed successfully");
        Ok(())
    } 
    
    /// 初始化PDA账户，用于已存在的mint账户
    pub fn initialize_pda_only(ctx: Context<InitializePdaOnly>, decimals: u8) -> Result<()> {
        msg!("Starting PDA-only initialization...");
        msg!("Authority: {}", ctx.accounts.authority.key());
        msg!("Mint: {}", ctx.accounts.token_mint.key());

        // 验证访问注册表已初始化
        require!(ctx.accounts.access_registry.initialized, WusdError::AccessRegistryNotInitialized);
        
        // 验证调用者是否有权限初始化
        require!(
            ctx.accounts.access_registry.authority == ctx.accounts.authority.key() || 
            ctx.accounts.access_registry.has_access(ctx.accounts.authority.key(), crate::access::AccessLevel::Debit),
            WusdError::Unauthorized
        );

        // 1. 初始化状态账户
        let authority_state = &mut ctx.accounts.authority_state;
        authority_state.admin = ctx.accounts.authority.key();
        authority_state.minter = ctx.accounts.authority.key();
        authority_state.pauser = ctx.accounts.authority.key();

        let mint_state = &mut ctx.accounts.mint_state;
        mint_state.mint = ctx.accounts.token_mint.key();
        mint_state.decimals = decimals;

        let pause_state = &mut ctx.accounts.pause_state;
        pause_state.paused = false;

        // 2. 转移mint的authority给authority_state PDA
        let mint_key = ctx.accounts.token_mint.key();
        let seeds = &[b"authority", mint_key.as_ref()]; 
        let (_authority_pda, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
        let _bump_bytes = &[bump];  
        token_2022::set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_2022::SetAuthority {
                    current_authority: ctx.accounts.authority.to_account_info(),
                    account_or_mint: ctx.accounts.token_mint.to_account_info(),
                }
            ),
            AuthorityType::MintTokens,
            Some(ctx.accounts.authority_state.key()),
        )?;
        
        // 转移freeze_account权限给authority_state PDA
        token_2022::set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_2022::SetAuthority {
                    current_authority: ctx.accounts.authority.to_account_info(),
                    account_or_mint: ctx.accounts.token_mint.to_account_info(),
                }
            ),
            AuthorityType::FreezeAccount,
            Some(ctx.accounts.authority_state.key()),
        )?;
        
        // 转移freeze_account权限给authority_state PDA
        token_2022::set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_2022::SetAuthority {
                    current_authority: ctx.accounts.authority.to_account_info(),
                    account_or_mint: ctx.accounts.token_mint.to_account_info(),
                }
            ),
            AuthorityType::FreezeAccount,
            Some(ctx.accounts.authority_state.key()),
        )?;

        // 3. 发出初始化事件
        emit!(InitializeEvent {
            authority: ctx.accounts.authority.key(),
            mint: ctx.accounts.token_mint.key(),
            decimals
        });

        msg!("PDA-only initialization completed successfully");
        Ok(())
    }
    
    /// 铸造WUSD代币 
    pub fn mint(ctx: Context<MintAccounts>, amount: u64, bump: u8) -> Result<()> {
        instructions::mint::mint(ctx, amount, bump) 
    }
    
    /// 处理授权许可请求，允许代币持有者授权其他账户使用其代币
    pub fn permit(ctx: Context<Permit>, params: PermitParams) -> Result<()> { 
        instructions::permit::permit(ctx, params) 
    }

    /// 转账WUSD代币 
    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        instructions::transfer::transfer(ctx, amount) 
    } 

    /// 使用授权额度转账WUSD代币 
    pub fn transfer_from(ctx: Context<TransferFrom>, amount: u64) -> Result<()> {
        instructions::transfer::transfer_from(ctx, amount) 
    } 

    /// 暂停合约
    pub fn pause(ctx: Context<Pause>) -> Result<()> {
        instructions::pause::pause(ctx)  
    }

     /// 恢复合约
    pub fn unpause(ctx: Context<Unpause>) -> Result<()> {
        instructions::pause::unpause(ctx)  
    }

    /// 销毁WUSD代币
    pub fn burn(ctx: Context<Burn>, amount: u64) -> Result<()> {
        instructions::burn::burn(ctx, amount)
    } 

    /// 添加操作员
    pub fn add_operator(ctx: Context<ManageOperator>, operator: Pubkey) -> Result<()> {
        instructions::operator::add_operator(ctx, operator)
    }

    /// 移除操作员
    pub fn remove_operator(ctx: Context<ManageOperator>, operator: Pubkey) -> Result<()> {
        instructions::operator::remove_operator(ctx, operator)
    }

    pub fn initialize_freeze_state(ctx: Context<InitializeFreezeState>) -> Result<()> {
        instructions::freeze::initialize_freeze_state(ctx)
    }

    /// 冻结账户
    pub fn freeze_account(ctx: Context<FreezeAccount>) -> Result<()> { 
        instructions::freeze::freeze_account(ctx)
    }

    /// 解冻账户
    pub fn unfreeze_account(ctx: Context<UnfreezeAccount>) -> Result<()> {
        instructions::freeze::unfreeze_account(ctx) 
    } 
}

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct InitializePdaOnly<'info> {
    /// 管理员账户
    #[account(mut)]
    pub authority: Signer<'info>,

    /// 权限管理账户
    #[account(
        init,
        payer = authority, 
        space = AuthorityState::SIZE,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// 代币铸币账户 - 注意这里不使用init约束，因为账户已经存在
    #[account(
        mut,
        owner = TOKEN_PROGRAM_ID,
        constraint = token_mint.mint_authority.contains(&authority.key()) @ WusdError::Unauthorized
    )]
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    /// 铸币状态账户
    #[account(
        init,
        payer = authority, 
        space = MintState::SIZE,
        seeds = [b"mint_state", token_mint.key().as_ref()],
        bump
    )]
    pub mint_state: Account<'info, MintState>,

    /// 暂停状态账户
    #[account(
        init,
        payer = authority, 
        space = PauseState::SIZE,
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump
    )]
    pub pause_state: Account<'info, PauseState>,
    
    /// 访问注册表账户，确保已初始化
    #[account(
        seeds = [b"access_registry"],
        bump,
        constraint = access_registry.initialized @ WusdError::AccessRegistryNotInitialized
    )]
    pub access_registry: Account<'info, AccessRegistryState>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, anchor_spl::token_2022::Token2022>,
    pub rent: Sysvar<'info, Rent>,
}   

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct Initialize<'info> {
    /// 管理员账户
    #[account(mut)]
    pub authority: Signer<'info>,

    /// 权限管理账户
    #[account(
        init,
        payer = authority, 
        space = AuthorityState::SIZE,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// 代币铸币账户
    #[account(
        init,
        payer = authority,
        mint::decimals = decimals,
        mint::authority = authority.key(),
        owner = TOKEN_PROGRAM_ID
    )]
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    /// 铸币状态账户
    #[account(
        init,
        payer = authority, 
        space = MintState::SIZE,
        seeds = [b"mint_state", token_mint.key().as_ref()],
        bump
    )]
    pub mint_state: Account<'info, MintState>,

    /// 暂停状态账户
    #[account(
        init,
        payer = authority, 
        space = PauseState::SIZE,
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump
    )]
    pub pause_state: Account<'info, PauseState>,
    
    /// 访问注册表账户，确保已初始化
    #[account(
        seeds = [b"access_registry"],
        bump,
        constraint = access_registry.initialized @ WusdError::AccessRegistryNotInitialized
    )]
    pub access_registry: Account<'info, AccessRegistryState>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, anchor_spl::token_2022::Token2022>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeAccessRegistry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>, 
     
    #[account(
        init,
        payer = authority, 
        space = AccessRegistryState::SIZE,
        seeds = [b"access_registry"],
        bump,
        constraint = !access_registry.initialized @ WusdError::Unauthorized
    )]
    pub access_registry: Account<'info, AccessRegistryState>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct InitializeEvent {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub decimals: u8,
}

#[event]
pub struct AccessRegistryInitializeEvent {
    pub authority: Pubkey,
}