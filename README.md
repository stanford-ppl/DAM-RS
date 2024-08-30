# DAM: The Dataflow Abstract Machine Simulator Framework

## Contents:
1. Using DAM
2. What is DAM?
3. What can I simulate with DAM?
4. Contributing
5. Publications


## Using DAM

DAM is a Rust package, currently GitHub-only (awaiting a [PR](https://github.com/Xudong-Huang/may/pull/108) merge into one of the dependencies.).

To use DAM, add the following line into your `Cargo.toml` under the `[dependencies]` section:

```toml
dam = {git = "https://github.com/stanford-ppl/DAM-RS.git"}
```

To build the documentation for DAM, run the following command inside of this repository:

```
cargo +nightly doc
```

## What is DAM

DAM is a framework for building high-performance parallel simulators for dataflow-like systems.
DAM comprises of two main components: contexts and channels.
Contexts represent the "nodes" in a computational graph, while channels encapsulate the communication between nodes (the "edges").

`Context` is a trait, which only requires two components:
1. `Context::run_falliable(&mut self) -> anyhow::Result<()>`, a function which encapsulates the entirety of the execution. This enables the use of near-arbitrary code within.
2. A constructor of any flavor, since `#[context_macro]` introduces a `ContextInfo` field used internally within DAM. This is intentional, as will be discussed later.

Communication channels are represented using `Sender`/`Receiver` pairs (or just a single `Sender` when using void channels).
In order to keep track of time, users must call `Sender::attach_sender(&dyn Context)` and `Receiver::attach_receiver(&dyn Context)` during the initialization phase.
We find that the constructor of a Context is the most logical place to insert these calls.

## What can I simulate using DAM?
DAM can simulate *anything* which can be described as *things connected by channels*, provided that the channels have non-zero latency.
Examples include:

- Hardware:
    - Modeling the [Sambanova SN40L](https://sambanova.ai/technology/sn40l-rdu-ai-chip)
- Programming Models:
    - [Streaming Tensor Patterns](https://ppl.stanford.edu/papers/YARCH24_STEP.pdf)
    - [Sparse Abstract Machine](https://arxiv.org/pdf/2208.14610)

## Contributing
If you are interested in contributing to DAM, feel free to shoot Nathan Zhang an email at `stanfurd@stanford.edu`.

## Publications

### The Dataflow Abstract Machine Simulator Framework
[ISCA'24](https://ieeexplore.ieee.org/document/10609587)

[Online PDF](https://ppl.stanford.edu/papers/DAM_ISCA24.pdf)
```bibtex
@inproceedings{dam,
  author={Zhang, Nathan and Lacouture, Rubens and Sohn, Gina and Mure, Paul and Zhang, Qizheng and Kjolstad, Fredrik and Olukotun, Kunle},
  booktitle={2024 ACM/IEEE 51st Annual International Symposium on Computer Architecture (ISCA)}, 
  title={The Dataflow Abstract Machine Simulator Framework}, 
  year={2024},
  volume={},
  number={},
  pages={532-547},
  keywords={Tensors;Machine learning algorithms;Dams;Large language models;Memory management;Machine learning;Parallel processing;Parallel Discrete Event Simulation;Dataflow Accelerators;Modeling},
  doi={10.1109/ISCA59077.2024.00046}}
```

### Papers Using DAM

#### Implementing and Optimizing the Scaled Dot-Product Attention on Streaming Dataflow
[ArXiv](https://arxiv.org/abs/2404.16629)

<details>
<summary>Citation</summary>

```bibtex
@misc{sohn2024implementingoptimizingscaleddotproduct,
      title={Implementing and Optimizing the Scaled Dot-Product Attention on Streaming Dataflow}, 
      author={Gina Sohn and Nathan Zhang and Kunle Olukotun},
      year={2024},
      eprint={2404.16629},
      archivePrefix={arXiv},
      primaryClass={cs.AR},
      url={https://arxiv.org/abs/2404.16629}, 
}
```
</details>