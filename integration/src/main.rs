use std::time::Duration;
use tracing::info;
use tracing_subscriber;
use polkadot_runtime::{Block as TestBlock, RuntimeApi as TestRtApi, WASM_BINARY as CODE, Runtime, SignedExtra};
use sc_client_db::Backend;
use sc_executor::{WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{LocalCallExecutor, TaskManager, TFullClient};
use sp_runtime::{AccountId32, CryptoTypeId, KeyTypeId, MultiAddress, MultiSignature, Storage};
use fudge_core::{EnvProvider, StandAloneBuilder};
use tokio::runtime::{Handle};
use sc_executor::sp_wasm_interface::HostFunctions;
use frame_support::inherent::{BlockT, InherentData, InherentIdentifier};
use frame_support::sp_runtime::app_crypto::sp_core::H256;
use sp_inherents::{Error, InherentDataProvider};
use fudge_core::inherent_provider::FakeTimestamp;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use sp_runtime::traits::HashFor;
use sp_storage::well_known_keys::CODE as CODE_KEY;
use polkadot_primitives::v1::{InherentData as ParachainsInherentData};
use sp_runtime::generic::BlockId;
use sp_runtime::sp_std::sync::Arc;
use fudge_utils::Signer;
use sp_core::sr25519;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");
const CRYPTO_TYPE: CryptoTypeId = CryptoTypeId(*b"test");

#[tokio::main]
async fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let key_store = sc_keystore::LocalKeystore::in_memory();
    let key_store: SyncCryptoStorePtr = key_store.into();
    let signer = Signer::new(key_store.clone(), CRYPTO_TYPE, KEY_TYPE);

    let mut host_functions = sp_io::SubstrateHostFunctions::host_functions();
    let manager = TaskManager::new(Handle::current(), None).unwrap();

    let sender = SyncCryptoStore::sr25519_generate_new(&(*key_store), KEY_TYPE, None).unwrap();
    let receiver = SyncCryptoStore::sr25519_generate_new(&(*key_store), KEY_TYPE, None).unwrap();

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

    let mut builder = StandAloneBuilder::<TestBlock, TestRtApi, TestExec, _, _>::new(backend, client.clone());

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

            let timestamp = FakeTimestamp::new(0, Duration::from_secs(12).as_millis() as u64, Some(0));

            let slot =
                sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
                    timestamp.current_time(),
                    std::time::Duration::from_secs(6),
                );

            Ok((parachain, uncles, timestamp, slot))
        }
    };

    let num = builder.with_state(|| {
        let (d1, d2) = (frame_system::Account::<Runtime>::get(AccountId32::from(sender.clone())), frame_system::Account::<Runtime>::get(AccountId32::from(receiver.clone())));
        frame_system::Pallet::<Runtime>::block_number()
    }).unwrap();

    println!("{:?}", num);

    builder.build_block(cid.clone(), manager.spawn_handle());
    let xt = builder.with_state(|| {
        let block = frame_system::Pallet::<Runtime>::block_number();
        build_transfer(signer, AccountId32::from(sender.clone()), AccountId32::from(receiver.clone()))
    });
    builder.append_extrinsic(xt.unwrap());
    builder.build_block(cid.clone(), manager.spawn_handle());

    let blocks = builder.blocks();

    println!("{:?}", blocks);
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

fn build_transfer(signer: Signer, send: AccountId32, recv: AccountId32) -> sp_runtime::generic::UncheckedExtrinsic<MultiAddress<AccountId32,()>, polkadot_runtime::Call, MultiSignature, SignedExtra> {
    let extra: SignedExtra = (
        frame_system::CheckSpecVersion::<Runtime>::new(),
        frame_system::CheckTxVersion::<Runtime>::new(),
        frame_system::CheckGenesis::<Runtime>::new(),
        frame_system::CheckMortality::<Runtime>::from(sp_runtime::generic::Era::Immortal),
        frame_system::CheckNonce::<Runtime>::from(0),
        frame_system::CheckWeight::<Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
        polkadot_runtime_common::claims::PrevalidateAttests::<Runtime>::new(),
    );

    let xt = signer.signed_ext(
        polkadot_runtime::Call::Balances(
            pallet_balances::Call::transfer {
                dest: MultiAddress::Id(recv),
                value: 1_000_000_000_000u128,
            }),
        MultiAddress::Id(send),
        extra
    );

    xt.unwrap()
}