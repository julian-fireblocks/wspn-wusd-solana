//! WUSD Token 程序  
mod error;
mod state;
mod instructions; 

use crate::error::WusdError;
use anchor_lang::prelude::*;
use anchor_spl::token_2022; 
use anchor_spl::token_interface::Mint;
use spl_token_2022::instruction::AuthorityType;    

use state::{AuthorityState, MintState, PauseState, AccessRegistryState};

use instructions::mint::*; 
use instructions::burn::*;
use instructions::transfer::*;
use instructions::permit::*;
use instructions::operator::*;
use instructions::pause::*;
use instructions::freeze::*;
use instructions::admin::*; 

// 辅助函数：初始化状态账户，减少栈使用
#[inline(always)]
fn initialize_state_accounts(
    auth_key: Pubkey,
    authority_state: &mut Account<AuthorityState>,
    mint_state: &mut Account<MintState>,
    pause_state: &mut Account<PauseState>,
    token_mint_key: Pubkey,
    decimals: u8,
) {
    authority_state.admin = auth_key;

    mint_state.mint = token_mint_key;
    mint_state.decimals = decimals;

    pause_state.paused = false;
}

// 辅助函数：转移权限，极度简化以减少栈使用
#[inline(always)]
fn transfer_authorities<'info>(
    token_program: &Program<'info, anchor_spl::token_2022::Token2022>,
    authority: &Signer<'info>,
    token_mint: &InterfaceAccount<'info, Mint>,
    authority_state: &Account<'info, AuthorityState>,
) -> Result<()> {
    // 极简化实现，减少局部变量和中间对象创建
    let auth_key = authority_state.key();
    
    // 转移mint权限 - 直接使用to_account_info()而不存储中间变量
    token_2022::set_authority(
        CpiContext::new(
            token_program.to_account_info(),
            token_2022::SetAuthority {
                current_authority: authority.to_account_info(),
                account_or_mint: token_mint.to_account_info(),
            }
        ),
        AuthorityType::MintTokens,
        Some(auth_key),
    )?;
    
    // 转移freeze权限 - 直接使用to_account_info()而不存储中间变量
    token_2022::set_authority(
        CpiContext::new(
            token_program.to_account_info(),
            token_2022::SetAuthority {
                current_authority: authority.to_account_info(),
                account_or_mint: token_mint.to_account_info(),
            }
        ),
        AuthorityType::FreezeAccount,
        Some(auth_key),
    )?;
    
    Ok(())
} 

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
        access_registry.operators = [Pubkey::default(); 3];
        access_registry.initialized = true;
        
        // 发出初始化事件
        emit!(AccessRegistryInitializeEvent {
            authority: ctx.accounts.authority.key()
        });
        
        Ok(())
    }

    pub fn initialize(ctx: Context<Initialize>, decimals: u8) -> Result<()> {
        msg!("Starting initialization...");
        
        // 极简化验证逻辑，减少栈使用
        if !ctx.accounts.access_registry.initialized {
            return Err(error!(WusdError::AccessRegistryNotInitialized));
        }
        
        // 1. 初始化状态账户 - 使用辅助函数减少栈使用
        initialize_state_accounts(
            ctx.accounts.authority.key(),
            &mut ctx.accounts.authority_state,
            &mut ctx.accounts.mint_state,
            &mut ctx.accounts.pause_state,
            ctx.accounts.token_mint.key(),
            decimals
        );

        // 2. 转移mint和freeze权限 - 使用辅助函数减少栈使用
        transfer_authorities(
            &ctx.accounts.token_program,
            &ctx.accounts.authority,
            &ctx.accounts.token_mint,
            &ctx.accounts.authority_state,
        )?;

        // 3. 发出初始化事件 - 简化事件参数
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

        // 极简化验证 - 减少栈使用
        if !ctx.accounts.access_registry.initialized {
            return Err(error!(WusdError::AccessRegistryNotInitialized));
        }
        
        // 1. 初始化状态账户 - 使用辅助函数减少栈使用
        initialize_state_accounts(
            ctx.accounts.authority.key(),
            &mut ctx.accounts.authority_state,
            &mut ctx.accounts.mint_state,
            &mut ctx.accounts.pause_state,
            ctx.accounts.token_mint.key(),
            decimals
        );

        // 2. 转移mint和freeze权限 - 使用辅助函数减少栈使用
        transfer_authorities(
            &ctx.accounts.token_program,
            &ctx.accounts.authority,
            &ctx.accounts.token_mint,
            &ctx.accounts.authority_state,
        )?;

        // 3. 发出初始化事件 - 简化事件参数
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
    /// 转移管理员权限
    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::admin::transfer_admin(ctx, new_admin)
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

    /// 权限管理账户 - 极简化约束条件
    #[account(
        init,
        payer = authority, 
        space = AuthorityState::SIZE,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// 代币铸币账户 - 极简化约束条件
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    /// 铸币状态账户 - 极简化约束条件
    #[account(
        init,
        payer = authority, 
        space = MintState::SIZE,
        seeds = [b"mint_state", token_mint.key().as_ref()],
        bump
    )]
    pub mint_state: Account<'info, MintState>,

    /// 暂停状态账户 - 极简化约束条件
    #[account(
        init,
        payer = authority, 
        space = PauseState::SIZE,
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump
    )]
    pub pause_state: Account<'info, PauseState>,
    
    /// 访问注册表账户 - 极简化约束条件
    #[account(seeds = [b"access_registry"], bump)]
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

    /// 权限管理账户 - 极简化约束条件
    #[account(
        init,
        payer = authority, 
        space = AuthorityState::SIZE,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// 代币铸币账户 - 极简化约束条件
    #[account(
        init,
        payer = authority,
        mint::decimals = decimals,
        mint::authority = authority.key()
    )]
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    /// 铸币状态账户 - 极简化约束条件
    #[account(
        init,
        payer = authority, 
        space = MintState::SIZE,
        seeds = [b"mint_state", token_mint.key().as_ref()],
        bump
    )]
    pub mint_state: Account<'info, MintState>,

    /// 暂停状态账户 - 极简化约束条件
    #[account(
        init,
        payer = authority, 
        space = PauseState::SIZE,
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump
    )]
    pub pause_state: Account<'info, PauseState>,
    
    /// 访问注册表账户 - 极简化约束条件
    #[account(seeds = [b"access_registry"], bump)]
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
        bump
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