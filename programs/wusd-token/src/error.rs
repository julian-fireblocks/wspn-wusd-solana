use anchor_lang::prelude::*;

#[error_code]
pub enum WusdError {
    #[msg("Contract is paused")]
    ContractPaused,
    #[msg("Invalid amount")]
    InvalidAmount,     
    #[msg("Unauthorized")]
    Unauthorized, 
    #[msg("Insufficient balance")]
    InsufficientBalance,     
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Insufficient allowance")]
    InsufficientAllowance,
    #[msg("Account is frozen")]
    AccountFrozen,
    #[msg("Account is already frozen")]
    AccountAlreadyFrozen,
    #[msg("Account is not frozen")]
    AccountNotFrozen, 
    #[msg("Invalid mint address")]
    InvalidMint, 
    #[msg("Expired approve")]
    ExpiredApprovet,
    #[msg("Invalid address")]
    InvalidAddress,   
}