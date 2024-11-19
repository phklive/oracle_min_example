extern crate alloc;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::AccountId,
    transaction::{TransactionArgs, TransactionScript},
};

use miden_oracle::{
    constants::{ORACLE_COMPONENT_LIBRARY, PUSH_DATA_TX_SCRIPT},
    data::OracleData,
    utils::{get_new_pk_and_authenticator, get_oracle_account, word_to_masm},
};
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};
use std::sync::Arc;

#[test]
fn test_oracle_write() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();

    let mut oracle_account = get_oracle_account(oracle_pub_key, oracle_account_id);

    // Construct oracle data
    let oracle_data_1 = OracleData {
        asset_pair: "BTC/USD".to_string(),
        price: 50000,
        decimals: 2,
        publisher_id: 1,
    };

    let oracle_data_2 = OracleData {
        asset_pair: "ETH/USD".to_string(),
        price: 10000,
        decimals: 2,
        publisher_id: 1,
    };

    let oracle_data_3 = OracleData {
        asset_pair: "SOL/USD".to_string(),
        price: 2000,
        decimals: 2,
        publisher_id: 1,
    };

    let oracle_data_4 = OracleData {
        asset_pair: "POL/USD".to_string(),
        price: 50,
        decimals: 2,
        publisher_id: 1,
    };

    let oracle_data_word_1 = oracle_data_1.to_word();
    let oracle_data_word_2 = oracle_data_2.to_word();
    let oracle_data_word_3 = oracle_data_3.to_word();
    let oracle_data_word_4 = oracle_data_4.to_word();

    // CONSTRUCT AND EXECUTE TX
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(oracle_account.clone()).build();
    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(oracle_auth.clone()));
    let block_ref = tx_context.tx_inputs().block_header().block_num();

    // Create transaction script to push the data
    let tx_script_code = format!(
        "{}",
        PUSH_DATA_TX_SCRIPT
            .replace("{1}", &word_to_masm(oracle_data_word_1))
            .replace("{2}", &word_to_masm(oracle_data_word_2))
            .replace("{3}", &word_to_masm(oracle_data_word_3))
            .replace("{4}", &word_to_masm(oracle_data_word_4))
            .replace(
                "[1]",
                &format!("{}", oracle_account.code().procedures()[1].mast_root()).to_string()
            )
    );

    let assembler = TransactionKernel::assembler();
    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        // Add the oracle account's component as a library to link
        // against so we can reference the account in the transaction script.
        assembler
            .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
            .expect("adding oracle library should not fail")
            .clone(),
    )
    .unwrap();
    let txn_args = TransactionArgs::with_tx_script(tx_script);

    let executed_transaction = executor
        .execute_transaction(oracle_account.id(), block_ref, &[], txn_args)
        .unwrap();

    oracle_account
        .apply_delta(executed_transaction.account_delta())
        .unwrap();

    // check that the oracle account has successfully been updated with the correct values
    assert_eq!(
        oracle_account.storage().slots()[1].value(),
        oracle_data_word_1
    );
    assert_eq!(
        oracle_account.storage().slots()[2].value(),
        oracle_data_word_2
    );
    assert_eq!(
        oracle_account.storage().slots()[3].value(),
        oracle_data_word_3
    );
    assert_eq!(
        oracle_account.storage().slots()[4].value(),
        oracle_data_word_4
    );
}

#[test]
fn test_oracle_read() {}
