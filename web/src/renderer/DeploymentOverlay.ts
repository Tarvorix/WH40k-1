import * as PIXI from 'pixi.js';
import type { BoardView, DeploymentZoneView } from '@/types/game';
import { PX_PER_INCH, DEPLOYMENT_ZONE_ALPHA, DEPLOYMENT_ZONE_BORDER_ALPHA, DEPLOYMENT_ZONE_BORDER_WIDTH } from './constants';
import { getFactionPrimaryColor } from '@/utils/colors';

export class DeploymentOverlay {
  private container: PIXI.Container;

  constructor(container: PIXI.Container) {
    this.container = container;
  }

  update(board: BoardView, phase?: string, decisionOwner?: number): void {
    this.container.removeChildren();

    const isDeployment = phase === 'PreBattle';

    for (const zone of board.deployment_zones) {
      // During deployment, brighten the active player's zone
      const isActiveZone = isDeployment && zone.player === decisionOwner;
      this.drawZone(zone, isActiveZone);
    }
  }

  private drawZone(zone: DeploymentZoneView, isActive: boolean): void {
    if (zone.vertices.length < 3) return;

    const g = new PIXI.Graphics();
    const color = getFactionPrimaryColor(zone.player);

    // Use brighter alpha for active deployment zone
    const fillAlpha = isActive ? 0.25 : DEPLOYMENT_ZONE_ALPHA;
    const borderAlpha = isActive ? 0.7 : DEPLOYMENT_ZONE_BORDER_ALPHA;
    const borderWidth = isActive ? 3 : DEPLOYMENT_ZONE_BORDER_WIDTH;

    // Fill polygon
    g.beginFill(color, fillAlpha);
    const first = zone.vertices[0];
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();
    g.endFill();

    // Border
    g.lineStyle(borderWidth, color, borderAlpha);
    g.moveTo(first.x * PX_PER_INCH, first.y * PX_PER_INCH);
    for (let i = 1; i < zone.vertices.length; i++) {
      g.lineTo(zone.vertices[i].x * PX_PER_INCH, zone.vertices[i].y * PX_PER_INCH);
    }
    g.closePath();

    // Make clickable during active deployment
    if (isActive) {
      g.eventMode = 'static';
      g.cursor = 'crosshair';
    }

    this.container.addChild(g);
  }
}
