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
    use crate::templates::sam::scatter_gather::{Gather, Scatter};
    use crate::templates::sam::test::config::Data;
    use crate::templates::sam::utils::read_inputs;
    use crate::templates::sam::wr_scanner::{CompressedWrScan, ValsWrScan, WrScanData};
    use crate::token_vec;

    #[test]
    fn test_par_matmul_ijk() {
        // let test_name = "matmul_ijk";
        let test_name = "mat_elemadd4";
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

        // let (bc_cj_out_ref_sender, bc_cj_out_ref_receiver) = mk_bounded();
        let (bc1_cj_out_ref_sender, bc1_cj_out_ref_receiver) = mk_bounded();
        let (bc2_cj_out_ref_sender, bc2_cj_out_ref_receiver) = mk_bounded();
        // let (bc3_cj_out_ref_sender, bc3_cj_out_ref_receiver) = mk_bounded();
        let mut broadcast1 = BroadcastContext::new(cj_out_ref_receiver);
        // broadcast1.add_target(bc_cj_out_ref_sender);
        broadcast1.add_target(bc1_cj_out_ref_sender);
        broadcast1.add_target(bc2_cj_out_ref_sender);
        // broadcast1.add_target(bc3_cj_out_ref_sender);

        // let (bk_out_crd_sender1, bk_out_crd_receiver1) = mk_bounded();
        // let (bk_out_crd_sender2, bk_out_crd_receiver2) = mk_bounded();
        // let (ck_out_crd_sender1, ck_out_crd_receiver1) = mk_bounded();
        // let (ck_out_crd_sender2, ck_out_crd_receiver2) = mk_bounded();

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

        // let (bk_out_ref_sender1, bk_out_ref_receiver1) = mk_bounded();
        // let (bk_out_ref_sender2, bk_out_ref_receiver2) = mk_bounded();
        // let (ck_out_ref_sender1, ck_out_ref_receiver1) = mk_bounded();
        // let (ck_out_ref_sender2, ck_out_ref_receiver2) = mk_bounded();

        // let mut scat = Scatter::new(bk_out_crd_receiver);
        // scat.add_target(bj_out_crd_sender1);
        // scat.add_target(bj_out_crd_sender2);

        let (bj_out_ref_sender1, bj_out_ref_receiver1) = mk_bounded();
        let (bj_out_ref_sender2, bj_out_ref_receiver2) = mk_bounded();
        let (bj_out_ref_sender3, bj_out_ref_receiver3) = mk_bounded();
        let (bj_out_ref_sender4, bj_out_ref_receiver4) = mk_bounded();

        let mut scat1 = Scatter::new(out_repeat_bj_receiver);
        scat1.add_target(bj_out_ref_sender1);
        scat1.add_target(bj_out_ref_sender2);
        scat1.add_target(bj_out_ref_sender3);
        scat1.add_target(bj_out_ref_sender4);

        // let mut scat2 = Scatter::new(ck_out_crd_receiver);
        // scat2.add_target(cj_out_crd_sender1);
        // scat2.add_target(cj_out_crd_sender2);

        let (cj_out_ref_sender1, cj_out_ref_receiver1) = mk_bounded();
        let (cj_out_ref_sender2, cj_out_ref_receiver2) = mk_bounded();
        let (cj_out_ref_sender3, cj_out_ref_receiver3) = mk_bounded();
        let (cj_out_ref_sender4, cj_out_ref_receiver4) = mk_bounded();

        let mut scat2 = Scatter::new(bc2_cj_out_ref_receiver);
        scat2.add_target(cj_out_ref_sender1);
        scat2.add_target(cj_out_ref_sender2);
        scat2.add_target(cj_out_ref_sender3);
        scat2.add_target(cj_out_ref_sender4);

        // fiberlookup_bk
        let (bk_out_crd_sender, bk_out_crd_receiver) = mk_bounded();
        let (bk_out_ref_sender, bk_out_ref_receiver) = mk_bounded();
        let bk_data = RdScanData::<u32, u32> {
            in_ref: bj_out_ref_receiver1,
            out_ref: bk_out_ref_sender,
            out_crd: bk_out_crd_sender,
        };
        let mut bk_rdscanner = CompressedCrdRdScan::new(bk_data, b1_seg.clone(), b1_crd.clone());

        // fiberlookup_bk
        let (bk_out_crd_sender1, bk_out_crd_receiver1) = mk_bounded();
        let (bk_out_ref_sender1, bk_out_ref_receiver1) = mk_bounded();
        let bk1_data = RdScanData::<u32, u32> {
            in_ref: bj_out_ref_receiver2,
            out_ref: bk_out_ref_sender1,
            out_crd: bk_out_crd_sender1,
        };
        let mut bk1_rdscanner = CompressedCrdRdScan::new(bk1_data, b1_seg.clone(), b1_crd.clone());

        // fiberlookup_bk
        let (bk_out_crd_sender2, bk_out_crd_receiver2) = mk_bounded();
        let (bk_out_ref_sender2, bk_out_ref_receiver2) = mk_bounded();
        let bk2_data = RdScanData::<u32, u32> {
            in_ref: bj_out_ref_receiver3,
            out_ref: bk_out_ref_sender2,
            out_crd: bk_out_crd_sender2,
        };
        let mut bk2_rdscanner = CompressedCrdRdScan::new(bk2_data, b1_seg.clone(), b1_crd.clone());

        // fiberlookup_bk
        let (bk_out_crd_sender3, bk_out_crd_receiver3) = mk_bounded();
        let (bk_out_ref_sender3, bk_out_ref_receiver3) = mk_bounded();
        let bk3_data = RdScanData::<u32, u32> {
            in_ref: bj_out_ref_receiver4,
            out_ref: bk_out_ref_sender3,
            out_crd: bk_out_crd_sender3,
        };
        let mut bk3_rdscanner = CompressedCrdRdScan::new(bk3_data, b1_seg.clone(), b1_crd.clone());

        // fiberlookup_ck
        let (ck_out_crd_sender, ck_out_crd_receiver) = mk_bounded();
        let (ck_out_ref_sender, ck_out_ref_receiver) = mk_bounded();
        let ck_data = RdScanData::<u32, u32> {
            in_ref: cj_out_ref_receiver1,
            out_ref: ck_out_ref_sender,
            out_crd: ck_out_crd_sender,
        };
        let mut ck_rdscanner = CompressedCrdRdScan::new(ck_data, c1_seg.clone(), c1_crd.clone());

        // fiberlookup_ck
        let (ck_out_crd_sender1, ck_out_crd_receiver1) = mk_bounded();
        let (ck_out_ref_sender1, ck_out_ref_receiver1) = mk_bounded();
        let ck1_data = RdScanData::<u32, u32> {
            in_ref: cj_out_ref_receiver2,
            out_ref: ck_out_ref_sender1,
            out_crd: ck_out_crd_sender1,
        };
        let mut ck1_rdscanner = CompressedCrdRdScan::new(ck1_data, c1_seg.clone(), c1_crd.clone());

        // fiberlookup_bk
        let (ck_out_crd_sender2, ck_out_crd_receiver2) = mk_bounded();
        let (ck_out_ref_sender2, ck_out_ref_receiver2) = mk_bounded();
        let ck2_data = RdScanData::<u32, u32> {
            in_ref: cj_out_ref_receiver3,
            out_ref: ck_out_ref_sender2,
            out_crd: ck_out_crd_sender2,
        };
        let mut ck2_rdscanner = CompressedCrdRdScan::new(ck2_data, c1_seg.clone(), c1_crd.clone());

        // fiberlookup_bk
        let (ck_out_crd_sender3, ck_out_crd_receiver3) = mk_bounded();
        let (ck_out_ref_sender3, ck_out_ref_receiver3) = mk_bounded();
        let ck3_data = RdScanData::<u32, u32> {
            in_ref: cj_out_ref_receiver4,
            out_ref: ck_out_ref_sender3,
            out_crd: ck_out_crd_sender3,
        };
        let mut ck3_rdscanner = CompressedCrdRdScan::new(ck3_data, c1_seg.clone(), c1_crd.clone());

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

        let (intersectk_out_ref1_sender1, intersectk_out_ref1_receiver1) = mk_bounded();
        let (intersectk_out_ref2_sender1, intersectk_out_ref2_receiver1) = mk_bounded();
        let intersectk1_data = CrdJoinerData::<u32, u32> {
            in_crd1: bk_out_crd_receiver1,
            in_ref1: bk_out_ref_receiver1,
            in_crd2: ck_out_crd_receiver1,
            in_ref2: ck_out_ref_receiver1,
            out_crd: void(),
            out_ref1: intersectk_out_ref1_sender1,
            out_ref2: intersectk_out_ref2_sender1,
        };
        let mut intersect_k1 = Intersect::new(intersectk1_data);

        let (intersectk_out_ref1_sender2, intersectk_out_ref1_receiver2) = mk_bounded();
        let (intersectk_out_ref2_sender2, intersectk_out_ref2_receiver2) = mk_bounded();
        let intersectk2_data = CrdJoinerData::<u32, u32> {
            in_crd1: bk_out_crd_receiver2,
            in_ref1: bk_out_ref_receiver2,
            in_crd2: ck_out_crd_receiver2,
            in_ref2: ck_out_ref_receiver2,
            out_crd: void(),
            out_ref1: intersectk_out_ref1_sender2,
            out_ref2: intersectk_out_ref2_sender2,
        };
        let mut intersect_k2 = Intersect::new(intersectk2_data);

        let (intersectk_out_ref1_sender3, intersectk_out_ref1_receiver3) = mk_bounded();
        let (intersectk_out_ref2_sender3, intersectk_out_ref2_receiver3) = mk_bounded();
        let intersectk3_data = CrdJoinerData::<u32, u32> {
            in_crd1: bk_out_crd_receiver3,
            in_ref1: bk_out_ref_receiver3,
            in_crd2: ck_out_crd_receiver3,
            in_ref2: ck_out_ref_receiver3,
            out_crd: void(),
            out_ref1: intersectk_out_ref1_sender3,
            out_ref2: intersectk_out_ref2_sender3,
        };
        let mut intersect_k3 = Intersect::new(intersectk3_data);

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
        let mut arrayvals_b = Array::<u32, f32, u32>::new(arrayvals_b_data, b_vals.clone());

        // arrayvals_b
        let (b_out_val_sender1, b_out_val_receiver1) = mk_boundedf();
        let arrayvals_b1_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref1_receiver1,
            out_val: b_out_val_sender1,
        };
        let mut arrayvals_b1 = Array::<u32, f32, u32>::new(arrayvals_b1_data, b_vals.clone());

        // arrayvals_b
        let (b_out_val_sender2, b_out_val_receiver2) = mk_boundedf();
        let arrayvals_b2_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref1_receiver2,
            out_val: b_out_val_sender2,
        };
        let mut arrayvals_b2 = Array::<u32, f32, u32>::new(arrayvals_b2_data, b_vals.clone());

        // arrayvals_b
        let (b_out_val_sender3, b_out_val_receiver3) = mk_boundedf();
        let arrayvals_b3_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref1_receiver3,
            out_val: b_out_val_sender3,
        };
        let mut arrayvals_b3 = Array::<u32, f32, u32>::new(arrayvals_b3_data, b_vals.clone());

        // arrayvals_c
        let (c_out_val_sender, c_out_val_receiver) = mk_boundedf();
        let arrayvals_c_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref2_receiver,
            out_val: c_out_val_sender,
        };
        let mut arrayvals_c = Array::<u32, f32, u32>::new(arrayvals_c_data, c_vals.clone());

        // arrayvals_c
        let (c_out_val_sender1, c_out_val_receiver1) = mk_boundedf();
        let arrayvals_c1_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref2_receiver1,
            out_val: c_out_val_sender1,
        };
        let mut arrayvals_c1 = Array::<u32, f32, u32>::new(arrayvals_c1_data, c_vals.clone());

        // arrayvals_b
        let (c_out_val_sender2, c_out_val_receiver2) = mk_boundedf();
        let arrayvals_c2_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref2_receiver2,
            out_val: c_out_val_sender2,
        };
        let mut arrayvals_c2 = Array::<u32, f32, u32>::new(arrayvals_c2_data, c_vals.clone());

        // arrayvals_b
        let (c_out_val_sender3, c_out_val_receiver3) = mk_boundedf();
        let arrayvals_c3_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectk_out_ref2_receiver3,
            out_val: c_out_val_sender3,
        };
        let mut arrayvals_c3 = Array::<u32, f32, u32>::new(arrayvals_c3_data, c_vals.clone());

        // mul ALU
        let (mul_out_sender, mul_out_receiver) = mk_boundedf();
        let mut mul = make_alu(
            b_out_val_receiver,
            c_out_val_receiver,
            mul_out_sender,
            ALUMulOp(),
        );

        // mul ALU
        let (mul_out_sender1, mul_out_receiver1) = mk_boundedf();
        let mut mul1 = make_alu(
            b_out_val_receiver1,
            c_out_val_receiver1,
            mul_out_sender1,
            ALUMulOp(),
        );

        // mul ALU
        let (mul_out_sender2, mul_out_receiver2) = mk_boundedf();
        let mut mul2 = make_alu(
            b_out_val_receiver2,
            c_out_val_receiver2,
            mul_out_sender2,
            ALUMulOp(),
        );

        // mul ALU
        let (mul_out_sender3, mul_out_receiver3) = mk_boundedf();
        let mut mul3 = make_alu(
            b_out_val_receiver3,
            c_out_val_receiver3,
            mul_out_sender3,
            ALUMulOp(),
        );

        let (out_val_sender, out_val_receiver) = mk_boundedf();
        let reduce_data = ReduceData::<f32, u32> {
            in_val: mul_out_receiver,
            out_val: out_val_sender,
        };
        let mut red = Reduce::new(reduce_data);

        let (out_val_sender1, out_val_receiver1) = mk_boundedf();
        let reduce1_data = ReduceData::<f32, u32> {
            in_val: mul_out_receiver1,
            out_val: out_val_sender1,
        };
        let mut red1 = Reduce::new(reduce1_data);

        let (out_val_sender2, out_val_receiver2) = mk_boundedf();
        let reduce2_data = ReduceData::<f32, u32> {
            in_val: mul_out_receiver2,
            out_val: out_val_sender2,
        };
        let mut red2 = Reduce::new(reduce2_data);

        let (out_val_sender3, out_val_receiver3) = mk_boundedf();
        let reduce3_data = ReduceData::<f32, u32> {
            in_val: mul_out_receiver3,
            out_val: out_val_sender3,
        };
        let mut red3 = Reduce::new(reduce3_data);

        let (out_final_val_sender, out_final_val_receiver) = mk_boundedf();
        let mut gat = Gather::new(out_final_val_sender);
        gat.add_target(out_val_receiver);
        gat.add_target(out_val_receiver1);
        gat.add_target(out_val_receiver2);
        gat.add_target(out_val_receiver3);

        // fiberwrite_Xvals
        let out_vals: Vec<f32> = Vec::new();
        let xvals_data = WrScanData::<f32, u32> {
            input: out_final_val_receiver,
        };
        let mut xvals = ValsWrScan::<f32, u32>::new(xvals_data, out_vals);

        let mut parent = BasicParentContext::default();
        // parent.add_child(&mut scat);
        parent.add_child(&mut scat1);
        parent.add_child(&mut scat2);
        // parent.add_child(&mut scat3);
        parent.add_child(&mut mul1);
        parent.add_child(&mut red1);
        parent.add_child(&mut red2);
        parent.add_child(&mut red3);
        parent.add_child(&mut gat);
        parent.add_child(&mut b_gen);
        parent.add_child(&mut broadcast);
        parent.add_child(&mut broadcast1);
        parent.add_child(&mut c_gen);
        parent.add_child(&mut bi_rdscanner);
        parent.add_child(&mut repsig_i);
        parent.add_child(&mut repsig_j);
        parent.add_child(&mut ci_repeat);
        parent.add_child(&mut ck_rdscanner);
        parent.add_child(&mut ck1_rdscanner);
        parent.add_child(&mut ck2_rdscanner);
        parent.add_child(&mut ck3_rdscanner);
        parent.add_child(&mut cj_rdscanner);
        parent.add_child(&mut bj_repeat);
        parent.add_child(&mut bk_rdscanner);
        parent.add_child(&mut bk1_rdscanner);
        parent.add_child(&mut bk2_rdscanner);
        parent.add_child(&mut bk3_rdscanner);
        parent.add_child(&mut intersect_k);
        parent.add_child(&mut intersect_k1);
        parent.add_child(&mut intersect_k2);
        parent.add_child(&mut intersect_k3);
        parent.add_child(&mut x0_wrscanner);
        parent.add_child(&mut x1_wrscanner);
        parent.add_child(&mut arrayvals_b);
        parent.add_child(&mut arrayvals_b1);
        parent.add_child(&mut arrayvals_b2);
        parent.add_child(&mut arrayvals_b3);
        parent.add_child(&mut arrayvals_c);
        parent.add_child(&mut arrayvals_c1);
        parent.add_child(&mut arrayvals_c2);
        parent.add_child(&mut arrayvals_c3);
        parent.add_child(&mut mul);
        parent.add_child(&mut mul2);
        parent.add_child(&mut mul3);
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
