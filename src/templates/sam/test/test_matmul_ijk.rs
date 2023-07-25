#[cfg(test)]
mod tests {

    use std::{fs, path::Path};

    use crate::channel::{bounded, bounded_with_flavor, void};
    use crate::context::broadcast_context::BroadcastContext;
    use crate::context::generator_context::GeneratorContext;
    use crate::context::parent::BasicParentContext;
    use crate::context::Context;
    use crate::templates::ops::ALUMulOp;
    use crate::templates::sam::accumulator::{Reduce, ReduceData};
    use crate::templates::sam::alu::make_alu;
    use crate::templates::sam::array::{Array, ArrayData};
    use crate::templates::sam::joiner::{CrdJoinerData, Intersect};
    use crate::templates::sam::primitive::{Repsiggen, Token};
    use crate::templates::sam::rd_scanner::{CompressedCrdRdScan, RdScanData};
    use crate::templates::sam::repeat::{RepSigGenData, Repeat, RepeatData, RepeatSigGen};
    use crate::templates::sam::test::config::Data;
    use crate::templates::sam::utils::read_inputs;
    use crate::templates::sam::wr_scanner::{CompressedWrScan, ValsWrScan, WrScanData};
    use crate::token_vec;

    #[test]
    fn test_matmul_ijk() {
        // let test_name = "matmul_ijk";
        let test_name = "mat_elemadd";
        let filename = home::home_dir().unwrap().join("sam_config.toml");
        let contents = fs::read_to_string(filename).unwrap();
        let data: Data = toml::from_str(&contents).unwrap();
        let formatted_dir = data.sam_config.sam_path;
        let base_path = Path::new(&formatted_dir).join(&test_name);
        let b0_seg_filename = base_path.join("tensor_B_mode_0_seg");
        let b0_crd_filename = base_path.join("tensor_B_mode_0_crd");
        let b1_seg_filename = base_path.join("tensor_B_mode_1_seg");
        let b1_crd_filename = base_path.join("tensor_B_mode_1_crd");
        let b_vals_filename = base_path.join("tensor_B_mode_vals");
        let c0_seg_filename = base_path.join("tensor_C_mode_0_seg");
        let c0_crd_filename = base_path.join("tensor_C_mode_0_crd");
        let c1_seg_filename = base_path.join("tensor_C_mode_1_seg");
        let c1_crd_filename = base_path.join("tensor_C_mode_1_crd");
        let c_vals_filename = base_path.join("tensor_C_mode_vals");

        let b0_seg = read_inputs::<u32>(&b0_seg_filename);
        let b0_crd = read_inputs::<u32>(&b0_crd_filename);
        let b1_seg = read_inputs::<u32>(&b1_seg_filename);
        let b1_crd = read_inputs::<u32>(&b1_crd_filename);
        let b_vals = read_inputs::<f32>(&b_vals_filename);
        let c0_seg = read_inputs::<u32>(&c0_seg_filename);
        let c0_crd = read_inputs::<u32>(&c0_crd_filename);
        let c1_seg = read_inputs::<u32>(&c1_seg_filename);
        let c1_crd = read_inputs::<u32>(&c1_crd_filename);
        let c_vals = read_inputs::<f32>(&c_vals_filename);

        let chan_size = 32784;

        let mk_bounded = || {
            bounded_with_flavor::<Token<u32, u32>>(
                chan_size,
                crate::channel::ChannelFlavor::Acyclic,
            )
        };
        let mk_boundedf = || {
            bounded_with_flavor::<Token<f32, u32>>(
                chan_size,
                crate::channel::ChannelFlavor::Acyclic,
            )
        };
        let mk_rsiggen_bounded =
            || bounded_with_flavor::<Repsiggen>(chan_size, crate::channel::ChannelFlavor::Acyclic);

        // fiberlookup_bi
        let (bi_out_ref_sender, bi_out_ref_receiver) = mk_bounded();
        let (bi_out_crd_sender, bi_out_crd_receiver) = mk_bounded();
        let (bi_in_ref_sender, bi_in_ref_receiver) = mk_bounded();
        // let (_bc_bi_in_ref_sender, _bc_bi_in_ref_receiver) = mk_bounded();
        // let (_bc1_bi_in_ref_sender, _bc1_bi_in_ref_receiver) =
        //     mk_bounded();

        let mut b_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            bi_in_ref_sender,
        );
        let bi_data = RdScanData::<u32, u32> {
            // in_ref: bc_bi_in_ref_receiver,
            in_ref: bi_in_ref_receiver,
            out_ref: bi_out_ref_sender,
            out_crd: bi_out_crd_sender,
        };

        let mut bi_rdscanner = CompressedCrdRdScan::new(bi_data, b0_seg, b0_crd);

        // fiberwrite_X0
        let x0_seg: Vec<u32> = Vec::new();
        let x0_crd: Vec<u32> = Vec::new();
        let x0_wrscanner_data = WrScanData::<u32, u32> {
            input: bi_out_crd_receiver,
        };
        let mut x0_wrscanner = CompressedWrScan::new(x0_wrscanner_data, x0_seg, x0_crd);

        // repeatsiggen
        let (bc_bi_out_ref_sender, bc_bi_out_ref_receiver) = mk_bounded();
        let (bc1_bi_out_ref_sender, bc1_bi_out_ref_receiver) = mk_bounded();
        let mut broadcast = BroadcastContext::new(bi_out_ref_receiver);
        broadcast.add_target(bc_bi_out_ref_sender);
        broadcast.add_target(bc1_bi_out_ref_sender);

        let (out_repsig_sender, out_repsig_receiver) = mk_rsiggen_bounded();
        let repsig_data = RepSigGenData::<u32, u32> {
            input: bc_bi_out_ref_receiver,
            out_repsig: out_repsig_sender,
        };
        let mut repsig_i = RepeatSigGen::new(repsig_data);

        // repeat
        let (ci_in_ref_sender, ci_in_ref_receiver) = mk_bounded();
        let mut c_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            ci_in_ref_sender,
        );
        let (out_repeat_sender, out_repeat_receiver) = mk_bounded();
        let ci_repeat_data = RepeatData::<u32, u32> {
            in_ref: ci_in_ref_receiver,
            in_repsig: out_repsig_receiver,
            out_ref: out_repeat_sender,
        };
        let mut ci_repeat = Repeat::new(ci_repeat_data);

        // fiberlookup_cj
        let (cj_out_crd_sender, cj_out_crd_receiver) = mk_bounded();
        let (cj_out_ref_sender, cj_out_ref_receiver) = mk_bounded();
        let cj_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_receiver,
            out_ref: cj_out_ref_sender,
            out_crd: cj_out_crd_sender,
        };
        let mut cj_rdscanner = CompressedCrdRdScan::new(cj_data, c0_seg, c0_crd);

        let (bc_cj_out_ref_sender, bc_cj_out_ref_receiver) = mk_bounded();
        let (bc1_cj_out_ref_sender, bc1_cj_out_ref_receiver) = mk_bounded();
        let mut broadcast1 = BroadcastContext::new(cj_out_ref_receiver);
        broadcast1.add_target(bc_cj_out_ref_sender);
        broadcast1.add_target(bc1_cj_out_ref_sender);

        // fiberlookup_ck
        let (ck_out_crd_sender, ck_out_crd_receiver) = mk_bounded();
        let (ck_out_ref_sender, ck_out_ref_receiver) = mk_bounded();
        let ck_data = RdScanData::<u32, u32> {
            in_ref: bc_cj_out_ref_receiver,
            out_ref: ck_out_ref_sender,
            out_crd: ck_out_crd_sender,
        };
        let mut ck_rdscanner = CompressedCrdRdScan::new(ck_data, c1_seg, c1_crd);

        // repeatsiggen
        let (out_repsig_j_sender, out_repsig_j_receiver) = mk_rsiggen_bounded();
        let repsig_j_data = RepSigGenData::<u32, u32> {
            input: bc1_cj_out_ref_receiver,
            out_repsig: out_repsig_j_sender,
        };
        let mut repsig_j = RepeatSigGen::new(repsig_j_data);

        // repeat
        let (out_repeat_bj_sender, out_repeat_bj_receiver) = mk_bounded();
        let bj_repeat_data = RepeatData::<u32, u32> {
            in_ref: bc1_bi_out_ref_receiver,
            in_repsig: out_repsig_j_receiver,
            out_ref: out_repeat_bj_sender,
        };
        let mut bj_repeat = Repeat::new(bj_repeat_data);

        // fiberlookup_bk
        let (bk_out_crd_sender, bk_out_crd_receiver) = mk_bounded();
        let (bk_out_ref_sender, bk_out_ref_receiver) = mk_bounded();
        let bk_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_bj_receiver,
            out_ref: bk_out_ref_sender,
            out_crd: bk_out_crd_sender,
        };
        let mut bk_rdscanner = CompressedCrdRdScan::new(bk_data, b1_seg, b1_crd);

        // interset_i
        // let (intersecti_out_crd_sender, _intersecti_out_crd_receiver) =
        //     mk_bounded();
        let (intersectk_out_ref1_sender, intersectk_out_ref1_receiver) = mk_bounded();
        let (intersectk_out_ref2_sender, intersectk_out_ref2_receiver) = mk_bounded();
        let intersectk_data = CrdJoinerData::<u32, u32> {
            in_crd1: bk_out_crd_receiver,
            in_ref1: bk_out_ref_receiver,
            in_crd2: ck_out_crd_receiver,
            in_ref2: ck_out_ref_receiver,
            out_crd: void(),
            out_ref1: intersectk_out_ref1_sender,
            out_ref2: intersectk_out_ref2_sender,
        };
        let mut intersect_k = Intersect::new(intersectk_data);

        // fiberwrite_x1
        let x1_seg: Vec<u32> = Vec::new();
        let x1_crd: Vec<u32> = Vec::new();
        let x1_wrscanner_data = WrScanData::<u32, u32> {
            input: cj_out_crd_receiver,
        };
        let mut x1_wrscanner = CompressedWrScan::new(x1_wrscanner_data, x1_seg, x1_crd);

        // arrayvals_b
        let (b_out_val_sender, b_out_val_receiver) = mk_boundedf();
        let arrayvals_b_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref1_receiver,
            out_val: b_out_val_sender,
        };
        let mut arrayvals_b = Array::<u32, f32, u32>::new(arrayvals_b_data, b_vals);

        // arrayvals_c
        let (c_out_val_sender, c_out_val_receiver) = mk_boundedf();
        let arrayvals_c_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref2_receiver,
            out_val: c_out_val_sender,
        };
        let mut arrayvals_c = Array::<u32, f32, u32>::new(arrayvals_c_data, c_vals);

        // mul ALU
        let (mul_out_sender, mul_out_receiver) = mk_boundedf();
        let mut mul = make_alu(
            b_out_val_receiver,
            c_out_val_receiver,
            mul_out_sender,
            ALUMulOp(),
        );

        let (out_val_sender, out_val_receiver) = mk_boundedf();
        let reduce_data = ReduceData::<f32, u32> {
            in_val: mul_out_receiver,
            out_val: out_val_sender,
        };
        let mut red = Reduce::new(reduce_data);

        // fiberwrite_Xvals
        let out_vals: Vec<f32> = Vec::new();
        let xvals_data = WrScanData::<f32, u32> {
            input: out_val_receiver,
        };
        let mut xvals = ValsWrScan::<f32, u32>::new(xvals_data, out_vals);

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut b_gen);
        parent.add_child(&mut broadcast);
        parent.add_child(&mut broadcast1);
        parent.add_child(&mut c_gen);
        parent.add_child(&mut bi_rdscanner);
        parent.add_child(&mut repsig_i);
        parent.add_child(&mut repsig_j);
        parent.add_child(&mut ci_repeat);
        parent.add_child(&mut ck_rdscanner);
        parent.add_child(&mut cj_rdscanner);
        parent.add_child(&mut bj_repeat);
        parent.add_child(&mut bk_rdscanner);
        parent.add_child(&mut intersect_k);
        parent.add_child(&mut x0_wrscanner);
        parent.add_child(&mut x1_wrscanner);
        parent.add_child(&mut arrayvals_b);
        parent.add_child(&mut arrayvals_c);
        parent.add_child(&mut mul);
        parent.add_child(&mut red);
        parent.add_child(&mut xvals);

        parent.init();
        parent.run();
        parent.cleanup();

        // dbg!(x0_wrscanner.crd_arr);
        // dbg!(x1_wrscanner.crd_arr);
        dbg!(xvals.out_val);

        // let fil = formatted_dir.to_str().unwrap();
    }
}
