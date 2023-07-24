#[cfg(test)]
mod tests {

    use std::{fs, path::Path};

    use dam_core::identifier::Identifiable;
    use dam_core::{ContextView, TimeViewable};

    use crate::channel::{bounded, bounded_with_flavor, unbounded, void};
    use crate::context::broadcast_context::BroadcastContext;
    use crate::context::generator_context::GeneratorContext;
    use crate::context::parent::BasicParentContext;
    use crate::context::print_context::PrintContext;
    use crate::context::Context;
    use crate::templates::ops::{ALUDivOp, ALUMulOp, ALUSubOp};
    use crate::templates::sam::accumulator::{MaxReduce, Reduce, ReduceData, Spacc1, Spacc1Data};
    use crate::templates::sam::alu::{make_alu, make_unary_alu};
    use crate::templates::sam::array::{Array, ArrayData};
    use crate::templates::sam::crd_manager::{CrdDrop, CrdManagerData};
    use crate::templates::sam::joiner::{CrdJoinerData, Intersect};
    use crate::templates::sam::primitive::{ALUExpOp, Repsiggen, Token};
    use crate::templates::sam::rd_scanner::{CompressedCrdRdScan, RdScanData};
    use crate::templates::sam::repeat::{RepSigGenData, Repeat, RepeatData, RepeatSigGen};
    use crate::templates::sam::scatter_gather::{Gather, Scatter};
    use crate::templates::sam::test::config::Data;
    use crate::templates::sam::utils::read_inputs;
    use crate::templates::sam::val_dropper::{ValDrop, ValDropData};
    use crate::templates::sam::wr_scanner::{CompressedWrScan, ValsWrScan, WrScanData};
    use crate::token_vec;

    #[test]
    fn test_par_multihead_attention() {
        // let test_name = "tensor4_mha";
        let test_name = "tensor4_mha";
        let filename = home::home_dir().unwrap().join("sam_config.toml");
        let contents = fs::read_to_string(filename).unwrap();
        let data: Data = toml::from_str(&contents).unwrap();
        let formatted_dir = data.sam_config.sam_path;
        let base_path = Path::new(&formatted_dir).join(&test_name);
        let q0_seg_filename = base_path.join("tensor_Q_mode_0_seg");
        let q0_crd_filename = base_path.join("tensor_Q_mode_0_crd");
        let q1_seg_filename = base_path.join("tensor_Q_mode_1_seg");
        let q1_crd_filename = base_path.join("tensor_Q_mode_1_crd");
        let q2_seg_filename = base_path.join("tensor_Q_mode_2_seg");
        let q2_crd_filename = base_path.join("tensor_Q_mode_2_crd");
        let q3_seg_filename = base_path.join("tensor_Q_mode_3_seg");
        let q3_crd_filename = base_path.join("tensor_Q_mode_3_crd");
        let q_vals_filename = base_path.join("tensor_Q_mode_vals");

        let k0_seg_filename = base_path.join("tensor_K_mode_0_seg");
        let k0_crd_filename = base_path.join("tensor_K_mode_0_crd");
        let k1_seg_filename = base_path.join("tensor_K_mode_1_seg");
        let k1_crd_filename = base_path.join("tensor_K_mode_1_crd");
        let k2_seg_filename = base_path.join("tensor_K_mode_2_seg");
        let k2_crd_filename = base_path.join("tensor_K_mode_2_crd");
        let k3_seg_filename = base_path.join("tensor_K_mode_3_seg");
        let k3_crd_filename = base_path.join("tensor_K_mode_3_crd");
        let k_vals_filename = base_path.join("tensor_K_mode_vals");

        let v0_seg_filename = base_path.join("tensor_V_mode_0_seg");
        let v0_crd_filename = base_path.join("tensor_V_mode_0_crd");
        let v1_seg_filename = base_path.join("tensor_V_mode_1_seg");
        let v1_crd_filename = base_path.join("tensor_V_mode_1_crd");
        let v2_seg_filename = base_path.join("tensor_V_mode_2_seg");
        let v2_crd_filename = base_path.join("tensor_V_mode_2_crd");
        let v3_seg_filename = base_path.join("tensor_V_mode_3_seg");
        let v3_crd_filename = base_path.join("tensor_V_mode_3_crd");
        let v_vals_filename = base_path.join("tensor_V_mode_vals");

        // let a0_seg_filename = base_path.join("tensor_A_mode_0_seg");
        // let a0_crd_filename = base_path.join("tensor_A_mode_0_crd");
        // let a1_seg_filename = base_path.join("tensor_A_mode_1_seg");
        // let a1_crd_filename = base_path.join("tensor_A_mode_1_crd");
        // let a2_seg_filename = base_path.join("tensor_A_mode_2_seg");
        // let a2_crd_filename = base_path.join("tensor_A_mode_2_crd");
        // let a3_seg_filename = base_path.join("tensor_A_mode_3_seg");
        // let a3_crd_filename = base_path.join("tensor_A_mode_3_crd");
        // let a_vals_filename = base_path.join("tensor_A_mode_vals");

        let q0_seg = read_inputs::<u32>(&q0_seg_filename);
        let q0_crd = read_inputs::<u32>(&q0_crd_filename);
        let q1_seg = read_inputs::<u32>(&q1_seg_filename);
        let q1_crd = read_inputs::<u32>(&q1_crd_filename);
        let q2_seg = read_inputs::<u32>(&q2_seg_filename);
        let q2_crd = read_inputs::<u32>(&q2_crd_filename);
        let q3_seg = read_inputs::<u32>(&q3_seg_filename);
        let q3_crd = read_inputs::<u32>(&q3_crd_filename);
        let q_vals = read_inputs::<f32>(&q_vals_filename);

        let k0_seg = read_inputs::<u32>(&k0_seg_filename);
        let k0_crd = read_inputs::<u32>(&k0_crd_filename);
        let k1_seg = read_inputs::<u32>(&k1_seg_filename);
        let k1_crd = read_inputs::<u32>(&k1_crd_filename);
        let k2_seg = read_inputs::<u32>(&k2_seg_filename);
        let k2_crd = read_inputs::<u32>(&k2_crd_filename);
        let k3_seg = read_inputs::<u32>(&k3_seg_filename);
        let k3_crd = read_inputs::<u32>(&k3_crd_filename);
        let k_vals = read_inputs::<f32>(&k_vals_filename);

        let v0_seg = read_inputs::<u32>(&v0_seg_filename);
        let v0_crd = read_inputs::<u32>(&v0_crd_filename);
        let v1_seg = read_inputs::<u32>(&v1_seg_filename);
        let v1_crd = read_inputs::<u32>(&v1_crd_filename);
        let v2_seg = read_inputs::<u32>(&v2_seg_filename);
        let v2_crd = read_inputs::<u32>(&v2_crd_filename);
        let v3_seg = read_inputs::<u32>(&v3_seg_filename);
        let v3_crd = read_inputs::<u32>(&v3_crd_filename);
        let v_vals = read_inputs::<f32>(&v_vals_filename);

        // let a0_seg = read_inputs::<u32>(&a0_seg_filename);
        // let a0_crd = read_inputs::<u32>(&a0_crd_filename);
        // let a1_seg = read_inputs::<u32>(&a1_seg_filename);
        // let a1_crd = read_inputs::<u32>(&a1_crd_filename);
        // let a2_seg = read_inputs::<u32>(&a2_seg_filename);
        // let a2_crd = read_inputs::<u32>(&a2_crd_filename);
        // let a3_seg = read_inputs::<u32>(&a3_seg_filename);
        // let a3_crd = read_inputs::<u32>(&a3_crd_filename);
        // let a_vals = read_inputs::<f32>(&a_vals_filename);

        let chan_size = 65536;

        // let mk_bounded = || {
        //     bounded_with_flavor::<Token<u32, u32>>(
        //         chan_size,
        //         crate::channel::ChannelFlavor::Acyclic,
        //     )
        // };
        // let mk_boundedf = || {
        //     bounded_with_flavor::<Token<f32, u32>>(
        //         chan_size,
        //         crate::channel::ChannelFlavor::Acyclic,
        //     )
        // };
        // let mk_intersect_bounded = || {
        //     bounded_with_flavor::<Token<u32, u32>>(
        //         chan_size,
        //         crate::channel::ChannelFlavor::Acyclic,
        //     )
        // };

        let mk_bounded = || bounded::<Token<u32, u32>>(chan_size);
        let mk_boundedf = || bounded::<Token<f32, u32>>(chan_size);
        let mk_intersect_bounded = || bounded::<Token<u32, u32>>(chan_size);

        // fiberlookup_bi
        let (qi_in_ref_sender, qi_in_ref_receiver) = mk_bounded();
        let (qi_out_ref_sender, qi_out_ref_receiver) = mk_bounded();
        let (qi_out_crd_sender, qi_out_crd_receiver) = mk_bounded();

        let (ki_in_ref_sender, ki_in_ref_receiver) = mk_bounded();
        let (ki_out_ref_sender, ki_out_ref_receiver) = mk_bounded();
        let (ki_out_crd_sender, ki_out_crd_receiver) = mk_bounded();

        let (vi_in_ref_sender, vi_in_ref_receiver) = mk_bounded();
        let (vi_out_ref_sender, vi_out_ref_receiver) = mk_bounded();
        let (vi_out_crd_sender, vi_out_crd_receiver) = mk_bounded();

        let mut q_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            qi_in_ref_sender,
        );
        let mut k_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            ki_in_ref_sender,
        );
        let mut v_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            vi_in_ref_sender,
        );
        let qi_data = RdScanData::<u32, u32> {
            // in_ref: bc_bi_in_ref_receiver,
            in_ref: qi_in_ref_receiver,
            out_ref: qi_out_ref_sender,
            out_crd: qi_out_crd_sender,
        };
        let mut qi_rdscanner = CompressedCrdRdScan::new(qi_data, q0_seg, q0_crd);

        let ki_data = RdScanData::<u32, u32> {
            // in_ref: bc_bi_in_ref_receiver,
            in_ref: ki_in_ref_receiver,
            out_ref: ki_out_ref_sender,
            out_crd: ki_out_crd_sender,
        };
        let mut ki_rdscanner = CompressedCrdRdScan::new(ki_data, k0_seg, k0_crd);

        let vi_data = RdScanData::<u32, u32> {
            in_ref: vi_in_ref_receiver,
            out_ref: vi_out_ref_sender,
            out_crd: vi_out_crd_sender,
        };
        let mut vi_rdscanner = CompressedCrdRdScan::new(vi_data, v0_seg, v0_crd);

        let (intersecti_out_crd_sender, intersecti_out_crd_receiver) = mk_intersect_bounded();
        let (intersecti_out_ref1_sender, intersecti_out_ref1_receiver) = mk_intersect_bounded();
        let (intersecti_out_ref2_sender, intersecti_out_ref2_receiver) = mk_intersect_bounded();
        let intersecti_data = CrdJoinerData::<u32, u32> {
            in_crd1: vi_out_crd_receiver,
            in_ref1: vi_out_ref_receiver,
            in_crd2: qi_out_crd_receiver,
            in_ref2: qi_out_ref_receiver,
            out_crd: intersecti_out_crd_sender,
            out_ref1: intersecti_out_ref1_sender,
            out_ref2: intersecti_out_ref2_sender,
        };
        let mut intersect_i = Intersect::new(intersecti_data);

        let (bc_ki_out_ref_sender, bc_ki_out_ref_receiver) = mk_bounded();
        let (bc1_ki_out_ref_sender, bc1_ki_out_ref_receiver) = mk_bounded();
        let mut broadcast = BroadcastContext::new(ki_out_ref_receiver);
        broadcast.add_target(bc_ki_out_ref_sender);
        broadcast.add_target(bc1_ki_out_ref_sender);

        let (bc_ki_out_crd_sender, bc_ki_out_crd_receiver) = mk_bounded();
        let (bc1_ki_out_crd_sender, bc1_ki_out_crd_receiver) = mk_bounded();
        let mut broadcast1 = BroadcastContext::new(ki_out_crd_receiver);
        broadcast1.add_target(bc_ki_out_crd_sender);
        broadcast1.add_target(bc1_ki_out_crd_sender);

        let (bc_intersecti_out_crd_sender, bc_intersecti_out_crd_receiver) = mk_intersect_bounded();
        let (bc1_intersecti_out_crd_sender, bc1_intersecti_out_crd_receiver) =
            mk_intersect_bounded();
        let mut broadcast2 = BroadcastContext::new(intersecti_out_crd_receiver);
        broadcast2.add_target(bc_intersecti_out_crd_sender);
        broadcast2.add_target(bc1_intersecti_out_crd_sender);

        let (intersecti2_out_crd_sender, intersecti2_out_crd_receiver) = mk_intersect_bounded();
        let (intersecti2_out_ref2_sender, intersecti2_out_ref2_receiver) = mk_intersect_bounded();
        let intersecti2_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc_ki_out_crd_receiver,
            in_ref1: bc_ki_out_ref_receiver,
            in_crd2: bc_intersecti_out_crd_receiver,
            in_ref2: intersecti_out_ref1_receiver,
            out_crd: intersecti2_out_crd_sender,
            out_ref1: void(),
            out_ref2: intersecti2_out_ref2_sender,
        };
        let mut intersect_i2 = Intersect::new(intersecti2_data);

        let (intersecti3_out_ref1_sender, intersecti3_out_ref1_receiver) = mk_intersect_bounded();
        let (intersecti3_out_ref2_sender, intersecti3_out_ref2_receiver) = mk_intersect_bounded();

        let intersecti3_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_ki_out_crd_receiver,
            in_ref1: bc1_ki_out_ref_receiver,
            in_crd2: bc1_intersecti_out_crd_receiver,
            in_ref2: intersecti_out_ref2_receiver,
            out_crd: void(),
            out_ref1: intersecti3_out_ref1_sender,
            out_ref2: intersecti3_out_ref2_sender,
        };
        let mut intersect_i3 = Intersect::new(intersecti3_data);

        let (vj_out_ref_sender, vj_out_ref_receiver) = mk_bounded();
        let (vj_out_crd_sender, vj_out_crd_receiver) = mk_bounded();
        let vj_data = RdScanData::<u32, u32> {
            in_ref: intersecti2_out_ref2_receiver,
            out_ref: vj_out_ref_sender,
            out_crd: vj_out_crd_sender,
        };
        let mut vj_rdscanner = CompressedCrdRdScan::new(vj_data, v2_seg, v2_crd);

        let (qj_out_ref_sender, qj_out_ref_receiver) = mk_bounded();
        let (qj_out_crd_sender, qj_out_crd_receiver) = mk_bounded();
        let qj_data = RdScanData::<u32, u32> {
            in_ref: intersecti3_out_ref2_receiver,
            out_ref: qj_out_ref_sender,
            out_crd: qj_out_crd_sender,
        };
        let mut qj_rdscanner = CompressedCrdRdScan::new(qj_data, q2_seg, q2_crd);

        let (kj_out_ref_sender, kj_out_ref_receiver) = mk_bounded();
        let (kj_out_crd_sender, kj_out_crd_receiver) = mk_bounded();
        let kj_data = RdScanData::<u32, u32> {
            in_ref: intersecti3_out_ref1_receiver,
            out_ref: kj_out_ref_sender,
            out_crd: kj_out_crd_sender,
        };
        let mut kj_rdscanner = CompressedCrdRdScan::new(kj_data, k2_seg, k2_crd);

        let (intersectj_out_crd_sender, intersectj_out_crd_receiver) = mk_intersect_bounded();
        let (intersectj_out_ref2_sender, intersectj_out_ref2_receiver) = mk_intersect_bounded();
        let intersectj_data = CrdJoinerData::<u32, u32> {
            in_crd1: vj_out_crd_receiver,
            in_ref1: vj_out_ref_receiver,
            in_crd2: qj_out_crd_receiver,
            in_ref2: qj_out_ref_receiver,
            out_crd: intersectj_out_crd_sender,
            out_ref1: void(),
            out_ref2: intersectj_out_ref2_sender,
        };
        let mut intersect_j = Intersect::new(intersectj_data);

        let (intersectj3_out_crd_sender, intersectj3_out_crd_receiver) = mk_intersect_bounded();
        let (intersectj3_out_ref1_sender, intersectj3_out_ref1_receiver) = mk_intersect_bounded();
        let (intersectj3_out_ref2_sender, intersectj3_out_ref2_receiver) = mk_intersect_bounded();

        let intersectj3_data = CrdJoinerData::<u32, u32> {
            in_crd1: kj_out_crd_receiver,
            in_ref1: kj_out_ref_receiver,
            in_crd2: intersectj_out_crd_receiver,
            in_ref2: intersectj_out_ref2_receiver,
            out_crd: intersectj3_out_crd_sender,
            out_ref1: intersectj3_out_ref1_sender,
            out_ref2: intersectj3_out_ref2_sender,
        };
        let mut intersect_j3 = Intersect::new(intersectj3_data);
        // dbg!(intersect_j.id());
        // dbg!(intersect_j3.id());

        let (bc_intersectj3_out_ref2_sender, bc_intersectj3_out_ref2_receiver) =
            mk_intersect_bounded();
        let (bc1_intersectj3_out_ref2_sender, bc1_intersectj3_out_ref2_receiver) =
            mk_intersect_bounded();
        let mut broadcast9 = BroadcastContext::new(intersectj3_out_ref2_receiver);
        broadcast9.add_target(bc_intersectj3_out_ref2_sender);
        broadcast9.add_target(bc1_intersectj3_out_ref2_sender);

        let (qk_out_ref_sender, qk_out_ref_receiver) = mk_bounded();
        let (qk_out_crd_sender, qk_out_crd_receiver) = mk_bounded();
        let qk_data = RdScanData::<u32, u32> {
            in_ref: bc_intersectj3_out_ref2_receiver,
            out_ref: qk_out_ref_sender,
            out_crd: qk_out_crd_sender,
        };
        let mut qk_rdscanner = CompressedCrdRdScan::new(qk_data, q1_seg, q1_crd);

        let (bc_qk_out_crd_sender, bc_qk_out_crd_receiver) = mk_bounded();
        let (bc1_qk_out_crd_sender, bc1_qk_out_crd_receiver) = mk_bounded();
        let (bc2_qk_out_crd_sender, bc2_qk_out_crd_receiver) = mk_bounded();
        let mut broadcast7 = BroadcastContext::new(qk_out_crd_receiver);
        broadcast7.add_target(bc_qk_out_crd_sender);
        broadcast7.add_target(bc1_qk_out_crd_sender);
        broadcast7.add_target(bc2_qk_out_crd_sender);

        // let (bc_qk_out_ref_sender, bc_qk_out_ref_receiver) = mk_bounded();
        // let (bc1_qk_out_ref_sender, bc1_qk_out_ref_receiver) = mk_bounded();
        // let (bc2_qk_out_ref_sender, bc2_qk_out_ref_receiver) = mk_bounded();
        // let mut broadcast3 = BroadcastContext::new(qk_out_ref_receiver);
        // broadcast3.add_target(bc_qk_out_ref_sender);
        // broadcast3.add_target(bc1_qk_out_ref_sender);
        // broadcast3.add_target(bc2_qk_out_ref_sender);

        // repeatsiggen
        let (out_repsig_k_sender, out_repsig_k_receiver) = bounded::<Repsiggen>(chan_size);
        let repsig_k_data = RepSigGenData::<u32, u32> {
            input: bc_qk_out_crd_receiver,
            out_repsig: out_repsig_k_sender,
        };
        let mut repsig_k = RepeatSigGen::new(repsig_k_data);

        let (bc_out_repsig_k_sender, bc_out_repsig_k_receiver) = bounded::<Repsiggen>(chan_size);
        let (bc1_out_repsig_k_sender, bc1_out_repsig_k_receiver) = bounded::<Repsiggen>(chan_size);
        let mut broadcast8 = BroadcastContext::new(out_repsig_k_receiver);
        broadcast8.add_target(bc_out_repsig_k_sender);
        broadcast8.add_target(bc1_out_repsig_k_sender);

        // repeat
        let (out_repeat_vk_sender, out_repeat_vk_receiver) = mk_bounded();
        let vk_repeat_data = RepeatData::<u32, u32> {
            in_ref: bc1_intersectj3_out_ref2_receiver,
            in_repsig: bc_out_repsig_k_receiver,
            out_ref: out_repeat_vk_sender,
        };
        let mut vk_repeat = Repeat::new(vk_repeat_data);

        // repeat
        let (out_repeat_kk_sender, out_repeat_kk_receiver) = mk_bounded();
        let kk_repeat_data = RepeatData::<u32, u32> {
            in_ref: intersectj3_out_ref1_receiver,
            in_repsig: bc1_out_repsig_k_receiver,
            out_ref: out_repeat_kk_sender,
        };
        let mut kk_repeat = Repeat::new(kk_repeat_data);

        let (qk_out_ref_sender1, qk_out_ref_receiver1) = mk_bounded();
        let (qk_out_ref_sender2, qk_out_ref_receiver2) = mk_bounded();

        let mut scat1 = Scatter::new(qk_out_ref_receiver);
        scat1.add_target(qk_out_ref_sender1);
        scat1.add_target(qk_out_ref_sender2);

        let (vk_out_ref_sender1, vk_out_ref_receiver1) = mk_bounded();
        let (vk_out_ref_sender2, vk_out_ref_receiver2) = mk_bounded();

        let mut scat2 = Scatter::new(out_repeat_vk_receiver);
        scat2.add_target(vk_out_ref_sender1);
        scat2.add_target(vk_out_ref_sender2);

        let (kk_out_ref_sender1, kk_out_ref_receiver1) = mk_bounded();
        let (kk_out_ref_sender2, kk_out_ref_receiver2) = mk_bounded();

        let mut scat3 = Scatter::new(out_repeat_kk_receiver);
        scat3.add_target(kk_out_ref_sender1);
        scat3.add_target(kk_out_ref_sender2);

        let (qk_out_crd_sender1, qk_out_crd_receiver1) = mk_bounded();
        let (qk_out_crd_sender2, qk_out_crd_receiver2) = mk_bounded();

        let mut scat4 = Scatter::new(bc2_qk_out_crd_receiver);
        scat4.add_target(qk_out_crd_sender1);
        scat4.add_target(qk_out_crd_sender2);

        // let (bc_qk_out_crd_sender, bc_qk_out_crd_receiver) = mk_bounded();
        // let (bc1_qk_out_crd_sender, bc1_qk_out_crd_receiver) = mk_bounded();
        // let (bc2_qk_out_crd_sender, bc2_qk_out_crd_receiver) = mk_bounded();
        // let mut broadcast7 = BroadcastContext::new(qk_out_crd_receiver);
        // broadcast7.add_target(bc_qk_out_crd_sender);
        // broadcast7.add_target(bc1_qk_out_crd_sender);
        // broadcast7.add_target(bc2_qk_out_crd_sender);

        let (kl_out_ref_sender1, kl_out_ref_receiver1) = mk_bounded();
        let (kl_out_crd_sender1, kl_out_crd_receiver1) = mk_bounded();
        let kl_data1 = RdScanData::<u32, u32> {
            in_ref: kk_out_ref_receiver1,
            out_ref: kl_out_ref_sender1,
            out_crd: kl_out_crd_sender1,
        };
        let mut kl_rdscanner1 = CompressedCrdRdScan::new(kl_data1, k1_seg.clone(), k1_crd.clone());

        let (kl_out_ref_sender2, kl_out_ref_receiver2) = mk_bounded();
        let (kl_out_crd_sender2, kl_out_crd_receiver2) = mk_bounded();
        let kl_data2 = RdScanData::<u32, u32> {
            in_ref: kk_out_ref_receiver2,
            out_ref: kl_out_ref_sender2,
            out_crd: kl_out_crd_sender2,
        };
        let mut kl_rdscanner2 = CompressedCrdRdScan::new(kl_data2, k1_seg.clone(), k1_crd.clone());

        // let (bc_kl_out_crd_sender, bc_kl_out_crd_receiver) = mk_bounded();
        // // let (bc1_kl_out_crd_sender, bc1_kl_out_crd_receiver) =
        // //     mk_bounded();
        // // let (bc2_kl_out_crd_sender, bc2_kl_out_crd_receiver) = mk_bounded();
        // let mut broadcast15 = BroadcastContext::new(kl_out_crd_receiver);
        // broadcast15.add_target(bc_kl_out_crd_sender);
        // broadcast15.add_target(bc1_kl_out_crd_sender);
        // broadcast15.add_target(bc2_kl_out_crd_sender);

        let (vl_out_ref_sender1, vl_out_ref_receiver1) = mk_bounded();
        let (vl_out_crd_sender1, vl_out_crd_receiver1) = mk_bounded();
        let vl_data1 = RdScanData::<u32, u32> {
            in_ref: vk_out_ref_receiver1,
            out_ref: vl_out_ref_sender1,
            out_crd: vl_out_crd_sender1,
        };
        let mut vl_rdscanner1 = CompressedCrdRdScan::new(vl_data1, v1_seg.clone(), v1_crd.clone());

        let (vl_out_ref_sender2, vl_out_ref_receiver2) = mk_bounded();
        let (vl_out_crd_sender2, vl_out_crd_receiver2) = mk_bounded();
        let vl_data2 = RdScanData::<u32, u32> {
            in_ref: vk_out_ref_receiver2,
            out_ref: vl_out_ref_sender2,
            out_crd: vl_out_crd_sender2,
        };
        let mut vl_rdscanner2 = CompressedCrdRdScan::new(vl_data2, v1_seg.clone(), v1_crd.clone());

        let (intersectl_out_crd_sender1, intersectl_out_crd_receiver1) = mk_intersect_bounded();
        let (intersectl_out_ref1_sender1, intersectl_out_ref1_receiver1) = mk_intersect_bounded();
        let (intersectl_out_ref2_sender1, intersectl_out_ref2_receiver1) = mk_intersect_bounded();
        let intersectl_data1 = CrdJoinerData::<u32, u32> {
            in_crd1: vl_out_crd_receiver1,
            in_ref1: vl_out_ref_receiver1,
            in_crd2: kl_out_crd_receiver1,
            in_ref2: kl_out_ref_receiver1,
            out_crd: intersectl_out_crd_sender1,
            out_ref1: intersectl_out_ref1_sender1,
            out_ref2: intersectl_out_ref2_sender1,
        };
        let mut intersect_l1 = Intersect::new(intersectl_data1);

        let (intersectl_out_crd_sender2, intersectl_out_crd_receiver2) = mk_intersect_bounded();
        let (intersectl_out_ref1_sender2, intersectl_out_ref1_receiver2) = mk_intersect_bounded();
        let (intersectl_out_ref2_sender2, intersectl_out_ref2_receiver2) = mk_intersect_bounded();
        let intersectl_data2 = CrdJoinerData::<u32, u32> {
            in_crd1: vl_out_crd_receiver2,
            in_ref1: vl_out_ref_receiver2,
            in_crd2: kl_out_crd_receiver2,
            in_ref2: kl_out_ref_receiver2,
            out_crd: intersectl_out_crd_sender2,
            out_ref1: intersectl_out_ref1_sender2,
            out_ref2: intersectl_out_ref2_sender2,
        };
        let mut intersect_l2 = Intersect::new(intersectl_data2);

        let (bc_intersectl_out_crd_sender1, bc_intersectl_out_crd_receiver1) = mk_bounded();
        let (bc1_intersectl_out_crd_sender1, bc1_intersectl_out_crd_receiver1) = mk_bounded();
        let (bc2_intersectl_out_crd_sender1, bc2_intersectl_out_crd_receiver1) = mk_bounded();
        let mut broadcast17 = BroadcastContext::new(intersectl_out_crd_receiver1);
        broadcast17.add_target(bc_intersectl_out_crd_sender1);
        broadcast17.add_target(bc1_intersectl_out_crd_sender1);
        broadcast17.add_target(bc2_intersectl_out_crd_sender1);

        let (vm_out_ref_sender1, vm_out_ref_receiver1) = mk_bounded();
        let (vm_out_crd_sender1, vm_out_crd_receiver1) = mk_bounded();
        let vm_data1 = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref1_receiver1,
            out_ref: vm_out_ref_sender1,
            out_crd: vm_out_crd_sender1,
        };
        let mut vm_rdscanner1 = CompressedCrdRdScan::new(vm_data1, v3_seg.clone(), v3_crd.clone());

        let (vm_out_ref_sender2, vm_out_ref_receiver2) = mk_bounded();
        let (vm_out_crd_sender2, vm_out_crd_receiver2) = mk_bounded();
        let vm_data2 = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref1_receiver2,
            out_ref: vm_out_ref_sender2,
            out_crd: vm_out_crd_sender2,
        };
        let mut vm_rdscanner2 = CompressedCrdRdScan::new(vm_data2, v3_seg.clone(), v3_crd.clone());

        let (km_out_ref_sender1, km_out_ref_receiver1) = mk_bounded();
        let (km_out_crd_sender1, km_out_crd_receiver1) = mk_bounded();
        let km_data1 = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref2_receiver1,
            out_ref: km_out_ref_sender1,
            out_crd: km_out_crd_sender1,
        };
        let mut km_rdscanner1 = CompressedCrdRdScan::new(km_data1, k3_seg.clone(), k3_crd.clone());

        let (km_out_ref_sender2, km_out_ref_receiver2) = mk_bounded();
        let (km_out_crd_sender2, km_out_crd_receiver2) = mk_bounded();
        let km_data2 = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref2_receiver2,
            out_ref: km_out_ref_sender2,
            out_crd: km_out_crd_sender2,
        };
        let mut km_rdscanner2 = CompressedCrdRdScan::new(km_data2, k3_seg.clone(), k3_crd.clone());

        // let (bc_km_out_ref_sender1, bc_km_out_ref_receiver1) = mk_bounded();
        // let (bc1_km_out_ref_sender1, bc1_km_out_ref_receiver1) = mk_bounded();
        // let mut broadcast23 = BroadcastContext::new(km_out_ref_receiver1);
        // broadcast23.add_target(bc_km_out_ref_sender1);
        // broadcast23.add_target(bc1_km_out_ref_sender1);

        let (bc_km_out_ref_sender2, bc_km_out_ref_receiver2) = mk_bounded();
        let (bc1_km_out_ref_sender2, bc1_km_out_ref_receiver2) = mk_bounded();
        let mut broadcast23 = BroadcastContext::new(km_out_ref_receiver2);
        broadcast23.add_target(bc_km_out_ref_sender2);
        broadcast23.add_target(bc1_km_out_ref_sender2);

        // repeatsiggen
        let (out_repsig_l_sender1, out_repsig_l_receiver1) = bounded::<Repsiggen>(chan_size);
        let repsig_l_data1 = RepSigGenData::<u32, u32> {
            input: bc_intersectl_out_crd_receiver1,
            out_repsig: out_repsig_l_sender1,
        };
        let mut repsig_l1 = RepeatSigGen::new(repsig_l_data1);

        // repeatsiggen
        let (out_repsig_l_sender2, out_repsig_l_receiver2) = bounded::<Repsiggen>(chan_size);
        let repsig_l_data2 = RepSigGenData::<u32, u32> {
            input: bc2_intersectl_out_crd_receiver1,
            out_repsig: out_repsig_l_sender2,
        };
        let mut repsig_l2 = RepeatSigGen::new(repsig_l_data2);

        let (bc_out_repsig_l_sender1, bc_out_repsig_l_receiver1) = bounded::<Repsiggen>(chan_size);
        let (bc1_out_repsig_l_sender1, bc1_out_repsig_l_receiver1) =
            bounded::<Repsiggen>(chan_size);
        let (bc2_out_repsig_l_sender1, bc2_out_repsig_l_receiver1) =
            bounded::<Repsiggen>(chan_size);
        let mut broadcast10 = BroadcastContext::new(out_repsig_l_receiver1);
        broadcast10.add_target(bc_out_repsig_l_sender1);
        broadcast10.add_target(bc1_out_repsig_l_sender1);
        broadcast10.add_target(bc2_out_repsig_l_sender1);

        let (bc_out_repsig_l_sender2, bc_out_repsig_l_receiver2) = bounded::<Repsiggen>(chan_size);
        let (bc1_out_repsig_l_sender2, bc1_out_repsig_l_receiver2) =
            bounded::<Repsiggen>(chan_size);
        let (bc2_out_repsig_l_sender2, bc2_out_repsig_l_receiver2) =
            bounded::<Repsiggen>(chan_size);
        let mut broadcast4 = BroadcastContext::new(out_repsig_l_receiver2);
        broadcast4.add_target(bc_out_repsig_l_sender2);
        broadcast4.add_target(bc1_out_repsig_l_sender2);
        broadcast4.add_target(bc2_out_repsig_l_sender2);

        // repeat
        let (out_repeat_ql_sender1, out_repeat_ql_receiver1) = mk_bounded();
        let ql_repeat_data = RepeatData::<u32, u32> {
            in_ref: qk_out_ref_receiver1,
            in_repsig: bc_out_repsig_l_receiver1,
            out_ref: out_repeat_ql_sender1,
        };
        let mut ql_repeat = Repeat::new(ql_repeat_data);

        // repeat
        let (out_repeat_ql_sender2, out_repeat_ql_receiver2) = mk_bounded();
        let ql_repeat_data2 = RepeatData::<u32, u32> {
            in_ref: qk_out_ref_receiver2,
            in_repsig: bc_out_repsig_l_receiver2,
            out_ref: out_repeat_ql_sender2,
        };
        let mut ql_repeat2 = Repeat::new(ql_repeat_data2);

        let (qm_out_ref_sender1, qm_out_ref_receiver1) = mk_bounded();
        let (qm_out_crd_sender1, qm_out_crd_receiver1) = mk_bounded();
        let qm_data1 = RdScanData::<u32, u32> {
            in_ref: out_repeat_ql_receiver1,
            out_ref: qm_out_ref_sender1,
            out_crd: qm_out_crd_sender1,
        };
        let mut qm_rdscanner1 = CompressedCrdRdScan::new(qm_data1, q3_seg.clone(), q3_crd.clone());

        let (qm_out_ref_sender2, qm_out_ref_receiver2) = mk_bounded();
        let (qm_out_crd_sender2, qm_out_crd_receiver2) = mk_bounded();
        let qm_data2 = RdScanData::<u32, u32> {
            in_ref: out_repeat_ql_receiver2,
            out_ref: qm_out_ref_sender2,
            out_crd: qm_out_crd_sender2,
        };
        let mut qm_rdscanner2 = CompressedCrdRdScan::new(qm_data2, q3_seg.clone(), q3_crd.clone());

        let (intersectm_out_crd_sender1, intersectm_out_crd_receiver1) = mk_intersect_bounded();
        let (intersectm_out_ref1_sender1, intersectm_out_ref1_receiver1) = mk_intersect_bounded();
        let (intersectm_out_ref2_sender1, intersectm_out_ref2_receiver1) = mk_intersect_bounded();
        let intersectm_data1 = CrdJoinerData::<u32, u32> {
            in_crd1: vm_out_crd_receiver1,
            in_ref1: vm_out_ref_receiver1,
            in_crd2: qm_out_crd_receiver1,
            in_ref2: qm_out_ref_receiver1,
            out_crd: intersectm_out_crd_sender1,
            out_ref1: intersectm_out_ref1_sender1,
            out_ref2: intersectm_out_ref2_sender1,
        };
        let mut intersect_m_1 = Intersect::new(intersectm_data1);
        // dbg!(intersect_m.id());

        let (intersectm_out_crd_sender2, intersectm_out_crd_receiver2) = mk_intersect_bounded();
        let (intersectm_out_ref1_sender2, intersectm_out_ref1_receiver2) = mk_intersect_bounded();
        let (intersectm_out_ref2_sender2, intersectm_out_ref2_receiver2) = mk_intersect_bounded();
        let intersectm_data2 = CrdJoinerData::<u32, u32> {
            in_crd1: vm_out_crd_receiver2,
            in_ref1: vm_out_ref_receiver2,
            in_crd2: qm_out_crd_receiver2,
            in_ref2: qm_out_ref_receiver2,
            out_crd: intersectm_out_crd_sender2,
            out_ref1: intersectm_out_ref1_sender2,
            out_ref2: intersectm_out_ref2_sender2,
        };
        let mut intersect_m_2 = Intersect::new(intersectm_data2);

        let (bc_km_out_ref_sender1, bc_km_out_ref_receiver1) = mk_bounded();
        let (bc1_km_out_ref_sender1, bc1_km_out_ref_receiver1) = mk_bounded();
        let mut broadcast11 = BroadcastContext::new(km_out_ref_receiver1);
        broadcast11.add_target(bc_km_out_ref_sender1);
        broadcast11.add_target(bc1_km_out_ref_sender1);

        let (bc_km_out_crd_sender1, bc_km_out_crd_receiver1) = mk_bounded();
        let (bc1_km_out_crd_sender1, bc1_km_out_crd_receiver1) = mk_bounded();
        let mut broadcast13 = BroadcastContext::new(km_out_crd_receiver1);
        broadcast13.add_target(bc_km_out_crd_sender1);
        broadcast13.add_target(bc1_km_out_crd_sender1);

        let (bc_intersectm_out_crd_sender1, bc_intersectm_out_crd_receiver1) =
            mk_intersect_bounded();
        let (bc1_intersectm_out_crd_sender1, bc1_intersectm_out_crd_receiver1) =
            mk_intersect_bounded();
        let mut broadcast12 = BroadcastContext::new(intersectm_out_crd_receiver1);
        broadcast12.add_target(bc_intersectm_out_crd_sender1);
        broadcast12.add_target(bc1_intersectm_out_crd_sender1);

        let (bc_intersectm_out_crd_sender2, bc_intersectm_out_crd_receiver2) =
            mk_intersect_bounded();
        let (bc1_intersectm_out_crd_sender2, bc1_intersectm_out_crd_receiver2) =
            mk_intersect_bounded();
        let mut broadcast19 = BroadcastContext::new(intersectm_out_crd_receiver2);
        broadcast19.add_target(bc_intersectm_out_crd_sender2);
        broadcast19.add_target(bc1_intersectm_out_crd_sender2);

        let (bc_km_out_crd_sender2, bc_km_out_crd_receiver2) = mk_intersect_bounded();
        let (bc1_km_out_crd_sender2, bc1_km_out_crd_receiver2) = mk_intersect_bounded();
        let mut broadcast18 = BroadcastContext::new(km_out_crd_receiver2);
        broadcast18.add_target(bc_km_out_crd_sender2);
        broadcast18.add_target(bc1_km_out_crd_sender2);

        let (intersectm2_out_ref2_sender1, intersectm2_out_ref2_receiver1) = mk_bounded();
        let intersectm2_data1 = CrdJoinerData::<u32, u32> {
            in_crd1: bc_km_out_crd_receiver1,
            in_ref1: bc_km_out_ref_receiver1,
            in_crd2: bc1_intersectm_out_crd_receiver1,
            in_ref2: intersectm_out_ref1_receiver1,
            out_crd: void(),
            out_ref1: void(),
            out_ref2: intersectm2_out_ref2_sender1,
        };
        let mut intersect_m2_1 = Intersect::new(intersectm2_data1);

        let (intersectm2_out_ref2_sender2, intersectm2_out_ref2_receiver2) = mk_bounded();
        let intersectm2_data2 = CrdJoinerData::<u32, u32> {
            in_crd1: bc_km_out_crd_receiver2,
            in_ref1: bc_km_out_ref_receiver2,
            in_crd2: bc_intersectm_out_crd_receiver2,
            in_ref2: intersectm_out_ref1_receiver2,
            out_crd: void(),
            out_ref1: void(),
            out_ref2: intersectm2_out_ref2_sender2,
        };
        let mut intersect_m2_2 = Intersect::new(intersectm2_data2);

        // dbg!(intersect_m2.id());

        let (intersectm3_out_crd_sender1, intersectm3_out_crd_receiver1) = mk_intersect_bounded();
        // let (intersectm3_out_ref1_sender, intersectm3_out_ref1_receiver) =
        let (intersectm3_out_ref1_sender1, intersectm3_out_ref1_receiver1) = mk_intersect_bounded();
        let (intersectm3_out_ref2_sender1, intersectm3_out_ref2_receiver1) = mk_intersect_bounded();

        let intersectm3_data1 = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_km_out_crd_receiver1,
            in_ref1: bc1_km_out_ref_receiver1,
            in_crd2: bc_intersectm_out_crd_receiver1,
            in_ref2: intersectm_out_ref2_receiver1,
            out_crd: intersectm3_out_crd_sender1,
            out_ref1: intersectm3_out_ref1_sender1,
            out_ref2: intersectm3_out_ref2_sender1,
        };
        let mut intersect_m3_1 = Intersect::new(intersectm3_data1);

        let (intersectm3_out_crd_sender2, intersectm3_out_crd_receiver2) = mk_intersect_bounded();
        // let (intersectm3_out_ref1_sender, intersectm3_out_ref1_receiver) =
        let (intersectm3_out_ref1_sender2, intersectm3_out_ref1_receiver2) = mk_intersect_bounded();
        let (intersectm3_out_ref2_sender2, intersectm3_out_ref2_receiver2) = mk_intersect_bounded();

        let intersectm3_data2 = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_km_out_crd_receiver2,
            in_ref1: bc1_km_out_ref_receiver2,
            in_crd2: bc1_intersectm_out_crd_receiver2,
            in_ref2: intersectm_out_ref2_receiver2,
            out_crd: intersectm3_out_crd_sender2,
            out_ref1: intersectm3_out_ref1_sender2,
            out_ref2: intersectm3_out_ref2_sender2,
        };
        let mut intersect_m3_2 = Intersect::new(intersectm3_data2);

        let (bc_intersectm3_out_crd_sender2, bc_intersectm3_out_crd_receiver2) =
            mk_intersect_bounded();
        let (bc1_intersectm3_out_crd_sender2, bc1_intersectm3_out_crd_receiver2) =
            mk_intersect_bounded();
        let mut broadcast5 = BroadcastContext::new(intersectm3_out_crd_receiver2);
        broadcast5.add_target(bc_intersectm3_out_crd_sender2);
        broadcast5.add_target(bc1_intersectm3_out_crd_sender2);

        let (bc_intersectm3_out_crd_sender1, bc_intersectm3_out_crd_receiver1) =
            mk_intersect_bounded();
        let (bc1_intersectm3_out_crd_sender1, bc1_intersectm3_out_crd_receiver1) =
            mk_intersect_bounded();
        let mut broadcast16 = BroadcastContext::new(intersectm3_out_crd_receiver1);
        broadcast16.add_target(bc_intersectm3_out_crd_sender1);
        broadcast16.add_target(bc1_intersectm3_out_crd_sender1);

        // arrayvals_q
        let (q_out_val_sender1, q_out_val_receiver1) = mk_boundedf();
        let arrayvals_q_data1 = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref2_receiver1,
            out_val: q_out_val_sender1,
        };
        let mut arrayvals_q1 = Array::<u32, f32, u32>::new(arrayvals_q_data1, q_vals.clone());

        // arrayvals_q
        let (q_out_val_sender2, q_out_val_receiver2) = mk_boundedf();
        let arrayvals_q_data2 = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref2_receiver2,
            out_val: q_out_val_sender2,
        };
        let mut arrayvals_q2 = Array::<u32, f32, u32>::new(arrayvals_q_data2, q_vals.clone());

        // arrayvals_k
        let (k_out_val_sender1, k_out_val_receiver1) = mk_boundedf();
        let arrayvals_k_data1 = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref1_receiver1,
            out_val: k_out_val_sender1,
        };
        let mut arrayvals_k1 = Array::<u32, f32, u32>::new(arrayvals_k_data1, k_vals.clone());

        // arrayvals_k
        let (k_out_val_sender2, k_out_val_receiver2) = mk_boundedf();
        let arrayvals_k_data2 = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref1_receiver2,
            out_val: k_out_val_sender2,
        };
        let mut arrayvals_k2 = Array::<u32, f32, u32>::new(arrayvals_k_data2, k_vals.clone());

        // arrayvals_v
        let (v_out_val_sender1, v_out_val_receiver1) = mk_boundedf();
        let arrayvals_v_data1 = ArrayData::<u32, f32, u32> {
            in_ref: intersectm2_out_ref2_receiver1,
            out_val: v_out_val_sender1,
        };
        let mut arrayvals_v1 = Array::<u32, f32, u32>::new(arrayvals_v_data1, v_vals.clone());

        // arrayvals_v
        let (v_out_val_sender2, v_out_val_receiver2) = mk_boundedf();
        let arrayvals_v_data2 = ArrayData::<u32, f32, u32> {
            in_ref: intersectm2_out_ref2_receiver2,
            out_val: v_out_val_sender2,
        };
        let mut arrayvals_v2 = Array::<u32, f32, u32>::new(arrayvals_v_data2, v_vals.clone());

        // mul ALU
        let (mul_out_sender1, mul_out_receiver1) = mk_boundedf();
        let mut mul1 = make_alu(
            q_out_val_receiver1,
            k_out_val_receiver1,
            mul_out_sender1,
            ALUMulOp(),
        );

        // let (bc_qval_sender, bc_qval_receiver) = mk_boundedf();
        // let (bc1_qval_sender, bc1_qval_receiver) = mk_boundedf();
        // let mut printc = PrintContext::new(q_out_val_receiver2);
        // printc.add_target(bc_qval_sender);
        // printc.add_target(bc1_km_out_ref_sender2);

        // mul ALU
        let (mul_out_sender2, mul_out_receiver2) = mk_boundedf();
        let mut mul2 = make_alu(
            // bc_qval_receiver,
            q_out_val_receiver2,
            k_out_val_receiver2,
            mul_out_sender2,
            ALUMulOp(),
        );

        // Reduce
        let (red_out_sender1, red_out_receiver1) = mk_boundedf();
        let red_data1 = ReduceData::<f32, u32> {
            in_val: mul_out_receiver1,
            out_val: red_out_sender1,
        };
        let mut red1 = Reduce::new(red_data1);

        // Reduce
        let (red_out_sender2, red_out_receiver2) = mk_boundedf();
        let red_data2 = ReduceData::<f32, u32> {
            in_val: mul_out_receiver2,
            out_val: red_out_sender2,
        };
        let mut red2 = Reduce::new(red_data2);

        let (bc_out_red_sender1, bc_out_red_receiver1) = mk_boundedf();
        let (bc1_out_red_sender1, bc1_out_red_receiver1) = mk_boundedf();
        let mut broadcast6 = BroadcastContext::new(red_out_receiver1);
        broadcast6.add_target(bc_out_red_sender1);
        broadcast6.add_target(bc1_out_red_sender1);

        let (bc_out_red_sender2, bc_out_red_receiver2) = mk_boundedf();
        let (bc1_out_red_sender2, bc1_out_red_receiver2) = mk_boundedf();
        let mut broadcast22 = BroadcastContext::new(red_out_receiver2);
        broadcast22.add_target(bc_out_red_sender2);
        broadcast22.add_target(bc1_out_red_sender2);

        // Max Reduce
        let (max_out_val_sender1, max_out_val_receiver1) = mk_boundedf();
        let max_data1 = ReduceData::<f32, u32> {
            in_val: bc_out_red_receiver1,
            out_val: max_out_val_sender1,
        };
        let mut max_red1 = MaxReduce::new(max_data1, f32::MIN);

        // Max Reduce
        let (max_out_val_sender2, max_out_val_receiver2) = mk_boundedf();
        let max_data2 = ReduceData::<f32, u32> {
            in_val: bc_out_red_receiver2,
            out_val: max_out_val_sender2,
        };
        let mut max_red2 = MaxReduce::new(max_data2, f32::MIN);

        let (rep_out_val_sender1, rep_out_val_receiver1) = mk_boundedf();
        let rep_data1 = RepeatData::<f32, u32> {
            in_ref: max_out_val_receiver1,
            in_repsig: bc1_out_repsig_l_receiver1,
            out_ref: rep_out_val_sender1,
        };
        let mut rep1 = Repeat::new(rep_data1);

        let (rep_out_val_sender2, rep_out_val_receiver2) = mk_boundedf();
        let rep_data2 = RepeatData::<f32, u32> {
            in_ref: max_out_val_receiver2,
            in_repsig: bc1_out_repsig_l_receiver2,
            out_ref: rep_out_val_sender2,
        };
        let mut rep2 = Repeat::new(rep_data2);

        // Sub ALU, using Add name to correspond to SAM implementation
        let (add_out_sender1, add_out_receiver1) = mk_boundedf();
        let mut add1 = make_alu(
            bc1_out_red_receiver1,
            rep_out_val_receiver1,
            add_out_sender1,
            ALUSubOp(),
        );

        // Sub ALU, using Add name to correspond to SAM implementation
        let (add_out_sender2, add_out_receiver2) = mk_boundedf();
        let mut add2 = make_alu(
            bc1_out_red_receiver2,
            rep_out_val_receiver2,
            add_out_sender2,
            ALUSubOp(),
        );

        // Exp
        let (exp_out_sender1, exp_out_receiver1) = mk_boundedf();
        let mut exp1 = make_unary_alu(add_out_receiver1, exp_out_sender1, ALUExpOp());

        // Exp
        let (exp_out_sender2, exp_out_receiver2) = mk_boundedf();
        let mut exp2 = make_unary_alu(add_out_receiver2, exp_out_sender2, ALUExpOp());

        let (bc_exp_out_sender1, bc_exp_out_receiver1) = mk_boundedf();
        let (bc1_exp_out_sender1, bc1_exp_out_receiver1) = mk_boundedf();
        let mut broadcast14 = BroadcastContext::new(exp_out_receiver1);
        broadcast14.add_target(bc_exp_out_sender1);
        broadcast14.add_target(bc1_exp_out_sender1);

        let (bc_exp_out_sender2, bc_exp_out_receiver2) = mk_boundedf();
        let (bc1_exp_out_sender2, bc1_exp_out_receiver2) = mk_boundedf();
        let mut broadcast20 = BroadcastContext::new(exp_out_receiver2);
        broadcast20.add_target(bc_exp_out_sender2);
        broadcast20.add_target(bc1_exp_out_sender2);

        // Reduce
        let (red1_out_sender1, red1_out_receiver1) = mk_boundedf();
        let red1_data1 = ReduceData::<f32, u32> {
            in_val: bc_exp_out_receiver1,
            out_val: red1_out_sender1,
        };
        let mut red1_1 = Reduce::new(red1_data1);

        // Reduce
        let (red1_out_sender2, red1_out_receiver2) = mk_boundedf();
        let red1_data2 = ReduceData::<f32, u32> {
            in_val: bc_exp_out_receiver2,
            out_val: red1_out_sender2,
        };
        let mut red1_2 = Reduce::new(red1_data2);

        let (rep1_out_val_sender1, rep1_out_val_receiver1) = mk_boundedf();
        let rep1_data1 = RepeatData::<f32, u32> {
            in_ref: red1_out_receiver1,
            in_repsig: bc2_out_repsig_l_receiver1,
            out_ref: rep1_out_val_sender1,
        };
        let mut rep1_1 = Repeat::new(rep1_data1);

        let (rep1_out_val_sender2, rep1_out_val_receiver2) = mk_boundedf();
        let rep1_data2 = RepeatData::<f32, u32> {
            in_ref: red1_out_receiver2,
            in_repsig: bc2_out_repsig_l_receiver2,
            out_ref: rep1_out_val_sender2,
        };
        let mut rep1_2 = Repeat::new(rep1_data2);

        // Div ALU
        let (div_out_sender1, div_out_receiver1) = mk_boundedf();
        let mut div1 = make_alu(
            bc1_exp_out_receiver1,
            rep1_out_val_receiver1,
            div_out_sender1,
            ALUDivOp(),
        );

        // Div ALU
        let (div_out_sender2, div_out_receiver2) = mk_boundedf();
        let mut div2 = make_alu(
            bc1_exp_out_receiver2,
            rep1_out_val_receiver2,
            div_out_sender2,
            ALUDivOp(),
        );

        // let (out_drop_val_sender, out_drop_val_receiver) = mk_boundedf();
        // let (out_drop_crd_sender, out_drop_crd_receiver) = unbounded::<Token<u32, u32>>();

        // let val_drop_data = ValDropData::<u32, f32, u32> {
        //     in_val: div_out_receiver,
        //     in_crd: bc1_kl_out_crd_receiver,
        //     out_val: out_drop_val_sender,
        //     out_crd: out_drop_crd_sender,
        // };

        // let mut val_drop = ValDrop::new(val_drop_data);

        let (out_repsig_m_sender1, out_repsig_m_receiver1) = bounded::<Repsiggen>(chan_size);
        let repsig_m_data1 = RepSigGenData::<u32, u32> {
            input: bc_intersectm3_out_crd_receiver1,
            out_repsig: out_repsig_m_sender1,
        };
        let mut repsigm1 = RepeatSigGen::new(repsig_m_data1);

        let (out_repsig_m_sender2, out_repsig_m_receiver2) = bounded::<Repsiggen>(chan_size);
        let repsig_m_data2 = RepSigGenData::<u32, u32> {
            input: bc_intersectm3_out_crd_receiver2,
            out_repsig: out_repsig_m_sender2,
        };
        let mut repsigm2 = RepeatSigGen::new(repsig_m_data2);

        let (rep_m_out_val_sender1, rep_m_out_val_receiver1) = mk_boundedf();
        let rep2_data1 = RepeatData::<f32, u32> {
            // in_ref: out_drop_val_receiver,
            in_ref: div_out_receiver1,
            in_repsig: out_repsig_m_receiver1,
            out_ref: rep_m_out_val_sender1,
        };
        let mut rep_m1 = Repeat::new(rep2_data1);

        let (rep_m_out_val_sender2, rep_m_out_val_receiver2) = mk_boundedf();
        let rep2_data2 = RepeatData::<f32, u32> {
            // in_ref: out_drop_val_receiver,
            in_ref: div_out_receiver2,
            in_repsig: out_repsig_m_receiver2,
            out_ref: rep_m_out_val_sender2,
        };
        let mut rep_m2 = Repeat::new(rep2_data2);

        // mul ALU
        let (mul2_out_sender1, mul2_out_receiver1) = mk_boundedf();
        let mut mul2_1 = make_alu(
            rep_m_out_val_receiver1,
            v_out_val_receiver1,
            mul2_out_sender1,
            ALUMulOp(),
        );

        // mul ALU
        let (mul2_out_sender2, mul2_out_receiver2) = mk_boundedf();
        let mut mul2_2 = make_alu(
            rep_m_out_val_receiver2,
            v_out_val_receiver2,
            mul2_out_sender2,
            ALUMulOp(),
        );

        let (drop_out_icrd_sender1, drop_out_icrd_receiver1) = mk_bounded();
        let crd_drop_data1 = CrdManagerData::<u32, u32> {
            in_crd_outer: qk_out_crd_receiver1,
            in_crd_inner: bc1_intersectl_out_crd_receiver1,
            out_crd_outer: void(),
            out_crd_inner: drop_out_icrd_sender1,
        };
        let mut drop1 = CrdDrop::new(crd_drop_data1);

        let (drop_out_icrd_sender2, drop_out_icrd_receiver2) = mk_bounded();
        let crd_drop_data2 = CrdManagerData::<u32, u32> {
            in_crd_outer: qk_out_crd_receiver2,
            in_crd_inner: intersectl_out_crd_receiver2,
            out_crd_outer: void(),
            out_crd_inner: drop_out_icrd_sender2,
        };
        let mut drop2 = CrdDrop::new(crd_drop_data2);

        let (out_spacc_val_sender1, out_spacc_val_receiver1) = mk_boundedf();
        let (out_spacc_icrd_sender1, out_spacc_icrd_receiver1) = mk_bounded();
        let spacc_data1 = Spacc1Data::<u32, f32, u32> {
            in_crd_outer: drop_out_icrd_receiver1,
            in_crd_inner: bc1_intersectm3_out_crd_receiver1,
            in_val: mul2_out_receiver1,
            out_val: out_spacc_val_sender1,
            out_crd_inner: out_spacc_icrd_sender1,
        };
        let mut spacc1 = Spacc1::new(spacc_data1);

        let (out_spacc_val_sender2, out_spacc_val_receiver2) = mk_boundedf();
        let (out_spacc_icrd_sender2, out_spacc_icrd_receiver2) = mk_bounded();
        let spacc_data2 = Spacc1Data::<u32, f32, u32> {
            in_crd_outer: drop_out_icrd_receiver2,
            in_crd_inner: bc1_intersectm3_out_crd_receiver2,
            in_val: mul2_out_receiver2,
            out_val: out_spacc_val_sender2,
            out_crd_inner: out_spacc_icrd_sender2,
        };
        let mut spacc2 = Spacc1::new(spacc_data2);

        let (out_final_val_sender, out_final_val_receiver) = mk_boundedf();
        let mut gat = Gather::new(out_final_val_sender);
        gat.add_target(out_spacc_val_receiver1);
        gat.add_target(out_spacc_val_receiver2);

        let (out_final_icrd_sender, out_final_icrd_receiver) = mk_bounded();
        let mut gat1 = Gather::new(out_final_icrd_sender);
        gat1.add_target(out_spacc_icrd_receiver1);
        gat1.add_target(out_spacc_icrd_receiver2);

        // fiberwrite_X0
        let x0_seg: Vec<u32> = Vec::new();
        let x0_crd: Vec<u32> = Vec::new();
        let x0_wrscanner_data = WrScanData::<u32, u32> {
            input: intersecti2_out_crd_receiver,
        };
        let mut x0_wrscanner = CompressedWrScan::new(x0_wrscanner_data, x0_seg, x0_crd);

        // fiberwrite_X1
        let x1_seg: Vec<u32> = Vec::new();
        let x1_crd: Vec<u32> = Vec::new();
        let x1_wrscanner_data = WrScanData::<u32, u32> {
            input: bc1_qk_out_crd_receiver,
        };
        let mut x1_wrscanner = CompressedWrScan::new(x1_wrscanner_data, x1_seg, x1_crd);

        // fiberwrite_X2
        let x2_seg: Vec<u32> = Vec::new();
        let x2_crd: Vec<u32> = Vec::new();
        let x2_wrscanner_data = WrScanData::<u32, u32> {
            input: intersectj3_out_crd_receiver,
        };
        let mut x2_wrscanner = CompressedWrScan::new(x2_wrscanner_data, x2_seg, x2_crd);

        // fiberwrite_X3
        let x3_seg: Vec<u32> = Vec::new();
        let x3_crd: Vec<u32> = Vec::new();
        let x3_wrscanner_data = WrScanData::<u32, u32> {
            input: out_final_icrd_receiver,
        };
        let mut x3_wrscanner = CompressedWrScan::new(x3_wrscanner_data, x3_seg, x3_crd);

        // fiberwrite_Xvals
        let out_vals: Vec<f32> = Vec::new();
        let xvals_data = WrScanData::<f32, u32> {
            input: out_final_val_receiver,
        };
        let mut xvals = ValsWrScan::<f32, u32>::new(xvals_data, out_vals);

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut q_gen);
        parent.add_child(&mut k_gen);
        parent.add_child(&mut v_gen);
        parent.add_child(&mut qi_rdscanner);
        parent.add_child(&mut ki_rdscanner);
        parent.add_child(&mut vi_rdscanner);
        parent.add_child(&mut broadcast);
        parent.add_child(&mut broadcast1);
        parent.add_child(&mut broadcast2);
        // parent.add_child(&mut broadcast3);
        parent.add_child(&mut broadcast4);
        parent.add_child(&mut broadcast5);
        parent.add_child(&mut broadcast6);
        parent.add_child(&mut broadcast7);
        parent.add_child(&mut broadcast8);
        parent.add_child(&mut broadcast9);
        parent.add_child(&mut broadcast10);
        parent.add_child(&mut broadcast11);
        parent.add_child(&mut broadcast12);
        parent.add_child(&mut broadcast13);
        parent.add_child(&mut broadcast14);
        parent.add_child(&mut broadcast16);
        parent.add_child(&mut broadcast17);
        parent.add_child(&mut broadcast18);
        parent.add_child(&mut broadcast19);
        parent.add_child(&mut broadcast20);
        parent.add_child(&mut broadcast22);
        parent.add_child(&mut broadcast23);
        parent.add_child(&mut drop2);
        parent.add_child(&mut intersect_i);
        parent.add_child(&mut intersect_i2);
        parent.add_child(&mut intersect_i3);
        parent.add_child(&mut intersect_j);
        // parent.add_child(&mut intersect_j2);
        parent.add_child(&mut intersect_j3);
        parent.add_child(&mut vj_rdscanner);
        parent.add_child(&mut kj_rdscanner);
        parent.add_child(&mut qj_rdscanner);
        parent.add_child(&mut qk_rdscanner);
        parent.add_child(&mut repsig_k);
        parent.add_child(&mut vk_repeat);
        parent.add_child(&mut kk_repeat);
        parent.add_child(&mut kl_rdscanner1);
        parent.add_child(&mut kl_rdscanner2);
        parent.add_child(&mut vl_rdscanner1);
        parent.add_child(&mut vl_rdscanner2);
        parent.add_child(&mut intersect_l1);
        parent.add_child(&mut intersect_l2);
        parent.add_child(&mut km_rdscanner1);
        parent.add_child(&mut km_rdscanner2);
        parent.add_child(&mut vm_rdscanner1);
        parent.add_child(&mut vm_rdscanner2);
        parent.add_child(&mut repsig_l1);
        parent.add_child(&mut repsig_l2);
        parent.add_child(&mut ql_repeat);
        parent.add_child(&mut qm_rdscanner1);
        parent.add_child(&mut qm_rdscanner2);
        parent.add_child(&mut intersect_m_1);
        parent.add_child(&mut intersect_m_2);
        parent.add_child(&mut intersect_m2_1);
        parent.add_child(&mut intersect_m2_2);
        parent.add_child(&mut intersect_m3_1);
        parent.add_child(&mut intersect_m3_2);
        parent.add_child(&mut arrayvals_q1);
        parent.add_child(&mut arrayvals_q2);
        parent.add_child(&mut arrayvals_k1);
        parent.add_child(&mut arrayvals_k2);
        parent.add_child(&mut arrayvals_v1);
        parent.add_child(&mut arrayvals_v2);
        parent.add_child(&mut mul1);
        parent.add_child(&mut mul2);
        parent.add_child(&mut mul2_1);
        parent.add_child(&mut mul2_2);
        parent.add_child(&mut xvals);
        parent.add_child(&mut red1);
        parent.add_child(&mut red2);
        parent.add_child(&mut red1_1);
        parent.add_child(&mut red1_2);
        parent.add_child(&mut max_red1);
        parent.add_child(&mut max_red2);
        parent.add_child(&mut rep1);
        parent.add_child(&mut rep2);
        parent.add_child(&mut rep1_1);
        parent.add_child(&mut rep1_2);
        parent.add_child(&mut add1);
        parent.add_child(&mut add2);
        parent.add_child(&mut exp1);
        parent.add_child(&mut exp2);
        // parent.add_child(&mut rep1);
        parent.add_child(&mut div1);
        parent.add_child(&mut div2);
        parent.add_child(&mut drop1);
        parent.add_child(&mut spacc2);
        // parent.add_child(&mut val_drop);
        parent.add_child(&mut repsigm1);
        parent.add_child(&mut repsigm2);
        parent.add_child(&mut rep_m1);
        parent.add_child(&mut rep_m2);
        parent.add_child(&mut spacc1);
        parent.add_child(&mut x0_wrscanner);
        parent.add_child(&mut x1_wrscanner);
        parent.add_child(&mut x2_wrscanner);
        parent.add_child(&mut x3_wrscanner);
        parent.add_child(&mut ql_repeat2);
        parent.add_child(&mut scat1);
        parent.add_child(&mut scat2);
        parent.add_child(&mut scat3);
        parent.add_child(&mut scat4);
        parent.add_child(&mut gat);
        parent.add_child(&mut gat1);
        // parent.add_child(&mut printc);

        parent.init();
        parent.run();
        parent.cleanup();
        // let fil = formatted_dir.to_str().unwrap();
        // dbg!(xvals.out_val);
        dbg!(xvals.view().tick_lower_bound());

        // assert_eq!(x0_wrscanner.crd_arr, a0_crd);
        // assert_eq!(x1_wrscanner.crd_arr, a1_crd);
        // assert_eq!(x2_wrscanner.crd_arr, a2_crd);
        // assert_eq!(x3_wrscanner.crd_arr, a3_crd);
        // assert_eq!(xvals.out_val, a_vals);
    }
}
