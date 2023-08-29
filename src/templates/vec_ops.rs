use crate::{
    templates::ops::{ALUOp, PipelineRegister},
    types::DAMType,
    RegisterALUOp,
};
use ndarray::{Array, Dim};

pub trait ElemwiseMax {
    fn max(self, rhs: Self) -> Self;
}
impl<A: PartialOrd + Copy> ElemwiseMax for Array<A, Dim<[usize; 1]>> {
    fn max(self, rhs: Array<A, Dim<[usize; 1]>>) -> Array<A, Dim<[usize; 1]>> {
        Array::from_vec(
            self.iter()
                .zip(&rhs)
                .map(|(i0, i1)| if i0 > i1 { *i0 } else { *i1 })
                .collect(),
        )
    }
}

RegisterALUOp!(ALUVecMaxOp, |(i0, i1), ()| [i0.max(i1)], T: ElemwiseMax);
