import * as PIXI from 'pixi.js';
import type { GameView, DecisionSurfaceView } from '@/types/game';
import { PX_PER_INCH, CHARGE_LINE_COLOR, CHARGE_LINE_ALPHA } from './constants';

export class ChargePreview {
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

    if (!selectedUnitId || !decisionSurface || gameState.phase !== 'Charge') return;

    const unit = gameState.units.find((u) => u.id === selectedUnitId);
    if (!unit?.position) return;

    // Show charge range (12" max charge distance)
    const maxChargeInches = 12;
    const g = new PIXI.Graphics();
    g.lineStyle(1, CHARGE_LINE_COLOR, 0.3);
    g.beginFill(CHARGE_LINE_COLOR, 0.05);
    g.drawCircle(
      unit.position.x * PX_PER_INCH,
      unit.position.y * PX_PER_INCH,
      maxChargeInches * PX_PER_INCH,
    );
    g.endFill();
    this.container.addChild(g);

    // Show lines to potential charge targets
    const chargeActions = decisionSurface.actions.filter(
      (a) => a.unit_id === selectedUnitId && a.command_type === 'Charge' && a.target_id != null,
    );

    for (const action of chargeActions) {
      const target = gameState.units.find((u) => u.id === action.target_id);
      if (!target?.position) continue;

      const line = new PIXI.Graphics();
      line.lineStyle(2, CHARGE_LINE_COLOR, CHARGE_LINE_ALPHA);

      // Dashed line
      const x1 = unit.position.x * PX_PER_INCH;
      const y1 = unit.position.y * PX_PER_INCH;
      const x2 = target.position.x * PX_PER_INCH;
      const y2 = target.position.y * PX_PER_INCH;

      const dx = x2 - x1;
      const dy = y2 - y1;
      const dist = Math.sqrt(dx * dx + dy * dy);
      const nx = dx / dist;
      const ny = dy / dist;

      let drawn = 0;
      let drawing = true;
      line.moveTo(x1, y1);

      while (drawn < dist) {
        const segLen = drawing ? 8 : 5;
        const nextDrawn = Math.min(drawn + segLen, dist);
        const ex = x1 + nx * nextDrawn;
        const ey = y1 + ny * nextDrawn;

        if (drawing) line.lineTo(ex, ey);
        else line.moveTo(ex, ey);

        drawn = nextDrawn;
        drawing = !drawing;
      }

      this.container.addChild(line);
    }
  }
}
