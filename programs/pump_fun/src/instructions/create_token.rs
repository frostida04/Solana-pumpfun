use anchor_lang::{ prelude::*, solana_program::{ program::invoke, system_instruction } };
use anchor_spl::{ token_2022::{ self, MintTo }, token_interface };

use std::mem::size_of;
use crate::{
    AppStats,
    TokenCreate,
    APP_STATS_SEED,
    AUTHORITY_SEED,
    TOKEN_ACCOUNT_SEED,
    TOKEN_CREATE_SEED,
};

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    // authority pda
    /// CHECK:
    #[account(seeds = [AUTHORITY_SEED, mint.key().as_ref()], bump)]
    pub authority: AccountInfo<'info>,

    // mint address
    #[account(
        mut,
        mint::authority = authority,
        token::token_program = token_program
    )]
    pub mint: Box<InterfaceAccount<'info, token_interface::Mint>>,

    // token account for pda
    #[account(
        init,
        payer = creator,
        token::authority = authority,
        token::mint = mint,
        token::token_program = token_program,
        seeds = [TOKEN_ACCOUNT_SEED, mint.key().as_ref()],
        bump
    )]
    pub token_account_for_pda: Box<InterfaceAccount<'info, token_interface::TokenAccount>>,

    // token create account
    #[account(
        init,
        payer = creator,
        space = size_of::<TokenCreate>() + 8,
        seeds = [TOKEN_CREATE_SEED, mint.key().as_ref()],
        bump
    )]
    pub token_create: Box<Account<'info, TokenCreate>>,

    // fee account
    /// CHECK:
    #[account(
      mut,
      constraint = fee_account.key() == app_stats.fee_account,
    )]
    pub fee_account: AccountInfo<'info>,

    // app state account
    #[account(seeds = [APP_STATS_SEED], bump)]
    pub app_stats: Box<Account<'info, AppStats>>,
    pub token_program: Interface<'info, token_interface::TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateToken<'info> {
    fn transfer_fee(&self, fee: u64) -> Result<()> {
        invoke(
            &system_instruction::transfer(self.creator.key, self.fee_account.key, fee),
            &[
                self.creator.to_account_info(),
                self.fee_account.clone(),
                self.system_program.to_account_info(),
            ]
        ).map_err(Into::into)
    }

    fn mint_ctx(&self) -> CpiContext<'info, 'info, 'info, 'info, MintTo<'info>> {
        CpiContext::new(self.token_program.to_account_info(), MintTo {
            mint: self.mint.to_account_info(),
            to: self.token_account_for_pda.to_account_info(),
            authority: self.authority.to_account_info(),
        })
    }
}


/*
    create token function
    @param 
    supply: token supply amount
    bump: authority pda bump
    
*/

pub fn create_token_handler(
    ctx: Context<CreateToken>,
    supply: u64,
    bump: u8
) -> Result<()> {
    ctx.accounts.transfer_fee(ctx.accounts.app_stats.fee_lamports)?;
    let seeds: &[&[u8]; 3] = &[
        AUTHORITY_SEED,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[bump],
    ];
    let signer_seeds: &[&[&[u8]]; 1] = &[&seeds[..]];
    token_2022::mint_to(ctx.accounts.mint_ctx().with_signer(signer_seeds), supply)?;
    let token_create: &mut Box<Account<TokenCreate>> = &mut ctx.accounts.token_create;
    token_create.creator = ctx.accounts.creator.key();
    token_create.supply = ctx.accounts.mint.supply;
    token_create.bump = bump;
    Ok(())
}
