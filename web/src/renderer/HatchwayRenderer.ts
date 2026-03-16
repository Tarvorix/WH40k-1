import * as PIXI from 'pixi.js';
import { PX_PER_INCH } from './constants';
import type { GameView } from '@/types/game';

// Hatchway state colors
const STATE_COLORS: Record<string, number> = {
  Open: 0x50c878,        // Green
  Closed: 0xe8a317,      // Yellow/amber
  Locked: 0xcc3333,      // Red
  OneWayOpened: 0x5599ee, // Blue
};

const HATCHWAY_WIDTH_PX = 4;
const HATCHWAY_ALPHA = 0.9;
const LABEL_FONT_SIZE = 8;

export class HatchwayRenderer {
  private container: PIXI.Container;
  private graphics: PIXI.Graphics;
  private labels: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
    this.graphics = new PIXI.Graphics();
    this.labels = new PIXI.Container();
    this.container.addChild(this.graphics);
    this.container.addChild(this.labels);
  }

  update(board: GameView['board']): void {
    this.graphics.clear();

    // Remove old labels
    while (this.labels.children.length > 0) {
      this.labels.removeChildAt(0);
    }

    if (!board.hatchways) return;

    for (const hatch of board.hatchways) {
      const color = STATE_COLORS[hatch.state] ?? 0xffffff;
      const cx = hatch.position.x * PX_PER_INCH;
      const cy = hatch.position.y * PX_PER_INCH;
      const halfW = (hatch.width * PX_PER_INCH) / 2;

      // Draw the hatchway as a thick colored line segment
      this.graphics.lineStyle(HATCHWAY_WIDTH_PX, color, HATCHWAY_ALPHA);

      if (hatch.orientation === 'Horizontal') {
        // Horizontal hatchway: extends left-right
        this.graphics.moveTo(cx - halfW, cy);
        this.graphics.lineTo(cx + halfW, cy);
      } else {
        // Vertical hatchway: extends up-down
        this.graphics.moveTo(cx, cy - halfW);
        this.graphics.lineTo(cx, cy + halfW);
      }

      // Draw a small state indicator circle at the center
      this.graphics.lineStyle(0);
      this.graphics.beginFill(color, 1);
      this.graphics.drawCircle(cx, cy, 4);
      this.graphics.endFill();

      // State label
      const label = new PIXI.Text(hatch.state.charAt(0), {
        fontSize: LABEL_FONT_SIZE,
        fill: 0xffffff,
        fontWeight: 'bold',
      });
      label.anchor.set(0.5, 0.5);
      label.position.set(cx, cy);
      this.labels.addChild(label);
    }
  }
}
