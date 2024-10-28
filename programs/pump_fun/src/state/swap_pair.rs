use anchor_lang::prelude::*;

use crate::{curve::LinearPriceCurve, Fees};

#[account]
pub struct SwapPair {
  pub token_account: Pubkey,
  pub native_account: Pubkey,
  pub mint: Pubkey,
  pub curve: LinearPriceCurve,
  pub fees: Fees,
  pub bump: u8,
}