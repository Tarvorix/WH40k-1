# WH40K NNUE Training Pipeline
# Source: implementation_v3.md Phase 8.4
#
# Provides:
#   model.py        - PyTorch NNUE model matching the Rust architecture
#   shard_loader.py - DataLoader for training shards from self-play
#   train.py        - Training loop with validation and checkpointing
#   export_weights.py - Export trained PyTorch weights to Rust .nnue format
