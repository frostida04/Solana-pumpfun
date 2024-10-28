use anchor_lang::prelude::*;

#[constant]
pub const SEED: &str = "anchor";
pub const APP_STATS_SEED: &[u8] = b"app-stats";
pub const AUTHORITY_SEED: &[u8] = b"authority";
pub const TOKEN_CREATE_SEED: &[u8] = b"token-create";
pub const TOKEN_ACCOUNT_SEED: &[u8] = b"token-account";
pub const SWAP_PAIR_SEED: &[u8] = b"swap-pair";
pub const FEE_PERCENTAGE: u16 = 100;
pub const DENOMINATOR: u16 = 10000;

