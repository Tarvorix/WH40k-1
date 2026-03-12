import * as PIXI from 'pixi.js';
import type { ObjectiveView } from '@/types/game';
import { PX_PER_INCH, OBJECTIVE_RADIUS, OBJECTIVE_LABEL_FONT_SIZE } from './constants';
import { OBJECTIVE_COLORS } from '@/utils/colors';

export class ObjectiveRenderer {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(objectives: ObjectiveView[]): void {
    this.container.removeChildren();

    for (const obj of objectives) {
      this.drawObjective(obj);
    }
  }

  private drawObjective(obj: ObjectiveView): void {
    const g = new PIXI.Graphics();
    const x = obj.position.x * PX_PER_INCH;
    const y = obj.position.y * PX_PER_INCH;

    let color: number;
    if (obj.control_status === 'Contested') {
      color = OBJECTIVE_COLORS.Contested;
    } else if (obj.controlling_player === 0) {
      color = OBJECTIVE_COLORS.player_0;
    } else if (obj.controlling_player === 1) {
      color = OBJECTIVE_COLORS.player_1;
    } else {
      color = OBJECTIVE_COLORS.Uncontrolled;
    }

    // Outer ring
    g.lineStyle(2, color, 0.8);
    g.drawCircle(x, y, OBJECTIVE_RADIUS);

    // Inner fill
    g.beginFill(color, 0.3);
    g.drawCircle(x, y, OBJECTIVE_RADIUS - 2);
    g.endFill();

    // Label
    const label = new PIXI.Text(obj.label, {
      fontSize: OBJECTIVE_LABEL_FONT_SIZE,
      fill: 0xFFFFFF,
      fontWeight: 'bold',
      fontFamily: 'Inter',
    });
    label.anchor.set(0.5);
    label.position.set(x, y);

    this.container.addChild(g);
    this.container.addChild(label);
  }
}
