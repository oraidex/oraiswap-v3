use cosmwasm_schema::write_api;
use oraiswap_v3_common::incentives_fund_manager::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg
    }
}
