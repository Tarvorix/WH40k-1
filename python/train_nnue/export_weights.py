"""
WH40K NNUE Weight Export

Exports trained PyTorch NNUE weights to the Rust .nnue model artifact format.
Handles the float-to-quantized conversion with proper scaling.

Quantization mapping:
    PyTorch float32 -> Rust quantized format
    Feature weights: f32 * FT_SCALE     -> i16
    Feature biases:  f32 * FT_SCALE     -> i32
    Hidden weights:  f32 * HIDDEN_SCALE -> i8
    Hidden biases:   f32 * HIDDEN_SCALE -> i32
    Output weights:  f32 * OUTPUT_SCALE -> i16
    Output bias:     f32 * OUTPUT_SCALE -> i32

Weight layout:
    PyTorch Linear: weight shape = [out_features, in_features]
    Rust NNUE: row-major [in_features * out_features]
    => Transpose PyTorch weights before flattening

Source: implementation_v3.md Phase 8.4, eval_nnue/src/lib.rs
"""

import argparse
from pathlib import Path

import torch

from .model import NnueModel

# Try to import bridge for .nnue file writing
try:
    import wh40k_trainer_bridge as wtb
    HAS_BRIDGE = True
except ImportError:
    HAS_BRIDGE = False


def export_to_nnue(
    model: NnueModel,
    output_path: str,
    model_id: str = "trained_model",
    generation: int = 0,
    description: str = "Python-trained NNUE",
) -> None:
    """Export a trained PyTorch NnueModel to Rust .nnue format.

    Args:
        model: Trained NnueModel instance.
        output_path: Output file path for the .nnue artifact.
        model_id: Unique model identifier.
        generation: Training generation number.
        description: Human-readable description.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available for .nnue export. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    # Quantize weights
    quantized = model.quantize_weights()

    # Create NnueWeights from quantized data
    weights = wtb.NnueWeights(
        feature_weights=quantized['feature_weights'],
        feature_biases=quantized['feature_biases'],
        hidden1_weights=quantized['hidden1_weights'],
        hidden1_biases=quantized['hidden1_biases'],
        hidden2_weights=quantized['hidden2_weights'],
        hidden2_biases=quantized['hidden2_biases'],
        output_weights=quantized['output_weights'],
        output_bias=quantized['output_bias'],
    )

    # Save to .nnue file
    weights.save(
        output_path,
        model_id=model_id,
        generation=generation,
        description=description,
    )

    print(f"Exported NNUE model to: {output_path}")
    print(f"  Model ID: {model_id}")
    print(f"  Generation: {generation}")


def import_from_nnue(nnue_path: str) -> NnueModel:
    """Import a Rust .nnue model artifact into a PyTorch NnueModel.

    Useful for:
    - Fine-tuning an existing model
    - Inspecting model weights
    - Evaluating a model in Python

    Args:
        nnue_path: Path to the .nnue artifact file.

    Returns:
        NnueModel with dequantized float weights.
    """
    if not HAS_BRIDGE:
        raise ImportError(
            "wh40k_trainer_bridge not available. "
            "Build with: cd engine/crates/trainer_bridge && maturin develop"
        )

    # Load quantized weights from .nnue file
    rust_weights = wtb.NnueWeights.load(nnue_path)

    # Convert to dict for NnueModel.from_quantized
    weights_dict = {
        'feature_weights': rust_weights.feature_weights,
        'feature_biases': rust_weights.feature_biases,
        'hidden1_weights': rust_weights.hidden1_weights,
        'hidden1_biases': rust_weights.hidden1_biases,
        'hidden2_weights': rust_weights.hidden2_weights,
        'hidden2_biases': rust_weights.hidden2_biases,
        'output_weights': rust_weights.output_weights,
        'output_bias': rust_weights.output_bias,
    }

    model = NnueModel.from_quantized(weights_dict)
    print(f"Imported NNUE model from: {nnue_path}")
    print(f"  Parameters: {model.parameter_count():,}")

    return model


def export_weights_json(model: NnueModel, output_path: str) -> None:
    """Export quantized weights as JSON (for debugging/inspection).

    Args:
        model: Trained NnueModel.
        output_path: Output JSON file path.
    """
    import json

    quantized = model.quantize_weights()

    # Add metadata
    export_data = {
        'format': 'wh40k_nnue_weights_json',
        'version': 1,
        'architecture': {
            'input_size': 1203,
            'accumulator_size': 128,
            'hidden1_size': 32,
            'hidden2_size': 32,
            'output_size': 1,
        },
        'weights': quantized,
    }

    with open(output_path, 'w') as f:
        json.dump(export_data, f)

    print(f"Exported weights JSON to: {output_path}")


def main():
    """CLI entry point for weight export/import."""
    parser = argparse.ArgumentParser(description="WH40K NNUE Weight Export/Import")
    subparsers = parser.add_subparsers(dest='command')

    # Export
    export_parser = subparsers.add_parser('export', help='Export PyTorch model to .nnue')
    export_parser.add_argument('checkpoint', type=str, help='Path to PyTorch checkpoint (.pt)')
    export_parser.add_argument('output', type=str, help='Output .nnue file path')
    export_parser.add_argument('--model-id', type=str, default='exported', help='Model ID')
    export_parser.add_argument('--generation', type=int, default=0, help='Generation number')

    # Import
    import_parser = subparsers.add_parser('import', help='Import .nnue to PyTorch')
    import_parser.add_argument('nnue_path', type=str, help='Path to .nnue file')
    import_parser.add_argument('--output', type=str, default=None, help='Output .pt checkpoint')

    # Inspect
    inspect_parser = subparsers.add_parser('inspect', help='Inspect .nnue model weights')
    inspect_parser.add_argument('nnue_path', type=str, help='Path to .nnue file')

    args = parser.parse_args()

    if args.command == 'export':
        model = NnueModel()
        checkpoint = torch.load(args.checkpoint, map_location='cpu', weights_only=True)
        if 'model_state_dict' in checkpoint:
            model.load_state_dict(checkpoint['model_state_dict'])
        else:
            model.load_state_dict(checkpoint)
        export_to_nnue(model, args.output, model_id=args.model_id, generation=args.generation)

    elif args.command == 'import':
        model = import_from_nnue(args.nnue_path)
        if args.output:
            torch.save(model.state_dict(), args.output)
            print(f"Saved PyTorch checkpoint: {args.output}")

    elif args.command == 'inspect':
        model = import_from_nnue(args.nnue_path)
        print(f"\nModel architecture: {model}")
        print(f"\nWeight statistics:")
        for name, param in model.named_parameters():
            print(f"  {name}: shape={list(param.shape)}, "
                  f"mean={param.mean().item():.6f}, "
                  f"std={param.std().item():.6f}, "
                  f"min={param.min().item():.6f}, "
                  f"max={param.max().item():.6f}")

    else:
        parser.print_help()


if __name__ == '__main__':
    main()
