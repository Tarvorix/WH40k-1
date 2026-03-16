"""
WH40K NNUE Model Definition (PyTorch)

Mirrors the Rust NNUE architecture exactly:
    Sparse Features (1209) -> Accumulator (128) -> Hidden1 (32) -> Hidden2 (32) -> Output (1)
                               [ClippedReLU]       [ClippedReLU]   [ClippedReLU]

Quantization scales (matching Rust eval_nnue):
    Feature weights: i16  (FT_SCALE = 64)
    Hidden weights:  i8   (HIDDEN_SCALE = 64)
    Output weights:  i16  (OUTPUT_SCALE = 400)

Source: implementation_v3.md Phase 8.4, eval_nnue/src/lib.rs
"""

import torch
import torch.nn as nn


# Architecture dimensions (must match Rust NnueDimensions::DEFAULT)
TOTAL_FEATURES = 1209
ACCUMULATOR_SIZE = 128
HIDDEN1_SIZE = 32
HIDDEN2_SIZE = 32
OUTPUT_SIZE = 1

# Quantization scales (must match Rust constants)
FT_SCALE = 64
HIDDEN_SCALE = 64
OUTPUT_SCALE = 400

# ClippedReLU range [0, 127] for quantized inference (7-bit activation)
CLIPPED_RELU_MAX = 127


class ClippedReLU(nn.Module):
    """ClippedReLU activation: clamp(x, 0, 1).

    In quantized inference this maps to clamp(x, 0, 127).
    In float training we use [0, 1] range.
    """

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return torch.clamp(x, 0.0, 1.0)


class NnueModel(nn.Module):
    """NNUE evaluation network matching the Rust quantized architecture.

    Architecture:
        1. Feature transformer: sparse_features -> accumulator (1209 -> 128)
        2. Hidden layer 1: accumulator -> hidden1 (128 -> 32)
        3. Hidden layer 2: hidden1 -> hidden2 (32 -> 32)
        4. Output: hidden2 -> scalar (32 -> 1)

    All layers use ClippedReLU activation.
    Training uses float32; quantization happens at export time.
    """

    def __init__(self):
        super().__init__()

        # Feature transformer (sparse input -> accumulator)
        self.feature_transform = nn.Linear(TOTAL_FEATURES, ACCUMULATOR_SIZE)
        self.ft_activation = ClippedReLU()

        # Hidden layer 1
        self.hidden1 = nn.Linear(ACCUMULATOR_SIZE, HIDDEN1_SIZE)
        self.h1_activation = ClippedReLU()

        # Hidden layer 2
        self.hidden2 = nn.Linear(HIDDEN1_SIZE, HIDDEN2_SIZE)
        self.h2_activation = ClippedReLU()

        # Output layer
        self.output = nn.Linear(HIDDEN2_SIZE, OUTPUT_SIZE)
        self.out_activation = nn.Tanh()  # Output in [-1, 1]

        self._init_weights()

    def _init_weights(self):
        """Initialize weights for stable training.

        Uses Kaiming uniform for linear layers since we use ReLU-like activations.
        Biases initialized to zero.
        """
        for module in [self.feature_transform, self.hidden1, self.hidden2]:
            nn.init.kaiming_uniform_(module.weight, nonlinearity='relu')
            nn.init.zeros_(module.bias)

        # Output layer uses Xavier since it's followed by Tanh
        nn.init.xavier_uniform_(self.output.weight)
        nn.init.zeros_(self.output.bias)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Forward pass.

        Args:
            x: Dense feature tensor of shape (batch, TOTAL_FEATURES).
               Or sparse features converted to dense.

        Returns:
            Evaluation score tensor of shape (batch, 1) in range [-1, 1].
        """
        # Feature transformer
        acc = self.feature_transform(x)
        acc = self.ft_activation(acc)

        # Hidden layers
        h1 = self.hidden1(acc)
        h1 = self.h1_activation(h1)

        h2 = self.hidden2(h1)
        h2 = self.h2_activation(h2)

        # Output
        out = self.output(h2)
        out = self.out_activation(out)

        return out

    def forward_sparse(
        self,
        indices: list[torch.Tensor],
        values: list[torch.Tensor],
        batch_size: int,
    ) -> torch.Tensor:
        """Forward pass from sparse feature inputs.

        More efficient for sparse NNUE features: only touches active features
        in the feature transformer.

        Args:
            indices: List of index tensors, one per sample in the batch.
            values: List of value tensors, one per sample in the batch.
            batch_size: Number of samples in the batch.

        Returns:
            Evaluation score tensor of shape (batch, 1) in range [-1, 1].
        """
        # Build dense input from sparse features
        dense = torch.zeros(batch_size, TOTAL_FEATURES, device=self.feature_transform.weight.device)
        for i, (idx, val) in enumerate(zip(indices, values)):
            if len(idx) > 0:
                dense[i, idx.long()] = val.float()

        return self.forward(dense)

    def quantize_weights(self) -> dict:
        """Quantize float weights to integer format matching Rust NNUE.

        Returns:
            Dictionary with quantized weight arrays ready for export.
        """
        with torch.no_grad():
            # Feature transformer: float -> i16 (scale by FT_SCALE)
            ft_w = torch.clamp(
                torch.round(self.feature_transform.weight.data * FT_SCALE),
                -32768, 32767
            ).to(torch.int16)
            ft_b = torch.clamp(
                torch.round(self.feature_transform.bias.data * FT_SCALE),
                -(2**31), 2**31 - 1
            ).to(torch.int32)

            # Hidden1: float -> i8 (scale by HIDDEN_SCALE)
            h1_w = torch.clamp(
                torch.round(self.hidden1.weight.data * HIDDEN_SCALE),
                -128, 127
            ).to(torch.int8)
            h1_b = torch.clamp(
                torch.round(self.hidden1.bias.data * HIDDEN_SCALE),
                -(2**31), 2**31 - 1
            ).to(torch.int32)

            # Hidden2: float -> i8 (scale by HIDDEN_SCALE)
            h2_w = torch.clamp(
                torch.round(self.hidden2.weight.data * HIDDEN_SCALE),
                -128, 127
            ).to(torch.int8)
            h2_b = torch.clamp(
                torch.round(self.hidden2.bias.data * HIDDEN_SCALE),
                -(2**31), 2**31 - 1
            ).to(torch.int32)

            # Output: float -> i16 (scale by OUTPUT_SCALE)
            out_w = torch.clamp(
                torch.round(self.output.weight.data * OUTPUT_SCALE),
                -32768, 32767
            ).to(torch.int16)
            out_b = torch.clamp(
                torch.round(self.output.bias.data * OUTPUT_SCALE),
                -(2**31), 2**31 - 1
            ).to(torch.int32)

        return {
            # Rust stores feature_weights as [input_size * accumulator_size] row-major
            # PyTorch Linear weight is [out_features, in_features], so we transpose
            'feature_weights': ft_w.t().contiguous().flatten().tolist(),
            'feature_biases': ft_b.flatten().tolist(),
            'hidden1_weights': h1_w.t().contiguous().flatten().tolist(),
            'hidden1_biases': h1_b.flatten().tolist(),
            'hidden2_weights': h2_w.t().contiguous().flatten().tolist(),
            'hidden2_biases': h2_b.flatten().tolist(),
            'output_weights': out_w.flatten().tolist(),
            'output_bias': int(out_b.item()),
        }

    @staticmethod
    def from_quantized(weights: dict) -> 'NnueModel':
        """Create a float model from quantized weights (for inference/fine-tuning).

        Args:
            weights: Dictionary with quantized weight arrays (from NnueWeights).

        Returns:
            NnueModel with dequantized float weights.
        """
        model = NnueModel()

        with torch.no_grad():
            # Feature transformer: i16 -> float (divide by FT_SCALE)
            ft_w = torch.tensor(weights['feature_weights'], dtype=torch.float32)
            ft_w = ft_w.view(TOTAL_FEATURES, ACCUMULATOR_SIZE).t() / FT_SCALE
            model.feature_transform.weight.data = ft_w

            ft_b = torch.tensor(weights['feature_biases'], dtype=torch.float32) / FT_SCALE
            model.feature_transform.bias.data = ft_b

            # Hidden1: i8 -> float (divide by HIDDEN_SCALE)
            h1_w = torch.tensor(weights['hidden1_weights'], dtype=torch.float32)
            h1_w = h1_w.view(ACCUMULATOR_SIZE, HIDDEN1_SIZE).t() / HIDDEN_SCALE
            model.hidden1.weight.data = h1_w

            h1_b = torch.tensor(weights['hidden1_biases'], dtype=torch.float32) / HIDDEN_SCALE
            model.hidden1.bias.data = h1_b

            # Hidden2: i8 -> float (divide by HIDDEN_SCALE)
            h2_w = torch.tensor(weights['hidden2_weights'], dtype=torch.float32)
            h2_w = h2_w.view(HIDDEN1_SIZE, HIDDEN2_SIZE).t() / HIDDEN_SCALE
            model.hidden2.weight.data = h2_w

            h2_b = torch.tensor(weights['hidden2_biases'], dtype=torch.float32) / HIDDEN_SCALE
            model.hidden2.bias.data = h2_b

            # Output: i16 -> float (divide by OUTPUT_SCALE)
            out_w = torch.tensor(weights['output_weights'], dtype=torch.float32)
            out_w = out_w.view(1, HIDDEN2_SIZE) / OUTPUT_SCALE
            model.output.weight.data = out_w

            model.output.bias.data = torch.tensor(
                [weights['output_bias'] / OUTPUT_SCALE], dtype=torch.float32
            )

        return model

    def parameter_count(self) -> int:
        """Total number of trainable parameters."""
        return sum(p.numel() for p in self.parameters())

    def __repr__(self) -> str:
        return (
            f"NnueModel("
            f"features={TOTAL_FEATURES}, "
            f"acc={ACCUMULATOR_SIZE}, "
            f"h1={HIDDEN1_SIZE}, "
            f"h2={HIDDEN2_SIZE}, "
            f"params={self.parameter_count():,})"
        )
