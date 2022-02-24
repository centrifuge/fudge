// Copyright 2021 Centrifuge Foundation (centrifuge.io).
//
// This file is part of the FUDGE project.
// FUDGE is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).
// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use frame_support::traits::Get;
use sc_client_api::{
    blockchain::ProvideCache, AuxStore, Backend as BackendT, BlockOf, HeaderBackend,
    UsageProvider,
};
use sc_client_db::Backend;
use sc_executor::RuntimeVersionOf;
use sc_service::{Configuration, SpawnTaskHandle, TFullClient};
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, Core as CoreApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, CanAuthorWith, Error as ConsensusError};
use sp_core::{
    traits::{CodeExecutor, ReadRuntimeVersion},
    Pair,
};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
use crate::{
    types::{StoragePair, Bytes},
    builder::core::{Builder, Operation}
};


pub struct StandAloneBuilder<
    Block,
    RtApi,
    Exec,
    B = Backend<Block>,
    C = TFullClient<Block, RtApi, Exec>,
> where
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
    + sc_block_builder::BlockBuilderProvider<B, Block, C>
{
    builder: Builder<Block, RtApi, Exec, B, C>,
    blocks: Vec<Block>
}

impl<Block, RtApi, Exec, B, C> StandAloneBuilder<Block, RtApi, Exec, B, C>
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
        + sc_block_builder::BlockBuilderProvider<B, Block, C>
{
    pub fn new(backend: Arc<B>, client: Arc<C>) -> Self {
        Self {
            builder: Builder::new(backend, client),
            blocks: Vec::new()
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

    pub fn append_xcm(&mut self, xcm: Bytes) -> &mut Self {
        todo!()
    }

    pub fn append_xcms(&mut self, xcms: Vec<Bytes>) -> &mut Self {
        todo!()
    }

    pub fn build_block<CID: CreateInherentDataProviders<Block, CidArgs>, CidArgs: Get<CidArgs>>(&mut self, cid: CID, handle: SpawnTaskHandle) -> &mut Self {
        let provider = futures::executor::block_on(cid.create_inherent_data_providers(self.builder.latest_block(), CidArgs::get())).unwrap();
        let block = self.builder.build_block(handle, provider.create_inherent_data().unwrap(), Default::default(), Duration::from_secs(60), 6_000_000);
        self.blocks.push(block);
        self
    }

    pub fn blocks(&self) -> Vec<Block> {
        self.blocks.clone()
    }

    pub fn with_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
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
        self.builder.with_state(Operation::Commit, None, exec)
    }

    pub fn with_mut_state_at<R>(
        &mut self,
        at: BlockId<Block>,
        exec: impl FnOnce() -> R,
    ) -> Result<R, String> {
        self.builder.with_state(Operation::Commit, Some(at), exec)
    }
}
