import * as PIXI from 'pixi.js';
import type { BoardView, TerrainView } from '@/types/game';
import { PX_PER_INCH, TERRAIN_ALPHA, TERRAIN_BORDER_WIDTH } from './constants';
import { TERRAIN_COLORS } from '@/utils/colors';

export class TerrainRenderer {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(board: BoardView): void {
    this.container.removeChildren();

    for (const terrain of board.terrain) {
      this.drawTerrain(terrain);
    }
  }

  private drawTerrain(terrain: TerrainView): void {
    const g = new PIXI.Graphics();

    const color = terrain.impassable
      ? TERRAIN_COLORS.impassable
      : terrain.blocks_los
        ? TERRAIN_COLORS.blocking
        : terrain.provides_cover
          ? TERRAIN_COLORS.cover
          : TERRAIN_COLORS.default;

    const x = terrain.rect.min_x * PX_PER_INCH;
    const y = terrain.rect.min_y * PX_PER_INCH;
    const w = (terrain.rect.max_x - terrain.rect.min_x) * PX_PER_INCH;
    const h = (terrain.rect.max_y - terrain.rect.min_y) * PX_PER_INCH;

    // Fill
    g.beginFill(color, TERRAIN_ALPHA);
    g.drawRect(x, y, w, h);
    g.endFill();

    // Border
    g.lineStyle(TERRAIN_BORDER_WIDTH, color, 0.8);
    g.drawRect(x, y, w, h);

    // Label
    const label = new PIXI.Text(terrain.name, {
      fontSize: 8,
      fill: 0xCCCCCC,
      fontFamily: 'Inter',
    });
    label.anchor.set(0.5);
    label.position.set(x + w / 2, y + h / 2);
    label.alpha = 0.7;

    this.container.addChild(g);
    this.container.addChild(label);
  }
}
