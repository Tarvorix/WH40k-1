"""
WH40K Alpharius Policy/Value Training Package.

AlphaZero-style dual-head network training for MCTS self-play.
Separate from the Perturabo NNUE training pipeline (train_nnue).

Source: implementation_v3.md Phase 10
"""

from .policy_value_model import (
    PolicyValueNet,
    PolicyValueLoss,
    TOTAL_FEATURES,
    ACTION_VOCAB_SIZE,
    INTENT_COUNT,
    MAX_UNIT_SLOTS,
    quantize_policy_value_model,
    export_policy_value_model,
    load_policy_value_model,
)

__all__ = [
    "PolicyValueNet",
    "PolicyValueLoss",
    "TOTAL_FEATURES",
    "ACTION_VOCAB_SIZE",
    "INTENT_COUNT",
    "MAX_UNIT_SLOTS",
    "quantize_policy_value_model",
    "export_policy_value_model",
    "load_policy_value_model",
]
