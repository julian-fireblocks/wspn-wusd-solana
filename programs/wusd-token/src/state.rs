use crate::error::WusdError;
use anchor_lang::prelude::*;

/// 授权额度状态账户，存储代币授权信息
#[account]
pub struct AllowanceState {
    /// 代币所有者地址
    pub owner: Pubkey,
    /// 被授权者地址
    pub spender: Pubkey,
    /// 授权额度
    pub amount: u64,
}

impl AllowanceState {
    /// 授权额度状态账户大小
    pub const SIZE: usize = 8 + 32 + 32 + 8;

    /// 初始化授权状态
    /// * `owner` - 代币所有者
    /// * `spender` - 被授权者
    /// * `amount` - 授权金额
    pub fn initialize(owner: Pubkey, spender: Pubkey, amount: u64) -> Self {
        Self {
            owner,
            spender,
            amount,
        }
    }

    /// 增加授权额度
    /// * `added_value` - 增加的额度
    pub fn increase_allowance(&mut self, added_value: u64) -> Result<()> {
        self.amount = self
            .amount
            .checked_add(added_value)
            .ok_or(error!(crate::error::WusdError::InvalidAmount))?;
        Ok(())
    }

    /// 减少授权额度
    /// * `subtracted_value` - 减少的额度
    pub fn decrease_allowance(&mut self, subtracted_value: u64) -> Result<()> {
        require!(
            self.amount >= subtracted_value,
            crate::error::WusdError::InvalidAmount
        );
        self.amount = self
            .amount
            .checked_sub(subtracted_value)
            .ok_or(error!(crate::error::WusdError::InvalidAmount))?;
        Ok(())
    }

    /// 验证授权额度是否足够
    /// * `amount` - 待验证的金额
    pub fn validate_allowance(&self, amount: u64) -> Result<()> {
        require!(
            self.amount >= amount,
            crate::error::WusdError::InvalidAmount
        );
        Ok(())
    }
}

/// 签名许可状态账户，用于EIP-2612兼容的签名授权
#[account]
pub struct PermitState {
    /// 所有者地址
    pub owner: Pubkey,
    /// 被授权者地址
    pub spender: Pubkey,
    /// 随机数，用于防止重放攻击
    pub nonce: u64,
    /// 授权额度
    pub amount: u64,
    /// 过期时间
    pub expiration: i64,
    /// PDA bump
    pub bump: u8,
}

impl PermitState {
    /// 许可状态账户大小
    pub const SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 1;

    /// 初始化签名许可状态
    /// * `owner` - 所有者地址
    pub fn initialize(
        owner: Pubkey,
        spender: Pubkey,
        amount: u64,
        expiration: i64,
        bump: u8,
    ) -> Self {
        Self {
            owner,
            spender,
            nonce: 0,
            amount,
            expiration,
            bump,
        }
    }

    /// 增加随机数
    pub fn increment_nonce(&mut self) {
        self.nonce = self.nonce.checked_add(1).unwrap_or(0);
    }

    /// 验证随机数
    /// * `expected_nonce` - 期望的随机数
    pub fn validate_nonce(&self, expected_nonce: u64) -> Result<()> {
        require!(
            self.nonce == expected_nonce,
            crate::error::WusdError::InvalidNonce
        );
        Ok(())
    }
}

/// 权限管理状态账户，存储合约的权限配置
#[account]
pub struct AuthorityState {
    /// 管理员地址
    pub admin: Pubkey,
    /// 铸币角色地址
    pub minter_role: Pubkey,
    /// 销毁角色地址
    pub burner_role: Pubkey,
    /// 暂停角色地址
    pub pauser_role: Pubkey,
    /// 冻结角色地址
    pub freezer_role: Pubkey,
}

impl AuthorityState {
    /// 权限管理状态账户大小
    /// discriminator + admin + minter_role + burner_role + pauser_role
    pub const SIZE: usize = 8 + 32 + 32 + 32 + 32 + 32;

    pub fn initialize(admin: Pubkey) -> Self {
        Self {
            admin,
            minter_role: Pubkey::default(),
            burner_role: Pubkey::default(),
            pauser_role: Pubkey::default(),
            freezer_role: Pubkey::default(),
        }
    }

    pub fn is_admin(&self, user: Pubkey) -> bool {
        self.admin == user
    }

    /// 转移管理员权限
    pub fn transfer_admin(&mut self, new_admin: Pubkey) -> Result<()> {
        require!(new_admin != Pubkey::default(), WusdError::InvalidAddress);
        self.admin = new_admin;
        Ok(())
    }

    /// 设置铸币角色
    pub fn set_minter_role(&mut self, minter: Pubkey) -> Result<()> {
        require!(minter != Pubkey::default(), WusdError::InvalidAddress);
        self.minter_role = minter;
        Ok(())
    }

    /// 设置销毁角色
    pub fn set_burner_role(&mut self, burner: Pubkey) -> Result<()> {
        require!(burner != Pubkey::default(), WusdError::InvalidAddress);
        self.burner_role = burner;
        Ok(())
    }

    /// 设置冻结角色
    pub fn set_freezer_role(&mut self, freezer: Pubkey) -> Result<()> {
        require!(freezer != Pubkey::default(), WusdError::InvalidAddress);
        self.freezer_role = freezer;
        Ok(())
    }

    /// 设置暂停角色
    pub fn set_pauser_role(&mut self, pauser: Pubkey) -> Result<()> {
        require!(pauser != Pubkey::default(), WusdError::InvalidAddress);
        self.pauser_role = pauser;
        Ok(())
    }

    /// 验证铸币角色
    pub fn is_minter(&self, user: Pubkey) -> bool {
        self.minter_role == user
    }

    /// 验证销毁角色
    pub fn is_burner(&self, user: Pubkey) -> bool {
        self.burner_role == user
    }

    /// 验证暂停角色
    pub fn is_pauser(&self, user: Pubkey) -> bool {
        self.pauser_role == user
    }

    /// 验证是否为冻结角色
    pub fn is_freezer(&self, user: Pubkey) -> bool {
        self.freezer_role == user
    }
}

/// 铸币状态账户，存储代币铸造相关信息
#[account]
pub struct MintState {
    /// 代币铸币账户地址
    pub mint: Pubkey,
    /// 代币精度
    pub decimals: u8,
}

impl MintState {
    pub const SIZE: usize = 8 + // discriminator
        32 + // mint
        1; // decimals
}

/// 暂停状态账户，用于控制合约的暂停/恢复
#[account]
pub struct PauseState {
    /// 合约是否暂停
    pub paused: bool,
}

impl PauseState {
    pub const SIZE: usize = 8 + 1; // paused     /// 设置暂停状态
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// 验证合约未暂停 - 简化实现以减少栈使用
    #[inline(always)]
    pub fn validate_not_paused(&self) -> Result<()> {
        if self.paused {
            Err(error!(WusdError::ContractPaused))
        } else {
            Ok(())
        }
    }
}

/// 账户冻结状态，用于控制账户的冻结/解冻
#[account]
pub struct FreezeState {
    /// 账户是否被冻结
    pub is_frozen: bool,
}

impl FreezeState {
    pub const SIZE: usize = 8 + 1; // is_frozen

    /// 检查账户是否被冻结
    pub fn check_frozen(&self) -> Result<()> {
        require!(!self.is_frozen, WusdError::AccountFrozen);
        Ok(())
    }

    /// 冻结账户
    pub fn freeze(&mut self) -> Result<()> {
        require!(!self.is_frozen, WusdError::AccountAlreadyFrozen);
        self.is_frozen = true;
        Ok(())
    }

    /// 解冻账户
    pub fn unfreeze(&mut self) {
        self.is_frozen = false;
    }
}
