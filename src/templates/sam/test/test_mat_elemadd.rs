use super::config;

#[cfg(test)]
mod tests {
    use std::fmt::format;
    use std::{fs, path::Path};

    use crate::channel::unbounded;
    use crate::context::generator_context::GeneratorContext;
    use crate::context::parent::BasicParentContext;
    use crate::context::{Context, ParentContext};
    use crate::templates::sam::array::{Array, ArrayData};
    use crate::templates::sam::joiner::{CrdJoinerData, Union};
    use crate::templates::sam::primitive::Token;
    use crate::templates::sam::rd_scanner::{CompressedCrdRdScan, RdScanData};
    use crate::templates::sam::test::config::{Config, Data};
    use crate::templates::sam::utils::read_inputs;
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
        let (ref_sender, ref_receiver) = unbounded::<Token<u32, u32>>();
        let (crd_sender, crd_receiver) = unbounded::<Token<u32, u32>>();
        let (in_ref_sender, in_ref_receiver) = unbounded::<Token<u32, u32>>();
        let data = RdScanData::<u32, u32> {
            in_ref: in_ref_receiver,
            out_ref: ref_sender,
            out_crd: crd_sender,
        };

        let mut gen1 =
            GeneratorContext::new(|| token_vec!(u32; u32; 0, "D").into_iter(), in_ref_sender);
        let mut fiberlookup_bi = CompressedCrdRdScan::new(data, b0_seg, b0_crd);
        let (crd_sender1, crd_receiver1) = unbounded::<Token<u32, u32>>();
        let (ref_sender1, ref_receiver1) = unbounded::<Token<u32, u32>>();
        let data1 = RdScanData::<u32, u32> {
            in_ref: ref_receiver,
            out_ref: ref_sender1,
            out_crd: crd_sender1,
        };
        let mut fiberlookup_bj = CompressedCrdRdScan::new(data1, b1_seg, b1_crd);
        let (crd_sender2, crd_receiver2) = unbounded::<Token<u32, u32>>();
        let (ref_sender2, ref_receiver2) = unbounded::<Token<u32, u32>>();
        let (in_ref_sender2, in_ref_receiver2) = unbounded::<Token<u32, u32>>();
        let mut gen2 =
            GeneratorContext::new(|| token_vec!(u32; u32; 0, "D").into_iter(), in_ref_sender2);
        let data2 = RdScanData::<u32, u32> {
            in_ref: in_ref_receiver2,
            out_ref: ref_sender2,
            out_crd: crd_sender2,
        };
        let mut fiberlookup_ci = CompressedCrdRdScan::new(data2, c0_seg, c0_crd);
        let (crd_sender3, crd_receiver3) = unbounded::<Token<u32, u32>>();
        let (ref_sender3, ref_receiver3) = unbounded::<Token<u32, u32>>();
        let data3 = RdScanData::<u32, u32> {
            in_ref: ref_receiver2,
            out_ref: ref_sender3,
            out_crd: crd_sender3,
        };
        let mut fiberlookup_cj = CompressedCrdRdScan::new(data3, c1_seg, c1_crd);

        let (out_crd_sender, out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (out_ref1_sender, out_ref1_receiver) = unbounded::<Token<u32, u32>>();
        let (out_ref2_sender, out_ref2_receiver) = unbounded::<Token<u32, u32>>();
        let union_data = CrdJoinerData::<u32, u32> {
            in_crd1: crd_receiver1,
            in_ref1: ref_receiver1,
            in_crd2: crd_receiver3,
            in_ref2: ref_receiver3,
            out_crd: out_crd_sender,
            out_ref1: out_ref1_sender,
            out_ref2: out_ref2_sender,
        };
        let mut union_j = Union::new(union_data);

        let (out_val_sender, out_val_receiver) = unbounded::<Token<u32, u32>>();
        let (out_val1_sender, out_val1_receiver) = unbounded::<Token<u32, u32>>();

        let arr_data = ArrayData::<u32, u32> {
            in_ref: out_ref1_receiver,
            out_val: out_val_sender,
        };
        let mut arr_b = Array::<f32, u32>::new(arr_data, b_vals);
        let arr1_data = ArrayData::<u32, u32> {
            in_ref: out_ref2_receiver,
            out_val: out_val1_sender,
        };
        let mut arr_c = Array::<f32, u32>::new(arr1_data, c_vals);

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut fiberlookup_bi);
        parent.add_child(&mut fiberlookup_bj);
        parent.add_child(&mut fiberlookup_ci);
        parent.add_child(&mut fiberlookup_cj);
        parent.add_child(&mut union_j);
        parent.add_child(arr_b);
        parent.add_child(arr_c);

        parent.init();
        parent.run();
        parent.cleanup();

        // let fil = formatted_dir.to_str().unwrap();
    }

    #[test]
    fn get_path() {
        let filename = "/home/rubensl/sam_config.toml";
        let contents = fs::read_to_string(filename).unwrap();
        let data: Data = toml::from_str(&contents).unwrap();

        dbg!(data);
    }
}
