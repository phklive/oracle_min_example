extern crate alloc;

use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_lib::transaction::TransactionKernel;
use miden_objects::assembly::Library;

use std::sync::{Arc, LazyLock};

pub static ORACLE_COMPONENT_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    let source_manager = Arc::new(DefaultSourceManager::default());
    let oracle_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle_component::oracle_module").unwrap(),
            ORACLE_ACCOUNT_CODE,
            &source_manager,
        )
        .unwrap();

    assembler
        .assemble_library([oracle_component_module])
        .expect("assembly should succeed")
});

pub const PUSH_DATA_TX_SCRIPT: &str = r#"
    use.oracle_component::oracle_module

    begin
        push.{1}
        push.{2}
        push.{3}
        push.{4}

        call.oracle_module::push_oracle_data

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
