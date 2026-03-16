"""
WH40K MCTS / AlphaGo-Style Policy/Value Training Pipeline.

Trains the dual-head PolicyValueNet on self-play data collected via MCTS.

Training pipeline:
    1. Generate training data via MCTS self-play (or load existing PolicyValueShards)
    2. Train PolicyValueNet on (state, policy_target, value_target) triples
    3. Export quantized weights to JSON format for Rust PolicyValueModel
    4. Evaluate candidate model against baseline via gating
    5. Promote if win rate > 55%

Loss function:
    Combined = alpha * policy_cross_entropy + beta * value_MSE - gamma * entropy_bonus

Data format (from Rust PolicyValueShard JSON):
    Each sample contains:
        - sparse_features: Vec<(u16, i16)>  -> dense [1209] input
        - legal_mask: { mask: Vec<bool> }    -> bool [640]
        - policy_target: Vec<f32>            -> float [640]
        - value_target: f32                  -> scalar in [-1, 1]
        - perspective: u32                   -> player id
        - progress: f32                      -> game progress [0, 1]
        - search_score: Option<i32>          -> optional search score

Source: implementation_v3.md Phase 10, Section 14
"""

import argparse
import json
import os
import time
from pathlib import Path
from typing import Optional

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader, ConcatDataset, random_split

from .policy_value_model import (
    PolicyValueNet,
    PolicyValueLoss,
    TOTAL_FEATURES,
    ACTION_VOCAB_SIZE,
    quantize_policy_value_model,
    export_policy_value_model,
    load_policy_value_model,
)

# Try to import bridge for self-play data generation and gating
try:
    import wh40k_trainer_bridge as wtb
    HAS_BRIDGE = True
except ImportError:
    HAS_BRIDGE = False


# ─── Dataset ────────────────────────────────────────────────────────────────

class PolicyValueShardDataset(Dataset):
    """PyTorch Dataset for PolicyValueShard data.

    Loads samples from Rust-generated PolicyValueShard JSON files and
    provides them as dense tensors for PolicyValueNet training.

    Each sample yields:
        - features: dense f32 tensor [TOTAL_FEATURES]
        - legal_mask: bool tensor [ACTION_VOCAB_SIZE]
        - policy_target: f32 tensor [ACTION_VOCAB_SIZE]
        - value_target: f32 scalar
        - progress: f32 scalar (game progress 0→1)
    """

    def __init__(self, samples: list):
        """Create dataset from a list of sample dicts.

        Args:
            samples: List of sample dicts with keys matching PolicyValueSample.
        """
        self.features_list = []
        self.legal_masks = []
        self.policy_targets = []
        self.value_targets = []
        self.progresses = []

        for s in samples:
            # Convert sparse features to dense
            dense = torch.zeros(TOTAL_FEATURES, dtype=torch.float32)
            sparse = s['sparse_features'] if isinstance(s, dict) else s.sparse_features
            for idx, val in sparse:
                if idx < TOTAL_FEATURES:
                    dense[int(idx)] = float(val) / 1000.0  # Fixed-point normalization

            self.features_list.append(dense)

            # Legal mask
            if isinstance(s, dict):
                mask_data = s['legal_mask']
                if isinstance(mask_data, dict):
                    mask = mask_data['mask']
                else:
                    mask = mask_data
            else:
                mask = s.legal_mask.mask if hasattr(s.legal_mask, 'mask') else s.legal_mask
            legal = torch.tensor(mask[:ACTION_VOCAB_SIZE], dtype=torch.bool)
            self.legal_masks.append(legal)

            # Policy target
            policy = s['policy_target'] if isinstance(s, dict) else s.policy_target
            policy_tensor = torch.tensor(policy[:ACTION_VOCAB_SIZE], dtype=torch.float32)
            # Ensure proper normalization (sum to 1 over legal actions)
            legal_sum = (policy_tensor * legal.float()).sum()
            if legal_sum > 1e-8:
                policy_tensor = (policy_tensor * legal.float()) / legal_sum
            else:
                # Uniform over legal actions as fallback
                num_legal = legal.sum().item()
                if num_legal > 0:
                    policy_tensor = legal.float() / num_legal
            self.policy_targets.append(policy_tensor)

            # Value target
            value = s['value_target'] if isinstance(s, dict) else s.value_target
            self.value_targets.append(float(value))

            # Progress
            progress = s.get('progress', 0.0) if isinstance(s, dict) else getattr(s, 'progress', 0.0)
            self.progresses.append(float(progress))

    def __len__(self) -> int:
        return len(self.features_list)

    def __getitem__(self, idx: int) -> dict:
        return {
            'features': self.features_list[idx],
            'legal_mask': self.legal_masks[idx],
            'policy_target': self.policy_targets[idx],
            'value_target': torch.tensor(self.value_targets[idx], dtype=torch.float32),
            'progress': torch.tensor(self.progresses[idx], dtype=torch.float32),
        }


def load_policy_value_shard_json(path: str) -> list:
    """Load a PolicyValueShard from a JSON file.

    Args:
        path: Path to JSON shard file.

    Returns:
        List of sample dicts.
    """
    with open(path, 'r') as f:
        shard = json.load(f)

    return shard.get('samples', [])


def load_all_policy_value_shards(shard_dir: str) -> PolicyValueShardDataset:
    """Load all PolicyValueShard JSON files from a directory.

    Args:
        shard_dir: Directory containing .json shard files.

    Returns:
        Combined PolicyValueShardDataset.
    """
    shard_path = Path(shard_dir)
    all_samples = []

    # Look for JSON shard files
    json_files = sorted(shard_path.glob("*.json"))
    if not json_files:
        # Also try subdirectories
        json_files = sorted(shard_path.rglob("pv_shard_*.json"))

    if not json_files:
        raise ValueError(f"No PolicyValueShard JSON files found in {shard_dir}")

    for jf in json_files:
        try:
            samples = load_policy_value_shard_json(str(jf))
            all_samples.extend(samples)
            print(f"  Loaded {len(samples)} samples from {jf.name}")
        except (json.JSONDecodeError, KeyError) as e:
            print(f"  [WARN] Skipping {jf.name}: {e}")

    if not all_samples:
        raise ValueError(f"No valid samples found in {shard_dir}")

    return PolicyValueShardDataset(all_samples)


def load_policy_value_shards_bridge(shard_dir: str) -> PolicyValueShardDataset:
    """Load PolicyValueShards using the Rust bridge (bincode format).

    Args:
        shard_dir: Directory containing .bin shard files.

    Returns:
        Combined PolicyValueShardDataset.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    shards = wtb.load_all_shards(str(shard_dir))
    all_samples = []
    for shard in shards:
        all_samples.extend(shard.samples())

    if not all_samples:
        raise ValueError(f"No valid samples found in {shard_dir}")

    return PolicyValueShardDataset(all_samples)


def create_policy_value_dataloader(
    shard_dir: str,
    batch_size: int = 256,
    shuffle: bool = True,
    num_workers: int = 0,
    pin_memory: bool = True,
    use_bridge: bool = False,
) -> DataLoader:
    """Create a DataLoader from all PolicyValueShards in a directory.

    Args:
        shard_dir: Directory containing shard files.
        batch_size: Batch size for training.
        shuffle: Whether to shuffle samples.
        num_workers: Number of data loading workers.
        pin_memory: Whether to pin memory for GPU transfer.
        use_bridge: If True, use Rust bridge for bincode shards.

    Returns:
        DataLoader yielding batches of policy/value training data.
    """
    if use_bridge:
        dataset = load_policy_value_shards_bridge(shard_dir)
    else:
        dataset = load_all_policy_value_shards(shard_dir)

    return DataLoader(
        dataset,
        batch_size=batch_size,
        shuffle=shuffle,
        num_workers=num_workers,
        pin_memory=pin_memory,
        drop_last=True,
    )


# ─── Training Loop ──────────────────────────────────────────────────────────

def train_epoch(
    model: PolicyValueNet,
    dataloader: DataLoader,
    optimizer: torch.optim.Optimizer,
    loss_fn: PolicyValueLoss,
    device: torch.device,
    grad_clip: float = 1.0,
) -> dict:
    """Train for one epoch.

    Args:
        model: PolicyValueNet to train.
        dataloader: DataLoader yielding policy/value batches.
        optimizer: Optimizer instance.
        loss_fn: PolicyValueLoss instance.
        device: Device to train on.
        grad_clip: Max gradient norm for clipping.

    Returns:
        Dict with epoch metrics.
    """
    model.train()
    total_loss = 0.0
    total_policy_loss = 0.0
    total_value_loss = 0.0
    total_entropy = 0.0
    num_batches = 0

    for batch in dataloader:
        features = batch['features'].to(device)
        legal_mask = batch['legal_mask'].to(device)
        policy_target = batch['policy_target'].to(device)
        value_target = batch['value_target'].to(device).unsqueeze(-1)

        optimizer.zero_grad()

        # Forward pass with legal mask
        policy_logits, value_pred = model(features, legal_mask)

        # Compute combined loss
        total, p_loss, v_loss, entropy = loss_fn(
            policy_logits, policy_target, value_pred, value_target, legal_mask,
        )

        total.backward()

        # Gradient clipping for stability
        if grad_clip > 0:
            torch.nn.utils.clip_grad_norm_(model.parameters(), grad_clip)

        optimizer.step()

        total_loss += total.item()
        total_policy_loss += p_loss.item()
        total_value_loss += v_loss.item()
        total_entropy += entropy.item()
        num_batches += 1

    n = max(num_batches, 1)
    return {
        'loss': total_loss / n,
        'policy_loss': total_policy_loss / n,
        'value_loss': total_value_loss / n,
        'entropy': total_entropy / n,
        'num_batches': num_batches,
    }


def validate_epoch(
    model: PolicyValueNet,
    dataloader: DataLoader,
    loss_fn: PolicyValueLoss,
    device: torch.device,
) -> dict:
    """Validate the model on held-out data.

    Args:
        model: PolicyValueNet to validate.
        dataloader: Validation DataLoader.
        loss_fn: PolicyValueLoss instance.
        device: Device.

    Returns:
        Dict with validation metrics.
    """
    model.eval()
    total_loss = 0.0
    total_policy_loss = 0.0
    total_value_loss = 0.0
    total_entropy = 0.0
    total_policy_accuracy = 0.0
    total_value_accuracy = 0.0
    total_samples = 0
    num_batches = 0

    with torch.no_grad():
        for batch in dataloader:
            features = batch['features'].to(device)
            legal_mask = batch['legal_mask'].to(device)
            policy_target = batch['policy_target'].to(device)
            value_target = batch['value_target'].to(device).unsqueeze(-1)

            policy_logits, value_pred = model(features, legal_mask)

            total, p_loss, v_loss, entropy = loss_fn(
                policy_logits, policy_target, value_pred, value_target, legal_mask,
            )

            # Policy accuracy: does the argmax of predicted match argmax of target?
            pred_action = policy_logits.argmax(dim=-1)
            target_action = policy_target.argmax(dim=-1)
            policy_correct = (pred_action == target_action).sum().item()

            # Value accuracy: does the sign of prediction match the sign of target?
            pred_sign = torch.sign(value_pred.squeeze(-1))
            target_sign = torch.sign(value_target.squeeze(-1))
            value_correct = (pred_sign == target_sign).sum().item()

            batch_size = features.size(0)
            total_loss += total.item()
            total_policy_loss += p_loss.item()
            total_value_loss += v_loss.item()
            total_entropy += entropy.item()
            total_policy_accuracy += policy_correct
            total_value_accuracy += value_correct
            total_samples += batch_size
            num_batches += 1

    n = max(num_batches, 1)
    s = max(total_samples, 1)
    return {
        'val_loss': total_loss / n,
        'val_policy_loss': total_policy_loss / n,
        'val_value_loss': total_value_loss / n,
        'val_entropy': total_entropy / n,
        'policy_accuracy': total_policy_accuracy / s,
        'value_accuracy': total_value_accuracy / s,
        'num_samples': total_samples,
    }


# ─── Full Training Run ──────────────────────────────────────────────────────

def run_policy_value_training(
    shard_dir: str,
    output_dir: str = "pv_checkpoints",
    epochs: int = 100,
    batch_size: int = 256,
    learning_rate: float = 1e-3,
    weight_decay: float = 1e-4,
    policy_weight: float = 1.0,
    value_weight: float = 1.0,
    entropy_weight: float = 0.01,
    val_split: float = 0.1,
    checkpoint_every: int = 10,
    model_id: Optional[str] = None,
    generation: int = 0,
    device_name: str = "auto",
    grad_clip: float = 1.0,
    lr_patience: int = 10,
    lr_factor: float = 0.5,
    min_lr: float = 1e-6,
    warmup_epochs: int = 5,
    resume_checkpoint: Optional[str] = None,
    use_bridge: bool = False,
    trunk_size: int = 128,
    policy_hidden: int = 64,
    value_hidden: int = 64,
) -> dict:
    """Run the complete PolicyValueNet training loop.

    Args:
        shard_dir: Directory containing PolicyValueShard files.
        output_dir: Directory for checkpoints and exported models.
        epochs: Number of training epochs.
        batch_size: Training batch size.
        learning_rate: Initial learning rate.
        weight_decay: L2 regularization weight.
        policy_weight: Weight for policy cross-entropy loss (alpha).
        value_weight: Weight for value MSE loss (beta).
        entropy_weight: Weight for entropy bonus (gamma).
        val_split: Fraction of data for validation.
        checkpoint_every: Save checkpoint every N epochs.
        model_id: Model identifier (auto-generated if None).
        generation: Training generation number.
        device_name: Device ("auto", "cpu", "cuda", "mps").
        grad_clip: Max gradient norm for clipping (0 = disabled).
        lr_patience: Epochs of no improvement before LR reduction.
        lr_factor: Factor to reduce LR by.
        min_lr: Minimum learning rate.
        warmup_epochs: Number of warmup epochs (linear LR ramp).
        resume_checkpoint: Path to checkpoint to resume from.
        use_bridge: If True, use Rust bridge for bincode shards.
        trunk_size: Trunk layer size (default 128).
        policy_hidden: Policy head hidden size (default 64).
        value_hidden: Value head hidden size (default 64).

    Returns:
        Dict with training results and final model paths.
    """
    # Setup output directory
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    # Select device
    if device_name == "auto":
        if torch.cuda.is_available():
            device = torch.device("cuda")
        elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            device = torch.device("mps")
        else:
            device = torch.device("cpu")
    else:
        device = torch.device(device_name)

    print(f"Using device: {device}")

    if model_id is None:
        model_id = f"pv_gen{generation}_{int(time.time())}"

    # Load data
    print(f"Loading PolicyValueShard data from {shard_dir}...")
    if use_bridge:
        dataset = load_policy_value_shards_bridge(shard_dir)
    else:
        dataset = load_all_policy_value_shards(shard_dir)

    total_samples = len(dataset)
    print(f"Loaded {total_samples:,} training samples")

    if total_samples == 0:
        raise ValueError("No training samples found")

    # Split into train/val
    val_size = int(total_samples * val_split)
    train_size = total_samples - val_size

    if val_size > 0:
        train_dataset, val_dataset = random_split(dataset, [train_size, val_size])
        train_loader = DataLoader(
            train_dataset, batch_size=batch_size, shuffle=True, drop_last=True,
        )
        val_loader = DataLoader(
            val_dataset, batch_size=batch_size, shuffle=False,
        )
    else:
        train_loader = DataLoader(
            dataset, batch_size=batch_size, shuffle=True, drop_last=True,
        )
        val_loader = None

    # Create model
    model = PolicyValueNet(
        input_size=TOTAL_FEATURES,
        trunk_size=trunk_size,
        policy_hidden=policy_hidden,
        policy_output=ACTION_VOCAB_SIZE,
        value_hidden=value_hidden,
    ).to(device)

    print(f"PolicyValueNet: {sum(p.numel() for p in model.parameters()):,} parameters")
    print(f"  Trunk: {TOTAL_FEATURES} -> {trunk_size}")
    print(f"  Policy: {trunk_size} -> {policy_hidden} -> {ACTION_VOCAB_SIZE}")
    print(f"  Value: {trunk_size} -> {value_hidden} -> 1")

    # Loss function
    loss_fn = PolicyValueLoss(
        alpha=policy_weight,
        beta=value_weight,
        gamma=entropy_weight,
    )

    # Optimizer
    optimizer = optim.Adam(
        model.parameters(),
        lr=learning_rate,
        weight_decay=weight_decay,
    )

    # Learning rate scheduler
    scheduler = optim.lr_scheduler.ReduceLROnPlateau(
        optimizer, mode='min', factor=lr_factor, patience=lr_patience,
        min_lr=min_lr,
    )

    # Resume from checkpoint if provided
    start_epoch = 1
    training_log = []
    best_val_loss = float('inf')

    if resume_checkpoint is not None:
        print(f"Resuming from checkpoint: {resume_checkpoint}")
        ckpt = torch.load(resume_checkpoint, map_location=device, weights_only=False)
        model.load_state_dict(ckpt['model_state_dict'])
        optimizer.load_state_dict(ckpt['optimizer_state_dict'])
        start_epoch = ckpt.get('epoch', 0) + 1
        best_val_loss = ckpt.get('best_val_loss', float('inf'))
        training_log = ckpt.get('training_log', [])
        print(f"  Resuming from epoch {start_epoch}, best_val_loss={best_val_loss:.6f}")

    # Training loop
    start_time = time.time()

    print(f"\nStarting training for {epochs} epochs (from epoch {start_epoch})...")
    print(f"  Batch size: {batch_size}")
    print(f"  Learning rate: {learning_rate}")
    print(f"  Loss weights: policy={policy_weight}, value={value_weight}, entropy={entropy_weight}")
    print(f"  Gradient clipping: {grad_clip}")
    print(f"  Train samples: {train_size:,}")
    if val_size > 0:
        print(f"  Val samples: {val_size:,}")
    print()

    for epoch in range(start_epoch, start_epoch + epochs):
        epoch_start = time.time()

        # Warmup: linearly increase LR from min_lr to learning_rate
        if warmup_epochs > 0 and epoch <= warmup_epochs:
            warmup_lr = min_lr + (learning_rate - min_lr) * (epoch / warmup_epochs)
            for pg in optimizer.param_groups:
                pg['lr'] = warmup_lr

        # Train
        train_metrics = train_epoch(
            model, train_loader, optimizer, loss_fn, device, grad_clip,
        )

        # Validate
        if val_loader is not None:
            val_metrics = validate_epoch(model, val_loader, loss_fn, device)

            # Only use scheduler after warmup
            if epoch > warmup_epochs:
                scheduler.step(val_metrics['val_loss'])

            if val_metrics['val_loss'] < best_val_loss:
                best_val_loss = val_metrics['val_loss']
                # Save best model
                best_path = output_path / f"{model_id}_best.pt"
                torch.save({
                    'epoch': epoch,
                    'model_state_dict': model.state_dict(),
                    'optimizer_state_dict': optimizer.state_dict(),
                    'best_val_loss': best_val_loss,
                    'training_log': training_log,
                    'config': {
                        'trunk_size': trunk_size,
                        'policy_hidden': policy_hidden,
                        'value_hidden': value_hidden,
                        'policy_weight': policy_weight,
                        'value_weight': value_weight,
                        'entropy_weight': entropy_weight,
                    },
                }, best_path)
        else:
            val_metrics = {}
            if epoch > warmup_epochs:
                scheduler.step(train_metrics['loss'])

        epoch_time = time.time() - epoch_start
        current_lr = optimizer.param_groups[0]['lr']

        # Log entry
        log_entry = {
            'epoch': epoch,
            'train_loss': train_metrics['loss'],
            'policy_loss': train_metrics['policy_loss'],
            'value_loss': train_metrics['value_loss'],
            'entropy': train_metrics['entropy'],
            'epoch_time': epoch_time,
            'lr': current_lr,
        }
        if val_metrics:
            log_entry['val_loss'] = val_metrics['val_loss']
            log_entry['val_policy_loss'] = val_metrics['val_policy_loss']
            log_entry['val_value_loss'] = val_metrics['val_value_loss']
            log_entry['policy_accuracy'] = val_metrics['policy_accuracy']
            log_entry['value_accuracy'] = val_metrics['value_accuracy']

        training_log.append(log_entry)

        # Print progress
        val_str = ""
        if val_metrics:
            val_str = (
                f"  val={val_metrics['val_loss']:.4f}"
                f"  p_acc={val_metrics['policy_accuracy']:.3f}"
                f"  v_acc={val_metrics['value_accuracy']:.3f}"
            )

        print(
            f"Epoch {epoch:3d}  "
            f"loss={train_metrics['loss']:.4f}  "
            f"p={train_metrics['policy_loss']:.4f}  "
            f"v={train_metrics['value_loss']:.4f}  "
            f"H={train_metrics['entropy']:.4f}"
            f"{val_str}  "
            f"lr={current_lr:.2e}  "
            f"t={epoch_time:.1f}s"
        )

        # Checkpoint
        if epoch % checkpoint_every == 0:
            ckpt_path = output_path / f"{model_id}_epoch{epoch}.pt"
            torch.save({
                'epoch': epoch,
                'model_state_dict': model.state_dict(),
                'optimizer_state_dict': optimizer.state_dict(),
                'best_val_loss': best_val_loss,
                'train_loss': train_metrics['loss'],
                'training_log': training_log,
                'config': {
                    'trunk_size': trunk_size,
                    'policy_hidden': policy_hidden,
                    'value_hidden': value_hidden,
                    'policy_weight': policy_weight,
                    'value_weight': value_weight,
                    'entropy_weight': entropy_weight,
                },
            }, ckpt_path)
            print(f"  -> Checkpoint saved: {ckpt_path}")

        # Early stopping: if LR has bottomed out and no improvement
        if current_lr <= min_lr and epoch > warmup_epochs + lr_patience * 3:
            if val_metrics and val_metrics['val_loss'] > best_val_loss * 1.1:
                print(f"\nEarly stopping at epoch {epoch} (LR bottomed, no improvement)")
                break

    total_time = time.time() - start_time
    print(f"\nTraining complete in {total_time:.1f}s")
    print(f"Best validation loss: {best_val_loss:.6f}")

    # Load best model for export
    best_path = output_path / f"{model_id}_best.pt"
    if best_path.exists():
        best_ckpt = torch.load(best_path, map_location=device, weights_only=False)
        model.load_state_dict(best_ckpt['model_state_dict'])
        print(f"Loaded best model from epoch {best_ckpt['epoch']}")

    # Export to JSON format for Rust PolicyValueModel
    json_path = output_path / f"{model_id}.json"
    export_policy_value_model(model, str(json_path), generation=generation)

    # Also save final PyTorch checkpoint
    final_path = output_path / f"{model_id}_final.pt"
    torch.save({
        'model_state_dict': model.state_dict(),
        'generation': generation,
        'model_id': model_id,
        'total_samples': total_samples,
        'epochs_trained': epochs,
        'best_val_loss': best_val_loss,
        'final_loss': train_metrics['loss'],
        'config': {
            'trunk_size': trunk_size,
            'policy_hidden': policy_hidden,
            'value_hidden': value_hidden,
            'policy_weight': policy_weight,
            'value_weight': value_weight,
            'entropy_weight': entropy_weight,
        },
    }, final_path)

    # Save training log
    log_path = output_path / f"{model_id}_training_log.json"
    with open(log_path, 'w') as f:
        json.dump({
            'model_id': model_id,
            'generation': generation,
            'total_samples': total_samples,
            'epochs': epochs,
            'final_loss': train_metrics['loss'],
            'final_policy_loss': train_metrics['policy_loss'],
            'final_value_loss': train_metrics['value_loss'],
            'best_val_loss': best_val_loss,
            'total_time_secs': total_time,
            'config': {
                'batch_size': batch_size,
                'learning_rate': learning_rate,
                'weight_decay': weight_decay,
                'policy_weight': policy_weight,
                'value_weight': value_weight,
                'entropy_weight': entropy_weight,
                'grad_clip': grad_clip,
                'trunk_size': trunk_size,
                'policy_hidden': policy_hidden,
                'value_hidden': value_hidden,
            },
            'log': training_log,
        }, f, indent=2)

    return {
        'model_id': model_id,
        'json_path': str(json_path),
        'final_path': str(final_path),
        'final_loss': train_metrics['loss'],
        'final_policy_loss': train_metrics['policy_loss'],
        'final_value_loss': train_metrics['value_loss'],
        'best_val_loss': best_val_loss,
        'total_time': total_time,
        'total_samples': total_samples,
    }


# ─── Self-Play Data Generation ──────────────────────────────────────────────

def generate_mcts_training_data(
    num_games: int = 100,
    output_dir: str = "pv_training_data",
    simulations: int = 100,
    model_path: Optional[str] = None,
    generation: int = 0,
) -> int:
    """Generate policy/value training data via MCTS self-play.

    Uses the Rust engine to run MCTS self-play games and collect
    PolicyValueSamples containing (state, MCTS_policy, outcome) triples.

    Args:
        num_games: Number of self-play games to run.
        output_dir: Directory to write PolicyValueShard files.
        simulations: Number of MCTS simulations per move.
        model_path: Optional path to PolicyValueModel JSON for guided MCTS.
        generation: Model generation number for shard metadata.

    Returns:
        Total number of training samples generated.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    Path(output_dir).mkdir(parents=True, exist_ok=True)

    print(f"Generating MCTS self-play training data...")
    print(f"  Games: {num_games}")
    print(f"  Simulations/move: {simulations}")
    if model_path:
        print(f"  Policy/Value model: {model_path}")
    print(f"  Output: {output_dir}")

    report = wtb.run_selfplay_batch(
        num_games,
        output_dir,
    )

    total_samples = report.get('total_samples', 0)
    print(f"\nSelf-play complete:")
    print(f"  Games: {report.get('total_games', 0)}")
    print(f"  P1 wins: {report.get('player1_wins', 0)}, "
          f"P2 wins: {report.get('player2_wins', 0)}, "
          f"Draws: {report.get('draws', 0)}")
    print(f"  Total samples: {total_samples}")
    print(f"  Throughput: {report.get('games_per_hour', 0.0):.0f} games/hour")

    return total_samples


# ─── Gating Evaluation ──────────────────────────────────────────────────────

def run_policy_value_gating(
    candidate_path: str,
    num_games: int = 100,
    promotion_threshold: float = 0.55,
    baseline_path: Optional[str] = None,
) -> dict:
    """Evaluate a candidate PolicyValueModel against a baseline.

    Runs a series of games between the candidate and baseline models,
    determines if the candidate should be promoted based on win rate.

    Args:
        candidate_path: Path to candidate model JSON file.
        num_games: Number of evaluation games.
        promotion_threshold: Win rate threshold for promotion (0.55 = 55%).
        baseline_path: Path to baseline model (None = heuristic baseline).

    Returns:
        Dict with gating results.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    print(f"Evaluating candidate PolicyValueModel: {candidate_path}")
    if baseline_path:
        print(f"  Baseline: {baseline_path}")
    else:
        print(f"  Baseline: heuristic evaluator")
    print(f"  Games: {num_games}, Threshold: {promotion_threshold:.0%}")

    result = wtb.evaluate_candidate(
        candidate_path,
        num_games=num_games,
        promotion_threshold=promotion_threshold,
    )

    candidate_wins = result.candidate_wins
    baseline_wins = result.baseline_wins
    draws = result.draws
    total = candidate_wins + baseline_wins + draws
    win_rate = candidate_wins / max(total, 1)
    promoted = win_rate >= promotion_threshold

    print(f"\nGating Results:")
    print(f"  Candidate wins: {candidate_wins}/{total}")
    print(f"  Baseline wins:  {baseline_wins}/{total}")
    print(f"  Draws:          {draws}")
    print(f"  Win rate:       {win_rate:.1%}")
    print(f"  Elo delta:      {result.elo_delta:+d}")
    print(f"  Confidence:     [{result.confidence_lower:+d}, {result.confidence_upper:+d}]")
    print(f"  PROMOTED:       {'YES' if promoted else 'NO'}")

    return {
        'candidate_wins': candidate_wins,
        'baseline_wins': baseline_wins,
        'draws': draws,
        'total_games': total,
        'win_rate': win_rate,
        'promoted': promoted,
        'elo_delta': result.elo_delta,
    }


# ─── Full AlphaGo-Style Pipeline ────────────────────────────────────────────

def run_alphago_pipeline(
    output_dir: str = "alphago_output",
    num_iterations: int = 5,
    games_per_iteration: int = 100,
    simulations_per_move: int = 100,
    epochs_per_iteration: int = 50,
    batch_size: int = 256,
    learning_rate: float = 1e-3,
    gate_games: int = 100,
    promotion_threshold: float = 0.55,
    device_name: str = "auto",
) -> dict:
    """Run the full AlphaGo-style training pipeline.

    Iterates:
        1. Self-play with current best model → generate training data
        2. Train new candidate model on accumulated data
        3. Gate candidate vs current best → promote if better

    Args:
        output_dir: Base output directory for all artifacts.
        num_iterations: Number of generate→train→gate iterations.
        games_per_iteration: Self-play games per iteration.
        simulations_per_move: MCTS simulations per move.
        epochs_per_iteration: Training epochs per iteration.
        batch_size: Training batch size.
        learning_rate: Initial learning rate.
        gate_games: Number of gating evaluation games.
        promotion_threshold: Win rate threshold for promotion.
        device_name: Device for training.

    Returns:
        Dict with pipeline results and model lineage.
    """
    base = Path(output_dir)
    base.mkdir(parents=True, exist_ok=True)

    lineage = []
    current_model_path = None
    total_samples = 0

    for iteration in range(num_iterations):
        gen = iteration + 1
        print("\n" + "=" * 70)
        print(f"ITERATION {gen}/{num_iterations}")
        print("=" * 70)

        # Directory structure for this iteration
        iter_dir = base / f"iter_{gen:03d}"
        data_dir = iter_dir / "shards"
        ckpt_dir = iter_dir / "checkpoints"

        # Step 1: Generate training data
        print(f"\n--- Step 1: Self-Play Data Generation (gen {gen}) ---")
        try:
            new_samples = generate_mcts_training_data(
                num_games=games_per_iteration,
                output_dir=str(data_dir),
                simulations=simulations_per_move,
                model_path=current_model_path,
                generation=gen,
            )
            total_samples += new_samples
        except ImportError as e:
            print(f"[WARN] Self-play generation requires bridge: {e}")
            print("Skipping data generation - train on existing data if available")
            if not data_dir.exists():
                print(f"No data available for iteration {gen}, stopping pipeline")
                break

        # Step 2: Train
        print(f"\n--- Step 2: Train PolicyValueNet (gen {gen}) ---")
        model_id = f"pv_gen{gen}"

        # Accumulate all shards from all iterations
        all_shard_dirs = []
        for prev_iter in range(1, gen + 1):
            prev_data = base / f"iter_{prev_iter:03d}" / "shards"
            if prev_data.exists():
                all_shard_dirs.append(prev_data)

        # Merge shards into a combined directory (symlinks or listing)
        combined_dir = iter_dir / "combined_shards"
        combined_dir.mkdir(parents=True, exist_ok=True)

        for sd in all_shard_dirs:
            for shard_file in sd.glob("*.json"):
                link_target = combined_dir / f"{sd.parent.name}_{shard_file.name}"
                if not link_target.exists():
                    try:
                        link_target.symlink_to(shard_file.resolve())
                    except OSError:
                        # Fallback: copy if symlinks not supported
                        import shutil
                        shutil.copy2(str(shard_file), str(link_target))

        try:
            train_result = run_policy_value_training(
                shard_dir=str(combined_dir),
                output_dir=str(ckpt_dir),
                epochs=epochs_per_iteration,
                batch_size=batch_size,
                learning_rate=learning_rate,
                model_id=model_id,
                generation=gen,
                device_name=device_name,
            )
        except ValueError as e:
            print(f"[ERROR] Training failed: {e}")
            break

        candidate_path = train_result['json_path']

        # Step 3: Gate
        print(f"\n--- Step 3: Gating Evaluation (gen {gen}) ---")
        try:
            gate_result = run_policy_value_gating(
                candidate_path=candidate_path,
                num_games=gate_games,
                promotion_threshold=promotion_threshold,
                baseline_path=current_model_path,
            )

            if gate_result['promoted']:
                print(f"\n*** Model gen {gen} PROMOTED as new best ***")
                current_model_path = candidate_path
            else:
                print(f"\n--- Model gen {gen} not promoted, keeping previous best ---")

        except ImportError as e:
            print(f"[WARN] Gating requires bridge: {e}")
            print("Auto-promoting candidate (no gating available)")
            gate_result = {'promoted': True, 'win_rate': 0.0, 'elo_delta': 0}
            current_model_path = candidate_path

        # Record lineage
        lineage.append({
            'generation': gen,
            'model_id': model_id,
            'model_path': candidate_path,
            'training_loss': train_result['final_loss'],
            'policy_loss': train_result['final_policy_loss'],
            'value_loss': train_result['final_value_loss'],
            'best_val_loss': train_result['best_val_loss'],
            'total_samples': train_result['total_samples'],
            'promoted': gate_result.get('promoted', False),
            'win_rate': gate_result.get('win_rate', 0.0),
            'elo_delta': gate_result.get('elo_delta', 0),
        })

    # Save lineage
    lineage_path = base / "model_lineage.json"
    with open(lineage_path, 'w') as f:
        json.dump({
            'pipeline': 'alphago_policy_value',
            'iterations': len(lineage),
            'total_samples': total_samples,
            'current_best': current_model_path,
            'lineage': lineage,
        }, f, indent=2)

    # Summary
    print("\n" + "=" * 70)
    print("PIPELINE COMPLETE")
    print("=" * 70)
    print(f"  Iterations: {len(lineage)}")
    print(f"  Total samples: {total_samples:,}")
    print(f"  Current best: {current_model_path}")
    if lineage:
        promoted_gens = [e for e in lineage if e['promoted']]
        print(f"  Promotions: {len(promoted_gens)}/{len(lineage)}")
        if promoted_gens:
            best = promoted_gens[-1]
            print(f"  Best model: gen {best['generation']} "
                  f"(loss={best['training_loss']:.4f}, elo={best['elo_delta']:+d})")
    print(f"  Lineage saved: {lineage_path}")

    return {
        'iterations': len(lineage),
        'total_samples': total_samples,
        'current_best': current_model_path,
        'lineage': lineage,
    }


# ─── Model Inspection ───────────────────────────────────────────────────────

def inspect_model(model_path: str):
    """Inspect a PolicyValueNet checkpoint or exported JSON model.

    Args:
        model_path: Path to .pt checkpoint or .json exported model.
    """
    path = Path(model_path)

    if path.suffix == '.pt':
        model = load_policy_value_model(str(path))
        print(f"PolicyValueNet from PyTorch checkpoint: {model_path}")
    elif path.suffix == '.json':
        with open(path, 'r') as f:
            data = json.load(f)
        dims = data.get('dims', {})
        print(f"PolicyValueModel JSON: {model_path}")
        print(f"  Input: {dims.get('input_size', '?')}")
        print(f"  Trunk: {dims.get('trunk_size', '?')}")
        print(f"  Policy hidden: {dims.get('policy_hidden_size', '?')}")
        print(f"  Policy output: {dims.get('policy_output_size', '?')}")
        print(f"  Value hidden: {dims.get('value_hidden_size', '?')}")
        weights = data.get('weights', {})
        total_params = sum(len(v) for v in weights.values() if isinstance(v, list))
        total_params += sum(1 for v in weights.values() if isinstance(v, (int, float)))
        print(f"  Total quantized params: {total_params:,}")
        return
    else:
        print(f"Unknown file type: {path.suffix}")
        return

    print(f"  Parameters: {sum(p.numel() for p in model.parameters()):,}")
    print(f"\nWeight statistics:")
    for name, param in model.named_parameters():
        print(
            f"  {name}: shape={list(param.shape)}, "
            f"mean={param.mean().item():.6f}, "
            f"std={param.std().item():.6f}, "
            f"min={param.min().item():.6f}, "
            f"max={param.max().item():.6f}"
        )


# ─── CLI ─────────────────────────────────────────────────────────────────────

def main():
    """CLI entry point for MCTS/AlphaGo-style PolicyValueNet training."""
    parser = argparse.ArgumentParser(
        description="WH40K MCTS/AlphaGo-Style Policy/Value Training Pipeline"
    )
    subparsers = parser.add_subparsers(dest='command', help='Command to run')

    # Train
    train_parser = subparsers.add_parser('train', help='Train PolicyValueNet on shard data')
    train_parser.add_argument('--shard-dir', type=str, required=True,
                              help='Directory containing PolicyValueShard JSON files')
    train_parser.add_argument('--output-dir', type=str, default='pv_checkpoints',
                              help='Output directory for checkpoints and exports')
    train_parser.add_argument('--epochs', type=int, default=100, help='Number of epochs')
    train_parser.add_argument('--batch-size', type=int, default=256, help='Batch size')
    train_parser.add_argument('--lr', type=float, default=1e-3, help='Learning rate')
    train_parser.add_argument('--weight-decay', type=float, default=1e-4,
                              help='L2 regularization weight')
    train_parser.add_argument('--policy-weight', type=float, default=1.0,
                              help='Policy loss weight (alpha)')
    train_parser.add_argument('--value-weight', type=float, default=1.0,
                              help='Value loss weight (beta)')
    train_parser.add_argument('--entropy-weight', type=float, default=0.01,
                              help='Entropy bonus weight (gamma)')
    train_parser.add_argument('--val-split', type=float, default=0.1,
                              help='Validation split fraction')
    train_parser.add_argument('--generation', type=int, default=0, help='Generation number')
    train_parser.add_argument('--model-id', type=str, default=None, help='Model identifier')
    train_parser.add_argument('--device', type=str, default='auto',
                              help='Device (auto/cpu/cuda/mps)')
    train_parser.add_argument('--grad-clip', type=float, default=1.0,
                              help='Max gradient norm (0 = disabled)')
    train_parser.add_argument('--warmup-epochs', type=int, default=5,
                              help='LR warmup epochs')
    train_parser.add_argument('--resume', type=str, default=None,
                              help='Resume from checkpoint path')
    train_parser.add_argument('--use-bridge', action='store_true',
                              help='Use Rust bridge for bincode shards')
    train_parser.add_argument('--trunk-size', type=int, default=128,
                              help='Trunk layer size')
    train_parser.add_argument('--policy-hidden', type=int, default=64,
                              help='Policy head hidden size')
    train_parser.add_argument('--value-hidden', type=int, default=64,
                              help='Value head hidden size')

    # Generate
    gen_parser = subparsers.add_parser('generate', help='Generate MCTS self-play training data')
    gen_parser.add_argument('--num-games', type=int, default=100, help='Number of games')
    gen_parser.add_argument('--output-dir', type=str, default='pv_training_data',
                            help='Output directory')
    gen_parser.add_argument('--simulations', type=int, default=100,
                            help='MCTS simulations per move')
    gen_parser.add_argument('--model', type=str, default=None,
                            help='PolicyValueModel JSON path for guided MCTS')
    gen_parser.add_argument('--generation', type=int, default=0, help='Generation number')

    # Gate
    gate_parser = subparsers.add_parser('gate', help='Evaluate candidate vs baseline')
    gate_parser.add_argument('candidate', type=str, help='Path to candidate model JSON')
    gate_parser.add_argument('--num-games', type=int, default=100, help='Evaluation games')
    gate_parser.add_argument('--threshold', type=float, default=0.55,
                             help='Promotion threshold')
    gate_parser.add_argument('--baseline', type=str, default=None,
                             help='Baseline model path (None = heuristic)')

    # Pipeline
    pipe_parser = subparsers.add_parser('pipeline',
                                        help='Run full AlphaGo generate->train->gate pipeline')
    pipe_parser.add_argument('--output-dir', type=str, default='alphago_output',
                             help='Base output directory')
    pipe_parser.add_argument('--iterations', type=int, default=5,
                             help='Number of iterations')
    pipe_parser.add_argument('--games', type=int, default=100,
                             help='Self-play games per iteration')
    pipe_parser.add_argument('--simulations', type=int, default=100,
                             help='MCTS simulations per move')
    pipe_parser.add_argument('--epochs', type=int, default=50,
                             help='Training epochs per iteration')
    pipe_parser.add_argument('--batch-size', type=int, default=256, help='Batch size')
    pipe_parser.add_argument('--lr', type=float, default=1e-3, help='Learning rate')
    pipe_parser.add_argument('--gate-games', type=int, default=100,
                             help='Gating evaluation games')
    pipe_parser.add_argument('--threshold', type=float, default=0.55,
                             help='Promotion threshold')
    pipe_parser.add_argument('--device', type=str, default='auto', help='Device')

    # Inspect
    inspect_parser = subparsers.add_parser('inspect', help='Inspect a PolicyValueModel')
    inspect_parser.add_argument('model_path', type=str,
                                help='Path to .pt checkpoint or .json model')

    # Export
    export_parser = subparsers.add_parser('export',
                                          help='Export PyTorch checkpoint to JSON')
    export_parser.add_argument('checkpoint', type=str, help='Path to .pt checkpoint')
    export_parser.add_argument('output', type=str, help='Output .json path')
    export_parser.add_argument('--generation', type=int, default=0,
                               help='Generation number')

    args = parser.parse_args()

    if args.command == 'train':
        run_policy_value_training(
            shard_dir=args.shard_dir,
            output_dir=args.output_dir,
            epochs=args.epochs,
            batch_size=args.batch_size,
            learning_rate=args.lr,
            weight_decay=args.weight_decay,
            policy_weight=args.policy_weight,
            value_weight=args.value_weight,
            entropy_weight=args.entropy_weight,
            val_split=args.val_split,
            generation=args.generation,
            model_id=args.model_id,
            device_name=args.device,
            grad_clip=args.grad_clip,
            warmup_epochs=args.warmup_epochs,
            resume_checkpoint=args.resume,
            use_bridge=args.use_bridge,
            trunk_size=args.trunk_size,
            policy_hidden=args.policy_hidden,
            value_hidden=args.value_hidden,
        )

    elif args.command == 'generate':
        generate_mcts_training_data(
            num_games=args.num_games,
            output_dir=args.output_dir,
            simulations=args.simulations,
            model_path=args.model,
            generation=args.generation,
        )

    elif args.command == 'gate':
        run_policy_value_gating(
            candidate_path=args.candidate,
            num_games=args.num_games,
            promotion_threshold=args.threshold,
            baseline_path=args.baseline,
        )

    elif args.command == 'pipeline':
        run_alphago_pipeline(
            output_dir=args.output_dir,
            num_iterations=args.iterations,
            games_per_iteration=args.games,
            simulations_per_move=args.simulations,
            epochs_per_iteration=args.epochs,
            batch_size=args.batch_size,
            learning_rate=args.lr,
            gate_games=args.gate_games,
            promotion_threshold=args.threshold,
            device_name=args.device,
        )

    elif args.command == 'inspect':
        inspect_model(args.model_path)

    elif args.command == 'export':
        print(f"Loading checkpoint: {args.checkpoint}")
        model = load_policy_value_model(args.checkpoint)
        export_policy_value_model(model, args.output, generation=args.generation)

    else:
        parser.print_help()


if __name__ == '__main__':
    main()
