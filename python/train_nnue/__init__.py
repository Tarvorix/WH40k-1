# WH40K NNUE Training Pipeline
# Source: implementation_v3.md Phase 8.4, Phase 10
#
# Provides:
#   model.py              - PyTorch NNUE model matching the Rust architecture
#   shard_loader.py       - DataLoader for training shards from self-play
#   train.py              - Training loop with validation and checkpointing
#   export_weights.py     - Export trained PyTorch weights to Rust .nnue format
#   policy_value_model.py - Dual-head PolicyValueNet for AlphaGo-style training
#   mcts_train.py         - MCTS/AlphaGo policy/value training pipeline
