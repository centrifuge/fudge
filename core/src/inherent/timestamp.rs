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

use sp_core::sp_std::sync::Arc;
use sp_inherents::{InherentData, InherentIdentifier};
use sp_runtime::SaturatedConversion;
use sp_std::{
	collections::btree_map::BTreeMap,
	sync::{
		atomic::{AtomicU64, Ordering},
		Mutex,
	},
	time::Duration,
};
use sp_timestamp::{InherentError, INHERENT_IDENTIFIER};
use thiserror::Error;

const DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET: &str = "fudge-timestamp";

type InnerError = Box<dyn std::error::Error>;

#[derive(Error, Debug)]
pub enum Error {
	#[error("lock for instances is poisoned: {0}")]
	InstancesLockPoisoned(InnerError),

	#[error("current time retrieval: {0}")]
	CurrentTimeRetrieval(InnerError),

	#[error("instance with ID {0:?} not found")]
	InstanceNotFound(InstanceId),
}

lazy_static::lazy_static!(
	pub static ref INSTANCES: Arc<Mutex<BTreeMap<InstanceId, CurrTimeProvider>>> = Arc::new(Mutex::new(BTreeMap::new()));
	pub static ref COUNTER: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
);

#[derive(Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug)]
pub struct InstanceId(u64);

pub type Ticks = u128;

#[derive(Clone, Debug)]
pub struct CurrTimeProvider {
	instance_id: InstanceId,
	start: Duration,
	delta: Duration,
	ticks: Ticks,
}

impl CurrTimeProvider {
	/// Overwrites an existing instance_id. If the instance wasn't already created, it returns an error.
	///
	/// To get the actual instance please use `CurrTimeProvider::get_instance()`.
	pub fn force_new(
		instance_id: InstanceId,
		delta: Duration,
		start: Option<Duration>,
	) -> Result<(), Error> {
		let storage = INSTANCES.clone();
		let mut locked_instances = storage.lock().map_err(|e| {
			tracing::error!(
				target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
				error = ?e,
				"Instances lock is poisoned",
			);

			Error::InstancesLockPoisoned(Box::<dyn std::error::Error>::from(e.to_string()))
		})?;

		let start = if let Some(start) = start {
			start
		} else {
			let now = std::time::SystemTime::now();
			let dur = now
				.duration_since(std::time::SystemTime::UNIX_EPOCH)
				.map_err(|e| {
					tracing::error!(
						target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
						error = ?e,
						"Could not get current time",
					);

					Error::CurrentTimeRetrieval(e.into())
				})?;

			dur
		};

		locked_instances
			.get_mut(&instance_id)
			.map(|time| {
				*time = CurrTimeProvider {
					instance_id,
					start,
					delta,
					ticks: 0,
				};
			})
			.ok_or({
				tracing::error!(
					target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
					"Instance not found",
				);

				Error::InstanceNotFound(instance_id)
			})
	}

	pub fn create_instance(delta: Duration, start: Option<Duration>) -> Result<InstanceId, Error> {
		let instance_id = InstanceId(COUNTER.fetch_add(1, Ordering::SeqCst));

		let start = if let Some(start) = start {
			start
		} else {
			let now = std::time::SystemTime::now();
			let dur = now
				.duration_since(std::time::SystemTime::UNIX_EPOCH)
				.map_err(|e| {
					tracing::error!(
						target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
						error = ?e,
						"Could not get current time",
					);

					Error::CurrentTimeRetrieval(e.into())
				})?;

			dur
		};

		let storage = INSTANCES.clone();
		let mut locked_instances = storage.lock().map_err(|e| {
			tracing::error!(
				target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
				error = ?e,
				"Instances lock is poisoned",
			);

			Error::InstancesLockPoisoned(Box::<dyn std::error::Error>::from(e.to_string()))
		})?;

		locked_instances.insert(
			instance_id,
			CurrTimeProvider {
				instance_id,
				start,
				delta,
				ticks: 0,
			},
		);

		Ok(instance_id)
	}

	pub fn get_instance(instance_id: InstanceId) -> Result<Self, Error> {
		let storage = INSTANCES.clone();
		let locked_instances = storage.lock().map_err(|e| {
			tracing::error!(
				target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
				error = ?e,
				"Instances lock is poisoned",
			);

			Error::InstancesLockPoisoned(Box::<dyn std::error::Error>::from(e.to_string()))
		})?;

		locked_instances
			.get(&instance_id)
			.map(|instance| instance.clone())
			.ok_or({
				tracing::error!(
					target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
					"Instance not found",
				);

				Error::InstanceNotFound(instance_id)
			})
	}

	fn update_time(&self) -> Result<(), Error> {
		let storage = INSTANCES.clone();
		let mut instances = storage.lock().map_err(|e| {
			tracing::error!(
				target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
				error = ?e,
				"Instances lock is poisoned",
			);

			Error::InstancesLockPoisoned(Box::<dyn std::error::Error>::from(e.to_string()))
		})?;

		let instance = instances.get_mut(&self.instance_id).ok_or({
			tracing::error!(
				target = DEFAULT_TIMESTAMP_PROVIDER_LOG_TARGET,
				"Instance not found",
			);

			Error::InstanceNotFound(self.instance_id.clone())
		})?;

		instance.ticks = instance.ticks + 1;

		Ok(())
	}

	pub fn current_time(&self) -> sp_timestamp::Timestamp {
		let delta: u128 = self.delta.as_millis().saturating_mul(self.ticks);
		let timestamp: u128 = self.start.as_millis().saturating_add(delta);
		sp_timestamp::Timestamp::new(timestamp.saturated_into::<u64>())
	}
}

#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for CurrTimeProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.current_time())?;
		self.update_time().map_err(|e| {
			sp_inherents::Error::Application(Box::<dyn std::error::Error + Send + Sync>::from(
				e.to_string(),
			))
		})
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
			InherentError::ValidAtTimestamp(_valid) => Some(Ok(())),
			o => Some(Err(sp_inherents::Error::Application(Box::from(o)))),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Sat Jan 01 2022 00:00:00 GMT+0000
	pub const START_DATE: u64 = 1640995200u64;

	#[test]
	fn steps_work_correctly() {
		const DELTA: u64 = 12u64;

		let delta = Duration::from_secs(DELTA);
		let instance_id =
			CurrTimeProvider::create_instance(delta, Some(Duration::from_secs(START_DATE)))
				.unwrap();
		let time = CurrTimeProvider::get_instance(instance_id).unwrap();

		assert_eq!(
			time.current_time().as_duration().as_secs() as u64,
			START_DATE
		);

		// Progress time by delta
		time.update_time().unwrap();
		let time = CurrTimeProvider::get_instance(instance_id).unwrap();
		assert_eq!(
			time.current_time().as_duration().as_secs() as u64,
			START_DATE + delta.as_secs() as u64
		);

		// Progress time by delta
		time.update_time().unwrap();
		let time = CurrTimeProvider::get_instance(instance_id).unwrap();
		assert_eq!(
			time.current_time().as_duration().as_secs() as u64,
			START_DATE + 2 * delta.as_secs() as u64
		);
	}

	#[test]
	fn instances_work_correctly() {
		const DELTA_A: u64 = 12u64;
		const DELTA_B: u64 = 6u64;

		let delta_a = Duration::from_secs(DELTA_A);
		let instance_a =
			CurrTimeProvider::create_instance(delta_a, Some(Duration::from_secs(START_DATE)))
				.unwrap();
		let time_a =
			CurrTimeProvider::get_instance(instance_a).expect("Instance is initialized. qed");

		let delta_b = Duration::from_secs(DELTA_A);
		let instance_b =
			CurrTimeProvider::create_instance(delta_b, Some(Duration::from_secs(START_DATE)))
				.unwrap();
		let time_b =
			CurrTimeProvider::get_instance(instance_b).expect("Instance is initialized. qed");

		assert_eq!(
			time_a.current_time().as_duration().as_secs() as u64,
			START_DATE
		);
		assert_eq!(
			time_b.current_time().as_duration().as_secs() as u64,
			START_DATE
		);

		// Progress time by delta
		time_a.update_time().unwrap();
		let time_a =
			CurrTimeProvider::get_instance(instance_a).expect("Instance is initialized. qed");
		time_b.update_time().unwrap();
		let time_b =
			CurrTimeProvider::get_instance(instance_b).expect("Instance is initialized. qed");
		assert_eq!(
			time_a.current_time().as_duration().as_secs() as u64,
			START_DATE + delta_a.as_secs() as u64
		);
		assert_eq!(
			time_b.current_time().as_duration().as_secs() as u64,
			START_DATE + delta_b.as_secs() as u64
		);

		// Progress time by delta
		time_a.update_time().unwrap();
		let time_a =
			CurrTimeProvider::get_instance(instance_a).expect("Instance is initialized. qed");
		time_b.update_time().unwrap();
		let time_b =
			CurrTimeProvider::get_instance(instance_b).expect("Instance is initialized. qed");
		assert_eq!(
			time_a.current_time().as_duration().as_secs() as u64,
			START_DATE + 2 * delta_a.as_secs() as u64
		);
		assert_eq!(
			time_b.current_time().as_duration().as_secs() as u64,
			START_DATE + 2 * delta_b.as_secs() as u64
		);

		let time_a_2 =
			CurrTimeProvider::get_instance(instance_a).expect("Instance is available. qed");
		let time_b_2 =
			CurrTimeProvider::get_instance(instance_b).expect("Instance is available. qed");

		assert_eq!(time_a.current_time(), time_a_2.current_time());
		assert_eq!(time_b.current_time(), time_b_2.current_time());
	}
}
