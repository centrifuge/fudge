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

use cumulus_primitives_parachain_inherent::{ParachainInherentData, INHERENT_IDENTIFIER};
use frame_support::inherent::{InherentData, InherentIdentifier};
use sp_api::StorageProof;
use sp_inherents::{Error, InherentDataProvider};

pub struct Inherent(ParachainInherentData);

impl Inherent {
	pub fn new() -> Self {
		Inherent(ParachainInherentData {
			validation_data: Default::default(),
			relay_chain_state: StorageProof::new(vec![]),
			downward_messages: vec![],
			horizontal_messages: Default::default(),
		})
	}
}

#[async_trait::async_trait]
impl InherentDataProvider for Inherent {
	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.0)
	}

	async fn try_handle_error(
		&self,
		_identifier: &InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), Error>> {
		todo!()
	}
}

/*
Implementation of mock from cumulus


#[async_trait::async_trait]
impl InherentDataProvider for MockValidationDataInherentDataProvider {
	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		// Calculate the mocked relay block based on the current para block
		let relay_parent_number =
			self.relay_offset + self.relay_blocks_per_para_block * self.current_para_block;

		// Use the "sproof" (spoof proof) builder to build valid mock state root and proof.
		let mut sproof_builder = RelayStateSproofBuilder::default();
		sproof_builder.para_id = self.xcm_config.para_id;

		// Process the downward messages and set up the correct head
		let mut downward_messages = Vec::new();
		let mut dmq_mqc = crate::MessageQueueChain(self.xcm_config.starting_dmq_mqc_head);
		for msg in &self.raw_downward_messages {
			let wrapped = InboundDownwardMessage { sent_at: relay_parent_number, msg: msg.clone() };

			dmq_mqc.extend_downward(&wrapped);
			downward_messages.push(wrapped);
		}
		sproof_builder.dmq_mqc_head = Some(dmq_mqc.head());

		// Process the hrmp messages and set up the correct heads
		// Begin by collecting them into a Map
		let mut horizontal_messages = BTreeMap::<ParaId, Vec<InboundHrmpMessage>>::new();
		for (para_id, msg) in &self.raw_horizontal_messages {
			let wrapped = InboundHrmpMessage { sent_at: relay_parent_number, data: msg.clone() };

			horizontal_messages.entry(*para_id).or_default().push(wrapped);
		}

		// Now iterate again, updating the heads as we go
		for (para_id, messages) in &horizontal_messages {
			let mut channel_mqc = crate::MessageQueueChain(
				*self
					.xcm_config
					.starting_hrmp_mqc_heads
					.get(para_id)
					.unwrap_or(&relay_chain::Hash::default()),
			);
			for message in messages {
				channel_mqc.extend_hrmp(message);
			}
			sproof_builder.upsert_inbound_channel(*para_id).mqc_head = Some(channel_mqc.head());
		}

		let (relay_parent_storage_root, proof) = sproof_builder.into_state_root_and_proof();

		inherent_data.put_data(
			INHERENT_IDENTIFIER,
			&ParachainInherentData {
				validation_data: PersistedValidationData {
					parent_head: Default::default(),
					relay_parent_storage_root,
					relay_parent_number,
					max_pov_size: Default::default(),
				},
				downward_messages,
				horizontal_messages,
				relay_chain_state: proof,
			},
		)
	}

	// Copied from the real implementation
	async fn try_handle_error(
		&self,
		_: &sp_inherents::InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		None
	}
}

 */
