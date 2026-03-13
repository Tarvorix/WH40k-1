import * as PIXI from 'pixi.js';
import type { GameView, DecisionSurfaceView, UnitView } from '@/types/game';
import { PX_PER_INCH, UNIT_CIRCLE_RADIUS, LONG_PRESS_MS, baseRadiusPx } from './constants';
import { CANVAS_WIDTH, CANVAS_HEIGHT } from './constants';
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
    onBoardClick: (boardX: number, boardY: number) => void,
  ): void {
    this.container.removeChildren();

    // Board-level click catcher — added FIRST so it's behind unit/position hit areas.
    // Clicks on units or action positions will be intercepted by their hit areas above;
    // only "empty board" clicks fall through to this rectangle.
    const boardHitArea = new PIXI.Graphics();
    boardHitArea.beginFill(0x000000, 0.001); // Nearly invisible
    boardHitArea.drawRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
    boardHitArea.endFill();
    boardHitArea.eventMode = 'static';
    boardHitArea.cursor = 'default';
    boardHitArea.on('pointerdown', (e: PIXI.FederatedPointerEvent) => {
      const local = e.getLocalPosition(this.container);
      const boardX = local.x / PX_PER_INCH;
      const boardY = local.y / PX_PER_INCH;
      onBoardClick(boardX, boardY);
    });
    this.container.addChild(boardHitArea);

    // Create invisible hit areas for each alive model — clicking any model
    // in a unit selects/targets that unit.
    for (const unit of gameState.units) {
      if (!unit.position || unit.status === 'Destroyed' || unit.status === 'Undeployed') continue;

      for (const model of unit.models) {
        if (!model.alive) continue;

        const radius = baseRadiusPx(model.base_size_mm) + 4;
        const hitArea = new PIXI.Graphics();
        hitArea.beginFill(0xFFFFFF, 0.001); // Nearly invisible
        hitArea.drawCircle(
          model.position.x * PX_PER_INCH,
          model.position.y * PX_PER_INCH,
          radius,
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
