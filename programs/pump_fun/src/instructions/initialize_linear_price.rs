use anchor_lang::prelude::*;
use anchor_spl::{ token::{ Mint, Token, TokenAccount }, token_interface };

use std::mem::size_of;
use crate::{ SwapPair, AUTHORITY_SEED, SWAP_PAIR_SEED };

#[derive(Accounts)]
pub struct InitializeLinearPrice<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    // pair account
    #[account(
        init,
        payer = creator,
        space = size_of::<SwapPair>() + 8,
        seeds = [SWAP_PAIR_SEED, mint.key().as_ref()],
        bump
    )]
    pub pair: Box<Account<'info, SwapPair>>,
    // mint address
    #[account(token::token_program = token_program_mint)]
    pub mint: Box<InterfaceAccount<'info, token_interface::Mint>>,
    // wsol mint address
    pub wsol: Box<Account<'info, Mint>>,

    /// CHECK:
    #[account(seeds = [AUTHORITY_SEED, mint.key().as_ref()], bump)]
    pub pda: AccountInfo<'info>,

    // token account for pda
    #[account(
      mut,
      token::mint = mint,
      token::authority = pda,
      token::token_program = token_program_mint,
    )]
    pub token_for_pda: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    // wsol account for pda
    #[account(mut, token::authority = pda, token::mint = wsol)]
    pub token_native_for_pda: Box<Account<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    pub token_program_mint: Interface<'info, token_interface::TokenInterface>,
    pub token_program: Program<'info, Token>,
}
/*
    initialize linear bonding curve pool by token creator
    example curve - 0.5 slope (i.e. price increases by "1 base RLY per base CC" for every 2 display CC AKA 2e8 CC), starting price of 50 RLY at 300 (display) CC
    @param
    slope numerator
    slope denominator
    initial token a price numerator : start price
    initial token a price denomiator
*/
pub fn initialize_linear_price_handler(
    ctx: Context<InitializeLinearPrice>,
    slope_numerator: u64,
    slope_denominator: u64,
    initial_token_a_price_numerator: u64,
    initial_token_a_price_denominator: u64,
    bump: u8
) -> Result<()> {
    let pair: &mut Box<Account<SwapPair>> = &mut ctx.accounts.pair;
    pair.token_account = ctx.accounts.token_for_pda.key();
    pair.native_account = ctx.accounts.token_native_for_pda.key();
    pair.mint = ctx.accounts.mint.key();
    pair.curve.slope_numerator = slope_numerator;
    pair.curve.slope_denominator = slope_denominator;
    pair.curve.initial_token_a_price_numerator = initial_token_a_price_numerator;
    pair.curve.initial_token_a_price_denominator = initial_token_a_price_denominator;
    pair.bump = bump;
    Ok(())
}
