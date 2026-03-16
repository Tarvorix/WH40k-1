import * as PIXI from 'pixi.js';
import { PX_PER_INCH } from './constants';
import type { GameView } from '@/types/game';

const WALL_COLOR = 0x8888aa;
const WALL_WIDTH = 3;
const WALL_ALPHA = 0.9;
const COMPARTMENT_FILL_ALPHA = 0.04;
const COMPARTMENT_BORDER_COLOR = 0x555577;
const COMPARTMENT_BORDER_WIDTH = 1;
const COMPARTMENT_BORDER_ALPHA = 0.3;

export class WallRenderer {
  private container: PIXI.Container;
  private wallGraphics: PIXI.Graphics;
  private compartmentGraphics: PIXI.Graphics;

  constructor(container: PIXI.Container) {
    this.container = container;
    this.compartmentGraphics = new PIXI.Graphics();
    this.wallGraphics = new PIXI.Graphics();
    this.container.addChild(this.compartmentGraphics);
    this.container.addChild(this.wallGraphics);
  }

  update(board: GameView['board']): void {
    this.wallGraphics.clear();
    this.compartmentGraphics.clear();

    // Draw compartment fills
    if (board.compartments) {
      for (const comp of board.compartments) {
        if (comp.vertices.length < 3) continue;

        // Fill
        this.compartmentGraphics.beginFill(0x222244, COMPARTMENT_FILL_ALPHA);
        this.compartmentGraphics.lineStyle(COMPARTMENT_BORDER_WIDTH, COMPARTMENT_BORDER_COLOR, COMPARTMENT_BORDER_ALPHA);

        const first = comp.vertices[0];
        this.compartmentGraphics.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
        for (let i = 1; i < comp.vertices.length; i++) {
          const v = comp.vertices[i];
          this.compartmentGraphics.lineTo(v.x * PX_PER_INCH, v.y * PX_PER_INCH);
        }
        this.compartmentGraphics.closePath();
        this.compartmentGraphics.endFill();
      }
    }

    // Draw wall segments
    if (board.walls) {
      this.wallGraphics.lineStyle(WALL_WIDTH, WALL_COLOR, WALL_ALPHA);

      for (const wall of board.walls) {
        this.wallGraphics.moveTo(wall.start.x * PX_PER_INCH, wall.start.y * PX_PER_INCH);
        this.wallGraphics.lineTo(wall.end.x * PX_PER_INCH, wall.end.y * PX_PER_INCH);
      }
    }
  }
}
