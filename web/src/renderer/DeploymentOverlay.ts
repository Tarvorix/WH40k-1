import * as PIXI from 'pixi.js';
import type { BoardView, DeploymentZoneView } from '@/types/game';
import { PX_PER_INCH, DEPLOYMENT_ZONE_ALPHA, DEPLOYMENT_ZONE_BORDER_ALPHA, DEPLOYMENT_ZONE_BORDER_WIDTH } from './constants';
import { getFactionPrimaryColor } from '@/utils/colors';

export class DeploymentOverlay {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(board: BoardView): void {
    this.container.removeChildren();

    for (const zone of board.deployment_zones) {
      this.drawZone(zone);
    }
  }

  private drawZone(zone: DeploymentZoneView): void {
    if (zone.vertices.length < 3) return;

    const g = new PIXI.Graphics();
    const color = getFactionPrimaryColor(zone.player);

    // Fill polygon
    g.beginFill(color, DEPLOYMENT_ZONE_ALPHA);
    const first = zone.vertices[0];
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();
    g.endFill();

    // Border
    g.lineStyle(DEPLOYMENT_ZONE_BORDER_WIDTH, color, DEPLOYMENT_ZONE_BORDER_ALPHA);
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();

    this.container.addChild(g);
  }
}
