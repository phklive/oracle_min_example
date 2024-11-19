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
fn test_oracle() {
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();

    let oracle_account = get_oracle_account(oracle_pub_key, oracle_account_id);

    println!("Oracle account: {:?}", oracle_account.code().procedures());

    // CONSTRUCT ORACLE DATA
    // --------------------------------------------------------------------------------------------
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

    let word_1 = oracle_data_1.to_word();
    let word_2 = oracle_data_2.to_word();
    let word_3 = oracle_data_3.to_word();
    let word_4 = oracle_data_4.to_word();

    println!("Oracle data to push : {:?}", word_1);

    // CONSTRUCT AND EXECUTE TX
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(oracle_account.clone()).build();
    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(oracle_auth.clone()));
    let block_ref = tx_context.tx_inputs().block_header().block_num();

    // Create transaction script to push the data
    let push_tx_script_code = format!(
        "{}",
        PUSH_DATA_TX_SCRIPT
            .replace("{1}", &word_to_masm(word_1))
            .replace("{2}", &word_to_masm(word_2))
            .replace("{3}", &word_to_masm(word_3))
            .replace("{4}", &word_to_masm(word_4))
            .replace(
                "[1]",
                &format!("{}", oracle_account.code().procedures()[1].mast_root()).to_string()
            )
    );

    println!("Push tx script code: {}", push_tx_script_code);
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let push_tx_script = TransactionScript::compile(
        push_tx_script_code,
        [],
        // Add the oracle account's component as a library to link
        // against so we can reference the account in the transaction script.
        assembler
            .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
            .expect("adding oracle library should not fail")
            .clone(),
    )
    .unwrap();
    let txn_args = TransactionArgs::with_tx_script(push_tx_script);

    let executed_transaction = executor
        .execute_transaction(oracle_account.id(), block_ref, &[], txn_args)
        .unwrap();

    // check that now the account has the data stored in its storage at slot 2
    println!("Account Delta: {:?}", executed_transaction.account_delta());
}
