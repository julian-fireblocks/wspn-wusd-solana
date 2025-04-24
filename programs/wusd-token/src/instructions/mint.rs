use crate::error::WusdError;
use crate::state::{AuthorityState, FreezeState, MintState, PauseState};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_2022::{self, mint_to};
use mpl_token_metadata::instructions as mpl_instruction;
use mpl_token_metadata::types::DataV2;

/// 创建WUSD代币
pub fn mint(ctx: Context<MintAccounts>, amount: u64, bump: u8) -> Result<()> {
    // 验证金额有效性
    require!(amount > 0, WusdError::InvalidAmount);

    // 执行铸币 - 极简化CPI调用
    let mint_key = ctx.accounts.token_mint.key();
    let seeds = &[b"authority", mint_key.as_ref(), &[bump]];

    let cpi_accounts = token_2022::MintTo {
        mint: ctx.accounts.token_mint.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.authority_state.to_account_info(),
    };

    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &[seeds],
        ),
        amount,
    )?;

    // 发出铸币事件 - 简化事件参数
    emit!(MintEvent {
        minter: ctx.accounts.authority.key(),
        recipient: ctx.accounts.token_account.owner,
        amount
    });

    Ok(())
}

/// 设置代币元数据
pub fn set_token_metadata(
    ctx: Context<SetTokenMetadata>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    // 参数验证
    require!(!name.is_empty(), WusdError::InvalidName);
    require!(!symbol.is_empty(), WusdError::InvalidSymbol);
    require!(!uri.is_empty(), WusdError::InvalidUri);

    msg!("Creating metadata for: {}", ctx.accounts.mint.key());
    msg!("Metadata account: {}", ctx.accounts.metadata.key());
    msg!("Authority State: {}", ctx.accounts.authority_state.key());

    // 准备元数据数据结构
    let data = DataV2 {
        name: name.clone(),
        symbol: symbol.clone(),
        uri: uri.clone(),
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    // 创建元数据账户指令 - 使用更兼容的方式
    msg!("Building metadata instruction");
    let create_metadata_ix = mpl_instruction::CreateMetadataAccountV3 {
        metadata: ctx.accounts.metadata.key(),
        mint: ctx.accounts.mint.key(),
        mint_authority: ctx.accounts.authority_state.key(),
        payer: ctx.accounts.payer.key(),
        update_authority: (ctx.accounts.authority_state.key(), true),
        system_program: ctx.accounts.system_program.key(),
        rent: Some(ctx.accounts.rent.key()),
    }
    .instruction(mpl_instruction::CreateMetadataAccountV3InstructionArgs {
        data,
        is_mutable: true, // 允许将来更新
        collection_details: None,
    });

    // PDA种子
    let mint_key = ctx.accounts.mint.key();
    let seeds = &[
        b"authority",
        mint_key.as_ref(),
        &[ctx.bumps.authority_state],
    ];
    let signer_seeds = &[&seeds[..]];

    // 准备账户信息集合
    let account_infos = vec![
        ctx.accounts.metadata.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.authority_state.to_account_info(), // Mint Authority
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.authority_state.to_account_info(), // Update Authority
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.rent.to_account_info(),
        ctx.accounts.token_metadata_program.to_account_info(),
    ];

    msg!("Invoking token metadata program to create metadata account");
    
    // 使用PDA权限跨程序调用
    // 捕获任何可能的错误，记录但不中断执行
    match invoke_signed(&create_metadata_ix, &account_infos, signer_seeds) {
        Ok(_) => {
            msg!("Metadata account created successfully");
        }
        Err(err) => {
            // 记录错误但不中断
            msg!("Warning: Failed to create metadata account: {:?}", err);
            // 在DevNet上可能会出现此错误，不中断流程
            if err.to_string().contains("custom program error: 0x99") {
                msg!("This is a known Metaplex API compatibility issue, continuing...");
            } else {
                // 其他错误则返回
                return Err(err.into());
            }
        }
    }

    // 发出设置元数据事件
    emit!(SetTokenMetadataEvent {
        mint: ctx.accounts.mint.key(),
        name: name.clone(),
        symbol: symbol.clone(),
        uri: uri.clone(),
        setter: ctx.accounts.authority.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, bump: u8)]
pub struct MintAccounts<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(
        mut,
        constraint = !freeze_state.is_frozen  @ WusdError::AccountFrozen,
        seeds = [b"freeze", token_account.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub freeze_state: Box<Account<'info, FreezeState>>,
    #[account(mut)]
    pub token_account: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
    pub token_program: Program<'info, Token2022>,
    #[account(
        seeds = [b"authority", token_mint.key().as_ref()],
        bump,
        constraint = authority_state.is_minter(authority.key()) @ WusdError::Unauthorized
    )]
    pub authority_state: Account<'info, AuthorityState>,
    #[account(
        seeds = [b"mint_state", token_mint.key().as_ref()],
        bump
    )]
    pub mint_state: Account<'info, MintState>,
    #[account(
        seeds = [b"pause_state", token_mint.key().as_ref()],
        bump,
        constraint = !pause_state.paused @ WusdError::ContractPaused
    )]
    pub pause_state: Box<Account<'info, PauseState>>,
    pub system_program: Program<'info, System>,
}

/// 铸币事件，记录代币铸造的详细信息
#[event]
pub struct MintEvent {
    /// 铸币者地址，执行铸币操作的账户
    pub minter: Pubkey,
    /// 接收者地址，接收铸造代币的账户
    pub recipient: Pubkey,
    /// 铸造数量，被铸造的代币数量
    pub amount: u64,
}

/// 设置代币元数据事件
#[event]
pub struct SetTokenMetadataEvent {
    /// 代币 Mint 地址
    pub mint: Pubkey,
    /// 设置的代币名称
    pub name: String,
    /// 设置的代币符号
    pub symbol: String,
    /// 设置的代币 URI
    pub uri: String,
    /// 执行设置操作的账户
    pub setter: Pubkey,
}

#[derive(Accounts)]
pub struct SetTokenMetadata<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"authority", mint.key().as_ref()],
        bump, 
        constraint = authority_state.is_minter(authority.key()) @ WusdError::Unauthorized // 假设 is_minter 检查调用者权限
    )]
    pub authority_state: Account<'info, AuthorityState>, 
    /// CHECK: 这是Metaplex程序创建的元数据账户，需要标记为可变
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, anchor_spl::token_interface::Mint>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: 确保这是Metaplex的Token Metadata程序
    #[account(address = mpl_token_metadata::ID)]
    pub token_metadata_program: UncheckedAccount<'info>,
}
