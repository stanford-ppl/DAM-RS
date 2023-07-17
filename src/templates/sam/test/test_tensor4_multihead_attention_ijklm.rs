#[cfg(test)]
mod tests {

    use std::{fs, path::Path};

    use crate::channel::bounded;
    use crate::context::broadcast_context::BroadcastContext;
    use crate::context::generator_context::GeneratorContext;
    use crate::context::parent::BasicParentContext;
    use crate::context::{Context, ParentContext};
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
        let test_name = "tensor4_fused_mul_T4";
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

        let chan_size = 4096;

        // fiberlookup_bi
        let (qi_in_ref_sender, qi_in_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (qi_out_ref_sender, qi_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (qi_out_crd_sender, qi_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);

        let (ki_in_ref_sender, ki_in_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (ki_out_ref_sender, ki_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (ki_out_crd_sender, ki_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);

        let (vi_in_ref_sender, vi_in_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (vi_out_ref_sender, vi_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (vi_out_crd_sender, vi_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);

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

        let (intersecti_out_crd_sender, intersecti_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersecti_out_ref1_sender, intersecti_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersecti_out_ref2_sender, intersecti_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
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

        let (bc_ki_out_ref_sender, bc_ki_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_ki_out_ref_sender, bc1_ki_out_ref_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast = BroadcastContext::new(ki_out_ref_receiver);
        broadcast.add_target(bc_ki_out_ref_sender);
        broadcast.add_target(bc1_ki_out_ref_sender);

        let (bc_ki_out_crd_sender, bc_ki_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_ki_out_crd_sender, bc1_ki_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast1 = BroadcastContext::new(ki_out_crd_receiver);
        broadcast1.add_target(bc_ki_out_crd_sender);
        broadcast1.add_target(bc1_ki_out_crd_sender);

        let (bc_intersecti_out_crd_sender, bc_intersecti_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (bc1_intersecti_out_crd_sender, bc1_intersecti_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast2 = BroadcastContext::new(intersecti_out_crd_receiver);
        broadcast2.add_target(bc_intersecti_out_crd_sender);
        broadcast2.add_target(bc1_intersecti_out_crd_sender);

        let (intersecti2_out_crd_sender, _intersecti2_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersecti2_out_ref1_sender, intersecti2_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersecti2_out_ref2_sender, intersecti2_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let intersecti2_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc_ki_out_crd_receiver,
            in_ref1: bc_ki_out_ref_receiver,
            in_crd2: bc_intersecti_out_crd_receiver,
            in_ref2: intersecti_out_ref1_receiver,
            out_crd: intersecti2_out_crd_sender,
            out_ref1: intersecti2_out_ref1_sender,
            out_ref2: intersecti2_out_ref2_sender,
        };
        let mut intersect_i2 = Intersect::new(intersecti2_data);

        let (intersecti3_out_crd_sender, intersecti3_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersecti3_out_ref1_sender, intersecti3_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersecti3_out_ref2_sender, intersecti3_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);

        let intersecti3_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_ki_out_crd_receiver,
            in_ref1: bc1_ki_out_ref_receiver,
            in_crd2: bc1_intersecti_out_crd_receiver,
            in_ref2: intersecti_out_ref2_receiver,
            out_crd: intersecti3_out_crd_sender,
            out_ref1: intersecti3_out_ref1_sender,
            out_ref2: intersecti3_out_ref2_sender,
        };
        let mut intersect_i3 = Intersect::new(intersecti3_data);

        let (vj_out_ref_sender, vj_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (vj_out_crd_sender, vj_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let vj_data = RdScanData::<u32, u32> {
            in_ref: intersecti2_out_ref2_receiver,
            out_ref: vj_out_ref_sender,
            out_crd: vj_out_crd_sender,
        };
        let mut vj_rdscanner = CompressedCrdRdScan::new(vj_data, v2_seg, v2_crd);

        let (qj_out_ref_sender, qj_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (qj_out_crd_sender, qj_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let qj_data = RdScanData::<u32, u32> {
            in_ref: intersecti3_out_ref2_receiver,
            out_ref: qj_out_ref_sender,
            out_crd: qj_out_crd_sender,
        };
        let mut qj_rdscanner = CompressedCrdRdScan::new(qj_data, q2_seg, q2_crd);

        let (kj_out_ref_sender, kj_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (kj_out_crd_sender, kj_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let kj_data = RdScanData::<u32, u32> {
            in_ref: intersecti3_out_ref1_receiver,
            out_ref: kj_out_ref_sender,
            out_crd: kj_out_crd_sender,
        };
        let mut kj_rdscanner = CompressedCrdRdScan::new(kj_data, k2_seg, k2_crd);

        let (intersectj_out_crd_sender, intersectj_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectj_out_ref1_sender, intersectj_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectj_out_ref2_sender, intersectj_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let intersectj_data = CrdJoinerData::<u32, u32> {
            in_crd1: vj_out_crd_receiver,
            in_ref1: vj_out_ref_receiver,
            in_crd2: qj_out_crd_receiver,
            in_ref2: qj_out_ref_receiver,
            out_crd: intersectj_out_crd_sender,
            out_ref1: intersectj_out_ref1_sender,
            out_ref2: intersectj_out_ref2_sender,
        };
        let mut intersect_j = Intersect::new(intersectj_data);

        let (bc_kj_out_ref_sender, bc_kj_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_kj_out_ref_sender, bc1_kj_out_ref_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast3 = BroadcastContext::new(kj_out_ref_receiver);
        broadcast3.add_target(bc_kj_out_ref_sender);
        broadcast3.add_target(bc1_kj_out_ref_sender);

        let (bc_kj_out_crd_sender, bc_kj_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_kj_out_crd_sender, bc1_kj_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast4 = BroadcastContext::new(kj_out_crd_receiver);
        broadcast4.add_target(bc_kj_out_crd_sender);
        broadcast4.add_target(bc1_kj_out_crd_sender);

        let (bc_intersectj_out_crd_sender, bc_intersectj_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (bc1_intersectj_out_crd_sender, bc1_intersectj_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast5 = BroadcastContext::new(intersectj_out_crd_receiver);
        broadcast5.add_target(bc_intersectj_out_crd_sender);
        broadcast5.add_target(bc1_intersectj_out_crd_sender);

        let (intersectj2_out_crd_sender, _intersectj2_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectj2_out_ref1_sender, intersectj2_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectj2_out_ref2_sender, intersectj2_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let intersectj2_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc_kj_out_crd_receiver,
            in_ref1: bc_kj_out_ref_receiver,
            in_crd2: bc_intersectj_out_crd_receiver,
            in_ref2: intersectj_out_ref1_receiver,
            out_crd: intersectj2_out_crd_sender,
            out_ref1: intersectj2_out_ref1_sender,
            out_ref2: intersectj2_out_ref2_sender,
        };
        let mut intersect_j2 = Intersect::new(intersectj2_data);

        let (intersectj3_out_crd_sender, intersectj3_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectj3_out_ref1_sender, intersectj3_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectj3_out_ref2_sender, intersectj3_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);

        let intersectj3_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_kj_out_crd_receiver,
            in_ref1: bc1_kj_out_ref_receiver,
            in_crd2: bc1_intersectj_out_crd_receiver,
            in_ref2: intersectj_out_ref2_receiver,
            out_crd: intersectj3_out_crd_sender,
            out_ref1: intersectj3_out_ref1_sender,
            out_ref2: intersectj3_out_ref2_sender,
        };
        let mut intersect_j3 = Intersect::new(intersectj3_data);

        let (bc_intersectj3_out_ref2_sender, bc_intersectj3_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (bc1_intersectj3_out_ref2_sender, bc1_intersectj3_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast9 = BroadcastContext::new(intersectj3_out_ref2_receiver);
        broadcast9.add_target(bc_intersectj3_out_ref2_sender);
        broadcast9.add_target(bc1_intersectj3_out_ref2_sender);

        let (qk_out_ref_sender, qk_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (qk_out_crd_sender, qk_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let qk_data = RdScanData::<u32, u32> {
            in_ref: bc_intersectj3_out_ref2_receiver,
            out_ref: qk_out_ref_sender,
            out_crd: qk_out_crd_sender,
        };
        let mut qk_rdscanner = CompressedCrdRdScan::new(qk_data, q1_seg, q1_crd);

        let (bc_qk_out_crd_sender, bc_qk_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_qk_out_crd_sender, bc1_qk_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast6 = BroadcastContext::new(qk_out_crd_receiver);
        broadcast6.add_target(bc_qk_out_crd_sender);
        broadcast6.add_target(bc1_qk_out_crd_sender);

        let (bc_qk_out_ref_sender, bc_qk_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_qk_out_ref_sender, bc1_qk_out_ref_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast7 = BroadcastContext::new(qk_out_ref_receiver);
        broadcast7.add_target(bc_qk_out_ref_sender);
        broadcast7.add_target(bc1_qk_out_ref_sender);

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
        let (out_repeat_vk_sender, out_repeat_vk_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let vk_repeat_data = RepeatData::<u32, u32> {
            in_ref: bc1_intersectj3_out_ref2_receiver,
            in_repsig: bc_out_repsig_k_receiver,
            out_ref: out_repeat_vk_sender,
        };
        let mut vk_repeat = Repeat::new(vk_repeat_data);

        // repeat
        let (out_repeat_kk_sender, out_repeat_kk_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let kk_repeat_data = RepeatData::<u32, u32> {
            in_ref: intersectj3_out_ref1_receiver,
            in_repsig: bc1_out_repsig_k_receiver,
            out_ref: out_repeat_kk_sender,
        };
        let mut kk_repeat = Repeat::new(kk_repeat_data);

        let (kl_out_ref_sender, kl_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (kl_out_crd_sender, kl_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let kl_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_kk_receiver,
            out_ref: kl_out_ref_sender,
            out_crd: kl_out_crd_sender,
        };
        let mut kl_rdscanner = CompressedCrdRdScan::new(kl_data, k1_seg, k1_crd);

        let (vl_out_ref_sender, vl_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (vl_out_crd_sender, vl_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let vl_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_vk_receiver,
            out_ref: vl_out_ref_sender,
            out_crd: vl_out_crd_sender,
        };
        let mut vl_rdscanner = CompressedCrdRdScan::new(vl_data, v1_seg, v1_crd);

        let (intersectl_out_crd_sender, intersectl_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectl_out_ref1_sender, intersectl_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectl_out_ref2_sender, intersectl_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let intersectl_data = CrdJoinerData::<u32, u32> {
            in_crd1: vl_out_crd_receiver,
            in_ref1: vl_out_ref_receiver,
            in_crd2: kl_out_crd_receiver,
            in_ref2: kl_out_ref_receiver,
            out_crd: intersectl_out_crd_sender,
            out_ref1: intersectl_out_ref1_sender,
            out_ref2: intersectl_out_ref2_sender,
        };
        let mut intersect_l = Intersect::new(intersectl_data);

        let (vm_out_ref_sender, vm_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (vm_out_crd_sender, vm_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let vm_data = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref1_receiver,
            out_ref: vm_out_ref_sender,
            out_crd: vm_out_crd_sender,
        };
        let mut vm_rdscanner = CompressedCrdRdScan::new(vm_data, v3_seg, v3_crd);

        let (km_out_ref_sender, km_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (km_out_crd_sender, km_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let km_data = RdScanData::<u32, u32> {
            in_ref: intersectl_out_ref2_receiver,
            out_ref: km_out_ref_sender,
            out_crd: km_out_crd_sender,
        };
        let mut km_rdscanner = CompressedCrdRdScan::new(km_data, k3_seg, k3_crd);

        // repeatsiggen
        let (out_repsig_l_sender, out_repsig_l_receiver) = bounded::<Repsiggen>(chan_size);
        let repsig_l_data = RepSigGenData::<u32, u32> {
            input: intersectl_out_crd_receiver,
            out_repsig: out_repsig_l_sender,
        };
        let mut repsig_l = RepeatSigGen::new(repsig_l_data);

        let (bc_out_repsig_l_sender, bc_out_repsig_l_receiver) = bounded::<Repsiggen>(chan_size);
        let (bc1_out_repsig_l_sender, bc1_out_repsig_l_receiver) = bounded::<Repsiggen>(chan_size);
        let mut broadcast10 = BroadcastContext::new(out_repsig_l_receiver);
        broadcast10.add_target(bc_out_repsig_l_sender);
        broadcast10.add_target(bc1_out_repsig_l_sender);

        // repeat
        let (out_repeat_ql_sender, out_repeat_ql_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let ql_repeat_data = RepeatData::<u32, u32> {
            in_ref: bc1_qk_out_ref_receiver,
            in_repsig: bc_out_repsig_l_receiver,
            out_ref: out_repeat_ql_sender,
        };
        let mut ql_repeat = Repeat::new(ql_repeat_data);

        let (qm_out_ref_sender, qm_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (qm_out_crd_sender, qm_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let qm_data = RdScanData::<u32, u32> {
            in_ref: out_repeat_ql_receiver,
            out_ref: qm_out_ref_sender,
            out_crd: qm_out_crd_sender,
        };
        let mut qm_rdscanner = CompressedCrdRdScan::new(qm_data, q3_seg, q3_crd);

        let (intersectm_out_crd_sender, intersectm_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectm_out_ref1_sender, intersectm_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectm_out_ref2_sender, intersectm_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let intersectm_data = CrdJoinerData::<u32, u32> {
            in_crd1: vm_out_crd_receiver,
            in_ref1: vm_out_ref_receiver,
            in_crd2: qm_out_crd_receiver,
            in_ref2: qm_out_ref_receiver,
            out_crd: intersectm_out_crd_sender,
            out_ref1: intersectm_out_ref1_sender,
            out_ref2: intersectm_out_ref2_sender,
        };
        let mut intersect_m = Intersect::new(intersectm_data);

        let (bc_km_out_ref_sender, bc_km_out_ref_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_km_out_ref_sender, bc1_km_out_ref_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast11 = BroadcastContext::new(km_out_ref_receiver);
        broadcast11.add_target(bc_km_out_ref_sender);
        broadcast11.add_target(bc1_km_out_ref_sender);

        let (bc_km_out_crd_sender, bc_km_out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (bc1_km_out_crd_sender, bc1_km_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast13 = BroadcastContext::new(km_out_crd_receiver);
        broadcast13.add_target(bc_km_out_crd_sender);
        broadcast13.add_target(bc1_km_out_crd_sender);

        let (bc_intersectm_out_crd_sender, bc_intersectm_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (bc1_intersectm_out_crd_sender, bc1_intersectm_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let mut broadcast12 = BroadcastContext::new(intersectm_out_crd_receiver);
        broadcast12.add_target(bc_intersectm_out_crd_sender);
        broadcast12.add_target(bc1_intersectm_out_crd_sender);

        let (intersectm2_out_crd_sender, _intersectm2_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectm2_out_ref1_sender, intersectm2_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectm2_out_ref2_sender, intersectm2_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let intersectm2_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc_km_out_crd_receiver,
            in_ref1: bc_km_out_ref_receiver,
            in_crd2: bc_intersectm_out_crd_receiver,
            in_ref2: intersectm_out_ref1_receiver,
            out_crd: intersectm2_out_crd_sender,
            out_ref1: intersectm2_out_ref1_sender,
            out_ref2: intersectm2_out_ref2_sender,
        };
        let mut intersect_m2 = Intersect::new(intersectm2_data);

        let (intersectm3_out_crd_sender, intersectm3_out_crd_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectm3_out_ref1_sender, intersectm3_out_ref1_receiver) =
            bounded::<Token<u32, u32>>(chan_size);
        let (intersectm3_out_ref2_sender, intersectm3_out_ref2_receiver) =
            bounded::<Token<u32, u32>>(chan_size);

        let intersectm3_data = CrdJoinerData::<u32, u32> {
            in_crd1: bc1_km_out_crd_receiver,
            in_ref1: bc1_km_out_ref_receiver,
            in_crd2: bc1_intersectm_out_crd_receiver,
            in_ref2: intersectm_out_ref2_receiver,
            out_crd: intersectm3_out_crd_sender,
            out_ref1: intersectm3_out_ref1_sender,
            out_ref2: intersectm3_out_ref2_sender,
        };
        let mut intersect_m3 = Intersect::new(intersectm3_data);

        // arrayvals_q
        let (q_out_val_sender, q_out_val_receiver) = bounded::<Token<f32, u32>>(chan_size);
        let arrayvals_q_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref2_receiver,
            out_val: q_out_val_sender,
        };
        let mut arrayvals_q = Array::<u32, f32, u32>::new(arrayvals_q_data, q_vals);

        // arrayvals_k
        let (k_out_val_sender, k_out_val_receiver) = bounded::<Token<f32, u32>>(chan_size);
        let arrayvals_k_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectm3_out_ref1_receiver,
            out_val: k_out_val_sender,
        };
        let mut arrayvals_k = Array::<u32, f32, u32>::new(arrayvals_k_data, k_vals);

        // arrayvals_v
        let (v_out_val_sender, v_out_val_receiver) = bounded::<Token<f32, u32>>(chan_size);
        let arrayvals_v_data = ArrayData::<u32, f32, u32> {
            in_ref: intersectm2_out_ref2_receiver,
            out_val: v_out_val_sender,
        };
        let mut arrayvals_v = Array::<u32, f32, u32>::new(arrayvals_v_data, v_vals);

        // mul ALU
        let (mul_out_sender, mul_out_receiver) = bounded::<Token<f32, u32>>(chan_size);
        let mut mul = make_alu(
            q_out_val_receiver,
            k_out_val_receiver,
            mul_out_sender,
            ALUMulOp(),
        );

        // fiberwrite_Xvals
        let out_vals: Vec<f32> = Vec::new();
        let xvals_data = WrScanData::<f32, u32> {
            input: mul_out_receiver,
        };
        let mut xvals = ValsWrScan::<f32, u32>::new(xvals_data, out_vals);

        // fiberwrite_X0
        // let x0_seg: Vec<u32> = Vec::new();
        // let x0_crd: Vec<u32> = Vec::new();
        // let x0_wrscanner_data = WrScanData::<u32, u32> {
        //     input: bi_out_crd_receiver,
        // };
        // let mut x0_wrscanner = CompressedWrScan::new(x0_wrscanner_data, x0_seg, x0_crd);

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
        parent.add_child(&mut broadcast3);
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
        parent.add_child(&mut intersect_i);
        parent.add_child(&mut intersect_i2);
        parent.add_child(&mut intersect_i3);
        parent.add_child(&mut intersect_j);
        parent.add_child(&mut intersect_j2);
        parent.add_child(&mut intersect_j3);
        parent.add_child(&mut vj_rdscanner);
        parent.add_child(&mut kj_rdscanner);
        parent.add_child(&mut qj_rdscanner);
        parent.add_child(&mut qk_rdscanner);
        parent.add_child(&mut repsig_k);
        parent.add_child(&mut vk_repeat);
        parent.add_child(&mut kk_repeat);
        parent.add_child(&mut kl_rdscanner);
        parent.add_child(&mut vl_rdscanner);
        parent.add_child(&mut intersect_l);
        parent.add_child(&mut km_rdscanner);
        parent.add_child(&mut vm_rdscanner);
        parent.add_child(&mut repsig_l);
        parent.add_child(&mut ql_repeat);
        parent.add_child(&mut qm_rdscanner);
        parent.add_child(&mut intersect_m);
        parent.add_child(&mut intersect_m2);
        parent.add_child(&mut intersect_m3);
        parent.add_child(&mut arrayvals_q);
        parent.add_child(&mut arrayvals_k);
        parent.add_child(&mut arrayvals_v);
        parent.add_child(&mut mul);
        parent.add_child(&mut xvals);

        parent.init();
        parent.run();
        parent.cleanup();
        // let fil = formatted_dir.to_str().unwrap();
        dbg!(xvals.out_val);
    }

    #[test]
    fn get_path() {
        let filename = "/home/rubensl/sam_config.toml";
        let contents = fs::read_to_string(filename).unwrap();
        let data: Data = toml::from_str(&contents).unwrap();

        dbg!(data);
    }
}
