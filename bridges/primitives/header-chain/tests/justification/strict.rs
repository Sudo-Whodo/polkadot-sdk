// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for Grandpa strict justification verifier code.

use bp_header_chain::justification::{
	required_justification_precommits, verify_justification, JustificationVerificationContext,
	JustificationVerificationError, PrecommitError,
};
use bp_test_utils::*;

type TestHeader = sp_runtime::testing::Header;

#[test]
fn valid_justification_accepted() {
	let authorities = vec![(ALICE, 1), (BOB, 1), (CHARLIE, 1)];
	let params = JustificationGeneratorParams {
		header: test_header(1),
		round: TEST_GRANDPA_ROUND,
		set_id: TEST_GRANDPA_SET_ID,
		authorities: authorities.clone(),
		ancestors: 7,
		forks: 3,
	};

	let justification = make_justification_for_header::<TestHeader>(params.clone());
	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&verification_context(TEST_GRANDPA_SET_ID),
			&justification,
		),
		Ok(()),
	);

	assert_eq!(justification.commit.precommits.len(), authorities.len());
	assert_eq!(justification.votes_ancestries.len(), params.ancestors as usize);
}

#[test]
fn valid_justification_accepted_with_single_fork() {
	let params = JustificationGeneratorParams {
		header: test_header(1),
		round: TEST_GRANDPA_ROUND,
		set_id: TEST_GRANDPA_SET_ID,
		authorities: vec![(ALICE, 1), (BOB, 1), (CHARLIE, 1)],
		ancestors: 5,
		forks: 1,
	};

	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&verification_context(TEST_GRANDPA_SET_ID),
			&make_justification_for_header::<TestHeader>(params)
		),
		Ok(()),
	);
}

#[test]
fn valid_justification_accepted_with_arbitrary_number_of_authorities() {
	use finality_grandpa::voter_set::VoterSet;
	use sp_consensus_grandpa::AuthorityId;

	let n = 15;
	let required_signatures = required_justification_precommits(n as _);
	let authorities = accounts(n).iter().map(|k| (*k, 1)).collect::<Vec<_>>();

	let params = JustificationGeneratorParams {
		header: test_header(1),
		round: TEST_GRANDPA_ROUND,
		set_id: TEST_GRANDPA_SET_ID,
		authorities: authorities.clone().into_iter().take(required_signatures as _).collect(),
		ancestors: n.into(),
		forks: required_signatures,
	};

	let authorities = authorities
		.iter()
		.map(|(id, w)| (AuthorityId::from(*id), *w))
		.collect::<Vec<(AuthorityId, _)>>();
	let voter_set = VoterSet::new(authorities).unwrap();

	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&JustificationVerificationContext { voter_set, authority_set_id: TEST_GRANDPA_SET_ID },
			&make_justification_for_header::<TestHeader>(params)
		),
		Ok(()),
	);
}

#[test]
fn justification_with_invalid_target_rejected() {
	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(2),
			&verification_context(TEST_GRANDPA_SET_ID),
			&make_default_justification::<TestHeader>(&test_header(1)),
		),
		Err(JustificationVerificationError::InvalidJustificationTarget),
	);
}

#[test]
fn justification_with_invalid_commit_rejected() {
	let mut justification = make_default_justification::<TestHeader>(&test_header(1));
	justification.commit.precommits.clear();

	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&verification_context(TEST_GRANDPA_SET_ID),
			&justification,
		),
		Err(JustificationVerificationError::TooLowCumulativeWeight),
	);
}

#[test]
fn justification_with_invalid_authority_signature_rejected() {
	let mut justification = make_default_justification::<TestHeader>(&test_header(1));
	justification.commit.precommits[0].signature =
		sp_core::crypto::UncheckedFrom::unchecked_from([1u8; 64]);

	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&verification_context(TEST_GRANDPA_SET_ID),
			&justification,
		),
		Err(JustificationVerificationError::Precommit(PrecommitError::InvalidAuthoritySignature)),
	);
}

#[test]
fn justification_with_duplicate_votes_ancestry() {
	let mut justification = make_default_justification::<TestHeader>(&test_header(1));
	justification.votes_ancestries.push(justification.votes_ancestries[0].clone());

	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&verification_context(TEST_GRANDPA_SET_ID),
			&justification,
		),
		Err(JustificationVerificationError::DuplicateVotesAncestries),
	);
}
#[test]
fn justification_with_redundant_votes_ancestry() {
	let mut justification = make_default_justification::<TestHeader>(&test_header(1));
	justification.votes_ancestries.push(test_header(10));

	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&verification_context(TEST_GRANDPA_SET_ID),
			&justification,
		),
		Err(JustificationVerificationError::RedundantVotesAncestries),
	);
}

#[test]
fn justification_is_invalid_if_we_dont_meet_threshold() {
	// Need at least three authorities to sign off or else the voter set threshold can't be reached
	let authorities = vec![(ALICE, 1), (BOB, 1)];

	let params = JustificationGeneratorParams {
		header: test_header(1),
		round: TEST_GRANDPA_ROUND,
		set_id: TEST_GRANDPA_SET_ID,
		authorities: authorities.clone(),
		ancestors: 2 * authorities.len() as u32,
		forks: 2,
	};

	assert_eq!(
		verify_justification::<TestHeader>(
			header_id::<TestHeader>(1),
			&verification_context(TEST_GRANDPA_SET_ID),
			&make_justification_for_header::<TestHeader>(params)
		),
		Err(JustificationVerificationError::TooLowCumulativeWeight),
	);
}
