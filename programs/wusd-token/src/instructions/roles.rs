use crate::error::WusdError;
use crate::state::AuthorityState;
use anchor_lang::prelude::*;

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
pub fn set_role(ctx: Context<SetRole>, role_type: RoleType, new_role: Pubkey) -> Result<()> {
    let authority_state = &mut ctx.accounts.authority_state;
    let previous_role = match role_type {
        RoleType::Minter => {
            authority_state.set_minter_role(new_role)?;
            authority_state.minter_role
        }
        RoleType::Burner => {
            authority_state.set_burner_role(new_role)?;
            authority_state.burner_role
        }
        RoleType::Pauser => {
            authority_state.set_pauser_role(new_role)?;
            authority_state.pauser_role
        }
        RoleType::Freezer => {
            authority_state.set_freezer_role(new_role)?;
            authority_state.freezer_role
        }
    };

    // 发出相应的角色转移事件
    match role_type {
        RoleType::Minter => emit!(MinterRoleTransferredEvent {
            previous_minter: previous_role,
            new_minter: new_role,
        }),
        RoleType::Burner => emit!(BurnerRoleTransferredEvent {
            previous_burner: previous_role,
            new_burner: new_role,
        }),
        RoleType::Pauser => emit!(PauserRoleTransferredEvent {
            previous_pauser: previous_role,
            new_pauser: new_role,
        }),
        RoleType::Freezer => emit!(FreezerRoleTransferredEvent {
            previous_freezer: previous_role,
            new_freezer: new_role,
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

/// Minter角色转移事件
#[event]
pub struct MinterRoleTransferredEvent {
    /// 前任Minter地址
    pub previous_minter: Pubkey,
    /// 新任Minter地址
    pub new_minter: Pubkey,
}

/// Pauser角色转移事件
#[event]
pub struct PauserRoleTransferredEvent {
    /// 前任Pauser地址
    pub previous_pauser: Pubkey,
    /// 新任Pauser地址
    pub new_pauser: Pubkey,
}

/// Freezer角色转移事件
#[event]
pub struct FreezerRoleTransferredEvent {
    /// 前任Freezer地址
    pub previous_freezer: Pubkey,
    /// 新任Freezer地址
    pub new_freezer: Pubkey,
}

/// Burner角色转移事件
#[event]
pub struct BurnerRoleTransferredEvent {
    /// 前任Burner地址
    pub previous_burner: Pubkey,
    /// 新任Burner地址
    pub new_burner: Pubkey,
}