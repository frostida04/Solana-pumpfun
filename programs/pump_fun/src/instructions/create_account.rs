use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};


#[derive(Accounts)]
pub struct CreateAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    // mint address
    pub mint: Box<Account<'info, Mint>>,

    // pda for token account
    /// CHECK:
    pub pda: AccountInfo<'info>,

    #[account(
      init,
      payer = signer,
      token::mint = mint,
      token::authority = pda,
    )]
    pub token_account: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/*
  create token pda account before mint token
*/
pub fn create_account_handler(
  _ctx: Context<CreateAccount>
) -> Result<()> {
  Ok(())
}