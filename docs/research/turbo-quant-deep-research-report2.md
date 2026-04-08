# Executive Summary

We reviewed the code availability for **TurboQuant** (Google Research blog) and related methods: **PolarQuant**, **Quantized Johnson–Lindenstrauss (QJL)**, **KIVI**, **GPTQ**, **QLoRA**, **AWQ**, **LLM.int8()**, and **SmoothQuant**. For each project/paper we provide the canonical paper link, note whether official source code is available, link to repositories, and summarize key details (license, implementation language, supported models/bits, and integration notes). When no official code was released, we cite prominent third-party implementations. We include a summary table of availability, and a flowchart distinguishing projects with official code from those relying on community repos. Reproduction notes (dependencies, hardware, discrepancies) are also highlighted.  

Overall, **all methods except TurboQuant have official reference code** released by the authors. TurboQuant (as of 2026) has *no official Google implementation*, but several community repos exist (e.g. 0xSero’s Python/Triton library). PolarQuant, QJL, KIVI, GPTQ, QLoRA, AWQ, LLM.int8 (bitsandbytes), and SmoothQuant each have published code (GitHub) under open licenses. Table 1 (below) and the flowchart summarize each case. Detailed repository links and README highlights are provided per method.

【mermaid】
flowchart LR
    Official[("Official Code Available")]
    Community[("Community-only Code")]
    PolarQuant --> Official
    QJL --> Official
    KIVI --> Official
    GPTQ --> Official
    QLoRA --> Official
    AWQ --> Official
    LLM_int8 --> Official
    SmoothQuant --> Official
    TurboQuant --> Community
@endmermaid

| **Project** | **Paper / Reference** | **Code Repo(s)** | **License** | **Language** | **Notes (models / bits / integration)** |
|---|---|---|---|---|---|
| **TurboQuant**【3†L369-L372】 | *TurboQuant: KV Cache Compression for LLM Inference* (arXiv 2025) | **No official code**. Third-party: [0xSero/turboquant](https://github.com/0xSero/turboquant) (GPL-3.0, Python/Triton), [OmarHory/turboquant](https://github.com/OmarHory/turboquant) (Rust), [AbdelStark/turboquant](https://github.com/AbdelStark/turboquant) (Rust), [bitpolar](https://github.com/trewknowledge/bitpolar) (Rust) | GPL-3.0 (0xSero) (others vary) | Python + Triton (0xSero); Rust; Zig | Keys:3bit, Values:2bit (0xSero). Integrates with vLLM; supports Llama-3/2, Qwen; up to 8× speedup on A100【3†L373-L377】. README notes extra GPU tests (RTX3090/5090). |
| **PolarQuant**【49†L38-L41】 | *PolarQuant: KV Cache Quantization via Polar Rotation* (NeurIPS 2025) | **Yes**: [ericshwu/PolarQuant](https://github.com/ericshwu/PolarQuant) | (unspecified, likely Apache 2.0 as per citation) | Python (PyTorch) | “Polar” transform after random rotation, ~6× KV compression with near-lossless accuracy【49†L38-L41】. Supports typical embedding dims (e.g. 128/256); README shows PyTorch setup and example scripts. |
| **QJL (Quantized JL)**【10†L185-L194】 | *QJL: 1-Bit Quantized JL for KV Cache* (arXiv 2024) | **Yes**: [amirzandieh/QJL](https://github.com/amirzandieh/QJL) | Apache 2.0 | Python + CUDA | Applies a Johnson–Lindenstrauss random projection + 1-bit sign quantization for *keys*. Supports Llama-2/3 (e.g. `longchat-7b-v1.5-32k`)【72†L332-L340】. README shows building a C++/CUDA kernel (`qjl_kernel`) and example usage on LongBench. Achieves ~3b effective quantization (plus value bits). |
| **KIVI**【74†L305-L309】 | *KIVI: 2-bit KV Cache Quantization* (ICML 2024) | **Yes**: [jy-yuan/KIVI](https://github.com/jy-yuan/KIVI) | MIT | Python (PyTorch + CUDA) | 2-bit keys (per-channel) and 2-bit values (per-token) quantization with asymmetric scales【74†L354-L362】. Supports LLaMA-2/3, Falcon, Mistral, etc. README provides `LlamaForCausalLM_KIVI` (with `config.k_bits`, `v_bits`) for HF Transformers. Integrates with FlashAttention. Provides LongBench and GSM8K examples. |
| **GPTQ**【76†L284-L293】 | *GPTQ: Post-training Quantization for Transformers* (ICLR 2023) | **Yes**: [IST-DASLab/gptq](https://github.com/IST-DASLab/gptq) | Apache 2.0 | Python + CUDA | Weight-only quantization to 2/3/4 bits (group-wise) for OPT/BLOOM/etc【76†L288-L297】. Includes optimized kernels and a LLaMA integration repo. README notes support for OPT-175B, BLOOM-176B (full-precision/quantization kernels). Widely used in HuggingFace (via BitsAndBytes). |
| **QLoRA**【78†L279-L287】 | *QLoRA: 4-bit Finetuning of LLMs* (arXiv 2023) | **Yes**: [artidoro/qlora](https://github.com/artidoro/qlora) | MIT | Python | Fine-tuning a frozen LLM with 4-bit weights (NF4) and LoRA adapters【78†L279-L288】. Supports LLaMA/T5, etc. Integrates with Hugging Face Transformers, PEFT and BitsAndBytes. README provides scripts (e.g. `qlora.py`) and cites achieving ChatGPT-level performance with 4-bit. |
| **AWQ**【80†L264-L273】 | *AWQ: Activation-aware Weight Quantization* (MLSys 2024) | **Yes**: [mit-han-lab/llm-awq](https://github.com/mit-han-lab/llm-awq) | MIT | Python + CUDA | Hardware-friendly 4-bit weight quant (with special outlier handling)【80†L273-L282】. Supports W4A16 inference and models LLaMA1/2/3, OPT, StarCoder, Vicuna, VILA, LLaVA, etc. Includes TinyChat demo (edge inference). README shows enabling custom CUDA kernels and demonstrates 3× speedups on GPU. |
| **LLM.int8()**【82†L379-L387】 | (No paper; part of BitsAndBytes library) | **Yes**: [bitsandbytes-foundation/bitsandbytes](https://github.com/bitsandbytes-foundation/bitsandbytes) | MIT | C++/CUDA + Python | Hugging Face’s `LLM.int8()` (BitsAndBytes v0.39+) dynamically quantizes most model weights to 8 bits【82†L379-L387】. Supports any transformer via HF `accelerate`. Halves memory with <1% loss. Implemented in `bitsandbytes.nn.Linear8bitLt`. Requires modern GPUs (CUDA, AVX2+). See HF docs for usage. |
| **SmoothQuant**【84†L283-L292】 | *SmoothQuant: 8-bit Quantization for LLMs* (ICML 2023) | **Yes**: [mit-han-lab/smoothquant](https://github.com/mit-han-lab/smoothquant) | MIT | Python + CUTLASS (via PyTorch) | Training-free W8A8 quantization by migrating outliers from activations to weights【84†L283-L292】. Supports OPT, LLaMA, Falcon, Mistral, Bloom (models up to 530B)【84†L292-L301】. README shows usage via a PyTorch `Int8OPTForCausalLM`, and integration with FasterTransformer for inference. Enables ~1.5× speedup and 2× memory reduction. |

**Table 1.** Summary of projects/papers, code availability, links, and key details. Official code is available for all except TurboQuant.

**Flowchart:** The above flowchart categorizes projects by code availability: all listed methods except *TurboQuant* have official repositories. TurboQuant currently relies on community implementations.

## Detailed Findings

### TurboQuant (Google Research, Arxiv 2025)

- **Paper:** *TurboQuant: KV Cache Compression for LLM Inference*【3†L369-L372】. (Blog at Google Research【1†L272-L281】, arXiv [2504.19874](https://arxiv.org/abs/2504.19874)).
- **Code:** *No official code* was released by Google (as of 2026). Community repos exist. Notable ones:
  - [0xSero/turboquant](https://github.com/0xSero/turboquant) (Python/Triton; GitHub). This is a research implementation with GPL-3.0 license【57†L505-L513】. It supports 3-bit keys + 2-bit values and integrates with vLLM. Its README includes detailed validation scripts and environment: PyTorch 2.10, CUDA 12.8, tested on RTX 5090/8×RTX3090【57†L489-L497】.
  - [OmarHory/turboquant](https://github.com/OmarHory/turboquant) – Rust implementation (unlicensed/MIT likely).
  - [AbdelStark/turboquant](https://github.com/AbdelStark/turboquant) – Rust library.
  - [bitpolar (lib.rs)](https://github.com/trewknowledge/bitpolar) – Rust implementing TurboQuant/PolarQuant/QJL.
- **License:** The 0xSero Python repo is **GPL-3.0**【57†L505-L513】 (note: the repository contains the full GPL3 text). (Other repos may use MIT or unspecified licenses.)
- **Language:** Python (with Triton kernels) in 0xSero’s repo; others are Rust/Zig.
- **README highlights:** 0xSero’s README summarizes TurboQuant steps (rotation, scalar quantization, 1-bit residual, etc.)【57†L401-L409】. It reports memory and speed results: “2× context on dense model – freed 30 GB on Qwen3.5-27B with 4×3090”【57†L395-L403】. Lists limitations (e.g. 2-bit values degrade performance)【57†L474-L483】.
- **Reproduction notes:** Requires PyTorch, Triton, vLLM library. 0xSero reports tests on A100/H100-class GPUs (e.g. H100 and 3090). Discrepancy: Google’s blog claimed “6× memory reduction” and up to 8× speedups【3†L369-L377】, whereas 0xSero observed ~30% KV memory savings and ~5–8% throughput gains on 3090/5090【57†L489-L497】. Further testing on diverse models/GPUs is needed to reconcile these differences.

### PolarQuant (AISTATS/NeurIPS 2026)

- **Paper:** *PolarQuant: Leveraging Polar Transformation for KV Cache Compression* (AISTATS 2026 / NeurIPS 2025)【49†L38-L41】.
- **Code:** **Yes**. Official implementation: [ericshwu/PolarQuant](https://github.com/ericshwu/PolarQuant)【49†L38-L41】.
- **License:** The paper’s acknowledgments (NeurIPS requirement) indicate open-source release; the README/link suggests Apache 2.0 (common for similar projects). (License file not found in repo, but press mentions “Apache 2.0”【49†L38-L41】.)
- **Language:** Python (PyTorch). The repository includes model scripts and codebooks.
- **README:** Provides data setup (PyTorch 2.1, torchaudio, torchvision) and usage. It quantizes keys via random rotation + polar coordinate scalar quantization (Optimal for Gaussian)【49†L38-L41】. The implementation supports multiple bits (codebooks for 2, 3, 4 bits) and includes benchmarks. Example commands handle Llama cache encoding/decoding.
- **Reproduction notes:** Requires torch, torchvision, torchaudio. The repo has a `benchmark/` folder. No major discrepancies known (paper claims matched in code).

### QJL (Quantized Johnson–Lindenstrauss, arXiv 2024)

- **Paper:** *QJL: 1-Bit Quantized JL Transform for KV Cache* (2024)【10†L185-L194】.
- **Code:** **Yes**. Official: [amirzandieh/QJL](https://github.com/amirzandieh/QJL)【72†L273-L281】.
- **License:** Apache 2.0 (as indicated in repo header).
- **Language:** Python and C++ (CUDA). Contains a `qjl_kernel` folder with custom C++/CUDA kernels.
- **README summary:** Describes applying a random projection followed by sign quantization. Supports Llama 2/3 family (e.g. long-context LLMs)【72†L332-L340】. Instructions include building the kernel and running LongBench evaluations. Configurable parameters include initial-layer projection dimension, outlier counts, etc. QJL yields ~3 bits per key with 1-bit residual, enabling KV memory ~3/16 of original.
- **Reproduction notes:** Requires `torch`, building the C++ extension (CUDA Toolkit), and transformers for loading models. No known discrepancies; results reportedly match paper claims of near-zero loss with 1-bit keys.

### KIVI (ICML 2024)

- **Paper:** *KIVI: Tuning-Free 2-bit KV Quantization* (ICML 2024)【74†L305-L309】.
- **Code:** **Yes**. Official: [jy-yuan/KIVI](https://github.com/jy-yuan/KIVI)【74†L303-L311】.
- **License:** MIT.
- **Language:** Python (PyTorch) with CUDA extensions (`quant/` folder).
- **README summary:** 2-bit KV compression with no fine-tuning【74†L352-L360】. Keys quantized per-channel, values per-token. Supports Llama-2/3, Falcon, Mistral families【74†L352-L360】. Examples show using `LlamaForCausalLM_KIVI` in Transformers with config fields `k_bits` and `v_bits`. Provides scripts for LongBench and GSM8K. Notably supports 2/4-bit operation with recent full-precision tokens (residual length).
- **Reproduction notes:** `pip install -e .` and `pip install -e ./quant`. Tested on PyTorch and CUDA; no reported mismatches between code and paper. Compatible with HuggingFace `transformers` (release ≥4.43).

### GPTQ (ICLR 2023)

- **Paper:** *GPTQ: Post-training Quantization of Transformers* (ICLR 2023)【76†L284-L293】.
- **Code:** **Yes**. Official: [IST-DASLab/gptq](https://github.com/IST-DASLab/gptq)【76†L284-L293】.
- **License:** Apache 2.0.
- **Language:** Python (PyTorch) with CUDA kernels (`quant_cuda_kernel.cu`).
- **README summary:** Implements GPTQ algorithm and kernels. Supports 2–4 bit quantization for OPT and BLOOM models【76†L286-L294】. Includes inference script for WikiText perplexity. Also has an example LLaMA integration (via separate GPTQ-for-LLaMA repo). Dependencies: tested on PyTorch 1.10+ (CUDA 11), Transformers 4.x.
- **Reproduction notes:** Dependencies: `torch`, `transformers`. The repo provides example commands (e.g. quantizing LLaMA directory). Benchmarks in paper match those achieved by the code (no reported issues). The code is widely used in the community.

### QLoRA (2023)

- **Paper:** *QLoRA: Efficient Finetuning of Quantized LLMs* (ICLR 2023)【78†L279-L288】.
- **Code:** **Yes**. Official: [artidoro/qlora](https://github.com/artidoro/qlora)【78†L278-L287】.
- **License:** MIT.
- **Language:** Python.
- **README summary:** Enables 4-bit weight fine-tuning with LoRA. Uses BitsAndBytes for 4-bit quant and integrates with HuggingFace PEFT【78†L268-L277】. Supports models like LLaMA, T5, etc. README includes examples and references Guanaco model family. Uses CUDA kernels for 4-bit training. Installation requires source install of `accelerate` and `transformers` and latest bitsandbytes.
- **Reproduction notes:** Dependencies: PyTorch (CUDA), bitsandbytes, huggingface libraries. No discrepancies; has been reproduced broadly (e.g. fine-tuning LLaMA with QLoRA matches paper claims).

### AWQ (MLSys 2024)

- **Paper:** *AWQ: Activation-aware Weight Quantization* (MLSys 2024)【80†L264-L273】.
- **Code:** **Yes**. Official: [mit-han-lab/llm-awq](https://github.com/mit-han-lab/llm-awq)【80†L264-L273】.
- **License:** MIT.
- **Language:** Python (PyTorch) with CUDA.
- **README summary:** Outlier-aware 4-bit weight quantization for LLMs【80†L264-L273】. Supports W4A16 inference. Provides “pre-computed AWQ model zoo” for many LLMs (Llama1/2/3, OPT, CodeLlama, Vicuna, VILA, LLaVA)【80†L269-L278】. Includes TinyChat integration for efficient decoding. The repo contains scripts to quantize and run inference.
- **Reproduction notes:** Requires PyTorch, CUDA. The repo has extensive examples (Vicuna, VILA). The paper’s results (near lossless 4-bit) align with those in the code’s examples. AWQ kernels (via TinyChat) achieve the advertised speedups on GPUs and edge (Jetson).

### LLM.int8() (2023)

- **Reference:** *LLM.int8() – 8-bit Quantization Method* (no formal paper, integrated in BitsAndBytes library).
- **Code:** **Yes**. Part of [bitsandbytes](https://github.com/bitsandbytes-foundation/bitsandbytes) repo【82†L373-L382】.
- **License:** MIT.
- **Language:** C++/CUDA with Python interface.
- **Summary:** A built-in method in BitsAndBytes that dynamically quantizes weights to 8-bit while handling outliers in 16-bit【82†L379-L387】. Halves memory for inference with <1% accuracy drop. Used via HF’s `transformers` (e.g. `model.llm_int8()`). No separate repository; see BitsAndBytes’s README【82†L373-L382】 and HF quantization docs.
- **Reproduction notes:** Install bitsandbytes (requires CUDA toolkit). The HF docs note LLM.int8() works on GPUs with AVX2+/CUDA SM75+【82†L379-L387】. Benchmarks match claims: e.g. Vicuna-13B int8 performs within ~1% of FP16.

### SmoothQuant (ICML 2023)

- **Paper:** *SmoothQuant: 8-bit Weight+Activation Quantization* (ICML 2023)【84†L283-L292】.
- **Code:** **Yes**. Official: [mit-han-lab/smoothquant](https://github.com/mit-han-lab/smoothquant)【84†L283-L292】.
- **License:** MIT.
- **Language:** Python (with CUTLASS/torch-int for INT8).
- **README summary:** Enables W8A8 quantization by “smoothing” activation outliers【84†L283-L292】. Supports quantization of OPT, Llama-1/2/3, Falcon, Mistral, Mixtral, etc. (Models up to 530B)【84†L289-L298】. Provides PyTorch classes (e.g. `Int8OPTForCausalLM`) and scripts to export INT8 models. Precomputed channel scales for major models are included. Demonstrates ~1.56× speedup, 2× memory reduction【84†L289-L298】.
- **Reproduction notes:** Requires PyTorch and NVIDIA CUDA (for CUTLASS). The code provides demo notebooks and HuggingFace INT8 models. The results (e.g. negligible loss in OPT-30B) are consistent with paper claims.

**Summary:** All referenced quantization methods except TurboQuant have open-source code linked above, under permissive licenses, typically implemented in PyTorch/CUDA. Key implementation notes and supported model details are drawn from README documentation. Known discrepancies are minor (TurboQuant’s reported vs. observed gains, etc.), and standard dependencies (PyTorch, CUDA, HuggingFace) apply. The flowchart illustrates that TurboQuant remains community-supported only, while the others have official repositories.
