use miden_crypto::{Word, ONE, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountBuilder, AccountId, StorageSlot},
    testing::account_component::AccountMockComponent,
    transaction::{TransactionArgs, TransactionScript},
    vm::AdviceInputs,
    Digest,
};

use miden_oracle::{
    constants::{ORACLE_COMPONENT_LIBRARY, WRITE_DATA_TX_SCRIPT},
    data::OracleData,
    utils::{get_new_pk_and_authenticator, get_oracle_account, word_to_masm},
};
use miden_tx::{
    testing::{MockChain, TransactionContextBuilder},
    TransactionExecutor,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::sync::Arc;

#[test]
fn test_oracle_write() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();
    let oracle_storage_slots = vec![StorageSlot::Value(Word::default()); 4];

    let mut oracle_account =
        get_oracle_account(oracle_pub_key, oracle_account_id, oracle_storage_slots);

    let [oracle_data_1, oracle_data_2, oracle_data_3, oracle_data_4] = mock_oracle_data();

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

    // Create transaction script to write the data to the oracle account
    let tx_script_code = format!(
        "{}",
        WRITE_DATA_TX_SCRIPT
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
fn test_oracle_read() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (oracle_pub_key, _) = get_new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();

    let [oracle_data_1, oracle_data_2, oracle_data_3, oracle_data_4] = mock_oracle_data();

    let oracle_data_word_1 = oracle_data_1.to_word();
    let oracle_data_word_2 = oracle_data_2.to_word();
    let oracle_data_word_3 = oracle_data_3.to_word();
    let oracle_data_word_4 = oracle_data_4.to_word();

    let oracle_storage_slots = vec![
        StorageSlot::Value(oracle_data_word_1),
        StorageSlot::Value(oracle_data_word_2),
        StorageSlot::Value(oracle_data_word_3),
        StorageSlot::Value(oracle_data_word_4),
    ];

    let oracle_account =
        get_oracle_account(oracle_pub_key, oracle_account_id, oracle_storage_slots);

    let (native_account, _) = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(TransactionKernel::testing_assembler(), vec![])
                .unwrap(),
        )
        .nonce(ONE)
        .build_testing()
        .unwrap();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), oracle_account.clone()]);

    mock_chain.seal_block(None);

    let advice_inputs = get_mock_fpi_adv_inputs(&oracle_account, &mock_chain);

    let code = format!(
        "
        use.std::sys

        use.miden::tx

        begin
            ### get oracle data 1 ###

            # pad the stack for the `execute_foreign_procedure`execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item` account procedure
            push.{get_item_foreign_hash}

            # push the foreign account id
            push.{foreign_account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{oracle_data_1} assert_eqw
            # => []

            ### get oracle data 2 ###

            # pad the stack for the `execute_foreign_procedure`execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.1

            # get the hash of the `get_item` account procedure
            push.{get_item_foreign_hash}

            # push the foreign account id
            push.{foreign_account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{oracle_data_2} assert_eqw
            # => []

            ### get oracle data 3 ###

            # pad the stack for the `execute_foreign_procedure`execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.2

            # get the hash of the `get_item` account procedure
            push.{get_item_foreign_hash}

            # push the foreign account id
            push.{foreign_account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{oracle_data_3} assert_eqw
            # => []

            ### get oracle data 4 ###

            # pad the stack for the `execute_foreign_procedure`execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.3

            # get the hash of the `get_item` account procedure
            push.{get_item_foreign_hash}

            # push the foreign account id
            push.{foreign_account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{oracle_data_4} assert_eqw
            # => []

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        foreign_account_id = oracle_account.id(),
        get_item_foreign_hash = oracle_account.code().procedures()[1].mast_root(),
        oracle_data_1 = &word_to_masm(oracle_data_word_1),
        oracle_data_2 = &word_to_masm(oracle_data_word_2),
        oracle_data_3 = &word_to_masm(oracle_data_word_3),
        oracle_data_4 = &word_to_masm(oracle_data_word_4)
    );

    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let mut executor: TransactionExecutor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), None).with_tracing();

    // load the mast forest of the foreign account's code to be able to create an account procedure
    // index map and execute the specified foreign procedure
    executor.load_account_code(oracle_account.code());

    executor
        .execute_transaction(
            native_account.id(),
            block_ref,
            &note_ids,
            tx_context.tx_args().clone(),
        )
        .map_err(|e| e.to_string())
        .unwrap();
}

// HELPER FUNCTIONS
// ================================================================================================

fn mock_oracle_data() -> [OracleData; 4] {
    [
        OracleData {
            asset_pair: "BTC/USD".to_string(),
            price: 50000,
            decimals: 2,
            publisher_id: 1,
        },
        OracleData {
            asset_pair: "ETH/USD".to_string(),
            price: 10000,
            decimals: 2,
            publisher_id: 1,
        },
        OracleData {
            asset_pair: "SOL/USD".to_string(),
            price: 2000,
            decimals: 2,
            publisher_id: 1,
        },
        OracleData {
            asset_pair: "POL/USD".to_string(),
            price: 50,
            decimals: 2,
            publisher_id: 1,
        },
    ]
}

fn get_mock_fpi_adv_inputs(foreign_account: &Account, mock_chain: &MockChain) -> AdviceInputs {
    let foreign_id_root = Digest::from([foreign_account.id().into(), ZERO, ZERO, ZERO]);
    let foreign_id_and_nonce = [
        foreign_account.id().into(),
        ZERO,
        ZERO,
        foreign_account.nonce(),
    ];
    let foreign_vault_root = foreign_account.vault().commitment();
    let foreign_storage_root = foreign_account.storage().commitment();
    let foreign_code_root = foreign_account.code().commitment();

    let mut inputs = AdviceInputs::default()
        .with_map([
            // ACCOUNT_ID |-> [ID_AND_NONCE, VAULT_ROOT, STORAGE_ROOT, CODE_ROOT]
            (
                foreign_id_root,
                [
                    &foreign_id_and_nonce,
                    foreign_vault_root.as_elements(),
                    foreign_storage_root.as_elements(),
                    foreign_code_root.as_elements(),
                ]
                .concat(),
            ),
            // STORAGE_ROOT |-> [[STORAGE_SLOT_DATA]]
            (
                foreign_storage_root,
                foreign_account.storage().as_elements(),
            ),
            // CODE_ROOT |-> [[ACCOUNT_PROCEDURE_DATA]]
            (foreign_code_root, foreign_account.code().as_elements()),
        ])
        .with_merkle_store(mock_chain.accounts().into());

    for slot in foreign_account.storage().slots() {
        // if there are storage maps, we populate the merkle store and advice map
        if let StorageSlot::Map(map) = slot {
            // extend the merkle store and map with the storage maps
            inputs.extend_merkle_store(map.inner_nodes());
            // populate advice map with Sparse Merkle Tree leaf nodes
            inputs.extend_map(
                map.leaves()
                    .map(|(_, leaf)| (leaf.hash(), leaf.to_elements())),
            );
        }
    }

    inputs
}
