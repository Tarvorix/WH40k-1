import * as PIXI from 'pixi.js';
import {
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
  private boardWidthInches: number = BOARD_WIDTH_INCHES;
  private boardHeightInches: number = BOARD_HEIGHT_INCHES;

  constructor(container: PIXI.Container) {
    this.container = container;
    this.graphics = new PIXI.Graphics();
    this.container.addChild(this.graphics);
  }

  /** Get the current board pixel width. */
  get canvasWidth(): number {
    return this.boardWidthInches * PX_PER_INCH;
  }

  /** Get the current board pixel height. */
  get canvasHeight(): number {
    return this.boardHeightInches * PX_PER_INCH;
  }

  /** Set board dimensions from game state (in inches). */
  setBoardDimensions(widthInches: number, heightInches: number): void {
    this.boardWidthInches = widthInches;
    this.boardHeightInches = heightInches;
  }

  draw(): void {
    const g = this.graphics;
    g.clear();

    const cw = this.canvasWidth;
    const ch = this.canvasHeight;

    // Background fill
    g.beginFill(BOARD_BG_COLOR);
    g.drawRect(0, 0, cw, ch);
    g.endFill();

    // Grid lines (1" spacing)
    g.lineStyle(GRID_LINE_WIDTH, GRID_LINE_COLOR, GRID_LINE_ALPHA);

    // Vertical lines
    for (let x = 0; x <= this.boardWidthInches; x++) {
      const px = x * PX_PER_INCH;
      g.moveTo(px, 0);
      g.lineTo(px, ch);
    }

    // Horizontal lines
    for (let y = 0; y <= this.boardHeightInches; y++) {
      const py = y * PX_PER_INCH;
      g.moveTo(0, py);
      g.lineTo(cw, py);
    }

    // Board border
    g.lineStyle(BOARD_BORDER_WIDTH, BOARD_BORDER_COLOR, 1);
    g.drawRect(0, 0, cw, ch);
  }
}
