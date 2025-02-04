//! Tests involving client behavior when utxo-validation is enabled

use crate::helpers::{
    TestContext,
    TestSetupBuilder,
};
use fuel_core_interfaces::common::{
    fuel_tx::TransactionBuilder,
    fuel_vm::{
        consts::*,
        prelude::*,
    },
};
use fuel_crypto::SecretKey;
use fuel_gql_client::client::{
    types::TransactionStatus,
    PageDirection,
    PaginationRequest,
};
use futures::future::join_all;
use itertools::Itertools;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};
use std::collections::HashSet;

#[tokio::test]
async fn submit_utxo_verified_tx_with_min_gas_price() {
    let mut rng = StdRng::seed_from_u64(2322);
    let mut test_builder = TestSetupBuilder::new(2322);
    let (_, contract_id) = test_builder.setup_contract(vec![], None);
    // initialize 10 random transactions that transfer coins and call a contract
    let transactions = (1..=10)
        .into_iter()
        .map(|i| {
            TransactionBuilder::script(
                Opcode::RET(REG_ONE).to_bytes().into_iter().collect(),
                vec![],
            )
            .gas_limit(100)
            .gas_price(1)
            .add_unsigned_coin_input(
                SecretKey::random(&mut rng),
                rng.gen(),
                1000 + i,
                Default::default(),
                Default::default(),
                0,
            )
            .add_input(Input::Contract {
                utxo_id: Default::default(),
                balance_root: Default::default(),
                state_root: Default::default(),
                tx_pointer: Default::default(),
                contract_id,
            })
            .add_output(Output::Change {
                amount: 0,
                asset_id: Default::default(),
                to: rng.gen(),
            })
            .add_output(Output::Contract {
                input_index: 1,
                balance_root: Default::default(),
                state_root: Default::default(),
            })
            .finalize()
        })
        .collect_vec();

    // setup genesis block with coins that transactions can spend
    test_builder.config_coin_inputs_from_transactions(&transactions.iter().collect_vec());

    // spin up node
    let TestContext { client, .. } = test_builder.finalize().await;

    // submit transactions and verify their status
    for tx in transactions {
        let id = client.submit(&tx).await.unwrap();
        // verify that the tx returned from the api matches the submitted tx
        let ret_tx = client
            .transaction(&id.0.to_string())
            .await
            .unwrap()
            .unwrap()
            .transaction;

        let transaction_result = client
            .transaction_status(&ret_tx.id().to_string())
            .await
            .ok()
            .unwrap();

        if let TransactionStatus::Success { block_id, .. } = transaction_result.clone() {
            let block_exists = client.block(&block_id).await.unwrap();

            assert!(block_exists.is_some());
        }

        // Once https://github.com/FuelLabs/fuel-core/issues/50 is resolved this should rely on the Submitted Status rather than Success
        assert!(matches!(
            transaction_result,
            TransactionStatus::Success { .. }
        ));
    }
}

#[tokio::test]
async fn submit_utxo_verified_tx_below_min_gas_price_fails() {
    // initialize transaction
    let tx = TransactionBuilder::script(
        Opcode::RET(REG_ONE).to_bytes().into_iter().collect(),
        vec![],
    )
    .gas_limit(100)
    .gas_price(1)
    .finalize();

    // initialize node with higher minimum gas price
    let mut test_builder = TestSetupBuilder::new(2322u64);
    test_builder.min_gas_price = 10;
    let TestContext { client, .. } = test_builder.finalize().await;

    let result = client.submit(&tx).await;
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("The gas price is too low"));
}

// verify that dry run can disable utxo_validation by simulating a transaction with unsigned
// non-existent coin inputs
#[tokio::test]
async fn dry_run_override_utxo_validation() {
    let mut rng = StdRng::seed_from_u64(2322);

    let asset_id = rng.gen();
    let tx = TransactionBuilder::script(
        Opcode::RET(REG_ONE).to_bytes().into_iter().collect(),
        vec![],
    )
    .gas_limit(1000)
    .add_input(Input::coin_signed(
        rng.gen(),
        rng.gen(),
        1000,
        AssetId::default(),
        Default::default(),
        0,
        Default::default(),
    ))
    .add_input(Input::coin_signed(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        asset_id,
        Default::default(),
        0,
        Default::default(),
    ))
    .add_output(Output::change(rng.gen(), 0, asset_id))
    .add_witness(Default::default())
    .finalize();

    let client = TestSetupBuilder::new(2322).finalize().await.client;

    let log = client.dry_run_opt(&tx, Some(false)).await.unwrap();
    assert_eq!(2, log.len());

    assert!(matches!(log[0],
        Receipt::Return {
            val, ..
        } if val == 1));
}

// verify that dry run without utxo-validation override respects the node setting
#[tokio::test]
async fn dry_run_no_utxo_validation_override() {
    let mut rng = StdRng::seed_from_u64(2322);

    let asset_id = rng.gen();
    // construct a tx with invalid inputs
    let tx = TransactionBuilder::script(
        Opcode::RET(REG_ONE).to_bytes().into_iter().collect(),
        vec![],
    )
    .gas_limit(1000)
    .add_input(Input::coin_signed(
        rng.gen(),
        rng.gen(),
        1000,
        AssetId::default(),
        Default::default(),
        0,
        Default::default(),
    ))
    .add_input(Input::coin_signed(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        asset_id,
        Default::default(),
        0,
        Default::default(),
    ))
    .add_output(Output::change(rng.gen(), 0, asset_id))
    .add_witness(Default::default())
    .finalize();

    let client = TestSetupBuilder::new(2322).finalize().await.client;

    // verify that the client validated the inputs and failed the tx
    let res = client.dry_run_opt(&tx, None).await;
    assert!(res.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_tx_submission_produces_expected_blocks() {
    const TEST_TXS: usize = 10;

    let mut rng = StdRng::seed_from_u64(2322u64);
    let mut test_builder = TestSetupBuilder::new(100);

    // generate random txs
    let secret = SecretKey::random(&mut rng);
    let txs = (0..TEST_TXS)
        .into_iter()
        .map(|i| {
            TransactionBuilder::script(
                Opcode::RET(REG_ONE).to_bytes().into_iter().collect(),
                vec![],
            )
            .gas_limit(1000 + i as u64)
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                rng.gen_range(1..1000),
                Default::default(),
                Default::default(),
                0,
            )
            .add_output(Output::change(rng.gen(), 0, Default::default()))
            .finalize()
        })
        .collect_vec();

    // collect all tx ids
    let tx_ids: HashSet<_> = txs.iter().map(|tx| tx.id()).collect();

    // setup the genesis coins for spending
    test_builder.config_coin_inputs_from_transactions(&txs.iter().collect_vec());

    let TestContext { client, .. } = test_builder.finalize().await;

    let tasks = txs
        .into_iter()
        .map(|tx| {
            let client = client.clone();
            async move { client.submit(&tx).await }
        })
        .collect_vec();

    let _: Vec<_> = join_all(tasks)
        .await
        .into_iter()
        .try_collect()
        .expect("expected successful transactions");

    let total_blocks = client
        .blocks(PaginationRequest {
            results: TEST_TXS * 2,
            direction: PageDirection::Forward,
            cursor: None,
        })
        .await
        .unwrap();

    // ensure block heights are all unique
    let deduped = total_blocks
        .results
        .iter()
        .map(|b| b.height.0)
        .dedup()
        .collect_vec();

    // ensure all transactions are included across all the blocks
    let included_txs: HashSet<Bytes32> = total_blocks
        .results
        .iter()
        .flat_map(|b| b.transactions.iter().map(|t| t.id.clone().into()))
        .dedup_with_count()
        .map(|(count, id)| {
            assert_eq!(count, 1, "duplicate tx detected {}", id);
            id
        })
        .collect();

    assert_eq!(total_blocks.results.len(), deduped.len());
    assert_eq!(included_txs, tx_ids);
}
