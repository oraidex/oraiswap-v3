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
