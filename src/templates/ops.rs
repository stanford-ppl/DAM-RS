use crate::types::DAMType;

macro_rules! RegisterALUOp {
    ($name: ident, |($($prev_regs:ident),*), ($($next_regs:ident),*)| [$($new_next_regs:expr),*] $(, $($rules:tt)*)?) => {
        #[allow(non_snake_case, unused_assignments, unused_mut, unused_variables)]
        pub fn $name<T: DAMType>() -> ALUOp<T> where  $($($rules)*)* {

            ALUOp::<T> {
                name: stringify!($name),
                func: |in_regs, out_regs| -> Vec<PipelineRegister<T>> {
                    let mut prev_reg_ind: usize = 0;
                    $(
                    let $prev_regs = in_regs[prev_reg_ind].data.clone();
                    prev_reg_ind += 1;
                    )*

                    let mut next_reg_ind: usize = 0;
                    $(
                    let $next_regs = out_regs[next_reg_ind].data;
                    next_reg_ind += 1;
                    )*

                    let mut pipe_regs = Vec::<PipelineRegister<T>>::new();
                    $(
                    pipe_regs.push(PipelineRegister { data: $new_next_regs } );
                    )*
                    pipe_regs
                }
            }
        }
    };
}

#[derive(Debug)]
pub struct ALUOp<T: Clone> {
    // Func is (prev_regs, next_regs) -> new next_regs
    pub func: fn(&[PipelineRegister<T>], &[PipelineRegister<T>]) -> Vec<PipelineRegister<T>>,
    pub name: &'static str,
}

#[derive(Default, Debug, Clone)]
pub struct PipelineRegister<T: Clone> {
    pub data: T,
}

RegisterALUOp!(
    ALUAddOp,
    |(i0, i1), ()| [i0 + i1],
    T: std::ops::Add<T, Output = T>
);

RegisterALUOp!(
    ALUSubOp,
    |(i0, i1), ()| [i0 - i1],
    T: std::ops::Sub<T, Output = T>
);

RegisterALUOp!(
    ALUMulOp,
    |(i0, i1), ()| [i0 * i1],
    T: std::ops::Mul<T, Output = T>
);
