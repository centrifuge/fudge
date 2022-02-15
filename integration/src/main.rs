use tracing::info;
use tracing_subscriber;
use polkadot_runtime::{Block as TestBlock, RuntimeApi as TestRtApi, WASM_BINARY as CODE, Runtime, SignedExtra};
use sc_client_db::Backend;
use sc_executor::{WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{LocalCallExecutor, TaskManager, TFullClient};
use sp_runtime::{AccountId32, CryptoTypeId, KeyTypeId, MultiAddress, Storage};
use fudge_core::{EnvProvider, RelaychainBuilder};
use tokio::runtime::{Handle};
use sc_executor::sp_wasm_interface::HostFunctions;
use frame_support::inherent::{BlockT, InherentData, InherentIdentifier};
use frame_support::sp_runtime::app_crypto::sp_core::H256;
use sp_inherents::{Error, InherentDataProvider};
use fudge_core::inherent_provider::FakeTimestamp;
use sp_keystore::{SyncCryptoStore};
use sp_runtime::traits::HashFor;
use sp_storage::well_known_keys::CODE as CODE_KEY;
use polkadot_primitives::v1::{InherentData as ParachainsInherentData};
use sp_runtime::generic::BlockId;
use sp_runtime::sp_std::sync::Arc;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");
const CRYPTO_TYPE: CryptoTypeId = CryptoTypeId(*b"test");

#[tokio::main]
async fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let key_store = sc_keystore::LocalKeystore::in_memory();

    let mut host_functions = sp_io::SubstrateHostFunctions::host_functions();
    let manager = TaskManager::new(Handle::current(), None).unwrap();

    let sender = key_store.sr25519_generate_new(KEY_TYPE, None).unwrap();
    let receiver = key_store.sr25519_generate_new(KEY_TYPE, None).unwrap();

    let mut storage = Storage::default();
    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (AccountId32::from(sender), 10_000_000_000_000u128),
            (AccountId32::from(receiver), 10_000_000_000_000u128)
        ]
    }
        .assimilate_storage(&mut storage)
        .unwrap();

    let mut provider = EnvProvider::<TestBlock, TestRtApi, TestExec>::with_code(CODE.unwrap());
    provider.insert_storage(storage);

    let (client, backend) = provider
        .init_default(
            TestExec::new(
                WasmExecutionMethod::Interpreted,
                None,
                host_functions,
                6,
                None,
            ),
            Box::new(manager.spawn_handle())
        );
    let client = Arc::new(client);

    let mut builder = RelaychainBuilder::<TestBlock, TestRtApi, TestExec, _, _>::new(backend, client.clone());

    let is_some = builder.with_state(|| {
        let data = frame_support::storage::unhashed::get_raw(CODE_KEY).unwrap();
        let x = frame_system::Account::<Runtime>::get(AccountId32::from(sender));

        let data = frame_support::storage::unhashed::get_raw(CODE_KEY).unwrap();
    });

    let clone_client = client.clone();
    let cid = move |parent: H256, ()|  {
        let client = clone_client.clone();
        let parent_header = client.header(&BlockId::Hash(parent.clone())).unwrap().unwrap();

        async move {
            let parachain = DummyParachainsInherentData(ParachainsInherentData {
                bitfields: Vec::new(),
                backed_candidates: Vec::new(),
                disputes: Vec::new(),
                parent_header,
            });

            let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
                &*client,
                parent,
            )?;

            let timestamp = FakeTimestamp::new(0, None);

            let slot =
                sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
                    timestamp.current_time(),
                    std::time::Duration::from_secs(6),
                );

            Ok((parachain, uncles, timestamp, slot))
        }
    };

    builder.build_block(cid, manager.spawn_handle());
}

pub const PARACHAINS_INHERENT_IDENTIFIER: InherentIdentifier = *b"parachn0";
pub struct DummyParachainsInherentData(pub ParachainsInherentData);

#[async_trait::async_trait]
impl InherentDataProvider for DummyParachainsInherentData {
    fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
        inherent_data
            .put_data(PARACHAINS_INHERENT_IDENTIFIER, &self.0);

        Ok(())
    }

    async fn try_handle_error(&self, identifier: &InherentIdentifier, error: &[u8]) -> Option<Result<(), Error>> {
        todo!()
    }
}