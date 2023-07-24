use ndarray::{ArrayBase, Dim, OwnedRepr};

#[derive(Eq, PartialEq, Debug)]
pub enum PipelineReg<T: num::Num + Copy> {
    VectorReg(ArrayBase<OwnedRepr<T>, Dim<[usize; 1]>>),
    ScalarReg(T),
}

macro_rules! RegisterArithmeticOp {
    ($name: ident, $op: tt) => {
        impl<T> std::ops::$op<PipelineReg<T>> for PipelineReg<T>
        where
            T: num::Num + Copy,
        {
            type Output = PipelineReg<T>;
            fn $name(self, rhs: PipelineReg<T>) -> PipelineReg<T> {
                match (self, rhs) {
                    (PipelineReg::ScalarReg(in1), PipelineReg::ScalarReg(in2)) => {
                        // Scalar * Scalar
                        PipelineReg::ScalarReg(in1.$name(in2))
                    }
                    (PipelineReg::VectorReg(in1), PipelineReg::VectorReg(in2)) => {
                        // Vector * Vector
                        PipelineReg::VectorReg(in1.$name(in2))
                    }
                    (PipelineReg::ScalarReg(in1), PipelineReg::VectorReg(in2)) => {
                        // Scalar * Vector
                        PipelineReg::VectorReg(in2.map(|x| in1.$name(*x)))
                    }
                    /* */
                    (PipelineReg::VectorReg(in1), PipelineReg::ScalarReg(in2)) => {
                        // Vector * Scalar
                        PipelineReg::VectorReg(in1.map(|x| x.$name(in2)))
                    }
                }
            }
        }
    };
}

RegisterArithmeticOp!(add, Add);
RegisterArithmeticOp!(sub, Sub);
RegisterArithmeticOp!(mul, Mul);
RegisterArithmeticOp!(div, Div);

#[cfg(test)]
mod tests {
    use super::PipelineReg;
    use ndarray::array;

    #[test]
    fn reg_test() {
        // testing scalar & scalar
        let d_s = PipelineReg::ScalarReg(1);
        let e_s = PipelineReg::ScalarReg(2);
        let add_de = PipelineReg::ScalarReg(3);
        assert_eq!(d_s + e_s, add_de);

        // testing scalar & vector
        let a_v = PipelineReg::VectorReg(array![1, 2, 3]);
        let b_s = PipelineReg::ScalarReg(2);
        let c_v = PipelineReg::VectorReg(array![2, 4, 6]);
        assert_eq!(b_s * a_v, c_v);

        // testing vector & vector
        let f_v = PipelineReg::VectorReg(array![1, 2, 3]);
        let g_v = PipelineReg::VectorReg(array![1, 2, 3]);
        let add_fg_v = PipelineReg::VectorReg(array![2, 4, 6]);
        assert_eq!(f_v + g_v, add_fg_v);

        // testing vector & scalar
        let h_v = PipelineReg::VectorReg(array![1, 2, 3]);
        let i_s = PipelineReg::ScalarReg(2);
        let mul_hi_v = PipelineReg::VectorReg(array![2, 4, 6]);
        assert_eq!(h_v * i_s, mul_hi_v);
    }
}
