import * as PIXI from 'pixi.js';
import {
  CANVAS_WIDTH,
  CANVAS_HEIGHT,
  PX_PER_INCH,
  BOARD_WIDTH_INCHES,
  BOARD_HEIGHT_INCHES,
  GRID_LINE_COLOR,
  GRID_LINE_ALPHA,
  GRID_LINE_WIDTH,
  BOARD_BG_COLOR,
  BOARD_BORDER_COLOR,
  BOARD_BORDER_WIDTH,
} from './constants';

export class BoardRenderer {
  private container: PIXI.Container;
  private graphics: PIXI.Graphics;

  constructor(container: PIXI.Container) {
    this.container = container;
    this.graphics = new PIXI.Graphics();
    this.container.addChild(this.graphics);
  }

  draw(): void {
    const g = this.graphics;
    g.clear();

    // Background fill
    g.beginFill(BOARD_BG_COLOR);
    g.drawRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
    g.endFill();

    // Grid lines (1" spacing)
    g.lineStyle(GRID_LINE_WIDTH, GRID_LINE_COLOR, GRID_LINE_ALPHA);

    // Vertical lines
    for (let x = 0; x <= BOARD_WIDTH_INCHES; x++) {
      const px = x * PX_PER_INCH;
      g.moveTo(px, 0);
      g.lineTo(px, CANVAS_HEIGHT);
    }

    // Horizontal lines
    for (let y = 0; y <= BOARD_HEIGHT_INCHES; y++) {
      const py = y * PX_PER_INCH;
      g.moveTo(0, py);
      g.lineTo(CANVAS_WIDTH, py);
    }

    // Board border
    g.lineStyle(BOARD_BORDER_WIDTH, BOARD_BORDER_COLOR, 1);
    g.drawRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
  }
}
