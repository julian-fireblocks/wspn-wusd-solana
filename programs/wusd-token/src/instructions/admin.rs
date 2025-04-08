use anchor_lang::prelude::*;
use crate::state::AuthorityState;
use crate::error::WusdError;

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

/// 管理员转移事件
#[event]
pub struct AdminTransferredEvent {
    /// 前任管理员地址
    pub previous_admin: Pubkey,
    /// 新任管理员地址
    pub new_admin: Pubkey,
}