use anchor_lang::prelude::*;

use crate::dfs_precise_number::DFSPreciseNumber;

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct LinearPriceCurve {
    /// Slope of price increase (how much price of token B increases for every token A that's bonded to it) numerator
    pub slope_numerator: u64,
    /// Slope of price increase (how much price of token B increases for every token A that's bonded to it) denominator
    pub slope_denominator: u64,
    /// When there's 0 liquidity in the pool, what should the initial price point a0 defining the curve be?
    /// i.e. what is the cost of 1 b token (denominated in A) when there's 0 liquidity
    pub initial_token_a_price_numerator: u64,
    /// When there's 0 liquidity in the pool, what should the initial price point a0 defining the curve be?
    /// i.e. what is the cost of 1 b token (denominated in A) when there's 0 liquidity
    pub initial_token_a_price_denominator: u64,
}

fn solve_quadratic_positive_root(
  k_numerator: &DFSPreciseNumber,
  k_denominator: &DFSPreciseNumber,
  e_value_numerator: &DFSPreciseNumber,
  e_value_denominator: &DFSPreciseNumber,
  lhs_value: &DFSPreciseNumber,
  should_round_sqrt_up: bool,
) -> Option<DFSPreciseNumber> {
  // solve positive root of 0 = k*x^2 + e*x + c, where c == -lhs_value
  // => x = (-e + sqrt(e^2 - 4kc)) / 2k
  // => x = (sqrt(e^2 + 4*k*lhs) - e) / 2k

  // k * 4 * lhs
  let four_k_lhs = k_numerator
      .checked_mul(&(DFSPreciseNumber::new(4)?))?
      .checked_mul(lhs_value)?
      .checked_div(k_denominator)?;

  // e^2 + k * 4 * lhs
  let e2_plus_4_k_lhs = e_value_numerator
      .checked_mul(e_value_numerator)?
      .checked_div(e_value_denominator)?
      .checked_div(&e_value_denominator)?
      .checked_add(&four_k_lhs)?;

  // note we have to use u64 sqrt below (~10K compute) since PreciseNumber::sqrt (~100K compute)
  // and u128 sqrt (~50K compute) are both too expensive
  let sqrt_e2_plus_4_k_lhs = e2_plus_4_k_lhs.sqrt_u64(should_round_sqrt_up)?;

  // numerator is sqrt(e^2 + 4*k*lhs) - e
  let e_value = e_value_numerator.checked_div(e_value_denominator)?;
  // due to sqrt rounding, sometimes this None's if we rounded down the sqrt, so treat that as 0
  let numerator = match sqrt_e2_plus_4_k_lhs.checked_sub(&e_value) {
      Some(val) => val,
      None => DFSPreciseNumber::new(0)?,
  };

  // finally we return (sqrt(e^2-4kc) - e)/2k,
  // AKA numerator * k_denominator / k_numerator / 2 (do all the division last)
  numerator
      .checked_mul(k_denominator)?
      .checked_div(&k_numerator)?
      .checked_div(&(DFSPreciseNumber::new(2)?))
}

impl LinearPriceCurve {
    fn amt_a_locked_at_b_value_quadratic(
        &self,
        b_value: &DFSPreciseNumber
    ) -> Option<DFSPreciseNumber> {
        // The liquidity integral is `token_a_bonded = 0.5m*b^2 + a0*b + 0` (integration constant is 0 since we know
        // there's 0 token A bonded at b = 0)

        // 0.5 * m * b^2
        let half_m_b_squared = DFSPreciseNumber::new(self.slope_numerator.into())?
            .checked_mul(b_value)?
            .checked_mul(b_value)?
            .checked_div(&DFSPreciseNumber::new(self.slope_denominator.into())?)?
            .checked_div(&DFSPreciseNumber::new(2)?)?;

        // a0 * b (note a0 and b are always positive) - make sure to do division last
        let a0_times_b = DFSPreciseNumber::new(self.initial_token_a_price_numerator.into())?
            .checked_mul(b_value)?
            .checked_div(&DFSPreciseNumber::new(self.initial_token_a_price_denominator.into())?)?;

        half_m_b_squared.checked_add(&a0_times_b)
    }

    /// Returns the positive root for token_a_amount = 0.5m*b^2 + a0*b + 0
    /// (integration constant is always 0 since we know there's 0 token A bonded at b = 0)
    fn b_value_with_amt_a_locked_quadratic(
        &self,
        token_a_amount: &DFSPreciseNumber,
        should_round_sqrt_up: bool
    ) -> Option<DFSPreciseNumber> {
        // (We're using k/e for quadratic coefficients instead of a/b to not clash with token a/b names)

        // k = 0.5 * m
        // Note k is kept as a fraction since pre-dividing PreciseNumber loses a lot of
        // precision (only 12 decimal digits max) - we're going to be multiplying it against prices (k*b^2) so
        // no need to lose that precision (and as long as slope_numerator/price are all u64 there's plenty of
        // room in PreciseNumber to avoid overflow)
        let slope_numerator = DFSPreciseNumber::new(self.slope_numerator.into())?;
        let slope_denominator = DFSPreciseNumber::new(self.slope_denominator.into())?;
        let k_numerator = slope_numerator.checked_mul(&DFSPreciseNumber::new(1)?)?;
        let k_denominator = slope_denominator.checked_mul(&DFSPreciseNumber::new(2)?)?;

        // e = a0
        let e_value_numerator = DFSPreciseNumber::new(self.initial_token_a_price_numerator.into())?;
        let e_value_denominator = DFSPreciseNumber::new(
            self.initial_token_a_price_denominator.into()
        )?;

        // solve 0 = k*x^2 + e*x - token_a_amount
        solve_quadratic_positive_root(
            &k_numerator,
            &k_denominator,
            &e_value_numerator,
            &e_value_denominator,
            &token_a_amount,
            should_round_sqrt_up
        )
    }

    /// If `source_amount` will cause the swap to return all of its remaining `swap_destination_amount`,
    /// this returns the (maximum_token_a_amount, swap_destination_amount) that the swap can take
    /// Otherwise (if there's enough `swap_destination_amount` to handle all the `source_amount`), returns None
    fn maximum_a_remaining_for_swap_a_to_b(
        &self,
        a_start: &DFSPreciseNumber,
        b_start: &DFSPreciseNumber,
        source_amount: u128,
        swap_destination_amount: u128
    ) -> Option<(u128, u128)> {
        // if at b_start + swap_destination_amount (the maximum B that be given out by the swap),
        // then the A value is <= source_amount, so only take that amount of A instead and give them all the
        // Bs remaining
        let maximum_b_value = b_start.checked_add(
            &DFSPreciseNumber::new(swap_destination_amount)?
        )?;
        let maximum_a_locked = self.amt_a_locked_at_b_value_quadratic(&maximum_b_value)?;
        let maximum_a_remaining = maximum_a_locked.checked_sub(&a_start)?.to_imprecise()?;

        if maximum_a_remaining <= source_amount {
            return Some((maximum_a_remaining, swap_destination_amount));
        } else {
            return None;
        }
    }

    /// Swap's in user's collateral token and returns out the bonded token,
    /// moving right on the price curve and increasing the price of the bonded token
    pub fn swap_a_to_b(
        &self,
        source_amount: u128, // amount of user's token a (collateral token)
        swap_source_amount: u128, // swap's token a (collateral token)
        swap_destination_amount: u128 // swap's remaining token b (bonded token)
    ) -> Option<(u128, u128)> {
        // use swap_source_amount (collateral token) to determine where we are on the integration curve
        // note this only works if non-init deposits are disabled (and maybe if the initial deposit didn't have any token A in it?),
        // otherwise there could be some A token in the pool that isn't part of the bonding curve

        // quadratic formula version:
        let a_start = DFSPreciseNumber::new(swap_source_amount)?;

        let b_start = self.b_value_with_amt_a_locked_quadratic(&a_start, true)?;

        match
            self.maximum_a_remaining_for_swap_a_to_b(
                &a_start,
                &b_start,
                source_amount,
                swap_destination_amount
            )
        {
            Some(val) => {
                return Some(val);
            }
            // no need to return None here if checked_add fails, can just skip this check and do real calculation below
            None => (),
        }

        // otherwise, there's enough B tokens for all the A they put in, find the b_end value for the amount of A
        // they're putting in and give them `b_end - b_start` tokens out
        let a_end = a_start.checked_add(&DFSPreciseNumber::new(source_amount)?)?;

        let b_end = self.b_value_with_amt_a_locked_quadratic(&a_end, false)?;

        let difference = b_end.checked_sub(&b_start)?;
        // PreciseNumber rounds .5+ up by default, make sure to floor instead so we don't allow
        // dust to round up for free
        let destination_amount = difference.floor()?.to_imprecise()?;

        Some((source_amount, destination_amount))
    }

    pub fn swap_b_to_a(
        &self,
        source_amount: u128,
        _swap_source_amount: u128,
        swap_destination_amount: u128
    ) -> Option<(u128, u128)> {
        // use swap_destination_amount (collateral token) to determine where we are on the integration curve
        // note this only works if non-init deposits are disabled (and maybe if the initial deposit didn't have any token A in it?),
        // otherwise there could be some A token in the pool that isn't part of the bonding curve

        // make sure we round up here so that b_end and a_end are also over-estimated, which rounds down the final
        // token a output
        let b_start = self.b_value_with_amt_a_locked_quadratic(
            &DFSPreciseNumber::new(swap_destination_amount)?,
            true
        )?;

        // b_end can be negative if the user put in too many B tokens (handled below)
        let (b_end, b_end_is_negative) = b_start.unsigned_sub(
            &DFSPreciseNumber::new(source_amount)?
        );

        // make sure to use b_end.ceiling() when doing below calculations a_end so we don't round in favor of the user
        // if we use b_end directly, it's possible to gain tokens for free by swapping back and forth due to
        // rounding (see swap_large_price_a_u32 test)
        // (especially since sqrt_babylonian under estimates, we often will end up with a b_end/a_end that's too low
        // due to rounding)
        let b_end = b_end.ceiling()?;

        // if b_end < 0 (i.e. there aren't enough A tokens in the swap for all the B tokens they put in),
        // then just give them all of the a tokens (swap_destination_amount) and only take the B tokens required to
        // get down from b_start to 0. this only works if we assume 0 A locked at b = 0
        if b_end_is_negative {
            return Some((b_start.to_imprecise()?, swap_destination_amount));
        }

        // otherwise if there's enough A tokens locked in swap_destination_amount, figure out the A value at
        // b_end and give them the difference (swap_destination_amount - a_end) tokens
        let a_end = self.amt_a_locked_at_b_value_quadratic(&b_end)?;

        // PreciseNumber rounds .5+ up by default, make sure to floor instead so we don't allow
        // dust to round up for free
        let destination_amount = DFSPreciseNumber::new(swap_destination_amount)?
            .checked_sub(&a_end)?
            .floor()?
            .to_imprecise()?;

        Some((source_amount, destination_amount))
    }
}

pub fn to_u128(val: u64) -> Result<u128> {
    val.try_into()
        .map_err(|_| crate::error::SwapError::ConversionFailure.into())
}

pub fn to_u64(val: u128) -> Result<u64> {
    val.try_into()
        .map_err(|_| crate::error::SwapError::ConversionFailure.into())
}