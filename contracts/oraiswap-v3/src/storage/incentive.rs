use cosmwasm_schema::cw_serde;

use crate::{
    fee_growth::FeeGrowth, interface::AssetInfo, liquidity::Liquidity, token_amount::TokenAmount,
    ContractError,
};

#[cw_serde]
pub struct IncentiveRecord {
    pub id: u64,
    pub reward_per_sec: TokenAmount,
    pub reward_token: AssetInfo,
    pub remaining: TokenAmount,
    pub start_timestamp: u64,
    pub incentive_growth_global: FeeGrowth,
    pub last_updated: u64,
}

#[cw_serde]
#[derive(Eq, Copy, Default)]
pub struct TickIncentive {
    pub incentive_id: u64,
    pub incentive_growth_outside: FeeGrowth,
}

#[cw_serde]
pub struct PositionIncentives {
    pub incentive_id: u64,
    pub pending_rewards: TokenAmount,
    pub incentive_growth_inside: FeeGrowth,
}

impl IncentiveRecord {
    pub fn update_global_incentive_growth(
        &mut self,
        pool_liquidity: Liquidity,
        current_timestamp: u64,
    ) -> Result<(), ContractError> {
        if current_timestamp.lt(&self.start_timestamp) || current_timestamp.lt(&self.last_updated) {
            return Ok(());
        }

        let pass_time = current_timestamp - self.last_updated;

        let mut total_emit = self.reward_per_sec * TokenAmount(pass_time.into());
        if total_emit > self.remaining {
            total_emit = self.remaining;
        };

        let incentive_growth = FeeGrowth::from_fee(pool_liquidity, total_emit);
        match incentive_growth {
            Ok(value) => {
                self.incentive_growth_global += value;
                self.remaining = self.remaining - total_emit;
            }
            Err(_) => {
                //  Do nothing if there is an error when converting the amount to FeeGrowth
                // Potential errors in calculating incentive growth:
                // - overflow
                // - liquidity is zero
            }
        }
        self.last_updated = current_timestamp;

        Ok(())
    }
}

pub fn calculate_incentive_growth_inside(
    tick_lower: i32,
    tick_lower_incentive_growth_outside: FeeGrowth,
    tick_upper: i32,
    tick_upper_incentive_growth_outside: FeeGrowth,
    tick_current: i32,
    incentive_growth_global: FeeGrowth,
) -> FeeGrowth {
    // determine position relative to current tick
    let current_above_lower = tick_current >= tick_lower;
    let current_below_upper = tick_current < tick_upper;

    // calculate fee growth below
    let incentive_growth_below = if current_above_lower {
        tick_lower_incentive_growth_outside
    } else {
        incentive_growth_global.unchecked_sub(tick_lower_incentive_growth_outside)
    };

    // calculate fee growth above
    let incentive_growth_above = if current_below_upper {
        tick_upper_incentive_growth_outside
    } else {
        incentive_growth_global.unchecked_sub(tick_upper_incentive_growth_outside)
    };

    // calculate fee growth inside
    let incentive_growth_inside = incentive_growth_global
        .unchecked_sub(incentive_growth_below)
        .unchecked_sub(incentive_growth_above);

    incentive_growth_inside
}

#[cfg(test)]
mod tests {
    use decimal::*;

    use super::*;

    #[test]
    fn test_update_global_incentive_growth() {
        let mut record = IncentiveRecord {
            id: 0,
            reward_per_sec: TokenAmount(0),
            reward_token: AssetInfo::NativeToken {
                denom: "orai".to_string(),
            },
            remaining: TokenAmount(1000000),
            start_timestamp: 1000,
            incentive_growth_global: FeeGrowth(0),
            last_updated: 1000,
        };
        let mut pool_liquidity = Liquidity::new(1000000);

        // case 1: CurrentTimestamp < start_timestamp => no update
        record
            .update_global_incentive_growth(pool_liquidity, 900)
            .unwrap();
        assert_eq!(record.last_updated, 1000);
        assert_eq!(record.incentive_growth_global, FeeGrowth(0));

        // case 2: CurrentTimestamp < last_updated => no update
        record.last_updated = 1100;
        record
            .update_global_incentive_growth(pool_liquidity, 1099)
            .unwrap();
        assert_eq!(record.last_updated, 1100);
        assert_eq!(record.incentive_growth_global, FeeGrowth(0));

        // case 3: liquidity = 0 => still success, but don;t update incentive_growth_global
        pool_liquidity = Liquidity::new(0);
        record.reward_per_sec = TokenAmount(100);
        record
            .update_global_incentive_growth(pool_liquidity, 1200)
            .unwrap();
        assert_eq!(record.last_updated, 1200);
        assert_eq!(record.remaining, TokenAmount(1000000));
        assert_eq!(record.incentive_growth_global, FeeGrowth(0));

        // case 4: overflow => still success, but don;t update incentive_growth_global
        pool_liquidity = Liquidity::new(1);
        record.reward_per_sec = TokenAmount(1000);
        record
            .update_global_incentive_growth(pool_liquidity, 1300)
            .unwrap();
        assert_eq!(record.last_updated, 1300);
        assert_eq!(record.remaining, TokenAmount(1000000));
        assert_eq!(record.incentive_growth_global, FeeGrowth(0));

        // case 4: happy case
        pool_liquidity = Liquidity::new(1000);
        record.reward_per_sec = TokenAmount(100);
        record
            .update_global_incentive_growth(pool_liquidity, 1400)
            .unwrap();
        assert_eq!(record.last_updated, 1400);
        assert_eq!(record.remaining, TokenAmount(990000));
        assert_eq!(
            record.incentive_growth_global,
            FeeGrowth(100000000000000000000000000000000000)
        );

        // case 5: total emit > remaining reward
        pool_liquidity = Liquidity::new(100000);
        record.reward_per_sec = TokenAmount(10000);
        record
            .update_global_incentive_growth(pool_liquidity, 1500)
            .unwrap();
        assert_eq!(record.last_updated, 1500);
        assert_eq!(record.remaining, TokenAmount(0));
        assert_eq!(
            record.incentive_growth_global,
            FeeGrowth(199000000000000000000000000000000000)
        );

        // case 6: no reward remaining
        record
            .update_global_incentive_growth(pool_liquidity, 1600)
            .unwrap();
        assert_eq!(record.last_updated, 1600);
        assert_eq!(record.remaining, TokenAmount(0));
        assert_eq!(
            record.incentive_growth_global,
            FeeGrowth(199000000000000000000000000000000000)
        );
    }

    #[test]
    fn test_calculate_incentive_growth_inside() {
        // <──────────────                    ──────────────>
        // incentive_outside_t0| incentive_growth_inside |incentive_outside_t1
        //<───────────── t0 ────── C ────── t1 ───────────────────>

        // incentive_growth_inside = incentive_growth_global - t0.incentive_outside - t1.incentive_outside

        let incentive_growth_global = FeeGrowth::from_integer(15);

        let tick_lower_index = -2;
        let tick_lower_incentive_growth_outside = FeeGrowth::new(0);

        let tick_upper_index = 2;
        let tick_upper_incentive_growth_outside = FeeGrowth::from_integer(0);

        // current tick inside range
        // lower    current     upper
        // |        |           |
        // -2       0           2
        {
            // index and fee global
            let tick_current = 0;
            let incentive_growth_inside = calculate_incentive_growth_inside(
                tick_lower_index,
                tick_lower_incentive_growth_outside,
                tick_upper_index,
                tick_upper_incentive_growth_outside,
                tick_current,
                incentive_growth_global,
            );

            assert_eq!(incentive_growth_inside, FeeGrowth::from_integer(15)); // x incentive growth inside
        }
        //                      ───────incentive_outside_t0──────────>
        //                     |incentive_growth_inside| incentive_outside_t1
        // ─────── c ─────── t0 ──────────────> t1 ───────────>
        //
        // incentive_growth_inside = t0.incentive_outisde - t1.incentive_outside
        //
        // current tick below range
        // current  lower       upper
        // |        |           |
        // -4       2           2
        {
            let tick_current = -4;
            let incentive_growth_inside = calculate_incentive_growth_inside(
                tick_lower_index,
                tick_lower_incentive_growth_outside,
                tick_upper_index,
                tick_upper_incentive_growth_outside,
                tick_current,
                incentive_growth_global,
            );

            assert_eq!(incentive_growth_inside, FeeGrowth::new(0)); // incentive growth inside
        }

        // <──────────incentive_outside_t0──────────
        // incentive_outside_t1  | incentive_growth_inside|
        // ────────────── t1 ──────────────── t0 ─────── c ───────────>

        // incentive_growth_inside = t0.incentive_outisde - t1.incentive_outside

        // current tick upper range
        // lower    upper       current
        // |        |           |
        // -2       2           4
        {
            let tick_current = 4;
            let incentive_growth_inside = calculate_incentive_growth_inside(
                tick_lower_index,
                tick_lower_incentive_growth_outside,
                tick_upper_index,
                tick_upper_incentive_growth_outside,
                tick_current,
                incentive_growth_global,
            );

            assert_eq!(incentive_growth_inside, FeeGrowth::new(0)); // incentive growth inside
        }

        // current tick upper range
        // lower    upper       current
        // |        |           |
        // -2       2           3
        {
            let tick_lower_index = -2;
            let tick_lower_incentive_growth_outside = FeeGrowth::new(0);

            let tick_upper_index = 2;
            let tick_upper_incentive_growth_outside = FeeGrowth::new(1);

            let incentive_growth_global = FeeGrowth::from_integer(5);

            let tick_current = 3;
            let incentive_growth_inside = calculate_incentive_growth_inside(
                tick_lower_index,
                tick_lower_incentive_growth_outside,
                tick_upper_index,
                tick_upper_incentive_growth_outside,
                tick_current,
                incentive_growth_global,
            );

            assert_eq!(incentive_growth_inside, FeeGrowth::new(1)); // incentive growth inside
        }

        // subtracts upper tick if below
        let tick_upper_index = 2;
        let tick_upper_incentive_growth_outside = FeeGrowth::from_integer(2);

        // lower    current     upper
        // |        |           |
        // -2       0           2
        {
            let tick_current = 0;
            let incentive_growth_inside = calculate_incentive_growth_inside(
                tick_lower_index,
                tick_lower_incentive_growth_outside,
                tick_upper_index,
                tick_upper_incentive_growth_outside,
                tick_current,
                incentive_growth_global,
            );

            assert_eq!(incentive_growth_inside, FeeGrowth::from_integer(13)); // incentive growth inside
        }

        // subtracts lower tick if above
        let tick_upper_index = 2;
        let tick_upper_incentive_growth_outside = FeeGrowth::new(0);

        let tick_lower_index = -2;
        let tick_lower_incentive_growth_outside = FeeGrowth::from_integer(2);

        // current tick inside range
        // lower    current     upper
        // |        |           |
        // -2       0           2
        {
            let tick_current = 0;
            let incentive_growth_inside = calculate_incentive_growth_inside(
                tick_lower_index,
                tick_lower_incentive_growth_outside,
                tick_upper_index,
                tick_upper_incentive_growth_outside,
                tick_current,
                incentive_growth_global,
            );

            assert_eq!(incentive_growth_inside, FeeGrowth::from_integer(13)); // incentive growth inside
        }
    }

    #[test]
    fn test_domain_calculate_incentive_growth_inside() {
        let tick_current = 0;
        let incentive_growth_global = FeeGrowth::from_integer(20);

        let tick_lower_index = -20;
        let tick_lower_incentive_growth_outside = FeeGrowth::from_integer(20);

        let tick_upper_index = -10;
        let tick_upper_incentive_growth_outside = FeeGrowth::from_integer(15);

        let incentive_growth_inside = calculate_incentive_growth_inside(
            tick_lower_index,
            tick_lower_incentive_growth_outside,
            tick_upper_index,
            tick_upper_incentive_growth_outside,
            tick_current,
            incentive_growth_global,
        );

        assert_eq!(
            incentive_growth_inside,
            FeeGrowth::max_instance() - FeeGrowth::from_integer(5) + FeeGrowth::new(1)
        );
        assert_eq!(
            incentive_growth_inside,
            FeeGrowth::max_instance() - FeeGrowth::from_integer(5) + FeeGrowth::new(1)
        );
    }
}
