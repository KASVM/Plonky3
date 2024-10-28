//! Eventually this will hold a Vectorized Neon implementation of Poseidon2 for MontyField31
//! Currently this is essentially a placeholder to allow compilation on Neon devices.
//!
//! Converting the AVX2/AVX512 code across to Neon is on the TODO list.

use alloc::vec::Vec;
use core::marker::PhantomData;

use p3_field::AbstractField;
use p3_poseidon2::{
    mds_light_permutation, ExternalLayer, ExternalLayerConstants, ExternalLayerConstructor,
    InternalLayer, InternalLayerConstructor, MDSMat4,
};

use crate::{
    FieldParameters, InternalLayerBaseParameters, MontyField31, MontyParameters,
    PackedMontyField31Neon,
};

/// The internal layers of the Poseidon2 permutation for Monty31 fields.
///
/// This is currently not optimized for the Neon architecture but this is on the TODO list.
#[derive(Debug, Clone)]
pub struct Poseidon2InternalLayerMonty31<
    MP: MontyParameters,
    const WIDTH: usize,
    ILP: InternalLayerBaseParameters<MP, WIDTH>,
> {
    pub(crate) internal_constants: Vec<MontyField31<MP>>,
    _phantom: PhantomData<ILP>,
}

/// The external layers of the Poseidon2 permutation for Monty31 fields.
///
/// This is currently not optimized for the Neon architecture but this is on the TODO list.
#[derive(Debug, Clone)]
pub struct Poseidon2ExternalLayerMonty31<MP: MontyParameters, const WIDTH: usize> {
    pub(crate) external_constants: ExternalLayerConstants<MontyField31<MP>, WIDTH>,
}

impl<FP: FieldParameters, const WIDTH: usize, ILP: InternalLayerBaseParameters<FP, WIDTH>>
    InternalLayerConstructor<PackedMontyField31Neon<FP>>
    for Poseidon2InternalLayerMonty31<FP, WIDTH, ILP>
{
    fn new_from_constants(internal_constants: Vec<MontyField31<FP>>) -> Self {
        Self {
            internal_constants,
            _phantom: PhantomData,
        }
    }
}

impl<FP: FieldParameters, const WIDTH: usize>
    ExternalLayerConstructor<PackedMontyField31Neon<FP>, WIDTH>
    for Poseidon2ExternalLayerMonty31<FP, WIDTH>
{
    fn new_from_constants(
        external_constants: ExternalLayerConstants<MontyField31<FP>, WIDTH>,
    ) -> Self {
        Self { external_constants }
    }
}

impl<FP, ILP, const WIDTH: usize, const D: u64> InternalLayer<PackedMontyField31Neon<FP>, WIDTH, D>
    for Poseidon2InternalLayerMonty31<FP, WIDTH, ILP>
where
    FP: FieldParameters,
    ILP: InternalLayerBaseParameters<FP, WIDTH>,
{
    /// Perform the internal layers of the Poseidon2 permutation on the given state.
    fn permute_state(&self, state: &mut [PackedMontyField31Neon<FP>; WIDTH]) {
        self.internal_constants.iter().for_each(|&rc| {
            state[0] += rc;
            state[0] = state[0].exp_const_u64::<D>();
            ILP::generic_internal_linear_layer(state);
        })
    }
}

/// Compute a collection of Poseidon2 external layers.
/// One layer for every constant supplied.
#[inline]
fn external_rounds<FP, const WIDTH: usize, const D: u64>(
    state: &mut [PackedMontyField31Neon<FP>; WIDTH],
    packed_external_constants: &[[MontyField31<FP>; WIDTH]],
) where
    FP: FieldParameters,
{
    /*
        The external layer consists of the following 2 operations:

        s -> s + rc
        s -> s^d
        s -> Ms

        Where by s^d we mean to apply this power function element wise.

        Multiplication by M is implemented efficiently in p3_poseidon2/matrix.
    */
    packed_external_constants.iter().for_each(|round_consts| {
        state
            .iter_mut()
            .zip(round_consts.iter())
            .for_each(|(val, &rc)| {
                *val += rc;
                *val = val.exp_const_u64::<D>();
            });
        mds_light_permutation(state, &MDSMat4);
    });
}

impl<FP, const D: u64, const WIDTH: usize> ExternalLayer<PackedMontyField31Neon<FP>, WIDTH, D>
    for Poseidon2ExternalLayerMonty31<FP, WIDTH>
where
    FP: FieldParameters,
{
    /// Perform the initial external layers of the Poseidon2 permutation on the given state.
    fn permute_state_initial(&self, state: &mut [PackedMontyField31Neon<FP>; WIDTH]) {
        mds_light_permutation(state, &MDSMat4);
        external_rounds::<FP, WIDTH, D>(state, &self.external_constants.get_initial_constants());
    }

    /// Perform the terminal external layers of the Poseidon2 permutation on the given state.
    fn permute_state_terminal(&self, state: &mut [PackedMontyField31Neon<FP>; WIDTH]) {
        external_rounds::<FP, WIDTH, D>(state, &self.external_constants.get_terminal_constants());
    }
}
