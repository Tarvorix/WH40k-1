"""
WH40K NNUE Training Loop

Trains the NNUE evaluation network on self-play data.

Training pipeline:
    1. Generate training data via self-play (or load existing shards)
    2. Train NNUE on (state, outcome) pairs with MSE loss
    3. Export quantized weights to .nnue format
    4. Evaluate candidate model against baseline via gating
    5. Promote if win rate > 55%

Loss function:
    Primary: MSE between model output and game outcome {-1, 0, +1}
    Optional auxiliary: MSE between model output and search score (normalized)

Source: implementation_v3.md Phase 8.4, Section 14
"""

import argparse
import json
import time
from pathlib import Path

import torch
import torch.nn as nn
import torch.optim as optim

from .model import NnueModel, TOTAL_FEATURES, OUTPUT_SCALE
from .shard_loader import create_dataloader, generate_training_data
from .export_weights import export_to_nnue

# Try to import bridge for gating
try:
    import wh40k_trainer_bridge as wtb
    HAS_BRIDGE = True
except ImportError:
    HAS_BRIDGE = False


def train_epoch(
    model: NnueModel,
    dataloader: torch.utils.data.DataLoader,
    optimizer: torch.optim.Optimizer,
    device: torch.device,
    search_score_weight: float = 0.0,
) -> dict:
    """Train for one epoch.

    Args:
        model: NNUE model to train.
        dataloader: DataLoader yielding {features, outcome, search_score}.
        optimizer: Optimizer instance.
        device: Device to train on.
        search_score_weight: Weight for auxiliary search score loss (0.0 = disabled).

    Returns:
        Dict with epoch metrics: loss, outcome_loss, score_loss, num_batches.
    """
    model.train()
    total_loss = 0.0
    total_outcome_loss = 0.0
    total_score_loss = 0.0
    num_batches = 0

    mse = nn.MSELoss()

    for batch in dataloader:
        features = batch['features'].to(device)
        outcome = batch['outcome'].to(device).unsqueeze(1)  # (batch, 1)

        optimizer.zero_grad()

        pred = model(features)  # (batch, 1) in [-1, 1]

        # Primary loss: outcome prediction
        outcome_loss = mse(pred, outcome)
        loss = outcome_loss

        # Auxiliary loss: search score prediction
        if search_score_weight > 0:
            # Normalize search score to [-1, 1] range (100000 = win)
            search_score = batch['search_score'].to(device).unsqueeze(1)
            normalized_score = torch.tanh(search_score / 10000.0)
            score_loss = mse(pred, normalized_score)
            loss = loss + search_score_weight * score_loss
            total_score_loss += score_loss.item()

        loss.backward()
        optimizer.step()

        total_loss += loss.item()
        total_outcome_loss += outcome_loss.item()
        num_batches += 1

    return {
        'loss': total_loss / max(num_batches, 1),
        'outcome_loss': total_outcome_loss / max(num_batches, 1),
        'score_loss': total_score_loss / max(num_batches, 1),
        'num_batches': num_batches,
    }


def validate(
    model: NnueModel,
    dataloader: torch.utils.data.DataLoader,
    device: torch.device,
) -> dict:
    """Validate the model on held-out data.

    Args:
        model: NNUE model to validate.
        dataloader: Validation DataLoader.
        device: Device.

    Returns:
        Dict with validation metrics.
    """
    model.eval()
    total_loss = 0.0
    total_correct = 0
    total_samples = 0
    num_batches = 0

    mse = nn.MSELoss()

    with torch.no_grad():
        for batch in dataloader:
            features = batch['features'].to(device)
            outcome = batch['outcome'].to(device).unsqueeze(1)

            pred = model(features)
            loss = mse(pred, outcome)

            # Accuracy: does the sign of prediction match the sign of outcome?
            pred_sign = torch.sign(pred)
            outcome_sign = torch.sign(outcome)
            correct = (pred_sign == outcome_sign).sum().item()

            total_loss += loss.item()
            total_correct += correct
            total_samples += features.size(0)
            num_batches += 1

    return {
        'val_loss': total_loss / max(num_batches, 1),
        'accuracy': total_correct / max(total_samples, 1),
        'num_samples': total_samples,
    }


def run_training(
    shard_dir: str,
    output_dir: str = "checkpoints",
    epochs: int = 50,
    batch_size: int = 256,
    learning_rate: float = 1e-3,
    weight_decay: float = 1e-5,
    search_score_weight: float = 0.0,
    val_split: float = 0.1,
    checkpoint_every: int = 10,
    model_id: str | None = None,
    generation: int = 0,
    device_name: str = "auto",
) -> dict:
    """Run the complete NNUE training loop.

    Args:
        shard_dir: Directory containing training shard files.
        output_dir: Directory for checkpoints and exported models.
        epochs: Number of training epochs.
        batch_size: Training batch size.
        learning_rate: Initial learning rate.
        weight_decay: L2 regularization weight.
        search_score_weight: Weight for auxiliary search score loss.
        val_split: Fraction of data to use for validation.
        checkpoint_every: Save checkpoint every N epochs.
        model_id: Model identifier string (auto-generated if None).
        generation: Training generation number.
        device_name: Device to use ("auto", "cpu", "cuda", "mps").

    Returns:
        Dict with training results and final model path.
    """
    # Setup
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

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
        model_id = f"gen{generation}_{int(time.time())}"

    # Load data
    print(f"Loading training data from {shard_dir}...")
    full_loader = create_dataloader(shard_dir, batch_size=batch_size, shuffle=True)
    total_samples = len(full_loader.dataset)
    print(f"Loaded {total_samples:,} training samples")

    # Split into train/val
    val_size = int(total_samples * val_split)
    train_size = total_samples - val_size

    if val_size > 0:
        train_dataset, val_dataset = torch.utils.data.random_split(
            full_loader.dataset, [train_size, val_size]
        )
        train_loader = torch.utils.data.DataLoader(
            train_dataset, batch_size=batch_size, shuffle=True, drop_last=True,
        )
        val_loader = torch.utils.data.DataLoader(
            val_dataset, batch_size=batch_size, shuffle=False,
        )
    else:
        train_loader = full_loader
        val_loader = None

    # Create model
    model = NnueModel().to(device)
    print(f"Model: {model}")

    # Optimizer
    optimizer = optim.Adam(
        model.parameters(),
        lr=learning_rate,
        weight_decay=weight_decay,
    )

    # Learning rate scheduler: reduce on plateau
    scheduler = optim.lr_scheduler.ReduceLROnPlateau(
        optimizer, mode='min', factor=0.5, patience=5,
    )

    # Training loop
    best_val_loss = float('inf')
    training_log = []
    start_time = time.time()

    print(f"\n{'='*60}")
    print(f"  PERTURABO TRAINING — Gen {generation}")
    print(f"{'='*60}")
    print(f"  Batch size:     {batch_size}")
    print(f"  Learning rate:  {learning_rate}")
    print(f"  Train samples:  {train_size:,}")
    if val_size > 0:
        print(f"  Val samples:    {val_size:,}")
    print(f"  Epochs:         {epochs}")
    print(f"  Device:         {device}")
    print(f"{'='*60}\n")

    for epoch in range(1, epochs + 1):
        epoch_start = time.time()

        # Train
        train_metrics = train_epoch(
            model, train_loader, optimizer, device, search_score_weight,
        )

        # Validate
        if val_loader is not None:
            val_metrics = validate(model, val_loader, device)
            scheduler.step(val_metrics['val_loss'])

            if val_metrics['val_loss'] < best_val_loss:
                best_val_loss = val_metrics['val_loss']
                # Save best model
                best_path = output_path / f"{model_id}_best.pt"
                torch.save(model.state_dict(), best_path)
        else:
            val_metrics = {}
            scheduler.step(train_metrics['loss'])

        epoch_time = time.time() - epoch_start
        elapsed_total = time.time() - start_time
        eta_secs = (elapsed_total / epoch) * (epochs - epoch)
        pct = (epoch / epochs) * 100

        # Log
        log_entry = {
            'epoch': epoch,
            'train_loss': train_metrics['loss'],
            'outcome_loss': train_metrics['outcome_loss'],
            'epoch_time': epoch_time,
            'lr': optimizer.param_groups[0]['lr'],
        }
        if val_metrics:
            log_entry['val_loss'] = val_metrics['val_loss']
            log_entry['accuracy'] = val_metrics['accuracy']

        training_log.append(log_entry)

        # Print progress with percentage and ETA
        val_str = ""
        acc_str = ""
        if val_metrics:
            val_str = f"  val={val_metrics['val_loss']:.6f}"
            acc_str = f"  acc={val_metrics['accuracy']:.1%}"

        print(
            f"[{pct:5.1f}%] Epoch {epoch:3d}/{epochs}  "
            f"loss={train_metrics['loss']:.6f}"
            f"{val_str}{acc_str}  "
            f"lr={optimizer.param_groups[0]['lr']:.2e}  "
            f"{epoch_time:.1f}s  ETA: {eta_secs:.0f}s"
        )

        # Checkpoint
        if epoch % checkpoint_every == 0:
            ckpt_path = output_path / f"{model_id}_epoch{epoch}.pt"
            torch.save({
                'epoch': epoch,
                'model_state_dict': model.state_dict(),
                'optimizer_state_dict': optimizer.state_dict(),
                'train_loss': train_metrics['loss'],
                'training_log': training_log,
            }, ckpt_path)
            print(f"         Checkpoint saved: {ckpt_path}")

    total_time = time.time() - start_time
    print(f"\n{'='*60}")
    print(f"  TRAINING COMPLETE")
    print(f"{'='*60}")
    print(f"  Total time:         {total_time:.1f}s")
    print(f"  Final loss:         {train_metrics['loss']:.6f}")
    print(f"  Best val loss:      {best_val_loss:.6f}")
    if val_metrics:
        print(f"  Final accuracy:     {val_metrics['accuracy']:.1%}")
    print(f"{'='*60}")

    # Export final model to .nnue format
    nnue_path = output_path / f"{model_id}.nnue"
    export_to_nnue(
        model,
        str(nnue_path),
        model_id=model_id,
        generation=generation,
        description=f"Trained on {total_samples} samples for {epochs} epochs, final loss {train_metrics['loss']:.6f}",
    )
    print(f"Exported NNUE model: {nnue_path}")

    # Save training log
    log_path = output_path / f"{model_id}_training_log.json"
    with open(log_path, 'w') as f:
        json.dump({
            'model_id': model_id,
            'generation': generation,
            'total_samples': total_samples,
            'epochs': epochs,
            'final_loss': train_metrics['loss'],
            'best_val_loss': best_val_loss,
            'total_time_secs': total_time,
            'log': training_log,
        }, f, indent=2)

    return {
        'model_id': model_id,
        'nnue_path': str(nnue_path),
        'final_loss': train_metrics['loss'],
        'best_val_loss': best_val_loss,
        'total_time': total_time,
        'total_samples': total_samples,
    }


def run_gating(
    candidate_path: str,
    num_games: int = 100,
    promotion_threshold: float = 0.55,
) -> dict:
    """Evaluate a candidate model against the heuristic baseline.

    Args:
        candidate_path: Path to the candidate .nnue model file.
        num_games: Number of evaluation games.
        promotion_threshold: Win rate threshold for promotion.

    Returns:
        Dict with gating results.
    """
    if not HAS_BRIDGE:
        raise ImportError("wh40k_trainer_bridge not available")

    print(f"Evaluating candidate: {candidate_path}")
    print(f"  Games: {num_games}, Threshold: {promotion_threshold:.0%}")

    result = wtb.evaluate_candidate(
        candidate_path,
        num_games=num_games,
        promotion_threshold=promotion_threshold,
    )

    print(f"\nGating Results:")
    print(f"  Candidate wins: {result.candidate_wins}/{result.total_games}")
    print(f"  Baseline wins:  {result.baseline_wins}/{result.total_games}")
    print(f"  Draws:          {result.draws}")
    print(f"  Win rate:       {result.win_rate:.1%}")
    print(f"  Elo delta:      {result.elo_delta:+d}")
    print(f"  Confidence:     [{result.confidence_lower:+d}, {result.confidence_upper:+d}]")
    print(f"  PROMOTED:       {'YES' if result.promoted else 'NO'}")

    return {
        'candidate_wins': result.candidate_wins,
        'baseline_wins': result.baseline_wins,
        'draws': result.draws,
        'win_rate': result.win_rate,
        'promoted': result.promoted,
        'elo_delta': result.elo_delta,
    }


def main():
    """CLI entry point for NNUE training."""
    parser = argparse.ArgumentParser(description="WH40K NNUE Training Pipeline")
    subparsers = parser.add_subparsers(dest='command', help='Command to run')

    # Generate training data
    gen_parser = subparsers.add_parser('generate', help='Generate self-play training data')
    gen_parser.add_argument('--num-games', type=int, default=100, help='Number of games')
    gen_parser.add_argument('--output-dir', type=str, default='training_data', help='Output directory')

    # Train
    train_parser = subparsers.add_parser('train', help='Train NNUE model')
    train_parser.add_argument('--shard-dir', type=str, required=True, help='Training data directory')
    train_parser.add_argument('--output-dir', type=str, default='checkpoints', help='Output directory')
    train_parser.add_argument('--epochs', type=int, default=50, help='Number of epochs')
    train_parser.add_argument('--batch-size', type=int, default=256, help='Batch size')
    train_parser.add_argument('--lr', type=float, default=1e-3, help='Learning rate')
    train_parser.add_argument('--generation', type=int, default=0, help='Generation number')
    train_parser.add_argument('--model-id', type=str, default=None, help='Model identifier')
    train_parser.add_argument('--device', type=str, default='auto', help='Device (auto/cpu/cuda/mps)')
    train_parser.add_argument('--search-score-weight', type=float, default=0.0,
                              help='Weight for auxiliary search score loss')

    # Gate
    gate_parser = subparsers.add_parser('gate', help='Evaluate candidate vs baseline')
    gate_parser.add_argument('candidate', type=str, help='Path to candidate .nnue file')
    gate_parser.add_argument('--num-games', type=int, default=100, help='Evaluation games')
    gate_parser.add_argument('--threshold', type=float, default=0.55, help='Promotion threshold')

    # Perturabo vs Basic gating
    pvb_parser = subparsers.add_parser('perturabo-gate', help='Run Perturabo vs Basic tier gating (12 matchups)')
    pvb_parser.add_argument('--games-per-matchup', type=int, default=20, help='Games per matchup (default 20)')
    pvb_parser.add_argument('--nnue', type=str, default=None, help='Path to .nnue model (omit for heuristic baseline)')

    # Benchmark
    bench_parser = subparsers.add_parser('benchmark', help='Run throughput benchmark')
    bench_parser.add_argument('--num-games', type=int, default=10, help='Number of games')

    # Full pipeline
    pipeline_parser = subparsers.add_parser('pipeline', help='Run full generate -> train -> gate pipeline')
    pipeline_parser.add_argument('--num-games', type=int, default=100, help='Self-play games')
    pipeline_parser.add_argument('--epochs', type=int, default=50, help='Training epochs')
    pipeline_parser.add_argument('--gate-games', type=int, default=100, help='Gating games')
    pipeline_parser.add_argument('--generation', type=int, default=0, help='Generation number')
    pipeline_parser.add_argument('--output-dir', type=str, default='pipeline_output', help='Base output dir')

    args = parser.parse_args()

    if args.command == 'generate':
        generate_training_data(args.num_games, args.output_dir)

    elif args.command == 'train':
        run_training(
            shard_dir=args.shard_dir,
            output_dir=args.output_dir,
            epochs=args.epochs,
            batch_size=args.batch_size,
            learning_rate=args.lr,
            generation=args.generation,
            model_id=args.model_id,
            device_name=args.device,
            search_score_weight=args.search_score_weight,
        )

    elif args.command == 'gate':
        run_gating(
            candidate_path=args.candidate,
            num_games=args.num_games,
            promotion_threshold=args.threshold,
        )

    elif args.command == 'perturabo-gate':
        if not HAS_BRIDGE:
            raise ImportError("wh40k_trainer_bridge not available — run: cd engine/crates/trainer_bridge && maturin develop --release")
        results = wtb.perturabo_vs_basic(
            games_per_matchup=args.games_per_matchup,
            nnue_path=args.nnue,
        )

    elif args.command == 'benchmark':
        if not HAS_BRIDGE:
            raise ImportError("wh40k_trainer_bridge not available")
        result = wtb.benchmark(args.num_games)
        print(f"Benchmark results:")
        for k, v in result.items():
            print(f"  {k}: {v}")

    elif args.command == 'pipeline':
        base = Path(args.output_dir)
        data_dir = base / "shards"
        ckpt_dir = base / "checkpoints"
        pipeline_start = time.time()

        gen = args.generation

        # Step 1: Generate
        print("=" * 60)
        print(f"  PERTURABO PIPELINE — Gen {gen}")
        print(f"  Step 1/3: Self-Play Data Generation")
        print("=" * 60)
        print(f"  Games: {args.num_games}")
        print(f"  Output: {data_dir}")
        print()
        generate_training_data(args.num_games, str(data_dir))

        # Step 2: Train
        print("\n" + "=" * 60)
        print(f"  PERTURABO PIPELINE — Gen {gen}")
        print(f"  Step 2/3: NNUE Training")
        print("=" * 60)
        result = run_training(
            shard_dir=str(data_dir),
            output_dir=str(ckpt_dir),
            epochs=args.epochs,
            generation=gen,
        )

        # Step 3: Gate
        print("\n" + "=" * 60)
        print(f"  PERTURABO PIPELINE — Gen {gen}")
        print(f"  Step 3/3: Gating Evaluation")
        print("=" * 60)
        gate_result = run_gating(
            candidate_path=result['nnue_path'],
            num_games=args.gate_games,
        )

        # Summary
        pipeline_time = time.time() - pipeline_start
        print("\n" + "=" * 60)
        print(f"  PERTURABO PIPELINE COMPLETE — Gen {gen}")
        print("=" * 60)
        print(f"  Model ID:       {result['model_id']}")
        print(f"  NNUE path:      {result['nnue_path']}")
        print(f"  Samples:        {result['total_samples']:,}")
        print(f"  Final loss:     {result['final_loss']:.6f}")
        print(f"  Best val loss:  {result['best_val_loss']:.6f}")
        print(f"  Gating record:  {gate_result['candidate_wins']}W-{gate_result['baseline_wins']}L-{gate_result['draws']}D")
        print(f"  Win rate:       {gate_result['win_rate']:.1%}")
        print(f"  PROMOTED:       {'YES' if gate_result['promoted'] else 'NO'}")
        print(f"  Pipeline time:  {pipeline_time:.1f}s ({pipeline_time/60:.1f}m)")
        print("=" * 60)

    else:
        parser.print_help()


if __name__ == '__main__':
    main()
