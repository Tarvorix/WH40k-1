"""
WH40K Training Shard DataLoader

Loads training data from shards produced by the Rust self-play runner
and serves them as PyTorch tensors for NNUE training.

Shard format (bincode):
    - ShardHeader: version, engine_version, schema_version, sample_count, etc.
    - Vec<TrainingSample>: sparse_features, legal_mask, chosen_action, score, outcome, etc.

Source: implementation_v3.md Phase 8.4, selfplay crate Phase 8.1
"""

from pathlib import Path

import torch
from torch.utils.data import Dataset, DataLoader, ConcatDataset

# Try to import the Rust bridge; fall back to JSON shard loading if unavailable
try:
    import wh40k_trainer_bridge as wtb
    HAS_BRIDGE = True
except ImportError:
    HAS_BRIDGE = False

from .model import TOTAL_FEATURES


class ShardDataset(Dataset):
    """PyTorch Dataset backed by a single training shard.

    Each sample provides:
        - features: dense f32 tensor of shape (TOTAL_FEATURES,)
        - outcome: f32 scalar in {-1.0, 0.0, 1.0}
        - search_score: i32 scalar (centipawn-like score from search)
        - legal_mask: bool tensor of shape (ACTION_VOCAB_SIZE,)
        - chosen_action: u32 scalar vocab index

    The primary training target is 'outcome' (game result from perspective).
    The 'search_score' can be used as an auxiliary training signal.
    """

    def __init__(self, samples: list):
        """Create dataset from a list of training samples.

        Args:
            samples: List of TrainingSample objects (from bridge) or dicts.
        """
        self.features_list = []
        self.outcomes = []
        self.search_scores = []

        for s in samples:
            # Convert sparse features to dense
            dense = torch.zeros(TOTAL_FEATURES, dtype=torch.float32)
            sparse = s.sparse_features if hasattr(s, 'sparse_features') else s['sparse_features']
            for idx, val in sparse:
                if idx < TOTAL_FEATURES:
                    dense[idx] = float(val)

            self.features_list.append(dense)

            outcome = s.outcome if hasattr(s, 'outcome') else s['outcome']
            self.outcomes.append(outcome)

            score = s.search_score if hasattr(s, 'search_score') else s['search_score']
            self.search_scores.append(score)

    def __len__(self) -> int:
        return len(self.features_list)

    def __getitem__(self, idx: int) -> dict:
        return {
            'features': self.features_list[idx],
            'outcome': torch.tensor(self.outcomes[idx], dtype=torch.float32),
            'search_score': torch.tensor(self.search_scores[idx], dtype=torch.float32),
        }


def load_shard_dataset(shard_path: str | Path) -> ShardDataset:
    """Load a single shard file into a ShardDataset.

    Args:
        shard_path: Path to a .bin shard file.

    Returns:
        ShardDataset ready for use with DataLoader.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    shard = wtb.load_shard(str(shard_path))
    return ShardDataset(shard.samples())


def load_all_shards_dataset(shard_dir: str | Path) -> ConcatDataset:
    """Load all shard files from a directory into a single ConcatDataset.

    Args:
        shard_dir: Directory containing .bin shard files.

    Returns:
        ConcatDataset combining all shards.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    shards = wtb.load_all_shards(str(shard_dir))
    datasets = [ShardDataset(shard.samples()) for shard in shards]

    if not datasets:
        raise ValueError(f"No shard files found in {shard_dir}")

    return ConcatDataset(datasets)


def create_dataloader(
    shard_dir: str | Path,
    batch_size: int = 256,
    shuffle: bool = True,
    num_workers: int = 0,
    pin_memory: bool = True,
) -> DataLoader:
    """Create a DataLoader from all shards in a directory.

    Args:
        shard_dir: Directory containing training shard .bin files.
        batch_size: Batch size for training.
        shuffle: Whether to shuffle samples.
        num_workers: Number of data loading workers.
        pin_memory: Whether to pin memory for GPU transfer.

    Returns:
        DataLoader yielding batches of {features, outcome, search_score}.
    """
    dataset = load_all_shards_dataset(shard_dir)

    return DataLoader(
        dataset,
        batch_size=batch_size,
        shuffle=shuffle,
        num_workers=num_workers,
        pin_memory=pin_memory,
        drop_last=True,
    )


def generate_training_data(
    num_games: int = 100,
    output_dir: str = "training_data",
) -> int:
    """Generate training data by running self-play games.

    Args:
        num_games: Number of self-play games to run.
        output_dir: Directory to write training shards to.

    Returns:
        Total number of training samples generated.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    Path(output_dir).mkdir(parents=True, exist_ok=True)

    report = wtb.run_selfplay_batch(num_games, output_dir)
    print(f"Self-play complete:")
    print(f"  Games: {report['total_games']}")
    print(f"  P1 wins: {report['player1_wins']}, P2 wins: {report['player2_wins']}, Draws: {report['draws']}")
    print(f"  Total samples: {report['total_samples']}")
    print(f"  Throughput: {report['games_per_hour']:.0f} games/hour")

    return report['total_samples']
