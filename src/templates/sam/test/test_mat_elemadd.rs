#[cfg(test)]
mod tests {

    use std::time::Instant;
    use std::{fs, path::Path};

    use crate::context::generator_context::GeneratorContext;

    use crate::simulation::Program;
    use crate::templates::ops::ALUAddOp;
    use crate::templates::sam::alu::make_alu;
    use crate::templates::sam::array::{Array, ArrayData};
    use crate::templates::sam::joiner::{CrdJoinerData, Union};
    use crate::templates::sam::primitive::Token;
    use crate::templates::sam::rd_scanner::{CompressedCrdRdScan, RdScanData};
    use crate::templates::sam::test::config::Data;
    use crate::templates::sam::utils::read_inputs;
    use crate::templates::sam::wr_scanner::{CompressedWrScan, ValsWrScan};
    use crate::token_vec;

    #[test]
    fn test_mat_elemadd() {
        let test_name = "mat_elemadd2";
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

        let chan_size = 4096;

        let mut parent = Program::default();

        // fiberlookup_bi
        let (bi_out_ref_sender, bi_out_ref_receiver) = parent.bounded(chan_size);
        let (bi_out_crd_sender, bi_out_crd_receiver) = parent.bounded(chan_size);
        let (bi_in_ref_sender, bi_in_ref_receiver) = parent.bounded(chan_size);
        let bi_data = RdScanData::<u32, u32> {
            in_ref: bi_in_ref_receiver,
            out_ref: bi_out_ref_sender,
            out_crd: bi_out_crd_sender,
        };

        let b_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            bi_in_ref_sender,
        );
        let bi_rdscanner = CompressedCrdRdScan::new(bi_data, b0_seg, b0_crd);

        // fiberlookup_ci
        let (ci_out_crd_sender, ci_out_crd_receiver) = parent.bounded(chan_size);
        let (ci_out_ref_sender, ci_out_ref_receiver) = parent.bounded(chan_size);
        let (ci_in_ref_sender, ci_in_ref_receiver) = parent.bounded(chan_size);
        let ci_data = RdScanData::<u32, u32> {
            in_ref: ci_in_ref_receiver,
            out_ref: ci_out_ref_sender,
            out_crd: ci_out_crd_sender,
        };
        let c_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            ci_in_ref_sender,
        );
        let ci_rdscanner = CompressedCrdRdScan::new(ci_data, c0_seg, c0_crd);

        // union_i
        let (unioni_out_crd_sender, unioni_out_crd_receiver) = parent.bounded(chan_size);
        let (unioni_out_ref1_sender, unioni_out_ref1_receiver) = parent.bounded(chan_size);
        let (unioni_out_ref2_sender, unioni_out_ref2_receiver) = parent.bounded(chan_size);
        let unioni_data = CrdJoinerData::<u32, u32> {
            in_crd1: bi_out_crd_receiver,
            in_ref1: bi_out_ref_receiver,
            in_crd2: ci_out_crd_receiver,
            in_ref2: ci_out_ref_receiver,
            out_crd: unioni_out_crd_sender,
            out_ref1: unioni_out_ref1_sender,
            out_ref2: unioni_out_ref2_sender,
        };
        let union_i = Union::new(unioni_data);

        // fiberwrite_X0
        let x0_wrscanner = CompressedWrScan::new(unioni_out_crd_receiver);

        // fiberlookup_bj
        let (bj_out_crd_sender, bj_out_crd_receiver) = parent.bounded(chan_size);
        let (bj_out_ref_sender, bj_out_ref_receiver) = parent.bounded(chan_size);
        let bj_data = RdScanData::<u32, u32> {
            in_ref: unioni_out_ref1_receiver,
            out_ref: bj_out_ref_sender,
            out_crd: bj_out_crd_sender,
        };
        let bj_rdscanner = CompressedCrdRdScan::new(bj_data, b1_seg, b1_crd);

        // fiberlookup_cj
        let (cj_out_crd_sender, cj_out_crd_receiver) = parent.bounded(chan_size);
        let (cj_out_ref_sender, cj_out_ref_receiver) = parent.bounded(chan_size);
        let cj_data = RdScanData::<u32, u32> {
            in_ref: unioni_out_ref2_receiver,
            out_ref: cj_out_ref_sender,
            out_crd: cj_out_crd_sender,
        };
        let cj_rdscanner = CompressedCrdRdScan::new(cj_data, c1_seg, c1_crd);

        // union_j
        let (unionj_out_crd_sender, unionj_out_crd_receiver) = parent.bounded(chan_size);
        let (unionj_out_ref1_sender, unionj_out_ref1_receiver) = parent.bounded(chan_size);
        let (unionj_out_ref2_sender, unionj_out_ref2_receiver) = parent.bounded(chan_size);
        let unionj_data = CrdJoinerData::<u32, u32> {
            in_crd1: bj_out_crd_receiver,
            in_ref1: bj_out_ref_receiver,
            in_crd2: cj_out_crd_receiver,
            in_ref2: cj_out_ref_receiver,
            out_crd: unionj_out_crd_sender,
            out_ref1: unionj_out_ref1_sender,
            out_ref2: unionj_out_ref2_sender,
        };
        let union_j = Union::new(unionj_data);

        // fiberwrite_x1
        let x1_wrscanner = CompressedWrScan::new(unionj_out_crd_receiver);

        // arrayvals_b
        let (b_out_val_sender, b_out_val_receiver) = parent.bounded::<Token<f32, u32>>(chan_size);
        let arrayvals_b_data = ArrayData::<u32, f32, u32> {
            in_ref: unionj_out_ref1_receiver,
            out_val: b_out_val_sender,
        };
        let arrayvals_b = Array::<u32, f32, u32>::new(arrayvals_b_data, b_vals);

        // arrayvals_c
        let (c_out_val_sender, c_out_val_receiver) = parent.bounded::<Token<f32, u32>>(chan_size);
        let arrayvals_c_data = ArrayData::<u32, f32, u32> {
            in_ref: unionj_out_ref2_receiver,
            out_val: c_out_val_sender,
        };
        let arrayvals_c = Array::<u32, f32, u32>::new(arrayvals_c_data, c_vals);

        // Add ALU
        let (add_out_sender, add_out_receiver) = parent.bounded::<Token<f32, u32>>(chan_size);
        let add = make_alu(
            b_out_val_receiver,
            c_out_val_receiver,
            add_out_sender,
            ALUAddOp(),
        );

        // fiberwrite_Xvals
        let xvals = ValsWrScan::<f32, u32>::new(add_out_receiver);

        parent.add_child(b_gen);
        parent.add_child(c_gen);
        parent.add_child(bi_rdscanner);
        parent.add_child(bj_rdscanner);
        parent.add_child(union_i);
        parent.add_child(x0_wrscanner);
        parent.add_child(ci_rdscanner);
        parent.add_child(cj_rdscanner);
        parent.add_child(union_j);
        parent.add_child(x1_wrscanner);
        parent.add_child(arrayvals_b);
        parent.add_child(arrayvals_c);
        parent.add_child(add);
        parent.add_child(xvals);

        let now = Instant::now();
        parent.print_graph();
        parent.init();
        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);
        parent.run();

        // dbg!(x0_wrscanner.crd_arr);
        // dbg!(xvals.out_val);

        // let fil = formatted_dir.to_str().unwrap();
    }

    // #[test]
    // fn get_path() {
    //     let filename = "/home/rubensl/sam_config.toml";
    //     let contents = fs::read_to_string(filename).unwrap();
    //     let data: Data = toml::from_str(&contents).unwrap();

    //     dbg!(data);
    // }
}
