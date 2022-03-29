#![feature(prelude_import)]
#![feature(associated_type_bounds)]
#![allow(dead_code)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
pub use builder::{
    parachain::{FudgeParaBuild, FudgeParaChain, ParachainBuilder},
    relay_chain::{types as RelayChainTypes, RelayChainBuilder},
    stand_alone::StandAloneBuilder,
};
pub use provider::EnvProvider;
pub use types::{Bytes, StoragePair};
mod builder {
    mod core {
        extern crate sc_client_api;
        extern crate sc_client_db;
        extern crate sc_consensus;
        extern crate sc_service;
        extern crate sp_api;
        extern crate sp_consensus;
        extern crate sp_runtime;
        use crate::provider::ExternalitiesProvider;
        use codec::Encode;
        use frame_support::dispatch::TransactionPriority;
        use frame_support::pallet_prelude::{TransactionLongevity, TransactionSource, TransactionTag};
        use frame_support::sp_runtime::traits::NumberFor;
        use sc_client_api::{AuxStore, Backend as BackendT, BlockOf, HeaderBackend, UsageProvider};
        use sc_client_db::Backend;
        use sc_consensus::{BlockImport, BlockImportParams};
        use sc_executor::RuntimeVersionOf;
        use sc_service::TFullClient;
        use sc_transaction_pool_api::{PoolStatus, ReadyTransactions};
        use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
        use sp_block_builder::BlockBuilder;
        use sp_consensus::Environment;
        use sp_core::traits::CodeExecutor;
        use sp_runtime::{generic::BlockId, traits::One};
        use sp_state_machine::StorageProof;
        use sp_std::time::Duration;
        use std::fmt::{Debug, Display, Formatter};
        use std::future::Future;
        use std::pin::Pin;
        use std::{collections::HashMap, marker::PhantomData, sync::Arc};
        use self::sc_client_api::blockchain::Backend as BlockchainBackend;
        use self::sc_client_api::{BlockImportOperation, NewBlockState};
        use self::sc_consensus::ImportResult;
        use self::sc_service::{InPoolTransaction, SpawnTaskHandle, TransactionPool};
        use self::sp_api::HashFor;
        use self::sp_consensus::{InherentData, Proposal, Proposer};
        use self::sp_runtime::traits::{Block as BlockT, Hash as HashT, Header as HeaderT, Zero};
        use self::sp_runtime::Digest;
        use crate::StoragePair;
        use sc_client_api::backend::TransactionFor;
        use sp_storage::StateVersion;
        pub enum Operation {
            Commit,
            DryRun,
        }
        pub struct SimplePool<Block: BlockT> {
            pool: Vec<Block::Extrinsic>,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<Block: ::core::clone::Clone + BlockT> ::core::clone::Clone for SimplePool<Block>
        where
            Block::Extrinsic: ::core::clone::Clone,
        {
            #[inline]
            fn clone(&self) -> SimplePool<Block> {
                match *self {
                    SimplePool {
                        pool: ref __self_0_0,
                    } => SimplePool {
                        pool: ::core::clone::Clone::clone(&(*__self_0_0)),
                    },
                }
            }
        }
        impl<Block: BlockT> SimplePool<Block> {
            fn new() -> Self {
                SimplePool { pool: Vec::new() }
            }
            fn push(&mut self, xt: Block::Extrinsic) {
                self.pool.push(xt)
            }
        }
        pub struct ExtWrapper<Block: BlockT> {
            xt: Block::Extrinsic,
        }
        impl<Block: BlockT> ExtWrapper<Block> {
            pub fn new(xt: Block::Extrinsic) -> Self {
                Self { xt }
            }
        }
        impl<Block: BlockT> InPoolTransaction for ExtWrapper<Block> {
            type Transaction = Block::Extrinsic;
            type Hash = Block::Hash;
            fn data(&self) -> &Self::Transaction {
                &self.xt
            }
            fn hash(&self) -> &Self::Hash {
                ::core::panicking::panic("not yet implemented")
            }
            fn priority(&self) -> &TransactionPriority {
                ::core::panicking::panic("not yet implemented")
            }
            fn longevity(&self) -> &TransactionLongevity {
                ::core::panicking::panic("not yet implemented")
            }
            fn requires(&self) -> &[TransactionTag] {
                ::core::panicking::panic("not yet implemented")
            }
            fn provides(&self) -> &[TransactionTag] {
                ::core::panicking::panic("not yet implemented")
            }
            fn is_propagable(&self) -> bool {
                ::core::panicking::panic("not yet implemented")
            }
        }
        pub enum Error {}
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                unsafe { ::core::intrinsics::unreachable() }
            }
        }
        impl From<sc_transaction_pool_api::error::Error> for Error {
            fn from(_: sc_transaction_pool_api::error::Error) -> Self {
                ::core::panicking::panic("not yet implemented")
            }
        }
        impl Display for Error {
            fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
                ::core::panicking::panic("not yet implemented")
            }
        }
        impl std::error::Error for Error {}
        impl sc_transaction_pool_api::error::IntoPoolError for Error {}
        impl<Block: BlockT> Iterator for SimplePool<Block> {
            type Item = Arc<ExtWrapper<Block>>;
            fn next(&mut self) -> Option<Self::Item> {
                self.pool
                    .pop()
                    .and_then(|xt| Some(Arc::new(ExtWrapper::new(xt))))
            }
        }
        impl<Block: BlockT> ReadyTransactions for SimplePool<Block> {
            fn report_invalid(&mut self, _tx: &Self::Item) {
                ::core::panicking::panic("not yet implemented")
            }
        }
        impl<Block: BlockT> TransactionPool for SimplePool<Block> {
            type Block = Block;
            type Hash = Block::Hash;
            type InPoolTransaction = ExtWrapper<Block>;
            type Error = Error;
            fn submit_at(
                &self,
                _at: &BlockId<Self::Block>,
                _source: TransactionSource,
                _xts: Vec<sc_transaction_pool_api::TransactionFor<Self>>,
            ) -> sc_transaction_pool_api::PoolFuture<
                Vec<Result<sc_transaction_pool_api::TxHash<Self>, Self::Error>>,
                Self::Error,
            > {
                ::core::panicking::panic("not yet implemented")
            }
            fn submit_one(
                &self,
                _at: &BlockId<Self::Block>,
                _source: TransactionSource,
                _xt: sc_transaction_pool_api::TransactionFor<Self>,
            ) -> sc_transaction_pool_api::PoolFuture<
                sc_transaction_pool_api::TxHash<Self>,
                Self::Error,
            > {
                ::core::panicking::panic("not yet implemented")
            }
            fn submit_and_watch(
                &self,
                _at: &BlockId<Self::Block>,
                _source: TransactionSource,
                _xt: sc_transaction_pool_api::TransactionFor<Self>,
            ) -> sc_transaction_pool_api::PoolFuture<
                Pin<Box<sc_transaction_pool_api::TransactionStatusStreamFor<Self>>>,
                Self::Error,
            > {
                ::core::panicking::panic("not yet implemented")
            }
            fn ready_at(
                &self,
                _at: NumberFor<Self::Block>,
            ) -> Pin<
                Box<
                    dyn Future<
                            Output = Box<
                                dyn sc_transaction_pool_api::ReadyTransactions<
                                        Item = Arc<Self::InPoolTransaction>,
                                    > + Send,
                            >,
                        > + Send,
                >,
            > {
                let i = Box::new(std::iter::empty::<Arc<Self::InPoolTransaction>>())
                    as Box<
                        (dyn ReadyTransactions<Item = Arc<ExtWrapper<Block>>>
                             + std::marker::Send
                             + 'static),
                    >;
                Box::pin(async { i })
            }
            fn ready(
                &self,
            ) -> Box<
                dyn sc_transaction_pool_api::ReadyTransactions<Item = Arc<Self::InPoolTransaction>>
                    + Send,
            > {
                Box::new(self.clone())
            }
            fn remove_invalid(
                &self,
                _hashes: &[sc_transaction_pool_api::TxHash<Self>],
            ) -> Vec<Arc<Self::InPoolTransaction>> {
                Vec::new()
            }
            fn status(&self) -> sc_transaction_pool_api::PoolStatus {
                PoolStatus {
                    ready: self.pool.len(),
                    ready_bytes: self
                        .pool
                        .iter()
                        .fold(0, |weight, xt| weight + xt.size_hint()),
                    future: 0,
                    future_bytes: 0,
                }
            }
            fn import_notification_stream(
                &self,
            ) -> sc_transaction_pool_api::ImportNotificationStream<
                sc_transaction_pool_api::TxHash<Self>,
            > {
                ::core::panicking::panic("not yet implemented")
            }
            fn on_broadcasted(
                &self,
                _propagations: HashMap<sc_transaction_pool_api::TxHash<Self>, Vec<String>>,
            ) {
                ::core::panicking::panic("not yet implemented")
            }
            fn hash_of(
                &self,
                _xt: &sc_transaction_pool_api::TransactionFor<Self>,
            ) -> sc_transaction_pool_api::TxHash<Self> {
                ::core::panicking::panic("not yet implemented")
            }
            fn ready_transaction(
                &self,
                _hash: &sc_transaction_pool_api::TxHash<Self>,
            ) -> Option<Arc<Self::InPoolTransaction>> {
                ::core::panicking::panic("not yet implemented")
            }
        }
        pub struct TransitionCache<Block: BlockT> {
            extrinsics: Arc<SimplePool<Block>>,
            auxilliary: Vec<StoragePair>,
        }
        pub struct Builder<
            Block: BlockT,
            RtApi,
            Exec,
            B = Backend<Block>,
            C = TFullClient<Block, RtApi, Exec>,
        > {
            backend: Arc<B>,
            client: Arc<C>,
            cache: TransitionCache<Block>,
            _phantom: PhantomData<(Block, RtApi, Exec)>,
        }
        impl<Block, RtApi, Exec, B, C> Builder<Block, RtApi, Exec, B, C>
        where
            B: BackendT<Block> + 'static,
            Block: BlockT,
            RtApi: ConstructRuntimeApi<Block, C> + Send,
            Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
            C::Api: BlockBuilder<Block> + ApiExt<Block, StateBackend = B::State>,
            C: 'static
                + ProvideRuntimeApi<Block>
                + BlockOf
                + Send
                + Sync
                + AuxStore
                + UsageProvider<Block>
                + HeaderBackend<Block>
                + BlockImport<Block>
                + CallApiAt<Block>
                + sc_block_builder::BlockBuilderProvider<B, Block, C>,
        {
            /// Create a new Builder with provided backend and client.
            pub fn new(backend: Arc<B>, client: Arc<C>) -> Self {
                Builder {
                    backend: backend,
                    client: client,
                    cache: TransitionCache {
                        extrinsics: Arc::new(SimplePool::new()),
                        auxilliary: Vec::new(),
                    },
                    _phantom: PhantomData::default(),
                }
            }
            pub fn client(&self) -> Arc<C> {
                self.client.clone()
            }
            pub fn backend(&self) -> Arc<B> {
                self.backend.clone()
            }
            pub fn latest_block(&self) -> Block::Hash {
                self.client.info().best_hash
            }
            pub fn latest_header(&self) -> Block::Header {
                self.backend
                    .blockchain()
                    .header(BlockId::Hash(self.latest_block()))
                    .ok()
                    .flatten()
                    .expect("State is available. qed")
            }
            pub fn latest_code(&self) -> Vec<u8> {
                self.with_state(Operation::DryRun, None, || {
                    frame_support::storage::unhashed::get_raw(sp_storage::well_known_keys::CODE)
                        .unwrap()
                })
                .unwrap()
            }
            pub fn with_state<R>(
                &self,
                op: Operation,
                at: Option<BlockId<Block>>,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                let (state, at) = if let Some(req_at) = at {
                    (self.backend.state_at(req_at), req_at)
                } else {
                    let at = BlockId::Hash(self.client.info().best_hash);
                    (self.backend.state_at(at.clone()), at)
                };
                let state =
                    state.map_err(|_| "State at INSERT_AT_HERE not available".to_string())?;
                match op {
                    Operation::Commit => {
                        let mut op = self.backend.begin_operation().map_err(|_| {
                            "Unable to start state-operation on backend".to_string()
                        })?;
                        self.backend.begin_state_operation(&mut op, at).unwrap();
                        let res = if self
                            .backend
                            .blockchain()
                            .block_number_from_id(&at)
                            .unwrap()
                            .unwrap()
                            == Zero::zero()
                        {
                            self.mutate_genesis(&mut op, &state, exec)
                        } else {
                            let info = self.client.info();
                            if info.best_hash == info.finalized_hash {
                                self.backend
                                    .revert(NumberFor::<Block>::one(), true)
                                    .unwrap();
                            }
                            self.mutate_normal(&mut op, &state, exec, at)
                        };
                        self.backend.commit_operation(op).map_err(|_| {
                            "Unable to commit state-operation on backend".to_string()
                        })?;
                        res
                    }
                    Operation::DryRun => Ok(
                        ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state)
                            .execute_with(exec),
                    ),
                }
            }
            fn mutate_genesis<R>(
                &self,
                op: &mut B::BlockImportOperation,
                state: &B::State,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);
                let (r, changes) = ext.execute_with_mut(exec);
                let (_main_sc, _child_sc, _, tx, root, _tx_index) = changes.into_inner();
                op.update_db_storage(tx).unwrap();
                let genesis_block = Block::new(
                    Block::Header::new(
                        Zero::zero(),
                        <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
                            Vec::new(),
                            StateVersion::V0,
                        ),
                        root,
                        Default::default(),
                        Default::default(),
                    ),
                    Default::default(),
                );
                op.set_block_data(
                    genesis_block.deconstruct().0,
                    Some(::alloc::vec::Vec::new()),
                    None,
                    None,
                    NewBlockState::Final,
                )
                .map_err(|_| "Could not set block data".to_string())?;
                Ok(r)
            }
            fn mutate_normal<R>(
                &self,
                op: &mut B::BlockImportOperation,
                state: &B::State,
                exec: impl FnOnce() -> R,
                at: BlockId<Block>,
            ) -> Result<R, String> {
                let chain_backend = self.backend.blockchain();
                let mut header = chain_backend
                    .header(at)
                    .ok()
                    .flatten()
                    .expect("State is available. qed");
                let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);
                let (r, changes) = ext.execute_with_mut(exec);
                let (main_sc, child_sc, _, tx, root, tx_index) = changes.into_inner();
                header.set_state_root(root);
                op.update_db_storage(tx).unwrap();
                op.update_storage(main_sc, child_sc)
                    .map_err(|_| "Updating storage not possible.")
                    .unwrap();
                op.update_transaction_index(tx_index)
                    .map_err(|_| "Updating transaction index not possible.")
                    .unwrap();
                let body = chain_backend.body(at).expect("State is available. qed.");
                let indexed_body = chain_backend
                    .block_indexed_body(at)
                    .expect("State is available. qed.");
                let justifications = chain_backend
                    .justifications(at)
                    .expect("State is available. qed.");
                op.set_block_data(
                    header,
                    body,
                    indexed_body,
                    justifications,
                    NewBlockState::Final,
                )
                .unwrap();
                Ok(r)
            }
            /// Append a given set of key-value-pairs into the builder cache
            pub fn append_transition(&mut self, trans: StoragePair) -> &mut Self {
                self.cache.auxilliary.push(trans);
                self
            }
            /// Caches a given extrinsic in the builder. The extrinsic will be
            pub fn append_extrinsic(&mut self, ext: Block::Extrinsic) -> &mut Self {
                let pt = self.cache.extrinsics.as_ref() as *const SimplePool<Block>
                    as *mut SimplePool<Block>;
                let pool = unsafe { &mut *(pt) };
                pool.push(ext);
                self
            }
            /// Create a block from a given state of the Builder.
            pub fn build_block(
                &mut self,
                handle: SpawnTaskHandle,
                inherents: InherentData,
                digest: Digest,
                time: Duration,
                limit: usize,
            ) -> Proposal<Block, TransactionFor<B, Block>, StorageProof> {
                let mut factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
                    handle,
                    self.client.clone(),
                    self.cache.extrinsics.clone(),
                    None,
                    None,
                );
                let header = self
                    .backend
                    .blockchain()
                    .header(BlockId::Hash(self.latest_block()))
                    .ok()
                    .flatten()
                    .expect("State is available. qed");
                let proposer = futures::executor::block_on(factory.init(&header)).unwrap();
                futures::executor::block_on(proposer.propose(inherents, digest, time, Some(limit)))
                    .unwrap()
            }
            /// Import a block, that has been previosuly build
            pub fn import_block(
                &mut self,
                params: BlockImportParams<Block, C::Transaction>,
            ) -> Result<(), ()> {
                let client = self.client.as_ref() as *const C as *mut C;
                let client = unsafe { &mut *(client) };
                match futures::executor::block_on(client.import_block(params, Default::default()))
                    .unwrap()
                {
                    ImportResult::Imported(_) => Ok(()),
                    ImportResult::AlreadyInChain => Err(()),
                    ImportResult::KnownBad => Err(()),
                    ImportResult::UnknownParent => Err(()),
                    ImportResult::MissingState => Err(()),
                }
            }
        }
    }
    pub mod parachain {
        use crate::digest::DigestCreator;
        use crate::inherent::ArgsProvider;
        use crate::{
            builder::core::{Builder, Operation},
            types::{Bytes, StoragePair},
        };
        use codec::Encode;
        use polkadot_parachain::primitives::{BlockData, HeadData, Id, ValidationCode};
        use sc_client_api::{AuxStore, Backend as BackendT, BlockOf, HeaderBackend, UsageProvider};
        use sc_client_db::Backend;
        use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
        use sc_executor::RuntimeVersionOf;
        use sc_service::{SpawnTaskHandle, TFullClient};
        use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi, StorageProof};
        use sp_block_builder::BlockBuilder;
        use sp_consensus::{BlockOrigin, Proposal};
        use sp_core::traits::CodeExecutor;
        use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
        use sp_runtime::{generic::BlockId, traits::Block as BlockT};
        use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
        pub struct FudgeParaBuild {
            pub parent_head: HeadData,
            pub block: BlockData,
            pub code: ValidationCode,
        }
        pub struct FudgeParaChain {
            pub id: Id,
            pub head: HeadData,
            pub code: ValidationCode,
        }
        pub struct ParachainBuilder<
            Block: BlockT,
            RtApi,
            Exec,
            CIDP,
            ExtraArgs,
            DP,
            B = Backend<Block>,
            C = TFullClient<Block, RtApi, Exec>,
        > {
            builder: Builder<Block, RtApi, Exec, B, C>,
            cidp: CIDP,
            dp: DP,
            next: Option<(Block, StorageProof)>,
            imports: Vec<(Block, StorageProof)>,
            handle: SpawnTaskHandle,
            _phantom: PhantomData<ExtraArgs>,
        }
        impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
            ParachainBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
        where
            B: BackendT<Block> + 'static,
            Block: BlockT,
            RtApi: ConstructRuntimeApi<Block, C> + Send,
            Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
            CIDP: CreateInherentDataProviders<Block, ExtraArgs> + Send + Sync + 'static,
            CIDP::InherentDataProviders: Send,
            DP: DigestCreator,
            ExtraArgs: ArgsProvider<ExtraArgs>,
            C::Api: BlockBuilder<Block> + ApiExt<Block, StateBackend = B::State>,
            C: 'static
                + ProvideRuntimeApi<Block>
                + BlockOf
                + Send
                + Sync
                + AuxStore
                + UsageProvider<Block>
                + HeaderBackend<Block>
                + BlockImport<Block>
                + CallApiAt<Block>
                + sc_block_builder::BlockBuilderProvider<B, Block, C>,
        {
            pub fn new(
                handle: SpawnTaskHandle,
                backend: Arc<B>,
                client: Arc<C>,
                cidp: CIDP,
                dp: DP,
            ) -> Self {
                Self {
                    builder: Builder::new(backend, client),
                    cidp,
                    dp,
                    next: None,
                    imports: Vec::new(),
                    handle,
                    _phantom: Default::default(),
                }
            }
            pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
                self.builder.append_extrinsic(xt);
                self
            }
            pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
                xts.into_iter().for_each(|xt| {
                    self.builder.append_extrinsic(xt);
                });
                self
            }
            pub fn append_transition(&mut self, aux: StoragePair) -> &mut Self {
                self.builder.append_transition(aux);
                self
            }
            pub fn append_transitions(&mut self, auxs: Vec<StoragePair>) -> &mut Self {
                auxs.into_iter().for_each(|aux| {
                    self.builder.append_transition(aux);
                });
                self
            }
            pub fn append_xcm(&mut self, _xcm: Bytes) -> &mut Self {
                ::core::panicking::panic("not yet implemented")
            }
            pub fn append_xcms(&mut self, _xcms: Vec<Bytes>) -> &mut Self {
                ::core::panicking::panic("not yet implemented")
            }
            pub fn build_block(&mut self) -> Result<(), ()> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                let provider = self
                    .with_state(|| {
                        futures::executor::block_on(self.cidp.create_inherent_data_providers(
                            self.builder.latest_block(),
                            ExtraArgs::extra(),
                        ))
                        .unwrap()
                    })
                    .unwrap();
                let digest = self
                    .with_state(|| futures::executor::block_on(self.dp.create_digest()).unwrap())
                    .unwrap();
                let inherent = provider.create_inherent_data().unwrap();
                let Proposal { block, proof, .. } = self.builder.build_block(
                    self.handle.clone(),
                    inherent,
                    digest,
                    Duration::from_secs(60),
                    6_000_000,
                );
                self.next = Some((block, proof));
                Ok(())
            }
            pub fn head(&self) -> HeadData {
                HeadData(self.builder.latest_header().encode())
            }
            pub fn code(&self) -> ValidationCode {
                ValidationCode(self.builder.latest_code())
            }
            pub fn next_build(&self) -> Option<FudgeParaBuild> {
                if let Some((ref block, _)) = self.next {
                    Some(FudgeParaBuild {
                        parent_head: HeadData(self.builder.latest_header().encode()),
                        block: BlockData(block.clone().encode()),
                        code: ValidationCode(self.builder.latest_code()),
                    })
                } else {
                    None
                }
            }
            pub fn import_block(&mut self) -> &mut Self {
                let (block, proof) = self.next.take().unwrap();
                let (header, body) = block.clone().deconstruct();
                let mut params = BlockImportParams::new(BlockOrigin::NetworkInitialSync, header);
                params.body = Some(body);
                params.finalized = true;
                params.fork_choice = Some(ForkChoiceStrategy::Custom(true));
                self.builder.import_block(params).unwrap();
                self.imports.push((block, proof));
                self
            }
            pub fn imports(&self) -> Vec<(Block, StorageProof)> {
                self.imports.clone()
            }
            pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, String> {
                self.builder.with_state(Operation::DryRun, None, exec)
            }
            pub fn with_state_at<R>(
                &mut self,
                at: BlockId<Block>,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                self.builder.with_state(Operation::DryRun, Some(at), exec)
            }
            pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                self.builder.with_state(Operation::Commit, None, exec)
            }
            /// Mutating past states not supported yet...
            fn with_mut_state_at<R>(
                &mut self,
                at: BlockId<Block>,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                self.builder.with_state(Operation::Commit, Some(at), exec)
            }
        }
    }
    pub mod relay_chain {
        use crate::builder::parachain::FudgeParaChain;
        use crate::digest::DigestCreator;
        use crate::inherent::ArgsProvider;
        use crate::{
            builder::core::{Builder, Operation},
            types::{Bytes, StoragePair},
        };
        use cumulus_primitives_parachain_inherent::ParachainInherentData;
        use cumulus_relay_chain_local::RelayChainLocal;
        use parking_lot::Mutex;
        use polkadot_core_primitives::Block as PBlock;
        use polkadot_parachain::primitives::{Id, ValidationCodeHash};
        use polkadot_primitives::v1::OccupiedCoreAssumption;
        use polkadot_primitives::v2::ParachainHost;
        use polkadot_runtime_parachains::{paras, ParaLifecycle};
        use sc_client_api::{
            AuxStore, Backend as BackendT, BlockOf, BlockchainEvents, HeaderBackend, UsageProvider,
        };
        use sc_client_db::Backend;
        use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
        use sc_executor::RuntimeVersionOf;
        use sc_service::{SpawnTaskHandle, TFullBackend, TFullClient};
        use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi, StorageProof};
        use sp_block_builder::BlockBuilder;
        use sp_consensus::{BlockOrigin, NoNetwork, Proposal};
        use sp_consensus_babe::BabeApi;
        use sp_core::traits::CodeExecutor;
        use sp_core::H256;
        use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
        use sp_runtime::{generic::BlockId, traits::Block as BlockT};
        use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
        use types::*;
        /// Recreating private storage types for easier handling storage access
        pub mod types {
            use frame_support::traits::StorageInstance;
            use frame_support::{
                storage::types::{StorageMap, StorageValue, ValueQuery},
                Identity, Twox64Concat,
            };
            use polkadot_parachain::primitives::{
                HeadData, Id as ParaId, ValidationCode, ValidationCodeHash,
            };
            use polkadot_runtime_parachains::ParaLifecycle;
            pub struct ParaLifecyclesPrefix;
            impl StorageInstance for ParaLifecyclesPrefix {
                fn pallet_prefix() -> &'static str {
                    "ParaLifecycles"
                }
                const STORAGE_PREFIX: &'static str = "Parachains";
            }
            pub type ParaLifecycles =
                StorageMap<ParaLifecyclesPrefix, Twox64Concat, ParaId, ParaLifecycle>;
            pub struct ParachainsPrefix;
            impl StorageInstance for ParachainsPrefix {
                fn pallet_prefix() -> &'static str {
                    "Paras"
                }
                const STORAGE_PREFIX: &'static str = "Parachains";
            }
            pub type Parachains = StorageValue<ParachainsPrefix, Vec<ParaId>, ValueQuery>;
            pub struct HeadsPrefix;
            impl StorageInstance for HeadsPrefix {
                fn pallet_prefix() -> &'static str {
                    "Paras"
                }
                const STORAGE_PREFIX: &'static str = "Heads";
            }
            pub type Heads = StorageMap<HeadsPrefix, Twox64Concat, ParaId, HeadData>;
            pub struct CurrentCodeHashPrefix;
            impl StorageInstance for CurrentCodeHashPrefix {
                fn pallet_prefix() -> &'static str {
                    "Paras"
                }
                const STORAGE_PREFIX: &'static str = "CurrentCodeHash";
            }
            pub type CurrentCodeHash =
                StorageMap<CurrentCodeHashPrefix, Twox64Concat, ParaId, ValidationCodeHash>;
            pub struct CodeByHashPrefix;
            impl StorageInstance for CodeByHashPrefix {
                fn pallet_prefix() -> &'static str {
                    "Paras"
                }
                const STORAGE_PREFIX: &'static str = "CodeByHash";
            }
            pub type CodeByHash =
                StorageMap<CodeByHashPrefix, Identity, ValidationCodeHash, ValidationCode>;
            pub struct CodeByHashRefsPrefix;
            impl StorageInstance for CodeByHashRefsPrefix {
                fn pallet_prefix() -> &'static str {
                    "Paras"
                }
                const STORAGE_PREFIX: &'static str = "CodeByHashRefs";
            }
            pub type CodeByHashRefs =
                StorageMap<CodeByHashRefsPrefix, Identity, ValidationCodeHash, u32, ValueQuery>;
            pub struct PastCodeHashPrefix;
            impl StorageInstance for PastCodeHashPrefix {
                fn pallet_prefix() -> &'static str {
                    "Paras"
                }
                const STORAGE_PREFIX: &'static str = "PastCodeHash";
            }
            #[allow(type_alias_bounds)]
            pub type PastCodeHash<T: frame_system::Config> = StorageMap<
                PastCodeHashPrefix,
                Twox64Concat,
                (ParaId, T::BlockNumber),
                ValidationCodeHash,
            >;
        }
        pub struct InherentBuilder<C, B> {
            id: Id,
            client: Arc<C>,
            backend: Arc<B>,
        }
        impl<C, B> Clone for InherentBuilder<C, B> {
            fn clone(&self) -> Self {
                InherentBuilder {
                    id: self.id.clone(),
                    client: self.client.clone(),
                    backend: self.backend.clone(),
                }
            }
            fn clone_from(&mut self, _source: &Self) {
                ::core::panicking::panic("not yet implemented")
            }
        }
        impl<C> InherentBuilder<C, TFullBackend<PBlock>>
        where
            C::Api: BlockBuilder<PBlock>
                + ParachainHost<PBlock>
                + BabeApi<PBlock>
                + ApiExt<PBlock, StateBackend = <TFullBackend<PBlock> as BackendT<PBlock>>::State>,
            C: 'static
                + ProvideRuntimeApi<PBlock>
                + BlockOf
                + Send
                + Sync
                + AuxStore
                + UsageProvider<PBlock>
                + BlockchainEvents<PBlock>
                + HeaderBackend<PBlock>
                + BlockImport<PBlock>
                + CallApiAt<PBlock>
                + sc_block_builder::BlockBuilderProvider<TFullBackend<PBlock>, PBlock, C>,
        {
            pub async fn parachain_inherent(&self) -> Option<ParachainInherentData> {
                let parent = self.client.info().best_hash;
                let relay_interface = RelayChainLocal::new(
                    self.client.clone(),
                    self.backend.clone(),
                    Arc::new(Mutex::new(Box::new(NoNetwork {}))),
                    None,
                );
                let api = self.client.runtime_api();
                let persisted_validation_data = api
                    .persisted_validation_data(
                        &BlockId::Hash(parent),
                        self.id,
                        OccupiedCoreAssumption::TimedOut,
                    )
                    .unwrap()
                    .unwrap();
                ParachainInherentData::create_at(
                    parent,
                    &relay_interface,
                    &persisted_validation_data,
                    self.id,
                )
                .await
            }
        }
        pub struct RelayChainBuilder<
            Block: BlockT,
            RtApi,
            Exec,
            CIDP,
            ExtraArgs,
            DP,
            Runtime,
            B = Backend<Block>,
            C = TFullClient<Block, RtApi, Exec>,
        > {
            builder: Builder<Block, RtApi, Exec, B, C>,
            cidp: CIDP,
            dp: DP,
            next: Option<(Block, StorageProof)>,
            imports: Vec<(Block, StorageProof)>,
            handle: SpawnTaskHandle,
            _phantom: PhantomData<(ExtraArgs, Runtime)>,
        }
        impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, B, C>
            RelayChainBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, B, C>
        where
            B: BackendT<Block> + 'static,
            Block: BlockT,
            RtApi: ConstructRuntimeApi<Block, C> + Send,
            Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
            CIDP: CreateInherentDataProviders<Block, ExtraArgs> + Send + Sync + 'static,
            CIDP::InherentDataProviders: Send,
            DP: DigestCreator,
            ExtraArgs: ArgsProvider<ExtraArgs>,
            Runtime: paras::Config + frame_system::Config,
            C::Api:
                BlockBuilder<Block> + ParachainHost<Block> + ApiExt<Block, StateBackend = B::State>,
            C: 'static
                + ProvideRuntimeApi<Block>
                + BlockOf
                + Send
                + Sync
                + AuxStore
                + UsageProvider<Block>
                + BlockchainEvents<Block>
                + HeaderBackend<Block>
                + BlockImport<Block>
                + CallApiAt<Block>
                + sc_block_builder::BlockBuilderProvider<B, Block, C>,
        {
            pub fn new(
                handle: SpawnTaskHandle,
                backend: Arc<B>,
                client: Arc<C>,
                cidp: CIDP,
                dp: DP,
            ) -> Self {
                Self {
                    builder: Builder::new(backend, client),
                    cidp,
                    dp,
                    next: None,
                    imports: Vec::new(),
                    handle,
                    _phantom: Default::default(),
                }
            }
            pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
                self.builder.append_extrinsic(xt);
                self
            }
            pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
                xts.into_iter().for_each(|xt| {
                    self.builder.append_extrinsic(xt);
                });
                self
            }
            pub fn append_transition(&mut self, aux: StoragePair) -> &mut Self {
                self.builder.append_transition(aux);
                self
            }
            pub fn append_transitions(&mut self, auxs: Vec<StoragePair>) -> &mut Self {
                auxs.into_iter().for_each(|aux| {
                    self.builder.append_transition(aux);
                });
                self
            }
            pub fn append_xcm(&mut self, _xcm: Bytes) -> &mut Self {
                ::core::panicking::panic("not yet implemented")
            }
            pub fn append_xcms(&mut self, _xcms: Vec<Bytes>) -> &mut Self {
                ::core::panicking::panic("not yet implemented")
            }
            pub fn inherent_builder(&self, para_id: Id) -> InherentBuilder<C, B> {
                InherentBuilder {
                    id: para_id,
                    client: self.builder.client(),
                    backend: self.builder.backend(),
                }
            }
            pub fn onboard_para(&mut self, para: FudgeParaChain) -> &mut Self {
                self.with_mut_state(|| {
                    let FudgeParaChain { id, head, code } = para;
                    let current_block = frame_system::Pallet::<Runtime>::block_number();
                    let code_hash = code.hash();
                    Parachains::try_mutate::<(), (), _>(|paras| {
                        if !paras.contains(&id) {
                            paras.push(id);
                            paras.sort();
                        }
                        Ok(())
                    })
                    .unwrap();
                    let curr_code_hash = if let Some(curr_code_hash) = CurrentCodeHash::get(&id) {
                        PastCodeHash::<Runtime>::insert(&(id, current_block), curr_code_hash);
                        curr_code_hash
                    } else {
                        ValidationCodeHash::from(H256::zero())
                    };
                    if curr_code_hash != code_hash {
                        CurrentCodeHash::insert(&id, code_hash);
                        CodeByHash::insert(code_hash, code);
                        CodeByHashRefs::mutate(code_hash, |refs| {
                            if *refs == 0 {
                                *refs += 1;
                            }
                        });
                    }
                    ParaLifecycles::try_mutate::<_, (), (), _>(id, |para_lifecylce| {
                        if let Some(lifecycle) = para_lifecylce.as_mut() {
                            *lifecycle = ParaLifecycle::Parachain;
                        } else {
                            *para_lifecylce = Some(ParaLifecycle::Parachain);
                        }
                        Ok(())
                    })
                    .unwrap();
                    Heads::insert(&id, head);
                })
                .unwrap();
                self
            }
            pub fn build_block(&mut self) -> Result<Block, ()> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                let provider = self
                    .with_state(|| {
                        futures::executor::block_on(self.cidp.create_inherent_data_providers(
                            self.builder.latest_block(),
                            ExtraArgs::extra(),
                        ))
                        .unwrap()
                    })
                    .unwrap();
                let digest = self
                    .with_state(|| futures::executor::block_on(self.dp.create_digest()).unwrap())
                    .unwrap();
                let inherent = provider.create_inherent_data().unwrap();
                let Proposal { block, proof, .. } = self.builder.build_block(
                    self.handle.clone(),
                    inherent,
                    digest,
                    Duration::from_secs(60),
                    6_000_000,
                );
                self.next = Some((block.clone(), proof));
                Ok(block)
            }
            pub fn import_block(&mut self) -> &mut Self {
                let (block, proof) = self.next.take().unwrap();
                let (header, body) = block.clone().deconstruct();
                let mut params = BlockImportParams::new(BlockOrigin::NetworkInitialSync, header);
                params.body = Some(body);
                params.finalized = true;
                params.fork_choice = Some(ForkChoiceStrategy::Custom(true));
                self.builder.import_block(params).unwrap();
                self.imports.push((block, proof));
                self
            }
            pub fn imports(&self) -> Vec<(Block, StorageProof)> {
                self.imports.clone()
            }
            pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, String> {
                self.builder.with_state(Operation::DryRun, None, exec)
            }
            pub fn with_state_at<R>(
                &mut self,
                at: BlockId<Block>,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                self.builder.with_state(Operation::DryRun, Some(at), exec)
            }
            pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                self.builder.with_state(Operation::Commit, None, exec)
            }
            /// Mutating past states not supported yet...
            fn with_mut_state_at<R>(
                &mut self,
                at: BlockId<Block>,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                self.builder.with_state(Operation::Commit, Some(at), exec)
            }
        }
    }
    pub mod stand_alone {
        use crate::digest::DigestCreator;
        use crate::inherent::ArgsProvider;
        use crate::{
            builder::core::{Builder, Operation},
            types::StoragePair,
        };
        use sc_client_api::{AuxStore, Backend as BackendT, BlockOf, HeaderBackend, UsageProvider};
        use sc_client_db::Backend;
        use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
        use sc_executor::RuntimeVersionOf;
        use sc_service::{SpawnTaskHandle, TFullClient};
        use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
        use sp_block_builder::BlockBuilder;
        use sp_consensus::{BlockOrigin, Proposal};
        use sp_core::traits::CodeExecutor;
        use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
        use sp_runtime::{generic::BlockId, traits::Block as BlockT};
        use sp_state_machine::StorageProof;
        use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
        pub struct StandAloneBuilder<
            Block: BlockT,
            RtApi,
            Exec,
            CIDP,
            ExtraArgs,
            DP,
            B = Backend<Block>,
            C = TFullClient<Block, RtApi, Exec>,
        > {
            builder: Builder<Block, RtApi, Exec, B, C>,
            cidp: CIDP,
            dp: DP,
            next: Option<(Block, StorageProof)>,
            imports: Vec<(Block, StorageProof)>,
            handle: SpawnTaskHandle,
            _phantom: PhantomData<ExtraArgs>,
        }
        impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
            StandAloneBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
        where
            B: BackendT<Block> + 'static,
            Block: BlockT,
            RtApi: ConstructRuntimeApi<Block, C> + Send,
            Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
            CIDP: CreateInherentDataProviders<Block, ExtraArgs> + Send + Sync + 'static,
            CIDP::InherentDataProviders: Send,
            DP: DigestCreator,
            ExtraArgs: ArgsProvider<ExtraArgs>,
            C::Api: BlockBuilder<Block> + ApiExt<Block, StateBackend = B::State>,
            C: 'static
                + ProvideRuntimeApi<Block>
                + BlockOf
                + Send
                + Sync
                + AuxStore
                + UsageProvider<Block>
                + HeaderBackend<Block>
                + BlockImport<Block>
                + CallApiAt<Block>
                + sc_block_builder::BlockBuilderProvider<B, Block, C>,
        {
            pub fn new(
                handle: SpawnTaskHandle,
                backend: Arc<B>,
                client: Arc<C>,
                cidp: CIDP,
                dp: DP,
            ) -> Self {
                Self {
                    builder: Builder::new(backend, client),
                    cidp,
                    dp,
                    next: None,
                    imports: Vec::new(),
                    handle,
                    _phantom: Default::default(),
                }
            }
            pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
                self.builder.append_extrinsic(xt);
                self
            }
            pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
                xts.into_iter().for_each(|xt| {
                    self.builder.append_extrinsic(xt);
                });
                self
            }
            pub fn append_transition(&mut self, aux: StoragePair) -> &mut Self {
                self.builder.append_transition(aux);
                self
            }
            pub fn append_transitions(&mut self, auxs: Vec<StoragePair>) -> &mut Self {
                auxs.into_iter().for_each(|aux| {
                    self.builder.append_transition(aux);
                });
                self
            }
            pub fn build_block(&mut self) -> Result<Block, ()> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                let provider = self
                    .with_state(|| {
                        futures::executor::block_on(self.cidp.create_inherent_data_providers(
                            self.builder.latest_block(),
                            ExtraArgs::extra(),
                        ))
                        .unwrap()
                    })
                    .unwrap();
                let digest = self
                    .with_state(|| futures::executor::block_on(self.dp.create_digest()).unwrap())
                    .unwrap();
                let inherent = provider.create_inherent_data().unwrap();
                let Proposal { block, proof, .. } = self.builder.build_block(
                    self.handle.clone(),
                    inherent,
                    digest,
                    Duration::from_secs(60),
                    6_000_000,
                );
                self.next = Some((block.clone(), proof));
                Ok(block)
            }
            pub fn import_block(&mut self) -> &mut Self {
                let (block, proof) = self.next.take().unwrap();
                let (header, body) = block.clone().deconstruct();
                let mut params = BlockImportParams::new(BlockOrigin::NetworkInitialSync, header);
                params.body = Some(body);
                params.finalized = true;
                params.fork_choice = Some(ForkChoiceStrategy::Custom(true));
                self.builder.import_block(params).unwrap();
                self.imports.push((block, proof));
                self
            }
            pub fn imports(&self) -> Vec<(Block, StorageProof)> {
                self.imports.clone()
            }
            pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, String> {
                self.builder.with_state(Operation::DryRun, None, exec)
            }
            pub fn with_state_at<R>(
                &mut self,
                at: BlockId<Block>,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                self.builder.with_state(Operation::DryRun, Some(at), exec)
            }
            pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                self.builder.with_state(Operation::Commit, None, exec)
            }
            /// Mutating past states not supported yet...
            fn with_mut_state_at<R>(
                &mut self,
                at: BlockId<Block>,
                exec: impl FnOnce() -> R,
            ) -> Result<R, String> {
                if !self.next.is_none() {
                    ::core::panicking::panic("assertion failed: self.next.is_none()")
                };
                self.builder.with_state(Operation::Commit, Some(at), exec)
            }
        }
    }
    pub mod test {
        use fudge_companion;
        use std::marker::PhantomData;
        pub struct ParachainBuilder<Block, RtApi>(PhantomData<(Block, RtApi)>);
        pub struct RelaychainBuilder<RtApi>(PhantomData<RtApi>);
        use crate::FudgeParaChain;
        pub type FudgeResult<T> = std::result::Result<T, ()>;
        pub enum Chain {
            Relay,
            Para(u32),
        }
        pub struct TestEnv {
            parachain_1: (u32, ParachainBuilder<(), ()>),
            parachain_2: (u32, ParachainBuilder<u32, u32>),
            relay_chain: (),
        }
        impl TestEnv {
            pub fn new(
                relay_chain: (),
                parachain_1: ParachainBuilder<(), ()>,
                parachain_2: ParachainBuilder<u32, u32>,
            ) -> FudgeResult<Self> {
                let companion = Self {
                    relay_chain,
                    parachain_1,
                    parachain_2,
                };
                let para = FudgeParaChain {
                    id: (2001),
                    head: companion.parachain_1.head(),
                    code: companion.parachain_1.code(),
                };
                companion
                    .relay_chain
                    .onboard_para(para)
                    .map_err(|_| ())
                    .map(|_| ())?;
                let para = FudgeParaChain {
                    id: (2002),
                    head: companion.parachain_2.head(),
                    code: companion.parachain_2.code(),
                };
                companion
                    .relay_chain
                    .onboard_para(para)
                    .map_err(|_| ())
                    .map(|_| ())?;
                Ok(companion)
            }
            pub fn with_state<R>(&self, chain: Chain, exec: impl FnOnce() -> R) -> FudgeResult<()> {
                match chain {
                    Chain::Relay => self.relay_chain.with_state(exec),
                    Chain::Para(id) => match id {
                        (2001) => self
                            .parachain_1
                            .with_state(exec)
                            .map_err(|_| ())
                            .map(|_| ()),
                        (2002) => self
                            .parachain_2
                            .with_state(exec)
                            .map_err(|_| ())
                            .map(|_| ()),
                        _ => Err(()),
                    },
                }
            }
            pub fn with_mut_state<R>(
                &self,
                chain: Chain,
                exec: impl FnOnce() -> R,
            ) -> FudgeResult<()> {
                match chain {
                    Chain::Relay => self.relay_chain.with_mut_state(exec),
                    Chain::Para(id) => match id {
                        (2001) => self
                            .parachain_1
                            .with_mut_state(exec)
                            .map_err(|_| ())
                            .map(|_| ()),
                        (2002) => self
                            .parachain_2
                            .with_mut_state(exec)
                            .map_err(|_| ())
                            .map(|_| ()),
                        _ => Err(()),
                    },
                }
            }
            pub fn evolve(&mut self) -> FudgeResult<()> {
                self.relay_chain.build_block().map_err(|_| ()).map(|_| ())?;
                self.relay_chain
                    .import_block()
                    .map_err(|_| ())
                    .map(|_| ())?;
                self.parachain_1.build_block().map_err(|_| ()).map(|_| ())?;
                self.parachain_1
                    .import_block()
                    .map_err(|_| ())
                    .map(|_| ())?;
                let para = FudgeParaChain {
                    id: (2001),
                    head: self.parachain_1.head(),
                    code: self.parachain_1.code(),
                };
                self.relay_chain
                    .onboard_para(para)
                    .map_err(|_| ())
                    .map(|_| ())?;
                self.parachain_2.build_block().map_err(|_| ()).map(|_| ())?;
                self.parachain_2
                    .import_block()
                    .map_err(|_| ())
                    .map(|_| ())?;
                let para = FudgeParaChain {
                    id: (2002),
                    head: self.parachain_2.head(),
                    code: self.parachain_2.code(),
                };
                self.relay_chain
                    .onboard_para(para)
                    .map_err(|_| ())
                    .map(|_| ())?;
                self.relay_chain.build_block().map_err(|_| ()).map(|_| ())?;
                self.relay_chain
                    .import_block()
                    .map_err(|_| ())
                    .map(|_| ())?;
            }
        }
    }
}
mod digest {
    use sp_runtime::Digest;
    pub use babe::Digest as FudgeBabeDigest;
    mod babe {
        use sp_consensus_babe::digests::{PreDigest, SecondaryPlainPreDigest};
        use sp_std::time::Duration;
        use sp_timestamp::Timestamp;
        pub struct Digest;
        impl Digest {
            pub fn pre_digest(timestamp: Timestamp, slot_duration: Duration) -> PreDigest {
                let slot_wrap =
                    sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
                        timestamp,
                        slot_duration,
                    );
                PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
                    authority_index: 0,
                    slot: slot_wrap.slot(),
                })
            }
        }
    }
    pub trait DigestCreator {
        #[must_use]
        #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
        fn create_digest<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<Digest, ()>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait;
    }
    pub trait DigestProvider {
        #[must_use]
        #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
        fn build_digest<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<Digest, ()>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait;
        #[must_use]
        #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
        fn append_digest<'life0, 'life1, 'async_trait>(
            &'life0 self,
            digest: &'life1 mut Digest,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<(), ()>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            Self: 'async_trait;
    }
    impl<F, Fut> DigestCreator for F
    where
        F: Fn() -> Fut + Sync + Send,
        Fut: std::future::Future<Output = Result<Digest, ()>> + Send + 'static,
    {
        #[allow(
            clippy::let_unit_value,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn create_digest<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<Digest, ()>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) =
                    ::core::option::Option::None::<Result<Digest, ()>>
                {
                    return __ret;
                }
                let __self = self;
                let __ret: Result<Digest, ()> = { (*__self)().await };
                #[allow(unreachable_code)]
                __ret
            })
        }
    }
    impl DigestCreator for Box<dyn DigestCreator + Send + Sync> {
        #[allow(
            clippy::let_unit_value,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn create_digest<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<Digest, ()>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) =
                    ::core::option::Option::None::<Result<Digest, ()>>
                {
                    return __ret;
                }
                let __self = self;
                let __ret: Result<Digest, ()> = { (**__self).create_digest().await };
                #[allow(unreachable_code)]
                __ret
            })
        }
    }
}
mod inherent {
    pub use para_parachain::Inherent as FudgeInherentParaParachain;
    pub use relay_parachain::{
        DummyInherent as FudgeDummyInherentRelayParachain, Inherent as FudgeInherentRelayParachain,
    };
    pub use timestamp::CurrTimeProvider as FudgeInherentTimestamp;
    mod para_parachain {
        use cumulus_primitives_parachain_inherent::{ParachainInherentData, INHERENT_IDENTIFIER};
        use frame_support::inherent::{InherentData, InherentIdentifier};
        use sp_inherents::{Error, InherentDataProvider};
        pub struct Inherent(ParachainInherentData);
        impl Inherent {
            pub fn new(inherent: ParachainInherentData) -> Self {
                Inherent(inherent)
            }
        }
        impl InherentDataProvider for Inherent {
            fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
                inherent_data.put_data(INHERENT_IDENTIFIER, &self.0)
            }
            #[allow(
                clippy::let_unit_value,
                clippy::no_effect_underscore_binding,
                clippy::shadow_same,
                clippy::type_complexity,
                clippy::type_repetition_in_bounds,
                clippy::used_underscore_binding
            )]
            fn try_handle_error<'life0, 'life1, 'life2, 'async_trait>(
                &'life0 self,
                _identifier: &'life1 InherentIdentifier,
                _error: &'life2 [u8],
            ) -> ::core::pin::Pin<
                Box<
                    dyn ::core::future::Future<Output = Option<Result<(), Error>>>
                        + ::core::marker::Send
                        + 'async_trait,
                >,
            >
            where
                'life0: 'async_trait,
                'life1: 'async_trait,
                'life2: 'async_trait,
                Self: 'async_trait,
            {
                Box::pin(async move {
                    if let ::core::option::Option::Some(__ret) =
                        ::core::option::Option::None::<Option<Result<(), Error>>>
                    {
                        return __ret;
                    }
                    let __self = self;
                    let _identifier = _identifier;
                    let _error = _error;
                    let __ret: Option<Result<(), Error>> =
                        { ::core::panicking::panic("not yet implemented") };
                    #[allow(unreachable_code)]
                    __ret
                })
            }
        }
    }
    mod relay_parachain {
        use crate::builder::parachain::FudgeParaBuild;
        use polkadot_primitives::v1::{
            BackedCandidate, InherentData as ParachainsInherentData, PARACHAINS_INHERENT_IDENTIFIER,
        };
        use sp_inherents::{Error, InherentData, InherentDataProvider, InherentIdentifier};
        use sp_runtime::traits::Header;
        use sp_std::sync::Arc;
        pub struct DummyInherent<HDR> {
            parent_head: HDR,
        }
        impl<HDR: Header> DummyInherent<HDR> {
            pub fn new(parent_head: HDR) -> Self {
                Self { parent_head }
            }
        }
        impl<HDR: Header> InherentDataProvider for DummyInherent<HDR> {
            fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
                let inherent = ParachainsInherentData::<HDR> {
                    bitfields: ::alloc::vec::Vec::new(),
                    backed_candidates: ::alloc::vec::Vec::new(),
                    disputes: ::alloc::vec::Vec::new(),
                    parent_header: self.parent_head.clone(),
                };
                inherent_data.put_data(PARACHAINS_INHERENT_IDENTIFIER, &inherent)
            }
            #[allow(
                clippy::let_unit_value,
                clippy::no_effect_underscore_binding,
                clippy::shadow_same,
                clippy::type_complexity,
                clippy::type_repetition_in_bounds,
                clippy::used_underscore_binding
            )]
            fn try_handle_error<'life0, 'life1, 'life2, 'async_trait>(
                &'life0 self,
                _identifier: &'life1 InherentIdentifier,
                _error: &'life2 [u8],
            ) -> ::core::pin::Pin<
                Box<
                    dyn ::core::future::Future<Output = Option<Result<(), Error>>>
                        + ::core::marker::Send
                        + 'async_trait,
                >,
            >
            where
                'life0: 'async_trait,
                'life1: 'async_trait,
                'life2: 'async_trait,
                Self: 'async_trait,
            {
                Box::pin(async move {
                    if let ::core::option::Option::Some(__ret) =
                        ::core::option::Option::None::<Option<Result<(), Error>>>
                    {
                        return __ret;
                    }
                    let __self = self;
                    let _identifier = _identifier;
                    let _error = _error;
                    let __ret: Option<Result<(), Error>> =
                        { ::core::panicking::panic("not yet implemented") };
                    #[allow(unreachable_code)]
                    __ret
                })
            }
        }
        pub struct Inherent<HDR: Header, Client> {
            para_builds: Vec<FudgeParaBuild>,
            backed_candidates: Vec<BackedCandidate<HDR::Hash>>,
            parent_head: HDR,
            relay: Arc<Client>,
        }
        impl<HDR: Header, Client> Inherent<HDR, Client> {
            pub fn new(
                relay: Arc<Client>,
                parent_head: HDR,
                para_builds: Vec<FudgeParaBuild>,
                backed_candidates: Vec<BackedCandidate<HDR::Hash>>,
            ) -> Self {
                Self {
                    relay,
                    para_builds,
                    backed_candidates,
                    parent_head,
                }
            }
        }
        impl<HDR, Client> InherentDataProvider for Inherent<HDR, Client>
        where
            HDR: Header,
            Client: Sync + Send,
        {
            fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
                let inherent = ParachainsInherentData::<HDR> {
                    bitfields: ::alloc::vec::Vec::new(),
                    backed_candidates: ::alloc::vec::Vec::new(),
                    disputes: ::alloc::vec::Vec::new(),
                    parent_header: self.parent_head.clone(),
                };
                inherent_data.put_data(PARACHAINS_INHERENT_IDENTIFIER, &inherent)
            }
            #[allow(
                clippy::let_unit_value,
                clippy::no_effect_underscore_binding,
                clippy::shadow_same,
                clippy::type_complexity,
                clippy::type_repetition_in_bounds,
                clippy::used_underscore_binding
            )]
            fn try_handle_error<'life0, 'life1, 'life2, 'async_trait>(
                &'life0 self,
                _identifier: &'life1 InherentIdentifier,
                _error: &'life2 [u8],
            ) -> ::core::pin::Pin<
                Box<
                    dyn ::core::future::Future<Output = Option<Result<(), Error>>>
                        + ::core::marker::Send
                        + 'async_trait,
                >,
            >
            where
                'life0: 'async_trait,
                'life1: 'async_trait,
                'life2: 'async_trait,
                Self: 'async_trait,
            {
                Box::pin(async move {
                    if let ::core::option::Option::Some(__ret) =
                        ::core::option::Option::None::<Option<Result<(), Error>>>
                    {
                        return __ret;
                    }
                    let __self = self;
                    let _identifier = _identifier;
                    let _error = _error;
                    let __ret: Option<Result<(), Error>> =
                        { ::core::panicking::panic("not yet implemented") };
                    #[allow(unreachable_code)]
                    __ret
                })
            }
        }
    }
    mod timestamp {
        //! Inherent data providers that should only be used within FUDGE
        use sp_inherents::{InherentData, InherentIdentifier};
        use sp_std::collections::btree_map::BTreeMap;
        use sp_std::time::Duration;
        use sp_timestamp::{InherentError, INHERENT_IDENTIFIER};
        static mut INSTANCES: *mut BTreeMap<Instance, CurrTimeProvider> =
            0usize as *mut BTreeMap<Instance, CurrTimeProvider>;
        pub type Instance = u64;
        pub type Ticks = u128;
        pub struct CurrTimeProvider {
            instance: Instance,
            start: Duration,
            delta: Duration,
            ticks: Ticks,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for CurrTimeProvider {
            #[inline]
            fn clone(&self) -> CurrTimeProvider {
                match *self {
                    CurrTimeProvider {
                        instance: ref __self_0_0,
                        start: ref __self_0_1,
                        delta: ref __self_0_2,
                        ticks: ref __self_0_3,
                    } => CurrTimeProvider {
                        instance: ::core::clone::Clone::clone(&(*__self_0_0)),
                        start: ::core::clone::Clone::clone(&(*__self_0_1)),
                        delta: ::core::clone::Clone::clone(&(*__self_0_2)),
                        ticks: ::core::clone::Clone::clone(&(*__self_0_3)),
                    },
                }
            }
        }
        impl CurrTimeProvider {
            pub fn get_instance(instance: Instance) -> Self {
                let instances = unsafe {
                    if INSTANCES.is_null() {
                        let pt =
                            Box::into_raw(Box::new(BTreeMap::<Instance, CurrTimeProvider>::new()));
                        INSTANCES = pt;
                        &mut *INSTANCES
                    } else {
                        &mut *INSTANCES
                    }
                };
                instances.get(&instance).unwrap().clone()
            }
            pub fn new(instance: Instance, delta: Duration, start: Option<Duration>) -> Self {
                let instances = unsafe {
                    if INSTANCES.is_null() {
                        let pt =
                            Box::into_raw(Box::new(BTreeMap::<Instance, CurrTimeProvider>::new()));
                        INSTANCES = pt;
                        &mut *INSTANCES
                    } else {
                        &mut *INSTANCES
                    }
                };
                let start = if let Some(start) = start {
                    start
                } else {
                    let now = std::time::SystemTime::now();
                    let dur = now
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .expect("Current time is always after unix epoch; qed");
                    dur
                };
                instances
                    .entry(instance)
                    .or_insert(CurrTimeProvider {
                        instance,
                        start,
                        delta,
                        ticks: 0,
                    })
                    .clone()
            }
            pub fn update_time(&self) {
                let instances = unsafe { &mut *INSTANCES };
                instances.insert(
                    self.instance,
                    CurrTimeProvider {
                        start: self.start,
                        instance: self.instance,
                        delta: self.delta,
                        ticks: self.ticks + 1,
                    },
                );
            }
            pub fn current_time(&self) -> sp_timestamp::Timestamp {
                let delta: u128 = self.delta.as_millis() * self.ticks;
                let timestamp: u128 = self.start.as_millis() + delta;
                sp_timestamp::Timestamp::new(timestamp as u64)
            }
        }
        impl sp_inherents::InherentDataProvider for CurrTimeProvider {
            fn provide_inherent_data(
                &self,
                inherent_data: &mut InherentData,
            ) -> Result<(), sp_inherents::Error> {
                inherent_data
                    .put_data(INHERENT_IDENTIFIER, &self.current_time())
                    .unwrap();
                self.update_time();
                Ok(())
            }
            #[allow(
                clippy::let_unit_value,
                clippy::no_effect_underscore_binding,
                clippy::shadow_same,
                clippy::type_complexity,
                clippy::type_repetition_in_bounds,
                clippy::used_underscore_binding
            )]
            fn try_handle_error<'life0, 'life1, 'life2, 'async_trait>(
                &'life0 self,
                identifier: &'life1 InherentIdentifier,
                error: &'life2 [u8],
            ) -> ::core::pin::Pin<
                Box<
                    dyn ::core::future::Future<Output = Option<Result<(), sp_inherents::Error>>>
                        + ::core::marker::Send
                        + 'async_trait,
                >,
            >
            where
                'life0: 'async_trait,
                'life1: 'async_trait,
                'life2: 'async_trait,
                Self: 'async_trait,
            {
                Box::pin(async move {
                    if let ::core::option::Option::Some(__ret) =
                        ::core::option::Option::None::<Option<Result<(), sp_inherents::Error>>>
                    {
                        return __ret;
                    }
                    let __self = self;
                    let identifier = identifier;
                    let error = error;
                    let __ret: Option<Result<(), sp_inherents::Error>> = {
                        if *identifier != INHERENT_IDENTIFIER {
                            return None;
                        }
                        match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
                            InherentError::ValidAtTimestamp(_valid) => Some(Ok(())),
                            o => Some(Err(sp_inherents::Error::Application(Box::from(o)))),
                        }
                    };
                    #[allow(unreachable_code)]
                    __ret
                })
            }
        }
    }
    pub trait ArgsProvider<ExtraArgs> {
        fn extra() -> ExtraArgs;
    }
    impl ArgsProvider<()> for () {
        fn extra() -> () {
            ()
        }
    }
}
mod provider {
    use crate::provider::state_provider::StateProvider;
    pub use externalities_provider::ExternalitiesProvider;
    use sc_executor::RuntimeVersionOf;
    use sc_service::config::ExecutionStrategies;
    use sc_service::{
        ClientConfig, Configuration, KeystoreContainer, TFullBackend, TFullCallExecutor,
        TFullClient, TaskManager,
    };
    use sp_api::{BlockT, ConstructRuntimeApi};
    use sp_core::traits::{CodeExecutor, SpawnNamed};
    use sp_keystore::SyncCryptoStorePtr;
    use sp_runtime::BuildStorage;
    use sp_std::marker::PhantomData;
    use sp_std::str::FromStr;
    use sp_std::sync::Arc;
    use sp_storage::Storage;
    use std::path::PathBuf;
    mod externalities_provider {
        use sp_core::Hasher;
        use sp_externalities::Externalities;
        use sp_state_machine::{
            Backend, Ext, OverlayedChanges, StorageChanges, StorageTransactionCache,
        };
        use sp_storage::StateVersion;
        use std::panic::{AssertUnwindSafe, UnwindSafe};
        pub struct ExternalitiesProvider<'a, H, B>
        where
            H: Hasher,
            H::Out: codec::Codec + Ord + 'static,
            B: Backend<H>,
        {
            overlay: OverlayedChanges,
            storage_transaction_cache: StorageTransactionCache<<B as Backend<H>>::Transaction, H>,
            backend: &'a B,
        }
        impl<'a, H, B> ExternalitiesProvider<'a, H, B>
        where
            H: Hasher,
            H::Out: codec::Codec + Ord + 'static,
            B: Backend<H>,
        {
            pub fn new(backend: &'a B) -> Self {
                Self {
                    backend,
                    storage_transaction_cache: StorageTransactionCache::default(),
                    overlay: OverlayedChanges::default(),
                }
            }
            /// Get externalities implementation.
            pub fn ext(&mut self) -> Ext<H, B> {
                Ext::new(
                    &mut self.overlay,
                    &mut self.storage_transaction_cache,
                    &self.backend,
                    None,
                )
            }
            /// Execute the given closure while `self` is set as externalities.
            ///
            /// Returns the result of the given closure.
            pub fn execute_with<R>(&mut self, execute: impl FnOnce() -> R) -> R {
                let mut ext = self.ext();
                sp_externalities::set_and_run_with_externalities(&mut ext, execute)
            }
            pub fn execute_with_mut<R>(
                &mut self,
                execute: impl FnOnce() -> R,
            ) -> (R, StorageChanges<B::Transaction, H>) {
                let parent_hash = self.overlay.storage_root(
                    self.backend,
                    &mut self.storage_transaction_cache,
                    StateVersion::V0,
                );
                let mut ext = self.ext();
                ext.storage_start_transaction();
                let r = sp_externalities::set_and_run_with_externalities(&mut ext, execute);
                ext.storage_commit_transaction().unwrap();
                (
                    r,
                    self.overlay
                        .drain_storage_changes::<B, H>(
                            self.backend,
                            parent_hash,
                            &mut self.storage_transaction_cache,
                            StateVersion::V0,
                        )
                        .unwrap(),
                )
            }
            /// Execute the given closure while `self` is set as externalities.
            ///
            /// Returns the result of the given closure, if no panics occured.
            /// Otherwise, returns `Err`.
            #[allow(dead_code)]
            pub fn execute_with_safe<R>(
                &mut self,
                f: impl FnOnce() -> R + UnwindSafe,
            ) -> Result<R, String> {
                let mut ext = AssertUnwindSafe(self.ext());
                std::panic::catch_unwind(move || {
                    sp_externalities::set_and_run_with_externalities(&mut *ext, f)
                })
                .map_err(|e| {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Closure panicked: "],
                        &match (&e,) {
                            (arg0,) => {
                                [::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt)]
                            }
                        },
                    ));
                    res
                })
            }
        }
    }
    mod state_provider {
        use sc_client_api::Backend;
        use sc_client_db::{DatabaseSettings, DatabaseSource, KeepBlocks, TransactionStorageMode};
        use sc_service::PruningMode;
        use sp_core::storage::well_known_keys::CODE;
        use sp_database::MemDb;
        use sp_runtime::traits::Block as BlockT;
        use sp_runtime::BuildStorage;
        use sp_std::{marker::PhantomData, sync::Arc};
        use sp_storage::Storage;
        use std::path::PathBuf;
        pub const CANONICALIZATION_DELAY: u64 = 4096;
        pub struct StateProvider<B, Block> {
            backend: Arc<B>,
            pseudo_genesis: Storage,
            _phantom: PhantomData<Block>,
        }
        impl<B, Block> StateProvider<B, Block>
        where
            Block: BlockT,
            B: Backend<Block>,
        {
            pub fn insert_storage(&mut self, storage: Storage) -> &mut Self {
                let Storage {
                    top,
                    children_default: _children_default,
                } = storage;
                self.pseudo_genesis.top.extend(top.into_iter());
                self
            }
            pub fn backend(&self) -> Arc<B> {
                self.backend.clone()
            }
        }
        impl<Block> StateProvider<sc_client_db::Backend<Block>, Block>
        where
            Block: BlockT,
        {
            pub fn from_db(path: PathBuf) -> Self {
                let settings = DatabaseSettings {
                    state_cache_size: 0,
                    state_cache_child_ratio: None,
                    state_pruning: PruningMode::ArchiveAll,
                    source: DatabaseSource::RocksDb {
                        path: path.clone(),
                        cache_size: 0,
                    },
                    keep_blocks: KeepBlocks::All,
                    transaction_storage: TransactionStorageMode::BlockBody,
                };
                let backend = Arc::new(
                    sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY)
                        .map_err(|_| ())
                        .unwrap(),
                );
                Self {
                    backend,
                    pseudo_genesis: Storage::default(),
                    _phantom: Default::default(),
                }
            }
            pub fn from_spec() -> Self {
                ::core::panicking::panic("not yet implemented")
            }
            pub fn from_storage(storage: Storage) -> Self {
                let mut provider = StateProvider::empty_default(None);
                provider.insert_storage(storage);
                provider
            }
            pub fn empty_default(code: Option<&[u8]>) -> Self {
                let mut provider = StateProvider::with_in_mem_db().unwrap();
                let mut storage = Storage::default();
                if let Some(code) = code {
                    storage.top.insert(CODE.to_vec(), code.to_vec());
                }
                provider.insert_storage(storage);
                provider
            }
            fn with_in_mem_db() -> Result<Self, ()> {
                let settings = DatabaseSettings {
                    state_cache_size: 0,
                    state_cache_child_ratio: None,
                    state_pruning: PruningMode::ArchiveAll,
                    source: DatabaseSource::Custom(Arc::new(MemDb::new())),
                    keep_blocks: KeepBlocks::All,
                    transaction_storage: TransactionStorageMode::BlockBody,
                };
                let backend = Arc::new(
                    sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY).map_err(|_| ())?,
                );
                Ok(Self {
                    backend,
                    pseudo_genesis: Storage::default(),
                    _phantom: Default::default(),
                })
            }
        }
        impl<B, Block> BuildStorage for StateProvider<B, Block>
        where
            Block: BlockT,
            B: Backend<Block>,
        {
            fn build_storage(&self) -> Result<Storage, String> {
                Ok(self.pseudo_genesis.clone())
            }
            fn assimilate_storage(&self, _storage: &mut Storage) -> Result<(), String> {
                ::core::panicking::panic("not yet implemented")
            }
        }
    }
    pub struct EnvProvider<Block, RtApi, Exec>
    where
        Block: BlockT,
    {
        state: StateProvider<TFullBackend<Block>, Block>,
        _phantom: PhantomData<(Block, RtApi, Exec)>,
    }
    impl<Block, RtApi, Exec> EnvProvider<Block, RtApi, Exec>
    where
        Block: BlockT,
        Block::Hash: FromStr,
        RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send,
        Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
    {
        pub fn empty() -> Self {
            Self {
                state: StateProvider::empty_default(None),
                _phantom: Default::default(),
            }
        }
        pub fn with_code(code: &'static [u8]) -> Self {
            Self {
                state: StateProvider::empty_default(Some(code)),
                _phantom: Default::default(),
            }
        }
        pub fn from_spec(spec: &dyn BuildStorage) -> Self {
            let storage = spec.build_storage().unwrap();
            Self::from_storage(storage)
        }
        pub fn into_from_config(
            config: &Configuration,
            exec: Exec,
        ) -> (
            TFullClient<Block, RtApi, Exec>,
            Arc<TFullBackend<Block>>,
            KeystoreContainer,
            TaskManager,
        ) {
            sc_service::new_full_parts(config, None, exec)
                .map_err(|_| "err".to_string())
                .unwrap()
        }
        pub fn from_storage(storage: Storage) -> Self {
            Self {
                state: StateProvider::from_storage(storage),
                _phantom: Default::default(),
            }
        }
        pub fn from_db(path: PathBuf) -> Self {
            Self {
                state: StateProvider::from_db(path),
                _phantom: Default::default(),
            }
        }
        pub fn from_storage_with_code(storage: Storage, code: &'static [u8]) -> Self {
            let mut state = StateProvider::empty_default(Some(code));
            state.insert_storage(storage);
            Self {
                state,
                _phantom: Default::default(),
            }
        }
        pub fn insert_storage(&mut self, storage: Storage) -> &mut Self {
            self.state.insert_storage(storage);
            self
        }
        pub fn init_default(
            self,
            exec: Exec,
            handle: Box<dyn SpawnNamed>,
        ) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
            self.init(exec, handle, None, None)
        }
        pub fn init_with_config(
            self,
            exec: Exec,
            handle: Box<dyn SpawnNamed>,
            config: ClientConfig<Block>,
        ) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
            self.init(exec, handle, None, Some(config))
        }
        pub fn init_full(
            self,
            exec: Exec,
            handle: Box<dyn SpawnNamed>,
            keystore: SyncCryptoStorePtr,
            config: ClientConfig<Block>,
        ) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
            self.init(exec, handle, Some(keystore), Some(config))
        }
        pub fn init_with_keystore(
            self,
            exec: Exec,
            handle: Box<dyn SpawnNamed>,
            keystore: SyncCryptoStorePtr,
        ) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
            self.init(exec, handle, Some(keystore), None)
        }
        fn init(
            self,
            exec: Exec,
            handle: Box<dyn SpawnNamed>,
            keystore: Option<SyncCryptoStorePtr>,
            config: Option<ClientConfig<Block>>,
        ) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
            let backend = self.state.backend();
            let config = config.clone().unwrap_or(Self::client_config());
            let executor = sc_service::client::LocalCallExecutor::new(
                backend.clone(),
                exec,
                handle,
                config.clone(),
            )
            .unwrap();
            let extensions = sc_client_api::execution_extensions::ExecutionExtensions::new(
                ExecutionStrategies::default(),
                keystore,
                sc_offchain::OffchainDb::factory_from_backend(&*backend),
            );
            let client = sc_service::client::Client::<
                TFullBackend<Block>,
                TFullCallExecutor<Block, Exec>,
                Block,
                RtApi,
            >::new(
                backend.clone(),
                executor,
                &self.state,
                None,
                None,
                extensions,
                None,
                None,
                config.clone(),
            )
            .map_err(|_| "err".to_string())
            .unwrap();
            (client, backend)
        }
        fn client_config() -> ClientConfig<Block> {
            ClientConfig::default()
        }
    }
}
mod types {
    pub type Bytes = Vec<u8>;
    pub struct StoragePair {
        key: Bytes,
        value: Option<Bytes>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for StoragePair {
        #[inline]
        fn clone(&self) -> StoragePair {
            match *self {
                StoragePair {
                    key: ref __self_0_0,
                    value: ref __self_0_1,
                } => StoragePair {
                    key: ::core::clone::Clone::clone(&(*__self_0_0)),
                    value: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for StoragePair {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                StoragePair {
                    key: ref __self_0_0,
                    value: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "StoragePair");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "key",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "value",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
}
pub mod traits {
    ///! myChain traits and possible default implementations
    use super::StoragePair;
    /// A unification trait that must be super trait of all
    /// providers that inject keys, values
    ///
    /// Traits using this as super trait should call their respective
    /// calls to provide key-values inside the `injections` call+
    pub trait InjectionProvider {
        fn injections() -> Vec<StoragePair>;
    }
    /// A trait that can be implemented by chains to provide their own authority that
    /// they want to swap all the time when using mychain.
    pub trait AuthorityProvider: InjectionProvider {
        fn block_production() -> Vec<StoragePair>;
        fn misc() -> Vec<StoragePair>;
    }
    pub struct DefaultAuthorityProvider;
    impl InjectionProvider for DefaultAuthorityProvider {
        fn injections() -> Vec<StoragePair> {
            let mut pairs = DefaultAuthorityProvider::block_production();
            pairs.extend_from_slice(DefaultAuthorityProvider::misc().as_slice());
            pairs
        }
    }
    impl AuthorityProvider for DefaultAuthorityProvider {
        fn block_production() -> Vec<StoragePair> {
            ::core::panicking::panic("not yet implemented")
        }
        fn misc() -> Vec<StoragePair> {
            Vec::new()
        }
    }
}
