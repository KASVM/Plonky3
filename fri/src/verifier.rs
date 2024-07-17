use alloc::vec;
use alloc::vec::Vec;

use itertools::{izip, Itertools};
use p3_challenger::{CanObserve, FieldChallenger, GrindingChallenger};
use p3_commit::Mmcs;
use p3_field::{ExtensionField, Field};
use p3_matrix::Dimensions;
use p3_util::{log2_strict_usize, VecExt};

use crate::{CommitPhaseProofStep, FoldableLinearCode, FriConfig, FriProof};

#[derive(Debug)]
pub enum FriError<CommitMmcsErr, InputError> {
    InvalidProofShape,
    CommitPhaseMmcsError(CommitMmcsErr),
    InputError(InputError),
    FinalPolyMismatch,
    InvalidPowWitness,
}

pub fn verify<Code, Val, Challenge, M, Challenger, InputProof, InputError>(
    config: &FriConfig<M>,
    proof: &FriProof<Challenge, M, Challenger::Witness, InputProof>,
    challenger: &mut Challenger,
    open_input: impl Fn(usize, &InputProof) -> Result<Vec<(usize, Challenge)>, InputError>,
) -> Result<(), FriError<M::Error, InputError>>
where
    Val: Field,
    Challenge: ExtensionField<Val>,
    M: Mmcs<Challenge>,
    Challenger: FieldChallenger<Val> + GrindingChallenger + CanObserve<M::Commitment>,
    Code: FoldableLinearCode<Challenge>,
{
    let betas: Vec<Challenge> = proof
        .commit_phase_commits
        .iter()
        .map(|comm| {
            challenger.observe(comm.clone());
            challenger.sample_ext_element()
        })
        .collect();

    for &symbol in &proof.final_poly {
        challenger.observe_ext_element(symbol);
    }

    let log_final_poly_len = proof
        .final_poly
        .log_len()
        .ok_or(FriError::InvalidProofShape)?;

    dbg!(log_final_poly_len);

    let encoded_final_poly = Code::new(log_final_poly_len + config.log_blowup, config.log_blowup)
        .encode(&proof.final_poly);

    // let codeword = g.encode(&proof.final_poly, config.blowup());

    if proof.query_proofs.len() != config.num_queries {
        return Err(FriError::InvalidProofShape);
    }

    // Check PoW.
    if !challenger.check_witness(config.proof_of_work_bits, proof.pow_witness) {
        return Err(FriError::InvalidPowWitness);
    }

    let log_folding_arities = proof
        .query_proofs
        .iter()
        .map(|qp| qp.log_folding_arities())
        .all_equal_value()
        .unwrap();

    dbg!(&log_folding_arities);

    let log_max_height =
        config.log_blowup + log_final_poly_len + log_folding_arities.iter().sum::<usize>();

    dbg!(log_max_height);

    // let log_folding_arities = proof.query_proofs[0].log_folding_arities();
    // assert!(proof.query_proofs.iter().all_equal_value(|qp| &qp.log_folding_arities() == &log_folding_arities))

    // let index_bits = proof.commit_phase_commits.len() + log2_strict_usize(codeword.len());

    for qp in &proof.query_proofs {
        let index = challenger.sample_bits(log_max_height);
        let mut log_height = log_max_height;

        for step in &qp.commit_phase_openings {
            let log_folded_height = log_height - step.log_folding_arity();
            for o in &step.openings {
                let code = Code::new(log_folded_height + o.log_strict_len(), config.log_blowup);
            }
            log_height = log_folded_height;
        }

        /*
        let index = challenger.sample_bits(index_bits + g.extra_query_index_bits());
        let ro = open_input(index, &qp.input_proof).map_err(FriError::InputError)?;

        debug_assert!(
            ro.iter().tuple_windows().all(|((l, _), (r, _))| l > r),
            "reduced openings sorted by height descending"
        );

        let input_index = index >> g.extra_query_index_bits();

        let folded_eval = verify_query(
            g,
            config,
            input_index,
            izip!(
                &betas,
                &proof.commit_phase_commits,
                &qp.commit_phase_openings
            ),
            ro,
        )?;

        let final_index = input_index >> proof.commit_phase_commits.len();

        if codeword[final_index] != folded_eval {
            return Err(FriError::FinalPolyMismatch);
        }
        */
    }

    Ok(())
}

type CommitStep<'a, F, M> = (
    &'a F,
    &'a <M as Mmcs<F>>::Commitment,
    &'a CommitPhaseProofStep<F, M>,
);

fn verify_query<'a, F, M, InputError>(
    config: &FriConfig<M>,
    mut index: usize,
    steps: impl Iterator<Item = CommitStep<'a, F, M>>,
    reduced_openings: Vec<(usize, F)>,
) -> Result<F, FriError<M::Error, InputError>>
where
    F: Field,
    M: Mmcs<F> + 'a,
{
    let mut ro_iter = reduced_openings.into_iter().peekable();
    let (mut log_height, mut folded_eval) = ro_iter.next().unwrap();

    /*
    for (&beta, comm, opening) in steps {
        log_height -= 1;

        let index_sibling = index ^ 1;
        let index_pair = index >> 1;

        let mut evals = vec![folded_eval; 2];
        evals[index_sibling % 2] = opening.sibling_value;

        let dims = &[Dimensions {
            width: 2,
            height: 1 << log_height,
        }];
        config
            .mmcs
            .verify_batch(
                comm,
                dims,
                index_pair,
                &[evals.clone()],
                &opening.opening_proof,
            )
            .map_err(FriError::CommitPhaseMmcsError)?;

        index = index_pair;
        folded_eval = g.fold_row(index, log_height, beta, evals.into_iter());

        if let Some((_, ro)) = ro_iter.next_if(|(lh, _)| *lh == log_height) {
            folded_eval += ro;
        }
    }

    debug_assert!(
        ro_iter.next().is_none(),
        "verifier reduced_openings were not in descending order?",
    );
    */

    Ok(folded_eval)
}
