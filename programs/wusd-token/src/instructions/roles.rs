use crate::error::WusdError;  
use anchor_lang::prelude::*;
use crate::state::{AuthorityState,PauseState};

/// 角色类型枚举
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
pub enum RoleType {
    Minter,
    Burner,
    Pauser,
    Freezer,
}

/// 转移管理员权限的指令上下文
#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    /// 当前管理员，必须签名
    #[account(mut)]
    pub current_admin: Signer<'info>,

    /// 权限管理状态账户
    #[account(
        mut,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_admin(current_admin.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// CHECK: 代币铸币账户
    pub token_mint: AccountInfo<'info>,
}

/// 设置角色的通用指令上下文
#[derive(Accounts)]
pub struct SetRole<'info> {
    /// 当前管理员，必须签名
    #[account(mut)]
    pub admin: Signer<'info>,

    /// 权限管理状态账户
    #[account(
        mut,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_admin(admin.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// CHECK: 代币铸币账户
    pub token_mint: AccountInfo<'info>,
    
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
}

/// 用户自行移除角色的指令上下文
#[derive(Accounts)]
pub struct RemoveSelfRole<'info> {
    /// 当前用户，必须签名
    #[account(mut)]
    pub user: Signer<'info>,

    /// 权限管理状态账户
    #[account(
        mut,
        seeds = [b"authority", token_mint.key().as_ref()],
        bump
    )]
    pub authority_state: Account<'info, AuthorityState>,

    /// CHECK: 代币铸币账户
    pub token_mint: AccountInfo<'info>,
    
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Account<'info, PauseState>,
}

/// 转移管理员权限
/// * `ctx` - 转移管理员权限的上下文
/// * `new_admin` - 新管理员的公钥
pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
    // 调用AuthorityState中的transfer_admin方法
    ctx.accounts.authority_state.transfer_admin(new_admin)?;

    // 发出管理员转移事件
    emit!(AdminTransferredEvent {
        previous_admin: ctx.accounts.current_admin.key(),
        new_admin,
    });

    Ok(())
}

/// 设置角色
/// * `ctx` - 设置角色的上下文
/// * `role_type` - 角色类型
/// * `new_role` - 新角色的公钥
pub fn set_role(
    ctx: Context<SetRole>,
    role_type: RoleType,
    new_role: Pubkey,
    is_add: bool,
) -> Result<()> {
    let authority_state = &mut ctx.accounts.authority_state;

    match role_type {
        RoleType::Minter => {
            if is_add {
                authority_state.add_minter_role(new_role)?;
            } else {
                authority_state.remove_minter_role(new_role)?;
            }
        }
        RoleType::Burner => {
            if is_add {
                authority_state.add_burner_role(new_role)?;
            } else {
                authority_state.remove_burner_role(new_role)?;
            }
        }
        RoleType::Pauser => {
            if is_add {
                authority_state.add_pauser_role(new_role)?;
            } else {
                authority_state.remove_pauser_role(new_role)?;
            }
        }
        RoleType::Freezer => {
            if is_add {
                authority_state.add_freezer_role(new_role)?;
            } else {
                authority_state.remove_freezer_role(new_role)?;
            }
        }
    };

    // 发出相应的角色转移事件
    match role_type {
        RoleType::Minter => emit!(MinterRoleChangedEvent {
            minter: new_role,
            is_add: is_add,
        }),
        RoleType::Burner => emit!(BurnerRoleChangedEvent {
            burner: new_role,
            is_add: is_add,
        }),
        RoleType::Pauser => emit!(PauserRoleChangedEvent {
            pauser: new_role,
            is_add: is_add,
        }),
        RoleType::Freezer => emit!(FreezerRoleChangedEvent {
            freezer: new_role,
            is_add: is_add,
        }),
    }

    Ok(())
}

/// 用户自行移除角色
/// * `ctx` - 移除角色的上下文
/// * `role_type` - 角色类型
pub fn remove_self_role(
    ctx: Context<RemoveSelfRole>,
    role_type: RoleType,
) -> Result<()> {
    let user_key = ctx.accounts.user.key();
    let authority_state = &mut ctx.accounts.authority_state;

    match role_type {
        RoleType::Minter => {
            require!(authority_state.is_minter(user_key), WusdError::Unauthorized);
            authority_state.remove_minter_role(user_key)?;
        }
        RoleType::Burner => {
            require!(authority_state.is_burner(user_key), WusdError::Unauthorized);
            authority_state.remove_burner_role(user_key)?;
        }
        RoleType::Pauser => {
            require!(authority_state.is_pauser(user_key), WusdError::Unauthorized);
            authority_state.remove_pauser_role(user_key)?;
        }
        RoleType::Freezer => {
            require!(authority_state.is_freezer(user_key), WusdError::Unauthorized);
            authority_state.remove_freezer_role(user_key)?;
        }
    };

    // 发出相应的角色移除事件
    match role_type {
        RoleType::Minter => emit!(MinterRoleChangedEvent {
            minter: user_key,
            is_add: false,
        }),
        RoleType::Burner => emit!(BurnerRoleChangedEvent {
            burner: user_key,
            is_add: false,
        }),
        RoleType::Pauser => emit!(PauserRoleChangedEvent {
            pauser: user_key,
            is_add: false,
        }),
        RoleType::Freezer => emit!(FreezerRoleChangedEvent {
            freezer: user_key,
            is_add: false,
        }),
    }

    Ok(())
}

/// 管理员转移事件
#[event]
pub struct AdminTransferredEvent {
    /// 前任管理员地址
    pub previous_admin: Pubkey,
    /// 新任管理员地址
    pub new_admin: Pubkey,
}

/// Minter角色变更事件
#[event]
pub struct MinterRoleChangedEvent {
    /// 变更的Minter地址
    pub minter: Pubkey,
    /// 是否为添加操作
    pub is_add: bool,
}

/// Pauser角色变更事件
#[event]
pub struct PauserRoleChangedEvent {
    /// 变更的Pauser地址
    pub pauser: Pubkey,
    /// 是否为添加操作
    pub is_add: bool,
}

/// Freezer角色变更事件
#[event]
pub struct FreezerRoleChangedEvent {
    /// 变更的Freezer地址
    pub freezer: Pubkey,
    /// 是否为添加操作
    pub is_add: bool,
}

/// Burner角色变更事件
#[event]
pub struct BurnerRoleChangedEvent {
    /// 变更的Burner地址
    pub burner: Pubkey,
    /// 是否为添加操作
    pub is_add: bool,
}
