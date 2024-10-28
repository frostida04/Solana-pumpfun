use anchor_lang::prelude::*;


#[account]
pub struct TokenCreate {
  pub creator: Pubkey,
  pub mint: Pubkey,
  pub supply: u64,
  pub bump: u8,
  pub cap: u64,
}