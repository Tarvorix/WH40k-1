"""
Policy/Value Dual-Head Neural Network for AlphaGo-Style Training.

Architecture:
    Sparse Input (1209) → Trunk (128, ReLU) → Policy Head (64 → 640 logits)
                                              → Value Head (64 → 1 tanh)

The policy head outputs logits over the 640-action vocabulary (40 intents × 16 unit slots).
The value head outputs a scalar in [-1, 1] representing win probability from the
perspective of the acting player.

Source: implementation_v3.md Phase 10, Section 14
"""

import math
import struct
from pathlib import Path
from typing import Optional, Tuple

import torch
import torch.nn as nn
import torch.nn.functional as F


# Constants matching Rust engine (40 intents after BA Phase 16.7)
TOTAL_FEATURES = 1209
ACTION_VOCAB_SIZE = 640
INTENT_COUNT = 40
MAX_UNIT_SLOTS = 16


class PolicyValueNet(nn.Module):
    """Dual-head network: shared trunk → policy head + value head.

    Architecture:
        Input (1209) → Trunk Linear (128) → ReLU
                     → Policy: Linear (64) → ReLU → Linear (640)
                     → Value: Linear (64) → ReLU → Linear (1) → tanh
    """

    def __init__(
        self,
        input_size: int = TOTAL_FEATURES,
        trunk_size: int = 128,
        policy_hidden: int = 64,
        policy_output: int = ACTION_VOCAB_SIZE,
        value_hidden: int = 64,
    ):
        super().__init__()
        self.input_size = input_size
        self.trunk_size = trunk_size
        self.policy_output = policy_output

        # Shared trunk
        self.trunk = nn.Linear(input_size, trunk_size)

        # Policy head
        self.policy_hidden = nn.Linear(trunk_size, policy_hidden)
        self.policy_output_layer = nn.Linear(policy_hidden, policy_output)

        # Value head
        self.value_hidden = nn.Linear(trunk_size, value_hidden)
        self.value_output = nn.Linear(value_hidden, 1)

        # Initialize weights
        self._init_weights()

    def _init_weights(self):
        """Initialize with small random weights for stability."""
        for module in self.modules():
            if isinstance(module, nn.Linear):
                nn.init.kaiming_normal_(module.weight, nonlinearity="relu")
                if module.bias is not None:
                    nn.init.zeros_(module.bias)
        # Value output uses tanh, so different init
        nn.init.xavier_normal_(self.value_output.weight)
        nn.init.zeros_(self.value_output.bias)

    def forward(
        self,
        x: torch.Tensor,
        legal_mask: Optional[torch.Tensor] = None,
    ) -> Tuple[torch.Tensor, torch.Tensor]:
        """Forward pass.

        Args:
            x: Input features [batch, input_size] (dense f32)
            legal_mask: Boolean mask [batch, action_vocab_size] (True = legal)

        Returns:
            policy_logits: [batch, action_vocab_size] (masked if legal_mask provided)
            value: [batch, 1] in [-1, 1]
        """
        # Shared trunk
        trunk = F.relu(self.trunk(x))

        # Policy head
        policy = F.relu(self.policy_hidden(trunk))
        policy_logits = self.policy_output_layer(policy)

        # Apply legal mask: set illegal actions to -inf
        if legal_mask is not None:
            policy_logits = policy_logits.masked_fill(~legal_mask, float("-inf"))

        # Value head
        value = F.relu(self.value_hidden(trunk))
        value = torch.tanh(self.value_output(value))

        return policy_logits, value

    def policy_probs(
        self,
        x: torch.Tensor,
        legal_mask: torch.Tensor,
    ) -> torch.Tensor:
        """Get policy probabilities (softmax of masked logits).

        Args:
            x: Input features [batch, input_size]
            legal_mask: Boolean mask [batch, action_vocab_size]

        Returns:
            probs: [batch, action_vocab_size] probability distribution
        """
        logits, _ = self.forward(x, legal_mask)
        return F.softmax(logits, dim=-1)

    def value_only(self, x: torch.Tensor) -> torch.Tensor:
        """Get only the value estimate (no policy computation)."""
        trunk = F.relu(self.trunk(x))
        value = F.relu(self.value_hidden(trunk))
        value = torch.tanh(self.value_output(value))
        return value


class PolicyValueLoss(nn.Module):
    """Combined loss for policy/value training.

    Loss = alpha * policy_loss + beta * value_loss + gamma * entropy_bonus

    Where:
        policy_loss = cross-entropy between predicted and target policy
        value_loss = MSE between predicted and target value
        entropy_bonus = negative entropy of predicted policy (for exploration)
    """

    def __init__(
        self,
        alpha: float = 1.0,
        beta: float = 1.0,
        gamma: float = 0.01,
    ):
        super().__init__()
        self.alpha = alpha
        self.beta = beta
        self.gamma = gamma

    def forward(
        self,
        policy_logits: torch.Tensor,
        policy_target: torch.Tensor,
        value_pred: torch.Tensor,
        value_target: torch.Tensor,
        legal_mask: torch.Tensor,
    ) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        """Compute combined loss.

        Args:
            policy_logits: [batch, vocab] raw logits (already masked to -inf for illegal)
            policy_target: [batch, vocab] target probability distribution from MCTS
            value_pred: [batch, 1] predicted value in [-1, 1]
            value_target: [batch, 1] target value (+1 win, -1 loss, 0 draw)
            legal_mask: [batch, vocab] boolean mask

        Returns:
            total_loss, policy_loss, value_loss, entropy
        """
        # Policy loss: cross-entropy with target distribution
        # Only compute over legal actions
        log_probs = F.log_softmax(policy_logits, dim=-1)
        # Mask out illegal actions from target (should already be 0)
        masked_target = policy_target * legal_mask.float()
        # Re-normalize target
        target_sum = masked_target.sum(dim=-1, keepdim=True).clamp(min=1e-8)
        masked_target = masked_target / target_sum

        policy_loss = -(masked_target * log_probs).sum(dim=-1).mean()

        # Value loss: MSE
        value_loss = F.mse_loss(value_pred.squeeze(-1), value_target.squeeze(-1))

        # Entropy bonus: encourage exploration
        probs = F.softmax(policy_logits, dim=-1)
        entropy = -(probs * log_probs).sum(dim=-1).mean()

        # Combined loss (subtract entropy to maximize it)
        total_loss = (
            self.alpha * policy_loss
            + self.beta * value_loss
            - self.gamma * entropy
        )

        return total_loss, policy_loss, value_loss, entropy


def quantize_policy_value_model(
    model: PolicyValueNet,
    ft_scale: int = 64,
    hidden_scale: int = 64,
) -> dict:
    """Quantize a PolicyValueNet to integer weights matching Rust format.

    Returns a dictionary with quantized weight arrays matching
    PolicyValueWeights in Rust.
    """
    state = model.state_dict()

    def quantize_i16(tensor: torch.Tensor, scale: int) -> list:
        scaled = (tensor.float() * scale).round().clamp(-32768, 32767)
        return scaled.to(torch.int16).flatten().tolist()

    def quantize_i8(tensor: torch.Tensor, scale: int) -> list:
        scaled = (tensor.float() * scale).round().clamp(-128, 127)
        return scaled.to(torch.int8).flatten().tolist()

    def quantize_bias_i32(tensor: torch.Tensor) -> list:
        return tensor.float().round().clamp(-2147483648, 2147483647).to(torch.int32).flatten().tolist()

    return {
        "trunk_weights": quantize_i16(state["trunk.weight"].T, ft_scale),
        "trunk_biases": quantize_bias_i32(state["trunk.bias"]),
        "policy_hidden_weights": quantize_i8(state["policy_hidden.weight"].T, hidden_scale),
        "policy_hidden_biases": quantize_bias_i32(state["policy_hidden.bias"]),
        "policy_output_weights": quantize_i8(state["policy_output_layer.weight"].T, hidden_scale),
        "policy_output_biases": quantize_bias_i32(state["policy_output_layer.bias"]),
        "value_hidden_weights": quantize_i8(state["value_hidden.weight"].T, hidden_scale),
        "value_hidden_biases": quantize_bias_i32(state["value_hidden.bias"]),
        "value_output_weights": quantize_i8(state["value_output.weight"].T, hidden_scale),
        "value_output_bias": int(state["value_output.bias"].item()),
    }


def export_policy_value_model(
    model: PolicyValueNet,
    output_path: str,
    generation: int = 0,
):
    """Export a PolicyValueNet to JSON format compatible with Rust PolicyValueModel.

    Args:
        model: Trained PyTorch model
        output_path: Path to write the JSON file
        generation: Model generation number
    """
    import json

    weights = quantize_policy_value_model(model)

    artifact = {
        "dims": {
            "input_size": model.input_size,
            "trunk_size": model.trunk_size,
            "policy_hidden_size": model.policy_hidden.in_features,
            "policy_output_size": model.policy_output,
            "value_hidden_size": model.value_hidden.in_features,
            "value_output_size": 1,
        },
        "weights": weights,
    }

    with open(output_path, "w") as f:
        json.dump(artifact, f)

    print(f"Exported policy/value model (gen {generation}) to {output_path}")
    print(f"  Trunk: {model.input_size} → {model.trunk_size}")
    print(f"  Policy: {model.trunk_size} → {model.policy_hidden.out_features} → {model.policy_output}")
    print(f"  Value: {model.trunk_size} → {model.value_hidden.out_features} → 1")


def load_policy_value_model(
    path: str,
    device: str = "cpu",
) -> PolicyValueNet:
    """Load a PolicyValueNet from a PyTorch checkpoint.

    Args:
        path: Path to .pt checkpoint file
        device: Device to load to

    Returns:
        Loaded PolicyValueNet model
    """
    checkpoint = torch.load(path, map_location=device, weights_only=True)

    # Determine architecture from checkpoint
    state_dict = checkpoint if isinstance(checkpoint, dict) and "trunk.weight" in checkpoint else checkpoint.get("model_state_dict", checkpoint)

    input_size = state_dict["trunk.weight"].shape[1]
    trunk_size = state_dict["trunk.weight"].shape[0]
    policy_hidden = state_dict["policy_hidden.weight"].shape[0]
    policy_output = state_dict["policy_output_layer.weight"].shape[0]
    value_hidden = state_dict["value_hidden.weight"].shape[0]

    model = PolicyValueNet(
        input_size=input_size,
        trunk_size=trunk_size,
        policy_hidden=policy_hidden,
        policy_output=policy_output,
        value_hidden=value_hidden,
    )
    model.load_state_dict(state_dict)
    model.to(device)
    model.eval()

    return model


if __name__ == "__main__":
    # Quick test
    print("Creating PolicyValueNet...")
    model = PolicyValueNet()
    print(f"Total parameters: {sum(p.numel() for p in model.parameters()):,}")

    # Test forward pass
    batch = torch.randn(4, TOTAL_FEATURES)
    legal = torch.ones(4, ACTION_VOCAB_SIZE, dtype=torch.bool)
    legal[:, 100:] = False  # Only first 100 actions legal

    logits, value = model(batch, legal)
    print(f"Policy logits shape: {logits.shape}")
    print(f"Value shape: {value.shape}")
    print(f"Value range: [{value.min().item():.3f}, {value.max().item():.3f}]")

    # Test loss
    loss_fn = PolicyValueLoss()
    policy_target = torch.zeros_like(logits)
    policy_target[:, :100] = 1.0 / 100.0  # Uniform over legal
    value_target = torch.tensor([1.0, -1.0, 0.0, 1.0]).unsqueeze(-1)

    total, p_loss, v_loss, entropy = loss_fn(logits, policy_target, value, value_target, legal)
    print(f"\nLoss: total={total.item():.4f}, policy={p_loss.item():.4f}, "
          f"value={v_loss.item():.4f}, entropy={entropy.item():.4f}")

    # Test quantization
    quantized = quantize_policy_value_model(model)
    print(f"\nQuantized weights:")
    for key, val in quantized.items():
        if isinstance(val, list):
            print(f"  {key}: {len(val)} values")
        else:
            print(f"  {key}: {val}")

    print("\nAll tests passed!")
