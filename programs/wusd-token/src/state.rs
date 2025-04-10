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

}

/// 权限管理状态账户，存储合约的权限配置
#[account]
pub struct AuthorityState {
    /// 管理员地址
    pub admin: Pubkey,
    /// 铸币角色地址列表
    pub minter_roles: Vec<Pubkey>,
    /// 销毁角色地址列表
    pub burner_roles: Vec<Pubkey>,
    /// 暂停角色地址列表
    pub pauser_roles: Vec<Pubkey>,
    /// 冻结角色地址列表
    pub freezer_roles: Vec<Pubkey>,
}

impl AuthorityState {
    /// 权限管理状态账户大小
    /// discriminator + admin + minter_role + burner_role + pauser_role
    pub const SIZE: usize = 8 + 32 + (4 + 32) + (4 + 32) + (4 + 32) + (4 + 32); // 8 for discriminator, 32 for admin, 4 for each Vec length, 32 for each Pubkey

    pub fn initialize(admin: Pubkey) -> Self {
        Self {
            admin,
            minter_roles: Vec::new(),
            burner_roles: Vec::new(),
            pauser_roles: Vec::new(),
            freezer_roles: Vec::new(),
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

    /// 添加铸币角色
    pub fn add_minter_role(&mut self, minter: Pubkey) -> Result<()> {
        require!(minter != Pubkey::default(), WusdError::InvalidAddress);
        if !self.minter_roles.contains(&minter) {
            self.minter_roles.push(minter);
        }
        Ok(())
    }

    /// 移除铸币角色
    pub fn remove_minter_role(&mut self, minter: Pubkey) -> Result<()> {
        self.minter_roles.retain(|&x| x != minter);
        Ok(())
    }

    /// 添加销毁角色
    pub fn add_burner_role(&mut self, burner: Pubkey) -> Result<()> {
        require!(burner != Pubkey::default(), WusdError::InvalidAddress);
        if !self.burner_roles.contains(&burner) {
            self.burner_roles.push(burner);
        }
        Ok(())
    }

    /// 移除销毁角色
    pub fn remove_burner_role(&mut self, burner: Pubkey) -> Result<()> {
        self.burner_roles.retain(|&x| x != burner);
        Ok(())
    }

    /// 添加冻结角色
    pub fn add_freezer_role(&mut self, freezer: Pubkey) -> Result<()> {
        require!(freezer != Pubkey::default(), WusdError::InvalidAddress);
        if !self.freezer_roles.contains(&freezer) {
            self.freezer_roles.push(freezer);
        }
        Ok(())
    }

    /// 移除冻结角色
    pub fn remove_freezer_role(&mut self, freezer: Pubkey) -> Result<()> {
        self.freezer_roles.retain(|&x| x != freezer);
        Ok(())
    }

    /// 添加暂停角色
    pub fn add_pauser_role(&mut self, pauser: Pubkey) -> Result<()> {
        require!(pauser != Pubkey::default(), WusdError::InvalidAddress);
        if !self.pauser_roles.contains(&pauser) {
            self.pauser_roles.push(pauser);
        }
        Ok(())
    }

    /// 移除暂停角色
    pub fn remove_pauser_role(&mut self, pauser: Pubkey) -> Result<()> {
        self.pauser_roles.retain(|&x| x != pauser);
        Ok(())
    }

    /// 验证铸币角色
    pub fn is_minter(&self, user: Pubkey) -> bool {
        self.minter_roles.contains(&user)
    }

    /// 验证销毁角色
    pub fn is_burner(&self, user: Pubkey) -> bool {
        self.burner_roles.contains(&user)
    }

    /// 验证暂停角色
    pub fn is_pauser(&self, user: Pubkey) -> bool {
        self.pauser_roles.contains(&user)
    }

    /// 验证是否为冻结角色
    pub fn is_freezer(&self, user: Pubkey) -> bool {
        self.freezer_roles.contains(&user)
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
}

/// 账户冻结状态，用于控制账户的冻结/解冻
#[account]
pub struct FreezeState {
    /// 账户是否被冻结
    pub is_frozen: bool,
}

impl FreezeState {
    pub const SIZE: usize = 8 + 1; // is_frozen 
        /// 冻结账户
    pub fn freeze(&mut self) {
        self.is_frozen = true;
    }

    /// 解冻账户
    pub fn unfreeze(&mut self) {
        self.is_frozen = false;
    }
}
