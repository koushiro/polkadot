// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use super::{Trait, Module, Store};
use crate::configuration::{self, HostConfiguration};
use sp_std::prelude::*;
use sp_std::collections::{btree_map::BTreeMap, vec_deque::VecDeque};
use frame_support::{StorageMap, StorageValue, weights::Weight, traits::Get};
use primitives::v1::{Id as ParaId, UpwardMessage};

pub trait UmpSink {
	/// Process an incoming upward message and return the amount of weight it consumed.
	fn process_upward_message(origin: ParaId, msg: Vec<u8>) -> Weight;
}

impl UmpSink for () {
	fn process_upward_message(_: ParaId, _: Vec<u8>) -> Weight {
		0
	}
}

/// Routines related to the upward message passing.
impl<T: Trait> Module<T> {
	pub(super) fn outgoing_para_cleanup_ump(outgoing_para: ParaId) {
		<Self as Store>::RelayDispatchQueueSize::remove(&outgoing_para);
		<Self as Store>::RelayDispatchQueues::remove(&outgoing_para);
		<Self as Store>::NeedsDispatch::mutate(|v| {
			if let Ok(i) = v.binary_search(&outgoing_para) {
				v.remove(i);
			}
		});
		<Self as Store>::NextDispatchRoundStartWith::mutate(|v| {
			*v = v.filter(|p| *p == outgoing_para)
		});
	}

	/// Check that all the upward messages sent by a candidate pass the acceptance criteria. Returns
	/// false, if any of the messages doesn't pass.
	pub(crate) fn check_upward_messages(
		config: &HostConfiguration<T::BlockNumber>,
		para: ParaId,
		upward_messages: &[UpwardMessage],
	) -> bool {
		if upward_messages.len() as u32 > config.max_upward_message_num_per_candidate {
			return false;
		}

		let (mut para_queue_count, mut para_queue_size) =
			<Self as Store>::RelayDispatchQueueSize::get(&para);

		for msg in upward_messages {
			para_queue_count += 1;
			para_queue_size += msg.len() as u32;
		}

		// make sure that the queue is not overfilled.
		// we do it here only once since returning false invalidates the whole relay-chain block.
		if para_queue_count > config.max_upward_queue_count
			|| para_queue_size > config.max_upward_queue_size
		{
			return false;
		}

		true
	}

	/// Enacts all the upward messages sent by a candidate.
	pub(crate) fn enact_upward_messages(
		para: ParaId,
		upward_messages: Vec<UpwardMessage>,
	) -> Weight {
		let mut weight = 0;

		if !upward_messages.is_empty() {
			let (extra_cnt, extra_size) = upward_messages
				.iter()
				.fold((0, 0), |(cnt, size), d| (cnt + 1, size + d.len() as u32));

			<Self as Store>::RelayDispatchQueues::mutate(&para, |v| {
				v.extend(upward_messages.into_iter())
			});

			<Self as Store>::RelayDispatchQueueSize::mutate(
				&para,
				|(ref mut cnt, ref mut size)| {
					*cnt += extra_cnt;
					*size += extra_size;
				},
			);

			<Self as Store>::NeedsDispatch::mutate(|v| {
				if let Err(i) = v.binary_search(&para) {
					v.insert(i, para);
				}
			});

			weight += T::DbWeight::get().reads_writes(3, 3);
		}

		weight
	}

	/// Devote some time into dispatching pending upward messages.
	pub(crate) fn process_pending_upward_messages() {
		let mut used_weight_so_far = 0;

		let config = <configuration::Module<T>>::config();
		let mut cursor = NeedsDispatchCursor::new::<T>();
		let mut queue_cache = QueueCache::new();

		while let Some(dispatchee) = cursor.peek() {
			if used_weight_so_far >= config.preferred_dispatchable_upward_messages_step_weight {
				// Then check whether we've reached or overshoot the
				// preferred weight for the dispatching stage.
				//
				// if so - bail.
				break;
			}

			// dequeue the next message from the queue of the dispatchee
			let (upward_message, became_empty) = queue_cache.dequeue::<T>(dispatchee);
			if let Some(upward_message) = upward_message {
				used_weight_so_far +=
					T::UmpSink::process_upward_message(dispatchee, upward_message);
			}

			if became_empty {
				// the queue is empty now - this para doesn't need attention anymore.
				cursor.remove();
			} else {
				cursor.advance();
			}
		}

		cursor.flush::<T>();
		queue_cache.flush::<T>();
	}
}

/// To avoid constant fetching, deserializing and serialization the queues are cached.
struct QueueCache(BTreeMap<ParaId, VecDeque<UpwardMessage>>);

impl QueueCache {
	fn new() -> Self {
		Self(BTreeMap::new())
	}

	fn dequeue<T: Trait>(&mut self, para: ParaId) -> (Option<UpwardMessage>, bool) {
		let queue = self
			.0
			.entry(para)
			.or_insert_with(|| <Module<T> as Store>::RelayDispatchQueues::get(&para));
		let upward_message = queue.pop_front();
		let became_empty = queue.is_empty();
		(upward_message, became_empty)
	}

	fn flush<T: Trait>(self) {
		for (para, queue) in self.0 {
			if queue.is_empty() {
				// remove the entry altogether.
				<Module<T> as Store>::RelayDispatchQueues::remove(&para);
			} else {
				<Module<T> as Store>::RelayDispatchQueues::insert(&para, queue);
			}
		}
	}
}

/// A cursor that iterates over all entries in `NeedsDispatch`.
///
/// This cursor will start with the para indicated by `NextDispatchRoundStartWith` storage entry.
/// This cursor is cyclic meaning that after reaching the end it will jump to the beginning. Unlike
/// an iterator, this cursor allows removing items during the iteration.
///
/// This struct is not supposed to be dropped but rather to be consumed by [`flush`].
#[derive(Debug)]
struct NeedsDispatchCursor {
	needs_dispatch: Vec<ParaId>,
	cur_idx: usize,
}

impl NeedsDispatchCursor {
	fn new<T: Trait>() -> Self {
		let needs_dispatch: Vec<ParaId> = <Module<T> as Store>::NeedsDispatch::get();
		let start_with = <Module<T> as Store>::NextDispatchRoundStartWith::get();

		let start_with_idx = match start_with {
			Some(para) => match needs_dispatch.binary_search(&para) {
				Ok(found_idx) => found_idx,
				// well, that's weird, since the `NextDispatchRoundStartWith` is supposed to be reset.
				// let's select 0 as the starting index as a safe bet.
				Err(_supposed_idx) => 0,
			},
			None => 0,
		};

		Self {
			needs_dispatch,
			cur_idx: start_with_idx,
		}
	}

	fn peek(&self) -> Option<ParaId> {
		self.needs_dispatch.get(self.cur_idx).cloned()
	}

	fn advance(&mut self) {
		if !self.needs_dispatch.is_empty() {
			self.cur_idx = (self.cur_idx + 1) % self.needs_dispatch.len();
		}
	}

	fn remove(&mut self) {
		if self.needs_dispatch.is_empty() {
			return;
		}
		let _ = self.needs_dispatch.remove(self.cur_idx);
	}

	fn flush<T: Trait>(self) {
		let next_one = self.peek();
		<Module<T> as Store>::NextDispatchRoundStartWith::set(next_one);
		<Module<T> as Store>::NeedsDispatch::put(self.needs_dispatch);
	}
}

#[cfg(test)]
pub(crate) mod mock_sink {
	//! An implementation of a mock UMP sink that allows attaching a probe for mocking the weights
	//! and checking the sent messages.
	//!
	//! A default behavior of the UMP sink is to ignore an incoming message and return 0 weight.
	//!
	//! A probe can be attached to the mock UMP sink. When attached, the mock sink would consult the
	//! probe to check whether the received message was expected and what weight it should return.
	//!
	//! There are two rules on how to use a probe:
	//!
	//! 1. There can be only one active probe at a time. Creation of another probe while there is
	//!    already an active one leads to a panic. The probe is scoped to a thread where it was created.
	//!
	//! 2. All messages expected by the probe must be received by the time of dropping it. Unreceived
	//!    messages will lead to a panic while dropping a probe.

	use super::{UmpSink, UpwardMessage, ParaId};
	use std::cell::RefCell;
	use std::collections::vec_deque::VecDeque;
	use frame_support::weights::Weight;

	#[derive(Debug)]
	struct UmpExpectation {
		expected_origin: ParaId,
		expected_msg: UpwardMessage,
		mock_weight: Weight,
	}

	std::thread_local! {
		// `Some` here indicates that there is an active probe.
		static HOOK: RefCell<Option<VecDeque<UmpExpectation>>> = RefCell::new(None);
	}

	pub struct MockUmpSink;
	impl UmpSink for MockUmpSink {
		fn process_upward_message(actual_origin: ParaId, actual_msg: Vec<u8>) -> Weight {
			HOOK.with(|opt_hook| match &mut *opt_hook.borrow_mut() {
				Some(hook) => {
					let UmpExpectation {
						expected_origin,
						expected_msg,
						mock_weight,
					} = match hook.pop_front() {
						Some(expectation) => expectation,
						None => {
							panic!(
								"The probe is active but didn't expect the message:\n\n\t{:?}.",
								actual_msg,
							);
						}
					};
					assert_eq!(expected_origin, actual_origin);
					assert_eq!(expected_msg, actual_msg);
					mock_weight
				}
				None => 0,
			})
		}
	}

	pub struct Probe {
		_private: (),
	}

	impl Probe {
		pub fn new() -> Self {
			HOOK.with(|opt_hook| {
				let prev = opt_hook.borrow_mut().replace(VecDeque::default());

				// that can trigger if there were two probes were created during one session which
				// is may be a bit strict, but may save time figuring out what's wrong.
				// if you land here and you do need the two probes in one session consider
				// dropping the the existing probe explicitly.
				assert!(prev.is_none());
			});
			Self { _private: () }
		}

		/// Add an expected message.
		///
		/// The enqueued messages are processed in FIFO order.
		pub fn assert_msg(
			&mut self,
			expected_origin: ParaId,
			expected_msg: UpwardMessage,
			mock_weight: Weight,
		) {
			HOOK.with(|opt_hook| {
				opt_hook
					.borrow_mut()
					.as_mut()
					.unwrap()
					.push_back(UmpExpectation {
						expected_origin,
						expected_msg,
						mock_weight,
					})
			});
		}
	}

	impl Drop for Probe {
		fn drop(&mut self) {
			let _ = HOOK.try_with(|opt_hook| {
				let prev = opt_hook.borrow_mut().take().expect(
					"this probe was created and hasn't been yet destroyed;
					the probe cannot be replaced;
					there is only one probe at a time allowed;
					thus it cannot be `None`;
					qed",
				);

				if !prev.is_empty() {
					// some messages are left unchecked. We should notify the developer about this.
					// however, we do so only if the thread doesn't panic already. Otherwise, the
					// developer would get a SIGILL or SIGABRT without a meaningful error message.
					if !std::thread::panicking() {
						panic!(
							"the probe is dropped and not all expected messages arrived: {:?}",
							prev
						);
					}
				}
			});
			// an `Err` here signals here that the thread local was already destroyed.
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use super::mock_sink::Probe;
	use crate::router::tests::default_genesis_config;
	use crate::mock::{Configuration, Router, new_test_ext};

	struct GenesisConfigBuilder {
		max_upward_message_num_per_candidate: u32,
		max_upward_queue_count: u32,
		max_upward_queue_size: u32,
		preferred_dispatchable_upward_messages_step_weight: Weight,
	}

	impl Default for GenesisConfigBuilder {
		fn default() -> Self {
			Self {
				max_upward_message_num_per_candidate: 2,
				max_upward_queue_count: 4,
				max_upward_queue_size: 64,
				preferred_dispatchable_upward_messages_step_weight: 1000,
			}
		}
	}

	impl GenesisConfigBuilder {
		fn build(self) -> crate::mock::GenesisConfig {
			let mut genesis = default_genesis_config();
			let config = &mut genesis.configuration.config;

			config.max_upward_message_num_per_candidate = self.max_upward_message_num_per_candidate;
			config.max_upward_queue_count = self.max_upward_queue_count;
			config.max_upward_queue_size = self.max_upward_queue_size;
			config.preferred_dispatchable_upward_messages_step_weight =
				self.preferred_dispatchable_upward_messages_step_weight;
			genesis
		}
	}

	fn queue_upward_msg(para: ParaId, msg: UpwardMessage) {
		let msgs = vec![msg];
		assert!(Router::check_upward_messages(
			&Configuration::config(),
			para,
			&msgs,
		));
		let _ = Router::enact_upward_messages(para, msgs);
	}

	#[test]
	fn dispatch_empty() {
		new_test_ext(default_genesis_config()).execute_with(|| {
			// make sure that the case with empty queues is handled properly
			Router::process_pending_upward_messages();
		});
	}

	#[test]
	fn dispatch_single_message() {
		let a = ParaId::from(228);
		let msg = vec![1, 2, 3];

		new_test_ext(GenesisConfigBuilder::default().build()).execute_with(|| {
			let mut probe = Probe::new();

			probe.assert_msg(a, msg.clone(), 0);
			queue_upward_msg(a, msg);

			Router::process_pending_upward_messages();
		});
	}

	#[test]
	fn dispatch_resume_after_exceeding_dispatch_stage_weight() {
		let a = ParaId::from(128);
		let c = ParaId::from(228);
		let q = ParaId::from(911);

		let a_msg_1 = vec![1, 2, 3];
		let a_msg_2 = vec![3, 2, 1];
		let c_msg_1 = vec![3, 2, 1];
		let c_msg_2 = vec![3, 2, 1];
		let q_msg = b"we are Q".to_vec();

		new_test_ext(
			GenesisConfigBuilder {
				preferred_dispatchable_upward_messages_step_weight: 500,
				..Default::default()
			}
			.build(),
		)
		.execute_with(|| {
			queue_upward_msg(q, q_msg.clone());
			queue_upward_msg(c, c_msg_1.clone());
			queue_upward_msg(a, a_msg_1.clone());
			queue_upward_msg(a, a_msg_2.clone());

			// we expect only two first messages to fit in the first iteration.
			{
				let mut probe = Probe::new();

				probe.assert_msg(a, a_msg_1.clone(), 300);
				probe.assert_msg(c, c_msg_1.clone(), 300);
				Router::process_pending_upward_messages();

				drop(probe);
			}

			queue_upward_msg(c, c_msg_2.clone());

			// second iteration should process the second message.
			{
				let mut probe = Probe::new();

				probe.assert_msg(q, q_msg.clone(), 500);
				Router::process_pending_upward_messages();

				drop(probe);
			}

			// 3rd iteration.
			{
				let mut probe = Probe::new();

				probe.assert_msg(a, a_msg_2.clone(), 100);
				probe.assert_msg(c, c_msg_2.clone(), 100);
				Router::process_pending_upward_messages();

				drop(probe);
			}

			// finally, make sure that the queue is empty.
			{
				let probe = Probe::new();
				Router::process_pending_upward_messages();
				drop(probe);
			}
		});
	}
}
