import * as PIXI from 'pixi.js';
import type { GameView, DecisionSurfaceView } from '@/types/game';
import {
  PX_PER_INCH,
  UNIT_CIRCLE_RADIUS,
  MOVE_GHOST_ALPHA,
  MOVE_LINE_COLOR,
  MOVE_LINE_ALPHA,
  MOVE_LINE_WIDTH,
  RANGE_CIRCLE_ALPHA,
} from './constants';
import { getFactionPrimaryColor } from '@/utils/colors';

export class MovementPreview {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(
    gameState: GameView,
    decisionSurface: DecisionSurfaceView | null,
    selectedUnitId: number | null,
  ): void {
    this.container.removeChildren();

    if (!decisionSurface || !selectedUnitId) return;

    const unit = gameState.units.find((u) => u.id === selectedUnitId);
    if (!unit?.position) return;

    const factionId = gameState.players[unit.owner]?.faction_id ?? unit.owner;
    const color = getFactionPrimaryColor(factionId);

    // Show movement range circle
    if (gameState.phase === 'Movement' && !unit.turn_flags.has_moved) {
      const rangeInches = unit.movement;
      const g = new PIXI.Graphics();
      g.lineStyle(1, MOVE_LINE_COLOR, RANGE_CIRCLE_ALPHA);
      g.beginFill(MOVE_LINE_COLOR, RANGE_CIRCLE_ALPHA * 0.3);
      g.drawCircle(
        unit.position.x * PX_PER_INCH,
        unit.position.y * PX_PER_INCH,
        rangeInches * PX_PER_INCH,
      );
      g.endFill();
      this.container.addChild(g);
    }

    // Show ghost tokens at possible destinations
    const moveActions = decisionSurface.actions.filter(
      (a) => a.unit_id === selectedUnitId && a.position != null &&
        (a.command_type === 'Movement' || a.command_type === 'Setup'),
    );

    for (const action of moveActions) {
      if (!action.position) continue;

      const destX = action.position.x * PX_PER_INCH;
      const destY = action.position.y * PX_PER_INCH;

      // Dashed line from current to destination
      const startX = unit.position.x * PX_PER_INCH;
      const startY = unit.position.y * PX_PER_INCH;
      this.drawDashedLine(startX, startY, destX, destY, MOVE_LINE_COLOR, MOVE_LINE_ALPHA);

      // Ghost token
      const ghost = new PIXI.Graphics();
      ghost.beginFill(color, MOVE_GHOST_ALPHA);
      ghost.lineStyle(1, color, MOVE_GHOST_ALPHA + 0.2);
      ghost.drawCircle(destX, destY, UNIT_CIRCLE_RADIUS);
      ghost.endFill();
      this.container.addChild(ghost);
    }
  }

  private drawDashedLine(
    x1: number, y1: number,
    x2: number, y2: number,
    color: number, alpha: number,
  ): void {
    const g = new PIXI.Graphics();
    g.lineStyle(MOVE_LINE_WIDTH, color, alpha);

    const dx = x2 - x1;
    const dy = y2 - y1;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const dashLen = 6;
    const gapLen = 4;
    const nx = dx / dist;
    const ny = dy / dist;

    let drawn = 0;
    let drawing = true;

    g.moveTo(x1, y1);
    while (drawn < dist) {
      const segLen = drawing ? dashLen : gapLen;
      const nextDrawn = Math.min(drawn + segLen, dist);
      const ex = x1 + nx * nextDrawn;
      const ey = y1 + ny * nextDrawn;

      if (drawing) {
        g.lineTo(ex, ey);
      } else {
        g.moveTo(ex, ey);
      }

      drawn = nextDrawn;
      drawing = !drawing;
    }

    this.container.addChild(g);
  }
}
