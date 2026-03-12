import * as PIXI from 'pixi.js';
import type { GameView, DecisionSurfaceView } from '@/types/game';
import {
  PX_PER_INCH,
  ATTACK_LINE_COLOR,
  ATTACK_LINE_ALPHA,
  ATTACK_LINE_WIDTH,
  CHARGE_LINE_COLOR,
  CHARGE_LINE_ALPHA,
} from './constants';

export class AttackVisualization {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(
    gameState: GameView,
    decisionSurface: DecisionSurfaceView | null,
    selectedUnitId: number | null,
    targetUnitId: number | null,
  ): void {
    this.container.removeChildren();

    if (!selectedUnitId || !targetUnitId) return;

    const attacker = gameState.units.find((u) => u.id === selectedUnitId);
    const target = gameState.units.find((u) => u.id === targetUnitId);

    if (!attacker?.position || !target?.position) return;

    const x1 = attacker.position.x * PX_PER_INCH;
    const y1 = attacker.position.y * PX_PER_INCH;
    const x2 = target.position.x * PX_PER_INCH;
    const y2 = target.position.y * PX_PER_INCH;

    if (gameState.phase === 'Shooting') {
      this.drawShootingArrow(x1, y1, x2, y2);
    } else if (gameState.phase === 'Charge') {
      this.drawChargeLine(x1, y1, x2, y2);
    } else if (gameState.phase === 'Fight') {
      this.drawMeleeLine(x1, y1, x2, y2);
    }
  }

  private drawShootingArrow(x1: number, y1: number, x2: number, y2: number): void {
    const g = new PIXI.Graphics();
    g.lineStyle(ATTACK_LINE_WIDTH, ATTACK_LINE_COLOR, ATTACK_LINE_ALPHA);
    g.moveTo(x1, y1);
    g.lineTo(x2, y2);

    // Arrowhead
    const angle = Math.atan2(y2 - y1, x2 - x1);
    const arrowSize = 8;
    g.moveTo(x2, y2);
    g.lineTo(
      x2 - arrowSize * Math.cos(angle - Math.PI / 6),
      y2 - arrowSize * Math.sin(angle - Math.PI / 6),
    );
    g.moveTo(x2, y2);
    g.lineTo(
      x2 - arrowSize * Math.cos(angle + Math.PI / 6),
      y2 - arrowSize * Math.sin(angle + Math.PI / 6),
    );

    this.container.addChild(g);
  }

  private drawChargeLine(x1: number, y1: number, x2: number, y2: number): void {
    const g = new PIXI.Graphics();
    g.lineStyle(ATTACK_LINE_WIDTH + 1, CHARGE_LINE_COLOR, CHARGE_LINE_ALPHA);

    // Dashed line
    const dx = x2 - x1;
    const dy = y2 - y1;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const nx = dx / dist;
    const ny = dy / dist;
    const dashLen = 8;
    const gapLen = 5;

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

  private drawMeleeLine(x1: number, y1: number, x2: number, y2: number): void {
    const g = new PIXI.Graphics();
    g.lineStyle(ATTACK_LINE_WIDTH, 0xDC143C, 0.7);
    g.moveTo(x1, y1);
    g.lineTo(x2, y2);
    this.container.addChild(g);
  }
}
