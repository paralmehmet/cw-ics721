use cosmwasm_std::{from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Env, StdResult};
use ics721::{
    execute::Ics721Execute,
    state::CollectionData,
    token_types::Class,
    utils::{convert_owner_chain_address, get_collection_data},
};
use sg721_base::msg::{CollectionInfoResponse, QueryMsg};

use crate::state::{SgCollectionData, SgIcs721Contract};

impl Ics721Execute for SgIcs721Contract {
    type ClassData = SgCollectionData;

    /// sg-ics721 sends custom SgCollectionData, basically it extends ics721-base::state::CollectionData with additional collection_info.
    fn get_class_data(&self, deps: &DepsMut, sender: &Addr) -> StdResult<Option<Self::ClassData>> {
        let CollectionData {
            owner,
            contract_info,
            name,
            symbol,
            num_tokens,
        } = get_collection_data(deps, sender)?;
        let collection_info: CollectionInfoResponse = deps
            .querier
            .query_wasm_smart(sender, &QueryMsg::CollectionInfo {})?;

        Ok(Some(SgCollectionData {
            owner,
            contract_info,
            name,
            symbol,
            num_tokens,
            collection_info,
        }))
    }

    fn init_msg(&self, deps: Deps, env: &Env, class: &Class) -> StdResult<Binary> {
        // ics721 creator is used, in case no source owner in class data is provided (e.g. due to nft-transfer module).
        let ics721_contract_info = deps
            .querier
            .query_wasm_contract_info(env.contract.address.to_string())?;
        let mut instantiate_msg = sg721::InstantiateMsg {
            // source chain may not send optional collection data
            // if not, by default class id is used for name and symbol
            name: class.id.clone().into(),
            symbol: class.id.clone().into(),
            minter: env.contract.address.to_string(),
            collection_info: sg721::CollectionInfo {
                creator: ics721_contract_info.creator,
                description: "".to_string(),
                image: "https://arkprotocol.io".to_string(),
                external_link: None,
                explicit_content: None,
                start_trading_time: None,
                royalty_info: None,
            },
        };

        // unwrapped to collection data and in case of success, set creator, name and symbol
        if let Some(binary) = class.data.clone() {
            let class_data_result: StdResult<CollectionData> = from_json(binary);
            if class_data_result.is_ok() {
                let class_data = class_data_result?;
                match class_data.owner {
                    Some(owner) =>
                    // owner from source chain is used
                    {
                        instantiate_msg.collection_info.creator =
                            convert_owner_chain_address(env, owner.as_str())?
                    }
                    None =>
                    // ics721 creator is used, in case of none
                    {
                        let ics721_contract_info = deps
                            .querier
                            .query_wasm_contract_info(env.contract.address.to_string())?;
                        instantiate_msg.collection_info.creator = ics721_contract_info.creator;
                    }
                }
                // set name and symbol
                instantiate_msg.symbol = class_data.symbol;
                instantiate_msg.name = class_data.name;
            }
        }

        to_json_binary(&instantiate_msg)
    }
}