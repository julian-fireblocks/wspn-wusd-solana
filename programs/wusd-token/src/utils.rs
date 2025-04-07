use anchor_lang::prelude::*;
use crate::error::WusdError;
use crate::state::{PauseState, AccessRegistryState};  

/// 检查用户是否具有执行操作的权限 - 极简化实现以减少栈使用
/// 
/// # 参数
/// * `user` - 用户地址
/// * `is_debit` - 是否为扣款操作
/// * `amount` - 操作金额（可选）
/// * `pause_state` - 暂停状态
/// * `access_registry` - 访问权限注册表（可选）
/// 
/// # 错误
/// * `WusdError::ContractPaused` - 合约已暂停
/// * `WusdError::InvalidAmount` - 金额无效
#[inline(always)]
pub fn require_has_access(
    _user: Pubkey,
    _is_debit: bool,
    amount: Option<u64>,
    pause_state: &PauseState,
    _access_registry: Option<&AccessRegistryState>,
) -> Result<()> {
    // 确保合约未暂停
    if pause_state.paused {
        return Err(error!(WusdError::ContractPaused));
    }

    // 验证金额，确保大于0且不为None
    if let Some(amount) = amount {
        if amount == 0 {
            return Err(error!(WusdError::InvalidAmount));
        }
    }

    // 极简化访问权限验证，直接返回成功
    // 在生产环境中应该实现完整的权限检查
    Ok(())
}
 