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

//! Inherent data providers that should only be used within FUDGE

use sp_inherents::{InherentData, InherentIdentifier};
use sp_std::collections::btree_map::BTreeMap;
use sp_timestamp::InherentError;

static mut INSTANCES: *mut BTreeMap<Instance, Ticks> = 0usize as *mut BTreeMap<Instance, Ticks>;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"timstap0";

pub type Instance = u64;
pub type Ticks = u64;
pub type Moment = u64;

pub struct Inherent {
	instance: Instance,
	start: Moment,
	ticks: Ticks,
	delta: Moment,
}

impl Inherent {
	pub fn new(instance: Instance, delta: Moment, start: Option<Moment>) -> Self {
		let instances = unsafe {
			if INSTANCES.is_null() {
				let pt = Box::into_raw(Box::new(BTreeMap::<Instance, Ticks>::new()));
				INSTANCES = pt;

				&mut *INSTANCES
			} else {
				&mut *INSTANCES
			}
		};

		let ticks = instances.entry(instance).or_insert(0).clone();
		let start = if let Some(start) = start {
			start
		} else {
			let now = std::time::SystemTime::now();
			let dur = now
				.duration_since(std::time::SystemTime::UNIX_EPOCH)
				.expect("Current time is always after unix epoch; qed");

			// TODO: Is this correct?
			dur.as_millis() as u64
		};

		Inherent {
			instance,
			start,
			ticks,
			delta,
		}
	}

	pub fn current_time(&self) -> sp_timestamp::Timestamp {
		//let timestamp = self.start + (self.delta * self.ticks);
		let timestamp = self.start * self.ticks;

		let instances = unsafe { &mut *INSTANCES };

		let val = instances
			.get_mut(&self.instance)
			.expect("Instance is initialised. qed.");
		*val = *val + 1;

		sp_timestamp::Timestamp::new(timestamp)
	}
}

#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for Inherent {
	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.current_time())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if *identifier != INHERENT_IDENTIFIER {
			return None;
		}

		match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
			InherentError::ValidAtTimestamp(_valid) => {
				/*
				let max_drift = self.max_drift;
				let timestamp = self.timestamp;
				// halt import until timestamp is valid.
				// reject when too far ahead.
				if valid > timestamp + max_drift {
					return Some(Err(sp_inherents::Error::Application(Box::from(
						InherentError::TooFarInFuture,
					))))
				}

				let diff = valid.checked_sub(timestamp).unwrap_or_default();
				log::info!(
					target: "timestamp",
					"halting for block {} milliseconds in the future",
					diff.0,
				);

				futures_timer::Delay::new(diff.as_duration()).await;
				*/
				// TODO: Sane?
				Some(Ok(()))
			}
			o => Some(Err(sp_inherents::Error::Application(Box::from(o)))),
		}
	}
}
