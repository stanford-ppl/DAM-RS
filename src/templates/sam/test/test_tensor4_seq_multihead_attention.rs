#[cfg(test)]
mod tests {

    use std::{fs, path::Path};

    use crate::context::broadcast_context::BroadcastContext;
    use crate::context::generator_context::GeneratorContext;
    use crate::simulation::Program;
    use crate::templates::ops::{ALUDivOp, ALUMulOp, ALUSubOp};
    use crate::templates::sam::accumulator::{MaxReduce, Reduce, ReduceData, Spacc1, Spacc1Data};
    use crate::templates::sam::alu::{make_alu, make_unary_alu};
    use crate::templates::sam::array::{Array, ArrayData};
    use crate::templates::sam::crd_manager::{CrdDrop, CrdManagerData};
    use crate::templates::sam::joiner::{CrdJoinerData, Intersect};
    use crate::templates::sam::primitive::{ALUExpOp, Token};
    use crate::templates::sam::rd_scanner::{CompressedCrdRdScan, RdScanData};
    use crate::templates::sam::repeat::{RepSigGenData, Repeat, RepeatData, RepeatSigGen};
    use crate::templates::sam::test::config::Data;
    use crate::templates::sam::utils::read_inputs;

    use crate::templates::sam::wr_scanner::{CompressedWrScan, ValsWrScan};
    use crate::token_vec;

    #[test]
    fn test_multihead_attention() {
        // let test_name = "tensor4_mha";
        let test_name = "tensor4_mha3";
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

        let chan_size = 4096;
        let mut parent = Program::default();

        // fiberlookup_bi
        let (qi_in_ref_sender, qi_in_ref_receiver) = parent.bounded(chan_size);
        let (qi_out_ref_sender, qi_out_ref_receiver) = parent.bounded(chan_size);
        let (qi_out_crd_sender, qi_out_crd_receiver) = parent.bounded(chan_size);

        let (ki_in_ref_sender, ki_in_ref_receiver) = parent.bounded(chan_size);
        let (ki_out_ref_sender, ki_out_ref_receiver) = parent.bounded(chan_size);
        let (ki_out_crd_sender, ki_out_crd_receiver) = parent.bounded(chan_size);

        let (vi_in_ref_sender, vi_in_ref_receiver) = parent.bounded(chan_size);
        let (vi_out_ref_sender, vi_out_ref_receiver) = parent.bounded(chan_size);
        let (vi_out_crd_sender, vi_out_crd_receiver) = parent.bounded(chan_size);

        let q_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            qi_in_ref_sender,
        );
        let k_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            ki_in_ref_sender,
        );
        let v_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            vi_in_ref_sender,
        );
        let qi_data = RdScanData::<u32, u32> {
            // in_ref: bc_bi_in_ref_receiver,
            in_ref: qi_in_ref_receiver,
            out_ref: qi_out_ref_sender,
            out_crd: qi_out_crd_sender,
        };
        let qi_rdscanner = CompressedCrdRdScan::new(qi_data, q0_seg, q0_crd);

        let ki_data = RdScanData::<u32, u32> {
            // in_ref: bc_bi_in_ref_receiver,
            in_ref: ki_in_ref_receiver,
            out_ref: ki_out_ref_sender,
            out_crd: ki_out_crd_sender,
        };
        let ki_rdscanner = CompressedCrdRdScan::new(ki_data, k0_seg, k0_crd);

        let vi_data = RdScanData::<u32, u32> {
            in_ref: vi_in_ref_receiver,
            out_ref: vi_out_ref_sender,
            out_crd: vi_out_crd_sender,
        };
        let vi_rdscanner = CompressedCrdRdScan::new(vi_data, v0_seg, v0_crd);

        let (intersecti_out_crd_sender, intersecti_out_crd_receiver) = parent.bounded(chan_size);
        let (intersecti_out_ref1_sender, intersecti_out_ref1_receiver) = parent.bounded(chan_size);
        let (intersecti_out_ref2_sender, intersecti_out_ref2_receiver) = parent.bounded(chan_size);
        let intersecti_data = CrdJoinerData::<u32, u32> {
            in_crd1: vi_out_crd_receiver,
            in_ref1: vi_out_ref_receiver,
            in_crd2: qi_out_crd_receiver,
            in_ref2: qi_out_ref_receiver,
            out_crd: intersecti_out_crd_sender,
            out_ref1: intersecti_out_ref1_sender,
            out_ref2: intersecti_out_ref2_sender,
        };
        let intersect_i = Intersect::new(intersecti_data);

        let (bc_ki_out_ref_sender, bc_ki_out_ref_receiver) = parent.bounded(chan_size);
        let (bc1_ki_out_ref_sender, bc1_ki_out_ref_receiver) = parent.bounded(chan_size);
        let mut broadcast = BroadcastContext::new(ki_out_ref_receiver);
        broadcast.add_target(bc_ki_out_ref_sender);
        broadcast.add_target(bc1_ki_out_ref_sender);

        let (bc_ki_out_crd_sender, bc_ki_out_crd_receiver) = parent.bounded(chan_size);
        let (bc1_ki_out_crd_sender, bc1_ki_out_crd_receiver) = parent.bounded(chan_size);
        let mut broadcast1 = BroadcastContext::new(ki_out_crd_receiver);
        broadcast1.add_target(bc_ki_out_crd_sender);
        broadcast1.add_target(bc1_ki_out_crd_sender);

        let (bc_intersecti_out_crd_sender, bc_intersecti_out_crd_receiver) =
            parent.bounded(chan_size);
        let (bc1_intersecti_out_crd_sender, bc1_intersecti_out_crd_receiver) =
            parent.bounded(chan_size);
        let mut broadcast2 = BroadcastContext::new(intersecti_out_crd_receiver);
        broadcast2.add_target(bc_intersecti_out_crd_sender);
        broadcast2.add_target(bc1_intersecti_out_crd_sender);

        let (intersecti2_out_crd_sender, intersecti2_out_crd_receiver) = parent.bounded(chan_size);
        let (intersecti2_out_ref2_sender, intersecti2_out_ref2_receiver) =
            parent.bounded(chan_size);
        let intersecti2_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc_ki_out_crd_receiver,
            in_ref1: bc_ki_out_ref_receiver,
            in_crd2: bc_intersecti_out_crd_receiver,
            in_ref2: intersecti_out_ref1_receiver,
            out_crd: intersecti2_out_crd_sender,
            out_ref1: parent.void(),
            out_ref2: intersecti2_out_ref2_sender,
        };
        let intersect_i2 = Intersect::new(intersecti2_data);

        let (intersecti3_out_ref1_sender, intersecti3_out_ref1_receiver) =
            parent.bounded(chan_size);
        let (intersecti3_out_ref2_sender, intersecti3_out_ref2_receiver) =
            parent.bounded(chan_size);

        let intersecti3_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_ki_out_crd_receiver,
            in_ref1: bc1_ki_out_ref_receiver,
            in_crd2: bc1_intersecti_out_crd_receiver,
            in_ref2: intersecti_out_ref2_receiver,
            out_crd: parent.void(),
            out_ref1: intersecti3_out_ref1_sender,
            out_ref2: intersecti3_out_ref2_sender,
        };
        let intersect_i3 = Intersect::new(intersecti3_data);

        let (vj_out_ref_sender, vj_out_ref_receiver) = parent.bounded(chan_size);
        let (vj_out_crd_sender, vj_out_crd_receiver) = parent.bounded(chan_size);
        let vj_data = RdScanData::<u32, u32> {
            in_ref: intersecti2_out_ref2_receiver,
            out_ref: vj_out_ref_sender,
            out_crd: vj_out_crd_sender,
        };
        let vj_rdscanner = CompressedCrdRdScan::new(vj_data, v2_seg, v2_crd);

        let (qj_out_ref_sender, qj_out_ref_receiver) = parent.bounded(chan_size);
        let (qj_out_crd_sender, qj_out_crd_receiver) = parent.bounded(chan_size);
        let qj_data = RdScanData::<u32, u32> {
            in_ref: intersecti3_out_ref2_receiver,
            out_ref: qj_out_ref_sender,
            out_crd: qj_out_crd_sender,
        };
        let qj_rdscanner = CompressedCrdRdScan::new(qj_data, q2_seg, q2_crd);

        let (kj_out_ref_sender, kj_out_ref_receiver) = parent.bounded(chan_size);
        let (kj_out_crd_sender, kj_out_crd_receiver) = parent.bounded(chan_size);
        let kj_data = RdScanData::<u32, u32> {
            in_ref: intersecti3_out_ref1_receiver,
            out_ref: kj_out_ref_sender,
            out_crd: kj_out_crd_sender,
        };
        let kj_rdscanner = CompressedCrdRdScan::new(kj_data, k2_seg, k2_crd);

        let (intersectj_out_crd_sender, intersectj_out_crd_receiver) = parent.bounded(chan_size);
        let (intersectj_out_ref2_sender, intersectj_out_ref2_receiver) = parent.bounded(chan_size);
        let intersectj_data = CrdJoinerData::<u32, u32> {
            in_crd1: vj_out_crd_receiver,
            in_ref1: vj_out_ref_receiver,
            in_crd2: qj_out_crd_receiver,
            in_ref2: qj_out_ref_receiver,
            out_crd: intersectj_out_crd_sender,
            out_ref1: parent.void(),
            out_ref2: intersectj_out_ref2_sender,
        };
        let intersect_j = Intersect::new(intersectj_data);

        let (intersectj3_out_crd_sender, intersectj3_out_crd_receiver) = parent.bounded(chan_size);
        let (intersectj3_out_ref1_sender, intersectj3_out_ref1_receiver) =
            parent.bounded(chan_size);
        let (intersectj3_out_ref2_sender, intersectj3_out_ref2_receiver) =
            parent.bounded(chan_size);

        let intersectj3_data = CrdJoinerData::<u32, u32> {
            in_crd1: kj_out_crd_receiver,
            in_ref1: kj_out_ref_receiver,
            in_crd2: intersectj_out_crd_receiver,
            in_ref2: intersectj_out_ref2_receiver,
            out_crd: intersectj3_out_crd_sender,
            out_ref1: intersectj3_out_ref1_sender,
            out_ref2: intersectj3_out_ref2_sender,
        };
        let intersect_j3 = Intersect::new(intersectj3_data);
        // dbg!(intersect_j.id());
        // dbg!(intersect_j3.id());

        let (bc_intersectj3_out_ref2_sender, bc_intersectj3_out_ref2_receiver) =
            parent.bounded(chan_size);
        let (bc1_intersectj3_out_ref2_sender, bc1_intersectj3_out_ref2_receiver) =
            parent.bounded(chan_size);
        let mut broadcast9 = BroadcastContext::new(intersectj3_out_ref2_receiver);
        broadcast9.add_target(bc_intersectj3_out_ref2_sender);
        broadcast9.add_target(bc1_intersectj3_out_ref2_sender);

        let (qk_out_ref_sender, qk_out_ref_receiver) = parent.bounded(chan_size);
        let (qk_out_crd_sender, qk_out_crd_receiver) = parent.bounded(chan_size);
        let qk_data = RdScanData::<u32, u32> {
            in_ref: bc_intersectj3_out_ref2_receiver,
            out_ref: qk_out_ref_sender,
            out_crd: qk_out_crd_sender,
        };
        let qk_rdscanner = CompressedCrdRdScan::new(qk_data, q1_seg, q1_crd);

        let (bc_qk_out_crd_sender, bc_qk_out_crd_receiver) = parent.bounded(chan_size);
        let (bc1_qk_out_crd_sender, bc1_qk_out_crd_receiver) = parent.bounded(chan_size);
        let (bc2_qk_out_crd_sender, bc2_qk_out_crd_receiver) = parent.bounded(chan_size);
        let mut broadcast7 = BroadcastContext::new(qk_out_crd_receiver);
        broadcast7.add_target(bc_qk_out_crd_sender);
        broadcast7.add_target(bc1_qk_out_crd_sender);
        broadcast7.add_target(bc2_qk_out_crd_sender);

        // repeatsiggen
        let (out_repsig_k_sender, out_repsig_k_receiver) = parent.bounded(chan_size);
        let repsig_k_data = RepSigGenData::<u32, u32> {
            input: bc_qk_out_crd_receiver,
            // input: qk_out_crd_receiver,
            out_repsig: out_repsig_k_sender,
        };
        let repsig_k = RepeatSigGen::new(repsig_k_data);

        let (bc_out_repsig_k_sender, bc_out_repsig_k_receiver) = parent.bounded(chan_size);
        let (bc1_out_repsig_k_sender, bc1_out_repsig_k_receiver) = parent.bounded(chan_size);
        let mut broadcast8 = BroadcastContext::new(out_repsig_k_receiver);
        broadcast8.add_target(bc_out_repsig_k_sender);
        broadcast8.add_target(bc1_out_repsig_k_sender);

        // repeat
        let (out_repeat_vk_sender, out_repeat_vk_receiver) = parent.bounded(chan_size);
        let vk_repeat_data = RepeatData::<u32, u32> {
            in_ref: bc1_intersectj3_out_ref2_receiver,
            in_repsig: bc_out_repsig_k_receiver,
            out_ref: out_repeat_vk_sender,
        };
        let vk_repeat = Repeat::new(vk_repeat_data);

        // repeat
        let (out_repeat_kk_sender, out_repeat_kk_receiver) = parent.bounded(chan_size);
        let kk_repeat_data = RepeatData::<u32, u32> {
            in_ref: intersectj3_out_ref1_receiver,
            in_repsig: bc1_out_repsig_k_receiver,
            out_ref: out_repeat_kk_sender,
        };
        let kk_repeat = Repeat::new(kk_repeat_data);

        let (kl_out_ref_sender, kl_out_ref_receiver) = parent.bounded(chan_size);
        let (kl_out_crd_sender, kl_out_crd_receiver) = parent.bounded(chan_size);
        let kl_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_kk_receiver,
            out_ref: kl_out_ref_sender,
            out_crd: kl_out_crd_sender,
        };
        let kl_rdscanner = CompressedCrdRdScan::new(kl_data, k1_seg, k1_crd);

        // let (bc_kl_out_crd_sender, bc_kl_out_crd_receiver) = parent.bounded(chan_size);
        // // let (bc1_kl_out_crd_sender, bc1_kl_out_crd_receiver) =
        // //     parent.bounded(chan_size);
        // // let (bc2_kl_out_crd_sender, bc2_kl_out_crd_receiver) = parent.bounded(chan_size);
        // let mut broadcast15 = BroadcastContext::new(kl_out_crd_receiver);
        // broadcast15.add_target(bc_kl_out_crd_sender);
        // broadcast15.add_target(bc1_kl_out_crd_sender);
        // broadcast15.add_target(bc2_kl_out_crd_sender);

        let (vl_out_ref_sender, vl_out_ref_receiver) = parent.bounded(chan_size);
        let (vl_out_crd_sender, vl_out_crd_receiver) = parent.bounded(chan_size);
        let vl_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_vk_receiver,
            out_ref: vl_out_ref_sender,
            out_crd: vl_out_crd_sender,
        };
        let vl_rdscanner = CompressedCrdRdScan::new(vl_data, v1_seg, v1_crd);

        let (intersectl_out_crd_sender, intersectl_out_crd_receiver) = parent.bounded(chan_size);
        let (intersectl_out_ref1_sender, intersectl_out_ref1_receiver) = parent.bounded(chan_size);
        let (intersectl_out_ref2_sender, intersectl_out_ref2_receiver) = parent.bounded(chan_size);
        let intersectl_data = CrdJoinerData::<u32, u32> {
            in_crd1: vl_out_crd_receiver,
            in_ref1: vl_out_ref_receiver,
            in_crd2: kl_out_crd_receiver,
            in_ref2: kl_out_ref_receiver,
            out_crd: intersectl_out_crd_sender,
            out_ref1: intersectl_out_ref1_sender,
            out_ref2: intersectl_out_ref2_sender,
        };
        let intersect_l = Intersect::new(intersectl_data);
        // dbg!(intersect_l.id());

        let (bc_intersectl_out_crd_sender, bc_intersectl_out_crd_receiver) =
            parent.bounded(chan_size);
        let (bc1_intersectl_out_crd_sender, bc1_intersectl_out_crd_receiver) =
            parent.bounded(chan_size);
        let mut broadcast17 = BroadcastContext::new(intersectl_out_crd_receiver);
        broadcast17.add_target(bc_intersectl_out_crd_sender);
        broadcast17.add_target(bc1_intersectl_out_crd_sender);

        let (vm_out_ref_sender, vm_out_ref_receiver) = parent.bounded(chan_size);
        let (vm_out_crd_sender, vm_out_crd_receiver) = parent.bounded(chan_size);
        let vm_data = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref1_receiver,
            out_ref: vm_out_ref_sender,
            out_crd: vm_out_crd_sender,
        };
        let vm_rdscanner = CompressedCrdRdScan::new(vm_data, v3_seg, v3_crd);

        let (km_out_ref_sender, km_out_ref_receiver) = parent.bounded(chan_size);
        let (km_out_crd_sender, km_out_crd_receiver) = parent.bounded(chan_size);
        let km_data = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref2_receiver,
            out_ref: km_out_ref_sender,
            out_crd: km_out_crd_sender,
        };
        let km_rdscanner = CompressedCrdRdScan::new(km_data, k3_seg, k3_crd);

        // repeatsiggen
        let (out_repsig_l_sender, out_repsig_l_receiver) = parent.bounded(chan_size);
        let repsig_l_data = RepSigGenData::<u32, u32> {
            input: bc_intersectl_out_crd_receiver,
            out_repsig: out_repsig_l_sender,
        };
        let repsig_l = RepeatSigGen::new(repsig_l_data);

        let (bc_out_repsig_l_sender, bc_out_repsig_l_receiver) = parent.bounded(chan_size);
        let (bc1_out_repsig_l_sender, bc1_out_repsig_l_receiver) = parent.bounded(chan_size);
        let (bc2_out_repsig_l_sender, bc2_out_repsig_l_receiver) = parent.bounded(chan_size);
        let mut broadcast10 = BroadcastContext::new(out_repsig_l_receiver);
        broadcast10.add_target(bc_out_repsig_l_sender);
        broadcast10.add_target(bc1_out_repsig_l_sender);
        broadcast10.add_target(bc2_out_repsig_l_sender);

        // repeat
        let (out_repeat_ql_sender, out_repeat_ql_receiver) = parent.bounded(chan_size);
        let ql_repeat_data = RepeatData::<u32, u32> {
            in_ref: qk_out_ref_receiver,
            in_repsig: bc_out_repsig_l_receiver,
            out_ref: out_repeat_ql_sender,
        };
        let ql_repeat = Repeat::new(ql_repeat_data);

        let (qm_out_ref_sender, qm_out_ref_receiver) = parent.bounded(chan_size);
        let (qm_out_crd_sender, qm_out_crd_receiver) = parent.bounded(chan_size);
        let qm_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_ql_receiver,
            out_ref: qm_out_ref_sender,
            out_crd: qm_out_crd_sender,
        };
        let qm_rdscanner = CompressedCrdRdScan::new(qm_data, q3_seg, q3_crd);

        let (intersectm_out_crd_sender, intersectm_out_crd_receiver) = parent.bounded(chan_size);
        let (intersectm_out_ref1_sender, intersectm_out_ref1_receiver) = parent.bounded(chan_size);
        let (intersectm_out_ref2_sender, intersectm_out_ref2_receiver) = parent.bounded(chan_size);
        let intersectm_data = CrdJoinerData::<u32, u32> {
            in_crd1: vm_out_crd_receiver,
            in_ref1: vm_out_ref_receiver,
            in_crd2: qm_out_crd_receiver,
            in_ref2: qm_out_ref_receiver,
            out_crd: intersectm_out_crd_sender,
            out_ref1: intersectm_out_ref1_sender,
            out_ref2: intersectm_out_ref2_sender,
        };
        let intersect_m = Intersect::new(intersectm_data);
        // dbg!(intersect_m.id());

        let (bc_km_out_ref_sender, bc_km_out_ref_receiver) = parent.bounded(chan_size);
        let (bc1_km_out_ref_sender, bc1_km_out_ref_receiver) = parent.bounded(chan_size);
        let mut broadcast11 = BroadcastContext::new(km_out_ref_receiver);
        broadcast11.add_target(bc_km_out_ref_sender);
        broadcast11.add_target(bc1_km_out_ref_sender);

        let (bc_km_out_crd_sender, bc_km_out_crd_receiver) = parent.bounded(chan_size);
        let (bc1_km_out_crd_sender, bc1_km_out_crd_receiver) = parent.bounded(chan_size);
        let mut broadcast13 = BroadcastContext::new(km_out_crd_receiver);
        broadcast13.add_target(bc_km_out_crd_sender);
        broadcast13.add_target(bc1_km_out_crd_sender);

        let (bc_intersectm_out_crd_sender, bc_intersectm_out_crd_receiver) =
            parent.bounded(chan_size);
        let (bc1_intersectm_out_crd_sender, bc1_intersectm_out_crd_receiver) =
            parent.bounded(chan_size);
        let mut broadcast12 = BroadcastContext::new(intersectm_out_crd_receiver);
        broadcast12.add_target(bc_intersectm_out_crd_sender);
        broadcast12.add_target(bc1_intersectm_out_crd_sender);

        let (intersectm2_out_ref2_sender, intersectm2_out_ref2_receiver) =
            parent.bounded(chan_size);
        let intersectm2_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc_km_out_crd_receiver,
            in_ref1: bc_km_out_ref_receiver,
            in_crd2: bc_intersectm_out_crd_receiver,
            in_ref2: intersectm_out_ref1_receiver,
            out_crd: parent.void(),
            out_ref1: parent.void(),
            out_ref2: intersectm2_out_ref2_sender,
        };
        let intersect_m2 = Intersect::new(intersectm2_data);
        // dbg!(intersect_m2.id());

        let (intersectm3_out_crd_sender, intersectm3_out_crd_receiver) = parent.bounded(chan_size);
        // let (intersectm3_out_ref1_sender, intersectm3_out_ref1_receiver) =
        let (intersectm3_out_ref1_sender, intersectm3_out_ref1_receiver) =
            parent.bounded(chan_size);
        let (intersectm3_out_ref2_sender, intersectm3_out_ref2_receiver) =
            parent.bounded(chan_size);

        let intersectm3_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_km_out_crd_receiver,
            in_ref1: bc1_km_out_ref_receiver,
            in_crd2: bc1_intersectm_out_crd_receiver,
            in_ref2: intersectm_out_ref2_receiver,
            out_crd: intersectm3_out_crd_sender,
            out_ref1: intersectm3_out_ref1_sender,
            out_ref2: intersectm3_out_ref2_sender,
        };
        let intersect_m3 = Intersect::new(intersectm3_data);

        let (bc_intersectm3_out_crd_sender, bc_intersectm3_out_crd_receiver) =
            parent.bounded(chan_size);
        let (bc1_intersectm3_out_crd_sender, bc1_intersectm3_out_crd_receiver) =
            parent.bounded(chan_size);
        let mut broadcast16 = BroadcastContext::new(intersectm3_out_crd_receiver);
        broadcast16.add_target(bc_intersectm3_out_crd_sender);
        broadcast16.add_target(bc1_intersectm3_out_crd_sender);

        // arrayvals_q
        let (q_out_val_sender, q_out_val_receiver) = parent.bounded(chan_size);
        let arrayvals_q_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref2_receiver,
            out_val: q_out_val_sender,
        };
        let arrayvals_q = Array::<u32, f32, u32>::new(arrayvals_q_data, q_vals);

        // arrayvals_k
        let (k_out_val_sender, k_out_val_receiver) = parent.bounded(chan_size);
        let arrayvals_k_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref1_receiver,
            out_val: k_out_val_sender,
        };
        let arrayvals_k = Array::<u32, f32, u32>::new(arrayvals_k_data, k_vals);

        // arrayvals_v
        let (v_out_val_sender, v_out_val_receiver) = parent.bounded(chan_size);
        let arrayvals_v_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectm2_out_ref2_receiver,
            out_val: v_out_val_sender,
        };
        let arrayvals_v = Array::<u32, f32, u32>::new(arrayvals_v_data, v_vals);

        // mul ALU
        let (mul_out_sender, mul_out_receiver) = parent.bounded(chan_size);
        let mul = make_alu(
            q_out_val_receiver,
            k_out_val_receiver,
            mul_out_sender,
            ALUMulOp(),
        );

        // Reduce
        let (red_out_sender, red_out_receiver) = parent.bounded(chan_size);
        let red_data = ReduceData::<f32, u32> {
            in_val: mul_out_receiver,
            out_val: red_out_sender,
        };
        let red = Reduce::new(red_data);

        let (bc_out_red_sender, bc_out_red_receiver) = parent.bounded(chan_size);
        let (bc1_out_red_sender, bc1_out_red_receiver) = parent.bounded(chan_size);
        let mut broadcast6 = BroadcastContext::new(red_out_receiver);
        broadcast6.add_target(bc_out_red_sender);
        broadcast6.add_target(bc1_out_red_sender);

        // Max Reduce
        let (max_out_val_sender, max_out_val_receiver) = parent.bounded(chan_size);
        let max_data = ReduceData::<f32, u32> {
            in_val: bc_out_red_receiver,
            out_val: max_out_val_sender,
        };
        let max_red = MaxReduce::new(max_data, f32::MIN);

        let (rep_out_val_sender, rep_out_val_receiver) = parent.bounded(chan_size);
        let rep_data = RepeatData::<f32, u32> {
            in_ref: max_out_val_receiver,
            in_repsig: bc1_out_repsig_l_receiver,
            out_ref: rep_out_val_sender,
        };
        let rep = Repeat::new(rep_data);

        // Sub ALU, using Add name to correspond to SAM implementation
        let (add_out_sender, add_out_receiver) = parent.bounded(chan_size);
        let add = make_alu(
            bc1_out_red_receiver,
            rep_out_val_receiver,
            add_out_sender,
            ALUSubOp(),
        );

        // Exp
        let (exp_out_sender, exp_out_receiver) = parent.bounded(chan_size);
        let exp = make_unary_alu(add_out_receiver, exp_out_sender, ALUExpOp());

        let (bc_exp_out_sender, bc_exp_out_receiver) = parent.bounded(chan_size);
        let (bc1_exp_out_sender, bc1_exp_out_receiver) = parent.bounded(chan_size);
        let mut broadcast14 = BroadcastContext::new(exp_out_receiver);
        broadcast14.add_target(bc_exp_out_sender);
        broadcast14.add_target(bc1_exp_out_sender);

        // Reduce
        let (red1_out_sender, red1_out_receiver) = parent.bounded(chan_size);
        let red1_data = ReduceData::<f32, u32> {
            in_val: bc_exp_out_receiver,
            out_val: red1_out_sender,
        };
        let red1 = Reduce::new(red1_data);

        let (rep1_out_val_sender, rep1_out_val_receiver) = parent.bounded(chan_size);
        let rep1_data = RepeatData::<f32, u32> {
            in_ref: red1_out_receiver,
            in_repsig: bc2_out_repsig_l_receiver,
            out_ref: rep1_out_val_sender,
        };
        let rep1 = Repeat::new(rep1_data);

        // Div ALU
        let (div_out_sender, div_out_receiver) = parent.bounded(chan_size);
        let div = make_alu(
            bc1_exp_out_receiver,
            rep1_out_val_receiver,
            div_out_sender,
            ALUDivOp(),
        );

        // let (out_drop_val_sender, out_drop_val_receiver) = parent.bounded(chan_size);
        // let (out_drop_crd_sender, out_drop_crd_receiver) = unbounded::<Token<u32, u32>>();

        // let val_drop_data = ValDropData::<u32, f32, u32> {
        //     in_val: div_out_receiver,
        //     in_crd: bc1_kl_out_crd_receiver,
        //     out_val: out_drop_val_sender,
        //     out_crd: out_drop_crd_sender,
        // };

        // let mut val_drop = ValDrop::new(val_drop_data);

        let (out_repsig_m_sender, out_repsig_m_receiver) = parent.bounded(chan_size);
        let repsig_m_data = RepSigGenData::<u32, u32> {
            input: bc_intersectm3_out_crd_receiver,
            out_repsig: out_repsig_m_sender,
        };
        let repsigm = RepeatSigGen::new(repsig_m_data);

        let (rep_m_out_val_sender, rep_m_out_val_receiver) = parent.bounded(chan_size);
        let rep2_data = RepeatData::<f32, u32> {
            // in_ref: out_drop_val_receiver,
            in_ref: div_out_receiver,
            in_repsig: out_repsig_m_receiver,
            out_ref: rep_m_out_val_sender,
        };
        let rep_m = Repeat::new(rep2_data);

        // mul ALU
        let (mul2_out_sender, mul2_out_receiver) = parent.bounded(chan_size);
        let mul2 = make_alu(
            rep_m_out_val_receiver,
            v_out_val_receiver,
            mul2_out_sender,
            ALUMulOp(),
        );

        // let (drop_out_ocrd_sender, drop_out_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (drop_out_icrd_sender, drop_out_icrd_receiver) = parent.bounded(chan_size);

        let crd_drop_data = CrdManagerData::<u32, u32> {
            in_crd_outer: bc2_qk_out_crd_receiver,
            in_crd_inner: bc1_intersectl_out_crd_receiver,
            out_crd_outer: parent.void(),
            out_crd_inner: drop_out_icrd_sender,
        };

        let drop = CrdDrop::new(crd_drop_data);

        let (out_spacc_val_sender, out_spacc_val_receiver) = parent.bounded(chan_size);
        let (out_spacc_icrd_sender, out_spacc_icrd_receiver) = parent.bounded(chan_size);
        let spacc_data = Spacc1Data::<u32, f32, u32> {
            in_crd_outer: drop_out_icrd_receiver,
            in_crd_inner: bc1_intersectm3_out_crd_receiver,
            in_val: mul2_out_receiver,
            out_val: out_spacc_val_sender,
            out_crd_inner: out_spacc_icrd_sender,
        };
        let spacc = Spacc1::new(spacc_data);

        // fiberwrite_X0
        let x0_wrscanner = CompressedWrScan::new(intersecti2_out_crd_receiver);

        // fiberwrite_X1
        let x1_wrscanner = CompressedWrScan::new(bc1_qk_out_crd_receiver);

        // fiberwrite_X2
        let x2_wrscanner = CompressedWrScan::new(intersectj3_out_crd_receiver);

        // fiberwrite_X3
        let x3_wrscanner = CompressedWrScan::new(out_spacc_icrd_receiver);

        // fiberwrite_Xvals
        let xvals = ValsWrScan::<f32, u32>::new(out_spacc_val_receiver);

        parent.add_child(q_gen);
        parent.add_child(k_gen);
        parent.add_child(v_gen);
        parent.add_child(qi_rdscanner);
        parent.add_child(ki_rdscanner);
        parent.add_child(vi_rdscanner);
        parent.add_child(broadcast);
        parent.add_child(broadcast1);
        parent.add_child(broadcast2);
        // parent.add_child(&mut broadcast3);
        // parent.add_child(&mut broadcast4);
        // parent.add_child(&mut broadcast5);
        parent.add_child(broadcast6);
        parent.add_child(broadcast7);
        parent.add_child(broadcast8);
        parent.add_child(broadcast9);
        parent.add_child(broadcast10);
        parent.add_child(broadcast11);
        parent.add_child(broadcast12);
        parent.add_child(broadcast13);
        parent.add_child(broadcast14);
        parent.add_child(broadcast16);
        parent.add_child(broadcast17);
        parent.add_child(drop);
        parent.add_child(intersect_i);
        parent.add_child(intersect_i2);
        parent.add_child(intersect_i3);
        parent.add_child(intersect_j);
        // parent.add_child(&mut intersect_j2);
        parent.add_child(intersect_j3);
        parent.add_child(vj_rdscanner);
        parent.add_child(kj_rdscanner);
        parent.add_child(qj_rdscanner);
        parent.add_child(qk_rdscanner);
        parent.add_child(repsig_k);
        parent.add_child(vk_repeat);
        parent.add_child(kk_repeat);
        parent.add_child(kl_rdscanner);
        parent.add_child(vl_rdscanner);
        parent.add_child(intersect_l);
        parent.add_child(km_rdscanner);
        parent.add_child(vm_rdscanner);
        parent.add_child(repsig_l);
        parent.add_child(ql_repeat);
        parent.add_child(qm_rdscanner);
        parent.add_child(intersect_m);
        parent.add_child(intersect_m2);
        parent.add_child(intersect_m3);
        parent.add_child(arrayvals_q);
        parent.add_child(arrayvals_k);
        parent.add_child(arrayvals_v);
        parent.add_child(mul);
        parent.add_child(mul2);
        parent.add_child(xvals);
        parent.add_child(red);
        parent.add_child(red1);
        parent.add_child(max_red);
        parent.add_child(rep);
        parent.add_child(add);
        parent.add_child(exp);
        parent.add_child(rep1);
        parent.add_child(div);
        // parent.add_child(&mut val_drop);
        parent.add_child(repsigm);
        parent.add_child(rep_m);
        parent.add_child(spacc);
        parent.add_child(x0_wrscanner);
        parent.add_child(x1_wrscanner);
        parent.add_child(x2_wrscanner);
        parent.add_child(x3_wrscanner);

        parent.init();
        parent.run();
        // let fil = formatted_dir.to_str().unwrap();
        // dbg!(xvals.out_val);
        // dbg!(xvals.view().tick_lower_bound());

        // assert_eq!(x0_wrscanner.crd_arr, a0_crd);
        // assert_eq!(x1_wrscanner.crd_arr, a1_crd);
        // assert_eq!(x2_wrscanner.crd_arr, a2_crd);
        // assert_eq!(x3_wrscanner.crd_arr, a3_crd);
        // assert_eq!(xvals.out_val, a_vals);
    }
}
