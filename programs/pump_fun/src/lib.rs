pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod dfs_precise_number;
pub mod curve;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("79UwNMhNjXyKXAANRjjVnndagJPtEhadWseBpFuMrboF");

#[program]
pub mod pump_fun {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_lamports: u64) -> Result<()> {
        initialize::handler(ctx, fee_lamports)
    }
    pub fn create_token(ctx: Context<CreateToken>, supply: u64, bump: u8) -> Result<()> {
        create_token_handler(ctx, supply, bump)
    }

    pub fn initialize_linear_price(
        ctx: Context<InitializeLinearPrice>,
        slope_numerator: u64,
        slope_denominator: u64,
        initial_token_a_price_numerator: u64,
        initial_token_a_price_denominator: u64,
        bump: u8
    ) -> Result<()> {
        initialize_linear_price_handler(
            ctx,
            slope_numerator,
            slope_denominator,
            initial_token_a_price_numerator,
            initial_token_a_price_denominator,
            bump
        )
    }

    pub fn swap_to_token(ctx: Context<SwapToToken>, amount_in: u64) -> Result<()> {
        swap_to_token_handler(ctx, amount_in)
    }

    pub fn swap_to_sol(ctx: Context<SwapToSol>, amount_in: u64) -> Result<()> {
        swap_to_sol_handler(ctx, amount_in)
    }

    pub fn proxy_initialize(
        ctx: Context<ProxyInitialize>,
        init_amount_1: u64,
        open_time: u64
    ) -> Result<()> {
        proxy_initialize_handler(ctx, init_amount_1, open_time)
    }

    pub fn create_account(ctx: Context<CreateAccount>) -> Result<()> {
        create_account_handler(ctx)
    }
}
