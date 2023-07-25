use crate::{
    channel::{Receiver, Sender},
    context::Context,
    templates::{
        ops::ALUOp,
        pcu::{PCUConfig, PipelineStage, PCU},
    },
    types::{self, DAMType},
};

use super::primitive::Token;

macro_rules! RegisterArithmeticOp {
    ($name: ident, $op: tt, $identity: ident) => {
        // use num::*;
        // use std::ops::*;
        #[allow(non_snake_case, unused_assignments, unused_mut, unused_variables)]
        impl<
                ValType: Copy + std::ops::$op<Output = ValType>,
                StopType: PartialEq + std::fmt::Debug,
            > std::ops::$op<Token<ValType, StopType>> for Token<ValType, StopType>
        where
            Token<ValType, StopType>: Copy,
            ValType: num::Num,
            ValType: types::StaticallySized,
            StopType: types::StaticallySized,
        {
            type Output = Token<ValType, StopType>;
            fn $name(self, rhs: Token<ValType, StopType>) -> Token<ValType, StopType> {
                match (self, rhs) {
                    (Token::Val(in1), Token::Val(in2)) => {
                        // println!("t1: {:?}", in1);
                        // println!("t2: {:?}", in2);
                        // println!("");
                        Token::Val(in1.$name(in2))
                    }
                    (Token::Stop(in1), Token::Stop(in2)) => {
                        assert_eq!(in1, in2, "Stop tokens must be the same");
                        Token::Stop(in1)
                    }
                    (Token::Done, Token::Done) | (Token::Empty, Token::Empty) => self,
                    (Token::Empty, Token::Val(val)) => {
                        Token::Val(num::$identity::<ValType>().$name(val))
                    }
                    (Token::Val(val), Token::Empty) => {
                        Token::Val(val.$name(num::$identity::<ValType>()))
                    }
                    _ => {
                        panic!(
                            "Incorrect {:?} and {:?} tokens found in {:?}",
                            self,
                            rhs,
                            stringify!($name)
                        );
                    }
                }
            }
        }
    };
}

RegisterArithmeticOp!(add, Add, zero);
RegisterArithmeticOp!(sub, Sub, zero);
RegisterArithmeticOp!(mul, Mul, one);
RegisterArithmeticOp!(div, Div, zero);

pub fn make_alu<ValType: DAMType, StopType: DAMType>(
    arg1: Receiver<Token<ValType, StopType>>,
    arg2: Receiver<Token<ValType, StopType>>,
    res: Sender<Token<ValType, StopType>>,
    op: ALUOp<Token<ValType, StopType>>,
) -> impl Context {
    let ingress_op = PCU::<Token<ValType, StopType>>::READ_ALL_INPUTS;
    let egress_op = PCU::<Token<ValType, StopType>>::WRITE_ALL_RESULTS;

    let mut pcu = PCU::new(
        PCUConfig {
            pipeline_depth: 1,
            num_registers: 2,
        },
        ingress_op,
        egress_op,
    );

    pcu.push_stage(PipelineStage {
        op,
        forward: vec![],
        prev_register_ids: vec![0, 1],
        next_register_ids: vec![],
        output_register_ids: vec![0],
    });
    pcu.add_input_channel(arg1);
    pcu.add_input_channel(arg2);
    pcu.add_output_channel(res);

    pcu
}

pub fn make_unary_alu<ValType: DAMType, StopType: DAMType>(
    arg1: Receiver<Token<ValType, StopType>>,
    res: Sender<Token<ValType, StopType>>,
    op: ALUOp<Token<ValType, StopType>>,
) -> impl Context {
    let ingress_op = PCU::<Token<ValType, StopType>>::READ_ALL_INPUTS;
    let egress_op = PCU::<Token<ValType, StopType>>::WRITE_ALL_RESULTS;

    let mut pcu = PCU::new(
        PCUConfig {
            pipeline_depth: 5,
            num_registers: 1,
        },
        ingress_op,
        egress_op,
    );

    pcu.push_stage(PipelineStage {
        op,
        forward: vec![],
        prev_register_ids: vec![0],
        next_register_ids: vec![],
        output_register_ids: vec![0],
    });
    pcu.add_input_channel(arg1);
    pcu.add_output_channel(res);

    pcu
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext, Context,
        },
        simulation::Program,
        templates::{
            ops::ALUAddOp,
            sam::{
                alu::make_unary_alu,
                primitive::{ALUExpOp, Exp, Token},
            },
        },
        token_vec,
    };

    use super::make_alu;

    #[test]
    fn add_test() {
        let a: Token<u32, u32> = Token::Val(1u32);
        let b = Token::Val(2u32);
        let c = Token::Val(3u32);
        assert_eq!(a + b, c);
    }

    #[test]
    fn alu_test() {
        let mut parent = Program::default();
        let (arg1_send, arg1_recv) = parent.unbounded::<Token<u32, u32>>();
        let (arg2_send, arg2_recv) = parent.unbounded::<Token<u32, u32>>();
        let (pcu_out_send, pcu_out_recv) = parent.unbounded::<Token<u32, u32>>();
        // let mut alu = make_alu(arg1_recv, arg2_recv, pcu_out_send, ALUAddOp());
        let alu = make_alu(arg1_recv, arg2_recv, pcu_out_send, ALUAddOp());
        let gen1 = GeneratorContext::new(
            || {
                (0u32..1000)
                    .map(Token::Val)
                    .chain([Token::Empty, Token::Stop(0u32), Token::Done])
            },
            arg1_send,
        );
        let gen2 = GeneratorContext::new(
            || {
                [Token::Empty]
                    .into_iter()
                    .chain((0u32..1000).map(Token::Val))
                    .chain([Token::Stop(0u32), Token::Done])
            },
            arg2_send,
        );
        let checker = CheckerContext::new(
            || {
                [Token::Val(0u32)]
                    .into_iter()
                    .chain((1u32..1000).map(|a| a + (a - 1)).map(Token::Val))
                    .chain([Token::Val(999), Token::Stop(0), Token::Done])
            },
            pcu_out_recv,
        );
        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(alu);
        parent.add_child(checker);
        parent.init();
        parent.run();
        // dbg!(alu.view().tick_lower_bound());
    }

    #[test]
    fn exp_test() {
        let mut parent = Program::default();
        let (arg1_send, arg1_recv) = parent.unbounded::<Token<f32, u32>>();
        let (pcu_out_send, pcu_out_recv) = parent.unbounded::<Token<f32, u32>>();
        let unary_alu = make_unary_alu(arg1_recv, pcu_out_send, ALUExpOp());
        let gen1 = GeneratorContext::new(
            || token_vec!(f32; u32; 0.0, 2.0, 3.0, 4.0, 5.0, 3.0, "S0", "D0").into_iter(),
            arg1_send,
        );
        let checker = CheckerContext::new(
            || {
                token_vec!(f32; u32; 0.0, 2.0, 3.0, 4.0, 5.0, 3.0, "S0", "D0")
                    .into_iter()
                    .map(|a| a.exp())
            },
            pcu_out_recv,
        );
        parent.add_child(gen1);
        parent.add_child(unary_alu);
        parent.add_child(checker);
        parent.init();
        parent.run();
        // dbg!(unary_alu.view().tick_lower_bound());
    }
}
