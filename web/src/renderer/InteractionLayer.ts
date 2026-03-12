import * as PIXI from 'pixi.js';
import type { GameView, DecisionSurfaceView, UnitView } from '@/types/game';
import { PX_PER_INCH, UNIT_CIRCLE_RADIUS, LONG_PRESS_MS } from './constants';
import type { CameraController } from './CameraController';

export class InteractionLayer {
  private container: PIXI.Container;
  private longPressTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(
    gameState: GameView,
    decisionSurface: DecisionSurfaceView | null,
    camera: CameraController,
    onSelectUnit: (id: number | null) => void,
    onTargetUnit: (id: number | null) => void,
    onHoverUnit: (id: number | null) => void,
    onApplyAction: (index: number) => void,
  ): void {
    this.container.removeChildren();

    // Create invisible hit areas for each unit
    for (const unit of gameState.units) {
      if (!unit.position || unit.status === 'Destroyed' || unit.status === 'Undeployed') continue;

      const hitArea = new PIXI.Graphics();
      hitArea.beginFill(0xFFFFFF, 0.001); // Nearly invisible
      hitArea.drawCircle(
        unit.position.x * PX_PER_INCH,
        unit.position.y * PX_PER_INCH,
        UNIT_CIRCLE_RADIUS + 4,
      );
      hitArea.endFill();
      hitArea.eventMode = 'static';
      hitArea.cursor = 'pointer';

      hitArea.on('pointerdown', () => {
        // Check if this unit is a valid target in the decision surface
        if (decisionSurface) {
          const targetAction = decisionSurface.actions.find(
            (a) => a.target_id === unit.id,
          );
          if (targetAction) {
            onTargetUnit(unit.id);
            onApplyAction(targetAction.index);
            return;
          }
        }
        onSelectUnit(unit.id);
      });

      hitArea.on('pointerover', () => {
        onHoverUnit(unit.id);
      });

      hitArea.on('pointerout', () => {
        onHoverUnit(null);
      });

      this.container.addChild(hitArea);
    }

    // Create hit areas for actions with positions (deployment, movement destinations)
    if (decisionSurface) {
      for (const action of decisionSurface.actions) {
        if (!action.position) continue;

        const hitArea = new PIXI.Graphics();
        hitArea.beginFill(0x50C878, 0.15);
        hitArea.drawCircle(
          action.position.x * PX_PER_INCH,
          action.position.y * PX_PER_INCH,
          8,
        );
        hitArea.endFill();
        hitArea.eventMode = 'static';
        hitArea.cursor = 'pointer';

        hitArea.on('pointerdown', () => {
          onApplyAction(action.index);
        });

        this.container.addChild(hitArea);
      }
    }
  }
}
