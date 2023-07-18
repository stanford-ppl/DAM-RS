#[cfg(test)]
mod tests {

    use std::{fs, path::Path};

    use crate::channel::unbounded;
    use crate::context::broadcast_context::BroadcastContext;
    use crate::context::generator_context::GeneratorContext;
    use crate::context::parent::BasicParentContext;
    use crate::context::Context;
    use crate::templates::ops::{ALUDivOp, ALUSubOp};
    use crate::templates::sam::accumulator::{MaxReduce, Reduce, ReduceData};
    use crate::templates::sam::alu::{make_alu, make_unary_alu};
    use crate::templates::sam::array::{Array, ArrayData};
    use crate::templates::sam::primitive::{ALUExpOp, Repsiggen, Token};
    use crate::templates::sam::rd_scanner::{CompressedCrdRdScan, RdScanData};
    use crate::templates::sam::repeat::{RepSigGenData, Repeat, RepeatData, RepeatSigGen};
    use crate::templates::sam::test::config::Data;
    use crate::templates::sam::utils::read_inputs;
    use crate::templates::sam::val_dropper::{ValDrop, ValDropData};
    use crate::templates::sam::wr_scanner::{CompressedWrScan, ValsWrScan, WrScanData};
    use crate::token_vec;

    #[test]
    fn test_softmax() {
        let test_name = "tensor4_softmax_large";
        let filename = home::home_dir().unwrap().join("sam_config.toml");
        let contents = fs::read_to_string(filename).unwrap();
        let data: Data = toml::from_str(&contents).unwrap();
        let formatted_dir = data.sam_config.sam_path;
        let base_path = Path::new(&formatted_dir).join(&test_name);
        let b0_seg_filename = base_path.join("tensor_B_mode_0_seg");
        let b0_crd_filename = base_path.join("tensor_B_mode_0_crd");
        let b1_seg_filename = base_path.join("tensor_B_mode_1_seg");
        let b1_crd_filename = base_path.join("tensor_B_mode_1_crd");
        let b2_seg_filename = base_path.join("tensor_B_mode_2_seg");
        let b2_crd_filename = base_path.join("tensor_B_mode_2_crd");
        let b3_seg_filename = base_path.join("tensor_B_mode_3_seg");
        let b3_crd_filename = base_path.join("tensor_B_mode_3_crd");
        let b_vals_filename = base_path.join("tensor_B_mode_vals");

        let b0_seg = read_inputs::<u32>(&b0_seg_filename);
        let b0_crd = read_inputs::<u32>(&b0_crd_filename);
        let b1_seg = read_inputs::<u32>(&b1_seg_filename);
        let b1_crd = read_inputs::<u32>(&b1_crd_filename);
        let b2_seg = read_inputs::<u32>(&b2_seg_filename);
        let b2_crd = read_inputs::<u32>(&b2_crd_filename);
        let b3_seg = read_inputs::<u32>(&b3_seg_filename);
        let b3_crd = read_inputs::<u32>(&b3_crd_filename);
        let b_vals = read_inputs::<f32>(&b_vals_filename);

        let a_vals_filename = base_path.join("tensor_A_mode_vals");
        let a_vals = read_inputs::<f32>(&a_vals_filename);

        dbg!(a_vals.clone());

        // fiberlookup_bi
        let (bi_out_ref_sender, bi_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let (bi_out_crd_sender, bi_out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (bi_in_ref_sender, bi_in_ref_receiver) = unbounded::<Token<u32, u32>>();
        let bi_data = RdScanData::<u32, u32> {
            in_ref: bi_in_ref_receiver,
            out_ref: bi_out_ref_sender,
            out_crd: bi_out_crd_sender,
        };

        let mut b_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            bi_in_ref_sender,
        );
        let mut bi_rdscanner = CompressedCrdRdScan::new(bi_data, b0_seg, b0_crd);

        // fiberwrite_X0
        let x0_seg: Vec<u32> = Vec::new();
        let x0_crd: Vec<u32> = Vec::new();
        let x0_wrscanner_data = WrScanData::<u32, u32> {
            input: bi_out_crd_receiver,
        };
        let mut x0_wrscanner = CompressedWrScan::new(x0_wrscanner_data, x0_seg, x0_crd);

        // fiberlookup_bj
        let (bj_out_crd_sender, bj_out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (bj_out_ref_sender, bj_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let bj_data = RdScanData::<u32, u32> {
            in_ref: bi_out_ref_receiver,
            out_ref: bj_out_ref_sender,
            out_crd: bj_out_crd_sender,
        };
        let mut bj_rdscanner = CompressedCrdRdScan::new(bj_data, b1_seg, b1_crd);

        // fiberlookup_bk
        let (bk_out_crd_sender, bk_out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (bk_out_ref_sender, bk_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let bk_data = RdScanData::<u32, u32> {
            in_ref: bj_out_ref_receiver,
            out_ref: bk_out_ref_sender,
            out_crd: bk_out_crd_sender,
        };
        let mut bk_rdscanner = CompressedCrdRdScan::new(bk_data, b2_seg, b2_crd);

        // fiberlookup_bl
        let (bl_out_crd_sender, bl_out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (bl_out_ref_sender, bl_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let bl_data = RdScanData::<u32, u32> {
            in_ref: bk_out_ref_receiver,
            out_ref: bl_out_ref_sender,
            out_crd: bl_out_crd_sender,
        };
        let mut bl_rdscanner = CompressedCrdRdScan::new(bl_data, b3_seg, b3_crd);

        // fiberwrite_x1
        let x1_seg: Vec<u32> = Vec::new();
        let x1_crd: Vec<u32> = Vec::new();
        let x1_wrscanner_data = WrScanData::<u32, u32> {
            input: bj_out_crd_receiver,
        };
        let mut x1_wrscanner = CompressedWrScan::new(x1_wrscanner_data, x1_seg, x1_crd);

        // fiberwrite_x2
        let x2_seg: Vec<u32> = Vec::new();
        let x2_crd: Vec<u32> = Vec::new();
        let x2_wrscanner_data = WrScanData::<u32, u32> {
            input: bk_out_crd_receiver,
        };
        let mut x2_wrscanner = CompressedWrScan::new(x2_wrscanner_data, x2_seg, x2_crd);

        let (bc_bl_out_ref_sender, bc_bl_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let (bc1_bl_out_ref_sender, bc1_bl_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let (bc2_bl_out_ref_sender, _bc2_bl_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let mut broadcast3 = BroadcastContext::new(bl_out_ref_receiver);
        broadcast3.add_target(bc_bl_out_ref_sender);
        broadcast3.add_target(bc1_bl_out_ref_sender);
        broadcast3.add_target(bc2_bl_out_ref_sender);

        // arrayvals_b
        let (b_out_val_sender, b_out_val_receiver) = unbounded::<Token<f32, u32>>();
        let arrayvals_b_data = ArrayData::<u32, f32, u32> {
            in_ref: bc_bl_out_ref_receiver,
            out_val: b_out_val_sender,
        };
        let mut arrayvals_b = Array::<u32, f32, u32>::new(arrayvals_b_data, b_vals);

        let (bc_b_out_val_sender, bc_b_out_val_receiver) = unbounded::<Token<f32, u32>>();
        let (bc1_b_out_val_sender, bc1_b_out_val_receiver) = unbounded::<Token<f32, u32>>();
        let mut broadcast = BroadcastContext::new(b_out_val_receiver);
        broadcast.add_target(bc_b_out_val_sender);
        broadcast.add_target(bc1_b_out_val_sender);

        // Max Reduce
        let (max_out_val_sender, max_out_val_receiver) = unbounded::<Token<f32, u32>>();
        let max_data = ReduceData::<f32, u32> {
            in_val: bc_b_out_val_receiver,
            out_val: max_out_val_sender,
        };
        let mut max_red = MaxReduce::new(max_data, f32::MIN);

        let (out_repsig_sender, out_repsig_receiver) = unbounded::<Repsiggen>();
        let repsig_data = RepSigGenData::<u32, u32> {
            input: bc1_bl_out_ref_receiver,
            out_repsig: out_repsig_sender,
        };
        let mut repsig = RepeatSigGen::new(repsig_data);

        let (bc_out_repsig_sender, bc_out_repsig_receiver) = unbounded::<Repsiggen>();
        let (bc1_out_repsig_sender, bc1_out_repsig_receiver) = unbounded::<Repsiggen>();
        let mut broadcast2 = BroadcastContext::new(out_repsig_receiver);
        broadcast2.add_target(bc_out_repsig_sender);
        broadcast2.add_target(bc1_out_repsig_sender);

        let (rep_out_val_sender, rep_out_val_receiver) = unbounded::<Token<f32, u32>>();
        let rep_data = RepeatData::<f32, u32> {
            in_ref: max_out_val_receiver,
            in_repsig: bc_out_repsig_receiver,
            out_ref: rep_out_val_sender,
        };
        let mut rep = Repeat::new(rep_data);

        // Sub ALU, using Add name to correspond to SAM implementation
        let (add_out_sender, add_out_receiver) = unbounded::<Token<f32, u32>>();
        let mut add = make_alu(
            bc1_b_out_val_receiver,
            rep_out_val_receiver,
            add_out_sender,
            ALUSubOp(),
        );

        // Exp
        let (exp_out_sender, exp_out_receiver) = unbounded::<Token<f32, u32>>();
        let mut exp = make_unary_alu(add_out_receiver, exp_out_sender, ALUExpOp());

        let (bc_exp_out_sender, bc_exp_out_receiver) = unbounded::<Token<f32, u32>>();
        let (bc1_exp_out_sender, bc1_exp_out_receiver) = unbounded::<Token<f32, u32>>();
        let mut broadcast4 = BroadcastContext::new(exp_out_receiver);
        broadcast4.add_target(bc_exp_out_sender);
        broadcast4.add_target(bc1_exp_out_sender);

        // Reduce
        let (red_out_sender, red_out_receiver) = unbounded::<Token<f32, u32>>();
        let red_data = ReduceData::<f32, u32> {
            in_val: bc_exp_out_receiver,
            out_val: red_out_sender,
        };
        let mut red = Reduce::new(red_data);

        let (rep1_out_val_sender, rep1_out_val_receiver) = unbounded::<Token<f32, u32>>();
        let rep1_data = RepeatData::<f32, u32> {
            in_ref: red_out_receiver,
            in_repsig: bc1_out_repsig_receiver,
            out_ref: rep1_out_val_sender,
        };
        let mut rep1 = Repeat::new(rep1_data);

        // Div ALU
        let (div_out_sender, div_out_receiver) = unbounded::<Token<f32, u32>>();
        let mut div = make_alu(
            bc1_exp_out_receiver,
            rep1_out_val_receiver,
            div_out_sender,
            ALUDivOp(),
        );

        let (out_drop_val_sender, out_drop_val_receiver) = unbounded::<Token<f32, u32>>();
        let (out_drop_crd_sender, out_drop_crd_receiver) = unbounded::<Token<u32, u32>>();

        let val_drop_data = ValDropData::<u32, f32, u32> {
            in_val: div_out_receiver,
            in_crd: bl_out_crd_receiver,
            out_val: out_drop_val_sender,
            out_crd: out_drop_crd_sender,
        };

        let mut val_drop = ValDrop::new(val_drop_data);

        // fiberwrite_x3
        let x3_seg: Vec<u32> = Vec::new();
        let x3_crd: Vec<u32> = Vec::new();
        let x3_wrscanner_data = WrScanData::<u32, u32> {
            input: out_drop_crd_receiver,
        };
        let mut x3_wrscanner = CompressedWrScan::new(x3_wrscanner_data, x3_seg, x3_crd);

        // fiberwrite_Xvals
        let out_vals: Vec<f32> = Vec::new();
        let xvals_data = WrScanData::<f32, u32> {
            input: out_drop_val_receiver,
        };
        let mut xvals = ValsWrScan::<f32, u32>::new(xvals_data, out_vals);

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut b_gen);
        parent.add_child(&mut bi_rdscanner);
        parent.add_child(&mut bj_rdscanner);
        parent.add_child(&mut bk_rdscanner);
        parent.add_child(&mut bl_rdscanner);
        parent.add_child(&mut x0_wrscanner);
        parent.add_child(&mut x1_wrscanner);
        parent.add_child(&mut x2_wrscanner);
        parent.add_child(&mut x3_wrscanner);
        parent.add_child(&mut arrayvals_b);
        parent.add_child(&mut broadcast);
        parent.add_child(&mut broadcast2);
        parent.add_child(&mut broadcast3);
        parent.add_child(&mut broadcast4);
        parent.add_child(&mut max_red);
        parent.add_child(&mut repsig);
        parent.add_child(&mut rep);
        parent.add_child(&mut rep1);
        parent.add_child(&mut add);
        parent.add_child(&mut div);
        parent.add_child(&mut exp);
        parent.add_child(&mut red);
        parent.add_child(&mut xvals);
        parent.add_child(&mut val_drop);

        parent.init();
        parent.run();
        parent.cleanup();

        // println!("{:?}", x0_wrscanner.crd_arr);
        assert_eq!(xvals.out_val, a_vals, "assert failed");
        // println!("{:?}", xvals.out_val);
        // println!("{:?}", a_vals);

        // let fil = formatted_dir.to_str().unwrap();
    }
}
