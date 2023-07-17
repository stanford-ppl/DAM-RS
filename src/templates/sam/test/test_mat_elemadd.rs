#[cfg(test)]
mod tests {

    use std::{fs, path::Path};

    use crate::channel::bounded;
    use crate::context::generator_context::GeneratorContext;
    use crate::context::parent::BasicParentContext;
    use crate::context::{Context, ParentContext};
    use crate::templates::ops::ALUAddOp;
    use crate::templates::sam::alu::make_alu;
    use crate::templates::sam::array::{Array, ArrayData};
    use crate::templates::sam::joiner::{CrdJoinerData, Union};
    use crate::templates::sam::primitive::Token;
    use crate::templates::sam::rd_scanner::{CompressedCrdRdScan, RdScanData};
    use crate::templates::sam::test::config::Data;
    use crate::templates::sam::utils::read_inputs;
    use crate::templates::sam::wr_scanner::{CompressedWrScan, ValsWrScan, WrScanData};
    use crate::token_vec;

    #[test]
    fn test_mat_elemadd() {
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

        let chan_size = 8;

        // let mk_bounded = || bounded_with_flavor::<Token<u32, u32>>(chan_size, Acyclic);
        let mk_bounded = || bounded::<Token<u32, u32>>(chan_size);

        // fiberlookup_bi
        let (bi_out_ref_sender, bi_out_ref_receiver) = mk_bounded();
        let (bi_out_crd_sender, bi_out_crd_receiver) = mk_bounded();
        let (bi_in_ref_sender, bi_in_ref_receiver) = mk_bounded();
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

        // fiberlookup_ci
        let (ci_out_crd_sender, ci_out_crd_receiver) = mk_bounded();
        let (ci_out_ref_sender, ci_out_ref_receiver) = mk_bounded();
        let (ci_in_ref_sender, ci_in_ref_receiver) = mk_bounded();
        let ci_data = RdScanData::<u32, u32> {
            in_ref: ci_in_ref_receiver,
            out_ref: ci_out_ref_sender,
            out_crd: ci_out_crd_sender,
        };
        let mut c_gen = GeneratorContext::new(
            || token_vec!(u32; u32; 0, "D").into_iter(),
            ci_in_ref_sender,
        );
        let mut ci_rdscanner = CompressedCrdRdScan::new(ci_data, c0_seg, c0_crd);

        // union_i
        let (unioni_out_crd_sender, unioni_out_crd_receiver) = mk_bounded();
        let (unioni_out_ref1_sender, unioni_out_ref1_receiver) = mk_bounded();
        let (unioni_out_ref2_sender, unioni_out_ref2_receiver) = mk_bounded();
        let unioni_data = CrdJoinerData::<u32, u32> {
            in_crd1: bi_out_crd_receiver,
            in_ref1: bi_out_ref_receiver,
            in_crd2: ci_out_crd_receiver,
            in_ref2: ci_out_ref_receiver,
            out_crd: unioni_out_crd_sender,
            out_ref1: unioni_out_ref1_sender,
            out_ref2: unioni_out_ref2_sender,
        };
        let mut union_i = Union::new(unioni_data);

        // fiberwrite_X0
        let x0_seg: Vec<u32> = Vec::new();
        let x0_crd: Vec<u32> = Vec::new();
        let x0_wrscanner_data = WrScanData::<u32, u32> {
            input: unioni_out_crd_receiver,
        };
        let mut x0_wrscanner = CompressedWrScan::new(x0_wrscanner_data, x0_seg, x0_crd);

        // fiberlookup_bj
        let (bj_out_crd_sender, bj_out_crd_receiver) = mk_bounded();
        let (bj_out_ref_sender, bj_out_ref_receiver) = mk_bounded();
        let bj_data = RdScanData::<u32, u32> {
            in_ref: unioni_out_ref1_receiver,
            out_ref: bj_out_ref_sender,
            out_crd: bj_out_crd_sender,
        };
        let mut bj_rdscanner = CompressedCrdRdScan::new(bj_data, b1_seg, b1_crd);

        // fiberlookup_cj
        let (cj_out_crd_sender, cj_out_crd_receiver) = mk_bounded();
        let (cj_out_ref_sender, cj_out_ref_receiver) = mk_bounded();
        let cj_data = RdScanData::<u32, u32> {
            in_ref: unioni_out_ref2_receiver,
            out_ref: cj_out_ref_sender,
            out_crd: cj_out_crd_sender,
        };
        let mut cj_rdscanner = CompressedCrdRdScan::new(cj_data, c1_seg, c1_crd);

        // union_j
        let (unionj_out_crd_sender, unionj_out_crd_receiver) = mk_bounded();
        let (unionj_out_ref1_sender, unionj_out_ref1_receiver) = mk_bounded();
        let (unionj_out_ref2_sender, unionj_out_ref2_receiver) = mk_bounded();
        let unionj_data = CrdJoinerData::<u32, u32> {
            in_crd1: bj_out_crd_receiver,
            in_ref1: bj_out_ref_receiver,
            in_crd2: cj_out_crd_receiver,
            in_ref2: cj_out_ref_receiver,
            out_crd: unionj_out_crd_sender,
            out_ref1: unionj_out_ref1_sender,
            out_ref2: unionj_out_ref2_sender,
        };
        let mut union_j = Union::new(unionj_data);

        // fiberwrite_x1
        let x1_seg: Vec<u32> = Vec::new();
        let x1_crd: Vec<u32> = Vec::new();
        let x1_wrscanner_data = WrScanData::<u32, u32> {
            input: unionj_out_crd_receiver,
        };
        let mut x1_wrscanner = CompressedWrScan::new(x1_wrscanner_data, x1_seg, x1_crd);

        // arrayvals_b
        let (b_out_val_sender, b_out_val_receiver) = bounded::<Token<f32, u32>>(chan_size);
        let arrayvals_b_data = ArrayData::<u32, f32, u32> {
            in_ref: unionj_out_ref1_receiver,
            out_val: b_out_val_sender,
        };
        let mut arrayvals_b = Array::<u32, f32, u32>::new(arrayvals_b_data, b_vals);

        // arrayvals_c
        let (c_out_val_sender, c_out_val_receiver) = bounded::<Token<f32, u32>>(chan_size);
        let arrayvals_c_data = ArrayData::<u32, f32, u32> {
            in_ref: unionj_out_ref2_receiver,
            out_val: c_out_val_sender,
        };
        let mut arrayvals_c = Array::<u32, f32, u32>::new(arrayvals_c_data, c_vals);

        // Add ALU
        let (add_out_sender, add_out_receiver) = bounded::<Token<f32, u32>>(chan_size);
        let mut add = make_alu(
            b_out_val_receiver,
            c_out_val_receiver,
            add_out_sender,
            ALUAddOp(),
        );

        // fiberwrite_Xvals
        let out_vals: Vec<f32> = Vec::new();
        let xvals_data = WrScanData::<f32, u32> {
            input: add_out_receiver,
        };
        let mut xvals = ValsWrScan::<f32, u32>::new(xvals_data, out_vals);

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut b_gen);
        parent.add_child(&mut c_gen);
        parent.add_child(&mut bi_rdscanner);
        parent.add_child(&mut bj_rdscanner);
        parent.add_child(&mut union_i);
        parent.add_child(&mut x0_wrscanner);
        parent.add_child(&mut ci_rdscanner);
        parent.add_child(&mut cj_rdscanner);
        parent.add_child(&mut union_j);
        parent.add_child(&mut x1_wrscanner);
        parent.add_child(&mut arrayvals_b);
        parent.add_child(&mut arrayvals_c);
        parent.add_child(&mut add);
        parent.add_child(&mut xvals);

        parent.init();
        parent.run();
        parent.cleanup();

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
