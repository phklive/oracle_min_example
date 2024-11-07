extern crate alloc;

use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey}, EMPTY_WORD,
};
use miden_lib::{transaction::TransactionKernel, accounts::auth::RpoFalcon512};
use miden_objects::{
    accounts::{Account, AccountId, AccountComponent, StorageSlot},
    assets::AssetVault,
    transaction::{TransactionArgs, TransactionScript},
    Felt, Word,
};
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use std::sync::Arc;

pub const PUSH_DATA_TX_SCRIPT: &str = r#"
    begin
        push.{1}
        push.{2}
        push.{3}
        push.{4}

        call.[1]

        dropw dropw dropw dropw

        call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
        drop
    end
    "#;

pub const ORACLE_ACCOUNT_CODE: &str = r#"
    use.miden::account

    #! Pushes new price data into the oracle's data slots.
    #!
    #! Inputs:  [WORD_1, WORD_2, WORD_3, WORD_4]
    #! Outputs: []
    #!
    export.push_oracle_data
        push.0
        exec.account::set_item
        dropw
        # => [WORD_2, WORD_3, WORD_4]

        push.1
        exec.account::set_item
        dropw
        # => [WORD_3, WORD_4]

        push.2
        exec.account::set_item
        dropw
        # => [WORD_4]

        push.3
        exec.account::set_item
        dropw
        # => []
    end
"#;


#[test]
fn oracle_account_creation_and_pushing_data_to_read() {
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let oracle_account_id =
        AccountId::try_from(10376293541461622847_u64).unwrap();

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

    let word_1 = data_to_word(&oracle_data_1);
    let word_2 = data_to_word(&oracle_data_2);
    let word_3 = data_to_word(&oracle_data_3);
    let word_4 = data_to_word(&oracle_data_4);


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
            .replace("{1}", &word_to_masm(&word_1))
            .replace("{2}", &word_to_masm(&word_2))
            .replace("{3}", &word_to_masm(&word_3))
            .replace("{4}", &word_to_masm(&word_4))
            .replace(
                "[1]",
                &format!("{}", oracle_account.code().procedures()[1].mast_root()).to_string()
            )
    );

    println!("Push tx script code: {}", push_tx_script_code);
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let push_tx_script =
        TransactionScript::compile(push_tx_script_code, [], assembler.clone()).unwrap();
    let txn_args = TransactionArgs::with_tx_script(push_tx_script);

    let executed_transaction = executor
        .execute_transaction(oracle_account.id(), block_ref, &[], txn_args)
        .unwrap();

    // check that now the account has the data stored in its storage at slot 2
    println!("Account Delta: {:?}", executed_transaction.account_delta());
}


pub fn get_new_pk_and_authenticator(
) -> (Word, std::sync::Arc<dyn miden_tx::auth::TransactionAuthenticator>) {
    use alloc::sync::Arc;

    use miden_objects::accounts::AuthSecretKey;
    use miden_tx::auth::{BasicAuthenticator, TransactionAuthenticator};
    use rand::rngs::StdRng;

    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, Arc::new(authenticator) as Arc<dyn TransactionAuthenticator>)
}

fn get_oracle_account(oracle_public_key: Word, oracle_account_id: AccountId) -> Account {

    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    // This component supports all types of accounts for testing purposes.
    let oracle_component = AccountComponent::compile(
        ORACLE_ACCOUNT_CODE,
        assembler.clone(),
        vec![StorageSlot::Value(Word::default()); 4],
    )
    .unwrap()
    .with_supports_all_types();

    let (oracle_account_code, oracle_account_storage) = Account::initialize_from_components(
        oracle_account_id.account_type(),
        &[RpoFalcon512::new(PublicKey::new(oracle_public_key)).into(), oracle_component],
    )
    .unwrap();

    let oracle_account_vault = AssetVault::new(&[]).unwrap();

    Account::from_parts(
        oracle_account_id,
        oracle_account_vault,
        oracle_account_storage,
        oracle_account_code,
        Felt::new(1),
    )
}


pub struct OracleData {
    pub asset_pair: String, // store ASCII strings of up to 8 characters as the asset pair
    pub price: u64,
    pub decimals: u64,
    pub publisher_id: u64,
}

/// Word to MASM
pub fn word_to_masm(word: &Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Data to Word
pub fn data_to_word(data: &OracleData) -> Word {
    let mut word = EMPTY_WORD;

    // Asset pair
    let asset_pair_u32 =
        encode_asset_pair_to_u32(&data.asset_pair).expect("Invalid asset pair format");
    word[0] = Felt::new(asset_pair_u32 as u64);

    // Price
    word[1] = Felt::new(data.price);

    // Decimals
    word[2] = Felt::new(data.decimals);

    // Publisher ID
    word[3] = Felt::new(data.publisher_id);

    word
}

/// Encode asset pair string to u32
/// Only need to handle uppercase A-Z and '/' for asset pairs
pub fn encode_asset_pair_to_u32(s: &str) -> Option<u32> {
    // Validate input format
    if s.len() < 7 || s.len() > 8 || s.chars().nth(3) != Some('/') {
        return None;
    }

    let mut result: u32 = 0;
    let mut pos = 0;

    // First part (XXX) - 3 chars, 5 bits each = 15 bits
    for c in s[..3].chars() {
        let value = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            _ => return None,
        };
        result |= value << (pos * 5);
        pos += 1;
    }

    // Skip the '/' separator - we know it's position
    pos = 3;

    // Second part (YYY[Y]) - 3-4 chars, 5 bits each = 15-20 bits
    for c in s[4..].chars() {
        let value = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            _ => return None,
        };
        result |= value << (pos * 5);
        pos += 1;
    }

    Some(result)
}