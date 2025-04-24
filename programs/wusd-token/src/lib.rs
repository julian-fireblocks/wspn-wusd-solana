//! WUSD Token 程序  
mod error;
mod state;
mod instructions;  

use instructions::roles::*; 
use instructions::mint::*; 
use instructions::burn::*;
use instructions::transfer::*;
use instructions::approve::*; 
use instructions::pause::*; 
use instructions::freeze::*;  
use anchor_lang::prelude::*;
use anchor_spl::token_2022; 
use anchor_spl::token_interface::Mint;
use spl_token_2022::instruction::AuthorityType;
use state::{AuthorityState, MintState, PauseState}; 

declare_id!("GXtsGhmGUz4uFkCxsNSmJaxU9nr6SvgA3c6pxFc2MpPn");

// 辅助函数：初始化状态账户，减少栈使用
#[inline(always)]
fn initialize_state_accounts(
    admin_key: Pubkey,
    minter_key: Pubkey,
    pauser_key: Pubkey,
    authority_state: &mut Account<AuthorityState>,
    mint_state: &mut Account<MintState>,
    pause_state: &mut Account<PauseState>,
    token_mint_key: Pubkey,
    decimals: u8,
) -> Result<()> {
    authority_state.admin = admin_key; 
    authority_state.add_minter_role(minter_key)?;
    authority_state.add_pauser_role(pauser_key)?;

    mint_state.mint = token_mint_key;
    mint_state.decimals = decimals;

    pause_state.paused = false;
    Ok(())
}

// 辅助函数：转移权限，极度简化以减少栈使用
#[inline(always)]
fn transfer_authorities<'info>(
    token_program: &Program<'info, anchor_spl::token_2022::Token2022>,
    authority: &Signer<'info>,
    token_mint: &InterfaceAccount<'info, Mint>,
    authority_state: &Account<'info, AuthorityState>,
) -> Result<()> { 
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

#[program]
pub mod wusd_token {
    use super::*;  
    pub fn initialize(ctx: Context<Initialize>, decimals: u8) -> Result<()> {
        msg!("Starting initialization...");
        
        // 1. 初始化状态账户 - 使用辅助函数减少栈使用
        initialize_state_accounts(
            ctx.accounts.authority.key(),
            ctx.accounts.minter.key(),
            ctx.accounts.pauser.key(),
            &mut ctx.accounts.authority_state,
            &mut ctx.accounts.mint_state,
            &mut ctx.accounts.pause_state,
            ctx.accounts.token_mint.key(),
            decimals
        )?;

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

    /// 初始化冻结状态账户
    pub fn initialize_freeze_state(ctx: Context<InitializeFreezeState>) -> Result<()> {
        instructions::freeze::initialize_freeze_state(ctx)
    } 

    /// 转移管理员权限
    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::roles::transfer_admin(ctx, new_admin)
    }  
    
    /// 设置角色 (支持添加/移除)
    pub fn set_role(ctx: Context<SetRole>, role_type: RoleType, new_role: Pubkey, is_add: bool) -> Result<()> {
        instructions::roles::set_role(ctx, role_type, new_role, is_add)
    }
    
    /// 用户自行移除角色
    pub fn remove_self_role(ctx: Context<RemoveSelfRole>, role_type: RoleType) -> Result<()> {
        instructions::roles::remove_self_role(ctx, role_type)
    }
    
    /// 铸造WUSD代币 
    pub fn mint(ctx: Context<MintAccounts>, amount: u64, bump: u8) -> Result<()> {
        instructions::mint::mint(ctx, amount, bump) 
    }

    /// 设置代币元数据 
    pub fn set_token_metadata(ctx: Context<SetTokenMetadata>, name: String, symbol: String, uri: String,) -> Result<()> {
        instructions::mint::set_token_metadata(ctx, name, symbol, uri) 
    }
    
    /// 处理代表津贴请求，允许代币持有者授权其他账户使用其代币 
    pub fn approve(ctx: Context<Approve>, amount: u64, expiry_time: i64) -> Result<()> { 
        instructions::approve::approve_set(ctx, amount, expiry_time) 
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
    /// 冻结账户
    pub fn freeze_account(ctx: Context<FreezeOperationAccounts>) -> Result<()> { 
        instructions::freeze::handle_freeze_operation(ctx, FreezeOperation::Freeze)
    }

    /// 解冻账户
    pub fn unfreeze_account(ctx: Context<FreezeOperationAccounts>) -> Result<()> {
        instructions::freeze::handle_freeze_operation(ctx, FreezeOperation::Unfreeze)
    }   
}

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct Initialize<'info> {
    /// 管理员账户
    /// CHECK: 此账户用于验证签名和支付租金
    #[account(mut)]
    pub authority: Signer<'info>,

    /// 铸币者账户
    /// CHECK: 此账户仅用于验证签名，不直接读取或写入数据
    pub minter: UncheckedAccount<'info>,

    /// 暂停者账户
    /// CHECK: 此账户仅用于验证签名，不直接读取或写入数据
    pub pauser: UncheckedAccount<'info>,

    /// 权限管理账户 
    #[account(
        init,
        payer = authority, 
        space = AuthorityState::SIZE,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// 代币铸币账户 - 使用已存在的账户
    #[account(
        mut,
        mint::authority = authority.key(),
        constraint = token_mint.decimals == decimals,
        constraint = token_mint.is_initialized @ ProgramError::UninitializedAccount
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
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,  
    pub token_program: Program<'info, anchor_spl::token_2022::Token2022>, 
}

#[event]
pub struct InitializeEvent {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub decimals: u8,
}

