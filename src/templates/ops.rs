//! A common registration mechanism for describing operations, used in the [super::pcu::PCU] and other configurable processing elements.

use crate::types::DAMType;

/// Creates a new [ALUOp] struct.
/// ```
/// use dam::RegisterALUOp;
/// use dam::templates::ops::PipelineRegister;
/// use dam::templates::ops::ALUOp;
/// use dam::context_tools::DAMType;
/// RegisterALUOp!(
/// ALUAddOp,
/// |(i0, i1), ()| [i0 + i1],
/// T: std::ops::Add<T, Output = T>
/// );
///```
#[macro_export]
macro_rules! RegisterALUOp {
    ($name: ident, |($($prev_regs:ident),*), ($($next_regs:ident),*)| [$($new_next_regs:expr),*] $(, $($rules:tt)*)?) => {
        /// Autogenerated definition. See [ALUOp]
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

                    vec![$(PipelineRegister{data: $new_next_regs}),*]
                }
            }
        }
    };
}

/// An ALUOp describes a basic operation on registers
/// It has access to the input registers, output registers, and returns a new set of output registers.
/// The output registers are used to implement reductions and scans.
#[derive(Debug)]
pub struct ALUOp<T> {
    /// Func is (prev_regs, next_regs) -> new next_regs
    #[allow(clippy::type_complexity)]
    pub func: fn(&[PipelineRegister<T>], &[PipelineRegister<T>]) -> Vec<PipelineRegister<T>>,
    /// A name for identifying the operation, such as 'add' or 'mul'
    pub name: &'static str,
}

/// A simple register containing an underlying value.
#[derive(Default, Debug, Clone)]
pub struct PipelineRegister<T> {
    /// The inner value of a register.
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

RegisterALUOp!(
    ALUDivOp,
    |(i0, i1), ()| [i0 / i1],
    T: std::ops::Div<T, Output = T>
);
